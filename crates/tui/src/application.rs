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
mod editor;
mod file;
mod geometry;
mod input;
mod layout;
mod native;
mod paint;
mod reading;
mod resources;
mod run;
mod scene;
mod scroll;
mod selection;
mod source;
mod textarea;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use futures_util::{FutureExt, StreamExt, stream::FuturesUnordered};
pub use geometry::{CellRect, Placement};
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
use ratatui::buffer::Buffer;
#[cfg(test)]
use ratatui::layout::Rect;
pub use run::run;
pub use scene::{Direction as TerminalDirection, Layout as TerminalLayout, Node as TerminalScene};
use serde_json::{Value, json};
use std::{future::Future, pin::Pin};
use uuid::Uuid;

/// Trusted immutable recipe; resources belong only to its separately retained
/// `create` result. Measurement/paint stay pure; native input and command hooks
/// run against the original presenter-owned state, never a replacement frame.
/// 可信不可变 recipe；资源由独立原生 owner 持有，绘制不创建资源或执行命令。
pub trait Widget: std::any::Any {
    /// Immutable root constraints participate in the parent's original cell flow.
    fn layout(&self) -> TerminalLayout {
        TerminalLayout::default()
    }
    fn create(&self, _: NativeContext) -> Box<dyn std::any::Any> {
        Box::new(())
    }
    fn commit(&self, _: &mut dyn std::any::Any) -> maka_ui::Result<Option<NativeInput>> {
        Ok(None)
    }
    /// Poll original native observations independently of recipe revisions.
    fn observe(&self, _: &mut dyn std::any::Any) -> Option<NativeInput> {
        None
    }
    fn input(&self, _: &mut dyn std::any::Any, _: &str, _: &Event) -> maka_ui::Result<WidgetInput> {
        Ok(WidgetInput::Unhandled)
    }
    fn pointer_capture(&self, _: &dyn std::any::Any) -> bool {
        false
    }
    fn copy(&self, _: &dyn std::any::Any) -> maka_ui::Result<Option<String>> {
        Ok(None)
    }
    fn capture(
        &self,
        _: &dyn std::any::Any,
        _: &maka_ui::actions::CaptureTarget,
    ) -> maka_ui::Result<std::sync::Arc<dyn maka_plugins::call::ReadResource>> {
        Err(Error::Invalid("terminal widget capture unavailable".into()))
    }
    /// Called only after a successful native flush. The original resource queue
    /// retains an observation until the ordinary component callback accepts it.
    fn present(&self, _: &mut dyn std::any::Any, _: Box<dyn std::any::Any>) -> Option<NativeInput> {
        None
    }
    fn capture_continuation(
        &self,
        _: &dyn std::any::Any,
        _: &maka_ui::actions::CaptureTarget,
    ) -> maka_ui::Result<maka_ui::actions::ContinuationWitness> {
        Err(Error::Invalid(
            "terminal widget continuation unavailable".into(),
        ))
    }
    /// Observe only: no input dispatch or presentation mutation during admission.
    fn validate_continuation(
        &self,
        _: &dyn std::any::Any,
        _: &maka_ui::actions::CaptureTarget,
        _: &maka_ui::actions::ContinuationWitness,
    ) -> maka_ui::Result<bool> {
        Ok(false)
    }
    /// Successful presentation may suspend physical input and geometry while
    /// retaining the editor, capture and pending command observations.
    fn presentation(&self, _: &mut dyn std::any::Any, _: bool) {}
    fn deactivate(&self, _: &mut dyn std::any::Any) {}
    fn measure(&self, slots: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u32)>;
    /// Logical traversal includes fully clipped placements; writes and physical
    /// targets must honor Placement.clip. Only successful flush admits geometry.
    fn paint(
        &self,
        slots: &mut Slots<'_>,
        area: Placement,
        buffer: &mut Buffer,
    ) -> maka_ui::Result<()>;
}
/// Original renderer custody. Workers retain tickets until their real resources
/// settle; every owner shares its presenter's aggregate native budget.
#[derive(Clone)]
pub struct NativeContext {
    pub resources: std::sync::Arc<maka_plugins::call::Resources>,
    pub budget: maka_ui::budget::NativeBudget,
}

/// Owner-originated observation; never a guest-created native resource.
#[derive(Clone, PartialEq)]
pub struct NativeInput {
    pub key: String,
    pub value: Value,
}
pub enum WidgetInput {
    Unhandled,
    Consumed(Option<NativeInput>),
    /// An immediate business event; never replaces retryable native observations.
    Emitted(NativeInput),
}
type Job = Pin<Box<dyn Future<Output = (Uuid, TaskCompletion)> + Send>>;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Retirement {
    Exit,
    Replace,
}
enum SurfaceState {
    Live,
    Retiring {
        cause: Retirement,
        cleanup: Option<Result<(), String>>,
    },
    Closed {
        cause: Retirement,
    },
}
impl SurfaceState {
    fn is_live(&self) -> bool {
        matches!(self, Self::Live)
    }
    fn replacing(&self) -> bool {
        matches!(
            self,
            Self::Retiring {
                cause: Retirement::Replace,
                ..
            } | Self::Closed {
                cause: Retirement::Replace
            }
        )
    }
}

pub struct Presenter {
    application: Application<dyn Widget>,
    root: Mount,
    scene: Vec<paint::Target>,
    resources: resources::Resources,
    focus: Option<paint::Target>,
    drag: Option<paint::Target>,
    jobs: FuturesUnordered<Job>,
    surface: SurfaceState,
    reported: std::collections::BTreeSet<String>,
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
    /// Load immutable desired input without borrowing this terminal owner.
    pub fn load_committed_update(
        &self,
        source: maka_plugin_core::deployment::DeploymentRef,
    ) -> maka_ui::Result<
        impl Future<Output = maka_ui::Result<maka_ui::CommittedUpdate>> + Send + 'static + use<>,
    > {
        self.application.load_committed_update(source)
    }
    /// Acknowledge desired input only. Native retirement is synchronous; actual
    /// terminal restoration and replacement remain the surface owner's work.
    pub fn adopt_update(
        &mut self,
        update: maka_ui::CommittedUpdate,
        registrations: Vec<maka_ui::actions::RemoteService>,
    ) -> maka_ui::Result<()> {
        let prepared = self.application.prepare_update(update)?;
        self.adopt_prepared(prepared, registrations)
    }
    pub fn adopt_prepared(
        &mut self,
        update: maka_ui::PreparedUpdate,
        registrations: Vec<maka_ui::actions::RemoteService>,
    ) -> maka_ui::Result<()> {
        let was_live = self.surface.is_live() && !self.application.is_retiring(&self.root);
        self.application.adopt_prepared(update, registrations)?;
        if was_live && self.application.is_retiring(&self.root) {
            // Capture the install's effect now, not a later, unrelated realm fault.
            self.retire_surface(Retirement::Replace)?;
        }
        self.resources.deactivate(&self.application);
        Ok(())
    }
    pub fn deployment_status(&self) -> maka_ui::DeploymentStatus {
        self.application.deployment_status()
    }
    pub fn surface(&self) -> Mount {
        self.root.clone()
    }
    pub fn requires_surface_replacement(&self) -> bool {
        self.surface.replacing()
    }
    /// Resume the same application after actual old-surface cleanup. Pending
    /// original service work and its observers remain with this presenter.
    pub fn try_replace_surface(&mut self, props: Value) -> maka_ui::Result<bool> {
        if !matches!(
            self.surface,
            SurfaceState::Closed {
                cause: Retirement::Replace
            }
        ) {
            return Err(Error::Invalid(
                "original terminal cleanup not confirmed for replacement".into(),
            ));
        }
        self.application.advance_deployment()?;
        let status = self.application.deployment_status();
        if let Some(error) = status.error {
            return Err(Error::Invalid(error));
        }
        if status.converging {
            return Ok(false);
        }
        let root = self.application.mount_launch(props)?;
        self.root = root;
        self.surface = SurfaceState::Live;
        self.sync_native()?;
        Ok(true)
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
        let mut presenter = Self {
            application,
            root,
            scene: Vec::new(),
            resources: Default::default(),
            focus: None,
            drag: None,
            jobs: FuturesUnordered::new(),
            surface: SurfaceState::Live,
            reported: Default::default(),
        };
        if let Err(error) = presenter.sync_native() {
            // No terminal was created. Native initialization may still have
            // acquired resources: retain this actual owner until cleanup is
            // confirmed, including a failed native initialization fence.
            let mut reported = std::collections::BTreeSet::new();
            if let Err(failure) = presenter.surface_closed(&presenter.surface(), Ok(())) {
                reported.insert(sanitize(&failure.to_string()));
                eprintln!("{}", sanitize(&failure.to_string()));
            }
            loop {
                if let Err(failure) = presenter.pump() {
                    let failure = sanitize(&failure.to_string());
                    if reported.insert(failure.clone()) {
                        eprintln!("{failure}");
                    }
                }
                if presenter.can_shutdown() {
                    match presenter.shutdown() {
                        Ok(()) => break,
                        Err(failure) => {
                            let failure = sanitize(&failure.to_string());
                            if reported.insert(failure.clone()) {
                                eprintln!("{failure}");
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            return Err(error);
        }
        Ok(presenter)
    }
    /// Resolve raw coordinates immediately against the last successfully flushed
    /// scene. Crossterm supplies no screen epoch; never re-resolve a captured key.
    /// 输入立即绑定到最后成功输出的场景和真实 mount/revision。
    pub fn input(&mut self, event: Event) -> maka_ui::Result<()> {
        if !self.surface.is_live() || !self.resources.ready(&self.application, &self.root)? {
            return Ok(());
        }
        let target = match &event {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Tab | KeyCode::BackTab => {
                    let reverse =
                        key.code == KeyCode::BackTab || key.modifiers.contains(KeyModifiers::SHIFT);
                    let mut stops: Vec<_> = self
                        .scene
                        .iter()
                        .filter(|target| target.tab_index >= 0)
                        .collect();
                    stops.sort_by_key(|target| (target.tab_index == 0, target.tab_index));
                    if !stops.is_empty() {
                        let index = self
                            .focus
                            .as_ref()
                            .and_then(|focus| stops.iter().position(|target| target.same(focus)));
                        let n = stops.len();
                        let next = match (index, reverse) {
                            (Some(i), true) => (i + n - 1) % n,
                            (Some(i), false) => (i + 1) % n,
                            (None, true) => n - 1,
                            (None, false) => 0,
                        };
                        self.focus = Some(stops[next].clone());
                    }
                    None
                }
                KeyCode::Enter | KeyCode::Char(' ') if key.modifiers.is_empty() => {
                    self.focus.clone()
                }
                _ => self.focus.clone(),
            },
            Event::Paste(_) => self.focus.clone(),
            Event::Mouse(mouse)
                if matches!(
                    mouse.kind,
                    MouseEventKind::Down(MouseButton::Left)
                        | MouseEventKind::ScrollUp
                        | MouseEventKind::ScrollDown
                ) =>
            {
                self.scene
                    .iter()
                    .rev()
                    .find(|t| t.area.contains((mouse.column, mouse.row).into()))
                    .cloned()
            }
            Event::Mouse(mouse)
                if matches!(
                    mouse.kind,
                    MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)
                ) =>
            {
                self.drag.clone()
            }
            _ => None,
        };
        if let Some(target) = target {
            if self.application.is_retiring(&target.mount) {
                return Ok(());
            }
            let Some(native) = self.resources.input(&target, &event)? else {
                return Ok(());
            };
            let native = match native {
                WidgetInput::Emitted(emitted) => {
                    let revision = self.application.frame(&target.mount)?.revision;
                    self.application.dispatch(
                        &target.mount,
                        revision,
                        &emitted.key,
                        emitted.value,
                    )?;
                    self.focus = Some(target);
                    self.sync_native()?;
                    return Ok(());
                }
                other => other,
            };
            if let WidgetInput::Consumed(observed) = native {
                if self.resources.pointer_capture(&target)? {
                    self.drag = Some(target.clone());
                } else if self.drag.as_ref().is_some_and(|old| old.same(&target)) {
                    self.drag = None;
                }

                if let Some(observed) = observed {
                    self.resources.observe(&target.mount, observed);
                }
                if !matches!(event,Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown | MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)))
                {
                    self.focus = Some(target);
                }
                self.sync_native()?;
                return Ok(());
            }
            // Bubble unconsumed navigation through the original flushed scroll ancestors.
            for ancestor in target.ancestors.iter().rev() {
                if self.application.is_retiring(&ancestor.mount) {
                    continue;
                }
                if let Some(WidgetInput::Consumed(observed)) =
                    self.resources.input(ancestor, &event)?
                {
                    if let Some(observed) = observed {
                        self.resources.observe(&ancestor.mount, observed);
                    }
                    self.sync_native()?;
                    return Ok(());
                }
            }
            if target.kind != paint::TargetKind::Action {
                if matches!(event, Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left))
                {
                    self.focus = Some(target);
                }
                return Ok(());
            }
            if !matches!(event, Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left))
                && !matches!(event,Event::Key(key) if (key.code==KeyCode::Enter || key.code==KeyCode::Char(' ')) && key.modifiers.is_empty())
            {
                return Ok(());
            }
            let decision = self.application.dispatch(
                &target.mount,
                target.revision,
                &target.key,
                json!({"kind":"click"}),
            )?;
            if !decision.prevent_default {
                self.focus = Some(target);
            }
            self.sync_native()?;
        }
        Ok(())
    }
    /// Copy only the focused original live native scope through the bounded
    /// terminal writer. An OSC52 write is not an OS clipboard acknowledgement.
    pub fn copy(&mut self, writer: &mut impl std::io::Write) -> maka_ui::Result<bool> {
        let Some(target) = self.focus.clone() else {
            return Ok(false);
        };
        if self.application.is_retiring(&target.mount) {
            return Ok(false);
        }
        let Some(text) = self.resources.copy(&target)? else {
            return Ok(false);
        };
        crate::terminal::copy(writer, &text).map_err(|error| Error::Invalid(error.to_string()))?;
        Ok(true)
    }
    fn report(&mut self, error: &Error) {
        let diagnostic: String = sanitize(&error.to_string()).chars().take(2048).collect();
        if self.reported.len() < 64 && self.reported.insert(diagnostic.clone()) {
            eprintln!("{diagnostic}");
        }
    }
    pub fn pump(&mut self) -> maka_ui::Result<()> {
        let advancement = if self.surface.is_live() && !self.application.is_retiring(&self.root) {
            self.application.advance_deployment()
        } else {
            Ok(())
        };
        self.resources.deactivate(&self.application);
        if let Err(error) = advancement {
            self.report(&error);
        }
        self.sync_native()?;
        // Each join observer stays with the actual owner, independently of UI.
        while let Some(Some((id, completion))) = self.jobs.next().now_or_never() {
            if let Some(task) = self.application.finish_task(id, completion)? {
                self.execute(task);
            }
        }
        if self.surface.is_live() && !self.application.is_retiring(&self.root) {
            while let Some(action) = self.application.start_task(&self.root)? {
                self.execute(action);
            }
            let resources = &mut self.resources;
            self.application
                .deliver_action_results(&self.root, |target, recipe, witness| {
                    resources.validate_continuation(target, recipe, witness)
                })?;
        }
        self.sync_native()?;
        // Discard native hit references before
        // acknowledging retired children; root additionally requires Guard.
        let retired = self.application.retiring_mounts();
        self.scene.retain(|target| !retired.contains(&target.mount));
        if self
            .drag
            .as_ref()
            .is_some_and(|target| retired.contains(&target.mount))
        {
            self.drag = None;
        }
        if self
            .focus
            .as_ref()
            .is_some_and(|target| retired.contains(&target.mount))
        {
            self.focus = None;
        }
        let mut failure = None;
        for mount in retired {
            if mount == self.root {
                if let SurfaceState::Retiring { cause, cleanup } = &mut self.surface
                    && let Some(cleanup) = cleanup.as_ref()
                {
                    let cause = *cause;
                    let cleanup = cleanup.clone();
                    let native = match self.resources.close(&mount) {
                        Ok(false) => continue,
                        Ok(true) => Ok(()),
                        Err(error) => Err(error),
                    };
                    match self.application.close(&mount, native.and(cleanup)) {
                        Ok(()) => self.surface = SurfaceState::Closed { cause },
                        Err(error) => {
                            failure.get_or_insert(error);
                        }
                    }
                }
            } else {
                let cleanup = match self.resources.close(&mount) {
                    Ok(false) => continue,
                    Ok(true) => Ok(()),
                    Err(error) => Err(error),
                };
                if let Err(error) = self.application.close(&mount, cleanup) {
                    failure.get_or_insert(error);
                }
            }
        }
        if self.surface.is_live() && failure.is_none() && !self.application.is_retiring(&self.root)
        {
            let advancement = self.application.advance_deployment();
            self.resources.deactivate(&self.application);
            if let Err(error) = advancement {
                self.report(&error);
            }
            self.sync_native()?;
        }
        failure.map_or(Ok(()), Err)
    }
    pub fn retire(&mut self) -> maka_ui::Result<()> {
        self.retire_surface(Retirement::Exit)
    }
    fn retire_surface(&mut self, cause: Retirement) -> maka_ui::Result<()> {
        match &mut self.surface {
            SurfaceState::Live => {
                self.surface = SurfaceState::Retiring {
                    cause,
                    cleanup: None,
                }
            }
            SurfaceState::Retiring { cause: current, .. } => {
                if cause == Retirement::Exit {
                    *current = cause;
                }
            }
            SurfaceState::Closed { cause: current } => {
                if cause == Retirement::Exit {
                    *current = cause;
                }
                return Ok(());
            }
        }
        self.scene.clear();
        self.focus = None;
        self.drag = None;
        self.application.retire(&self.root)?;
        self.resources.deactivate(&self.application);
        Ok(())
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
    pub fn surface_closed(
        &mut self,
        surface: &Mount,
        result: Result<(), String>,
    ) -> maka_ui::Result<()> {
        if surface != &self.root {
            return Err(Error::Invalid(
                "terminal cleanup belongs to another surface".into(),
            ));
        }
        if self.surface.is_live() {
            self.retire_surface(Retirement::Exit)?;
        }
        let SurfaceState::Retiring { cleanup, .. } = &mut self.surface else {
            return Err(Error::Invalid("terminal cleanup already confirmed".into()));
        };
        if cleanup.is_some() {
            return Err(Error::Invalid(
                "terminal cleanup receipt already received".into(),
            ));
        }
        *cleanup = Some(result);
        self.pump()
    }
    /// Explicit successful restoration retry of the same failed physical surface.
    /// This cannot recover a different mount or acknowledge failed native resources.
    pub fn surface_recovered(&mut self, surface: &Mount) -> maka_ui::Result<()> {
        if surface != &self.root {
            return Err(Error::Invalid(
                "terminal recovery belongs to another surface".into(),
            ));
        }
        let SurfaceState::Retiring {
            cleanup: Some(cleanup),
            ..
        } = &mut self.surface
        else {
            return Err(Error::Invalid(
                "failed terminal restoration required".into(),
            ));
        };
        if cleanup.is_ok() {
            return Err(Error::Invalid(
                "failed terminal restoration required".into(),
            ));
        }
        *cleanup = Ok(());
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
