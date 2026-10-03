/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

//! Native terminal presentation of the shared plugin application kernel.
//! 统一插件应用内核的原生终端呈现器。
mod native;
mod paint;
mod run;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use futures_util::{FutureExt, StreamExt, stream::FuturesUnordered};
use maka_plugin_core::{
    composition::{Input, PresentationTarget},
    runtime::Mount,
};
use maka_plugins::package::Package;
use maka_ui::{
    Application, Error, NativeRegistry,
    actions::{ActionTask, TaskCompletion},
};
pub use native::registry;
pub use paint::{PaintedScene, Slots, sanitize};
use ratatui::{buffer::Buffer, layout::Rect};
pub use run::run;
use serde_json::{Value, json};
use std::{future::Future, pin::Pin};
use uuid::Uuid;

/// Trusted, immutable paint data. No resource, input subscription or OS owner.
/// 可信不可变绘制数据；真实资源生命周期不能藏在 recipe 中。
pub trait Widget: 'static {
    fn measure(&self, slots: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u16)>;
    fn paint(&self, slots: &mut Slots<'_>, area: Rect, buffer: &mut Buffer) -> maka_ui::Result<()>;
}
type Job = Pin<Box<dyn Future<Output = (Uuid, TaskCompletion)> + Send>>;
pub struct Presenter {
    application: Application<dyn Widget>,
    root: Mount,
    scene: Vec<paint::Target>,
    focus: Option<paint::Target>,
    jobs: FuturesUnordered<Job>,
    closing: bool,
    surface_cleanup: Option<Result<(), String>>,
}
impl Presenter {
    pub fn new(
        input: &Input,
        packages: Vec<Package>,
        scheduler: tokio::runtime::Handle,
        native: NativeRegistry<dyn Widget>,
        props: Value,
    ) -> maka_ui::Result<Self> {
        if input.environment.presentation != Some(PresentationTarget::Tui) {
            return Err(Error::Invalid("terminal deployment required".into()));
        }
        let application = Application::new(
            input,
            packages,
            scheduler,
            Default::default(),
            native,
            Default::default(),
        )?;
        Self::mount(application, props)
    }
    /// Construct on the actual terminal owner thread from its owned source.
    pub async fn start_from_store(
        store: std::sync::Arc<maka_event_log::EventLog>,
        source: maka_plugin_core::deployment::DeploymentRef,
        scheduler: tokio::runtime::Handle,
        native: NativeRegistry<dyn Widget>,
        props: Value,
        registrations: Vec<maka_ui::actions::RemoteService>,
    ) -> maka_ui::Result<Self> {
        let application = Application::start_from_store(
            store,
            source,
            PresentationTarget::Tui,
            scheduler,
            Default::default(),
            native,
            registrations,
        )
        .await?;
        Self::mount(application, props)
    }
    fn mount(mut application: Application<dyn Widget>, props: Value) -> maka_ui::Result<Self> {
        let root = match application.mount_launch(props) {
            Ok(root) => root,
            Err(error) => {
                // This presenter has not created a terminal or renderer resources.
                // Retain the foreground application until its actual modules close.
                let mut reported = std::collections::BTreeSet::new();
                loop {
                    match application.shutdown() {
                        Ok(()) => break,
                        Err(failure) => {
                            for mount in application.retiring_mounts() {
                                let _ = application.close(&mount, Ok(()));
                            }
                            let failure = sanitize(&failure.to_string());
                            if reported.insert(failure.clone()) {
                                eprintln!("{failure}");
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                return Err(error);
            }
        };
        Ok(Self {
            application,
            root,
            scene: Vec::new(),
            focus: None,
            jobs: FuturesUnordered::new(),
            closing: false,
            surface_cleanup: None,
        })
    }
    /// Resolve raw coordinates immediately against the last successfully flushed
    /// scene. Crossterm supplies no screen epoch; never re-resolve a captured key.
    /// 输入立即绑定到最后成功输出的场景和真实 mount/revision。
    pub fn input(&mut self, event: Event) -> maka_ui::Result<()> {
        if self.closing {
            return Ok(());
        }
        let target = match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Tab | KeyCode::BackTab => {
                    let reverse =
                        key.code == KeyCode::BackTab || key.modifiers.contains(KeyModifiers::SHIFT);
                    if !self.scene.is_empty() {
                        let index = self.focus.as_ref().and_then(|focus| {
                            self.scene.iter().position(|target| target.same(focus))
                        });
                        let n = self.scene.len();
                        let next = match (index, reverse) {
                            (Some(i), true) => (i + n - 1) % n,
                            (Some(i), false) => (i + 1) % n,
                            (None, true) => n - 1,
                            (None, false) => 0,
                        };
                        self.focus = Some(self.scene[next].clone());
                    }
                    None
                }
                KeyCode::Enter | KeyCode::Char(' ') if key.modifiers.is_empty() => {
                    self.focus.clone()
                }
                _ => None,
            },
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => self
                .scene
                .iter()
                .rev()
                .find(|t| t.area.contains((mouse.column, mouse.row).into()))
                .cloned(),
            _ => None,
        };
        if let Some(target) = target {
            self.focus = Some(target.clone());
            self.application.dispatch(
                &target.mount,
                target.revision,
                &target.key,
                json!({"kind":"click"}),
            )?;
        }
        Ok(())
    }
    pub fn pump(&mut self) -> maka_ui::Result<()> {
        // Each join observer stays with the actual owner, independently of UI.
        while let Some(Some((id, completion))) = self.jobs.next().now_or_never() {
            if let Some(task) = self.application.finish_task(id, completion)? {
                self.execute(task);
            }
        }
        if !self.closing && !self.application.is_retiring(&self.root) {
            self.application.capture_actions(&self.root, |_| {
                Err(Error::Invalid(
                    "terminal widget has no capture owner".into(),
                ))
            })?;
            while let Some(action) = self.application.start_task(&self.root)? {
                self.execute(action);
            }
            self.application.deliver_action_results(&self.root)?;
        }
        // Pure recipes have no native owners. Discard hit references before
        // acknowledging retired children; root additionally requires Guard.
        let retired = self.application.retiring_mounts();
        self.scene.retain(|target| !retired.contains(&target.mount));
        if self
            .focus
            .as_ref()
            .is_some_and(|target| retired.contains(&target.mount))
        {
            self.focus = None;
        }
        for mount in retired {
            if mount == self.root {
                if let Some(cleanup) = self.surface_cleanup.take() {
                    self.application.close(&mount, cleanup)?;
                }
            } else {
                self.application.close(&mount, Ok(()))?;
            }
        }
        Ok(())
    }
    pub fn retire(&mut self) -> maka_ui::Result<()> {
        self.closing = true;
        self.scene.clear();
        self.focus = None;
        self.application.retire(&self.root)
    }
    fn execute(&mut self, action: ActionTask) {
        let id = action.id;
        let task = self.application.scheduler().spawn(action.future);
        self.jobs.push(Box::pin(async move {
            let completion = task
                .await
                .unwrap_or_else(|error| TaskCompletion::Interrupted(error.to_string()));
            (id, completion)
        }));
    }
    /// Actual terminal restoration acknowledgement, not logical retirement.
    /// 接收实际终端恢复回执，不以逻辑退休冒充恢复完成。
    pub fn surface_closed(&mut self, result: Result<(), String>) -> maka_ui::Result<()> {
        self.retire()?;
        self.surface_cleanup = Some(result);
        self.pump()
    }
    pub fn can_shutdown(&self) -> bool {
        self.application.can_shutdown() && self.jobs.is_empty()
    }
    pub fn shutdown(&mut self) -> maka_ui::Result<()> {
        self.application.shutdown()
    }
}

#[cfg(test)]
#[path = "application/tests.rs"]
mod tests;
