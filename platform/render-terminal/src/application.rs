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
mod adaptive;
mod border;
mod checkpoint;
mod choice;
mod context_menu;
mod dialog;
mod editor;
mod file;
mod focus;
mod geometry;
mod input;
mod layout;
mod markdown_text;
mod morph_mark;
mod native;
mod paint;
mod pointer;
mod reading;
mod resources;
mod rows;
mod run;
mod scene;
mod scroll;
mod selection;
mod source;
mod terminal;
mod text_file;
mod textarea;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use futures_util::{FutureExt, StreamExt, stream::FuturesUnordered};
pub use geometry::{CellRect, Placement};
use maka_packages::Package;
use maka_plugin_core::{
    composition::{Input, PresentationTarget, RootKind},
    runtime::Mount,
};
use maka_ui_runtime::{
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
    /// Prove the complete immutable presentation recipe unchanged across a
    /// business revision. This permits an actual resize flush after native
    /// observations, while owner identity and opening admission stay separate.
    /// The original declarative scene, when this recipe uses the scene language.
    /// Deferred foreground painting resolves the original scene declaration.
    fn scene(&self) -> Option<&TerminalScene> {
        None
    }
    /// Interrupt an original pointer gesture without changing text or selection.
    fn cancel_pointer(&self, _: &mut dyn std::any::Any) {}
    /// Local pointer feedback against the last presented target. Leaving clears
    /// feedback; hovering never selects, focuses, or invokes guest code.
    fn hover(&self, _: &mut dyn std::any::Any, _: &str, _: Option<ratatui::layout::Position>) {}
    fn same_presentation(&self, _: &dyn std::any::Any) -> bool {
        false
    }
    /// Immutable root constraints participate in the parent's original cell flow.
    fn layout(&self, _: &Slots<'_>) -> maka_ui_runtime::Result<TerminalLayout> {
        Ok(TerminalLayout::default())
    }
    fn create(&self, _: NativeContext) -> Box<dyn std::any::Any> {
        Box::new(())
    }
    fn commit(&self, _: &mut dyn std::any::Any) -> maka_ui_runtime::Result<Option<NativeInput>> {
        Ok(None)
    }
    /// Reconcile focus eligibility from the committed structural route. This
    /// cannot publish geometry, take focus, or consume a presentation offer.
    fn prepare_focus(&self, _: &mut dyn std::any::Any, _: bool) {}
    /// Consume this exact eligible target after a successful flush and current
    /// original frame/owner admission. Geometry alone cannot consume an offer.
    /// Compact selectors may return to the original outer typing target.
    fn returns_focus(&self) -> bool {
        false
    }
    /// Exclusive native keyboard interaction, such as an embedded terminal.
    fn captures_keyboard(&self, _: &dyn std::any::Any) -> bool {
        false
    }
    /// Deliver all physical mouse buttons and motion to this native target.
    fn raw_pointer(&self, _: &dyn std::any::Any) -> bool {
        false
    }
    /// A native transient selector can consume Tab without acquiring focus-return ownership.
    fn handles_tab(&self, _: &dyn std::any::Any, _: &str) -> bool {
        false
    }
    /// A physical keyboard traversal enters this target. Returning false keeps
    /// the previous focus; restoration and ordinary paints do not call this.
    fn enter_focus(&self, _: &mut dyn std::any::Any, _: &str) -> bool {
        true
    }
    fn take_focus_exit(&self, _: &mut dyn std::any::Any) -> bool {
        false
    }
    fn present_focus(&self, _: &mut dyn std::any::Any) -> bool {
        false
    }
    /// Poll original native observations independently of recipe revisions.
    fn observe(&self, _: &mut dyn std::any::Any) -> Option<NativeInput> {
        None
    }
    fn input(
        &self,
        _: &mut dyn std::any::Any,
        _: &str,
        _: &Event,
    ) -> maka_ui_runtime::Result<WidgetInput> {
        Ok(WidgetInput::Unhandled)
    }
    fn pointer_capture(&self, _: &dyn std::any::Any) -> bool {
        false
    }
    fn copy(&self, _: &dyn std::any::Any) -> maka_ui_runtime::Result<Option<String>> {
        Ok(None)
    }
    fn capture(
        &self,
        _: &dyn std::any::Any,
        _: &maka_ui_runtime::actions::CaptureTarget,
    ) -> maka_ui_runtime::Result<std::sync::Arc<dyn maka_capabilities::call::ReadResource>> {
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
        _: &maka_ui_runtime::actions::CaptureTarget,
    ) -> maka_ui_runtime::Result<maka_ui_runtime::actions::ContinuationWitness> {
        Err(Error::Invalid(
            "terminal widget continuation unavailable".into(),
        ))
    }
    /// Observe only: no input dispatch or presentation mutation during admission.
    fn validate_continuation(
        &self,
        _: &dyn std::any::Any,
        _: &maka_ui_runtime::actions::CaptureTarget,
        _: &maka_ui_runtime::actions::ContinuationWitness,
    ) -> maka_ui_runtime::Result<bool> {
        Ok(false)
    }
    /// Successful presentation may suspend physical input and geometry while
    /// retaining the editor, capture and pending command observations.
    fn presentation(&self, _: &mut dyn std::any::Any, _: bool) {}
    fn deactivate(&self, _: &mut dyn std::any::Any) {}
    fn measure(&self, slots: &Slots<'_>, width: u16) -> maka_ui_runtime::Result<(u16, u32)>;
    /// Logical traversal includes fully clipped placements; writes and physical
    /// targets must honor Placement.clip. Only successful flush admits geometry.
    fn paint(
        &self,
        slots: &mut Slots<'_>,
        area: Placement,
        buffer: &mut Buffer,
    ) -> maka_ui_runtime::Result<()>;
}
/// Original renderer custody. Workers retain tickets until their real resources
/// settle; every owner shares its presenter's aggregate native budget.
#[derive(Clone)]
pub struct NativeContext {
    pub checkpoint: Option<maka_ui_runtime::checkpoint::Scope>,
    pub resources: std::sync::Arc<maka_capabilities::call::Resources>,
    pub budget: maka_ui_runtime::budget::NativeBudget,
}

/// Owner-originated observation; never a guest-created native resource.
#[derive(Clone, PartialEq)]
pub struct NativeInput {
    pub key: String,
    pub value: Value,
}
pub enum WidgetInput {
    ExitFocus,
    Unhandled,
    /// Native editing yields a navigation gesture to its declared focus scope.
    Navigate,
    Consumed(Option<NativeInput>),
    /// Handled pointer work that preserves the original keyboard focus.
    PreserveFocus,
    /// An immediate business event; never replaces retryable native observations.
    Emitted(NativeInput),
    /// A physical native submission with an original, revision-checked default.
    SubmitCopy(NativeInput, String),
}
type Job = Pin<Box<dyn Future<Output = (Uuid, TaskCompletion)> + Send>>;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Retirement {
    Exit,
    Replace(RootKind),
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
                cause: Retirement::Replace(_),
                ..
            } | Self::Closed {
                cause: Retirement::Replace(_)
            }
        )
    }
}

pub struct Presenter {
    animation_origin: std::time::Instant,
    window_focused: bool,
    application: Application<dyn Widget>,
    root: Mount,
    scene: Vec<paint::Target>,
    keyboard: Option<std::rc::Rc<paint::Ancestor>>,
    resources: resources::Resources,
    focus: Option<paint::Target>,
    drag: Option<paint::Target>,
    hover: Option<paint::Target>,
    native_return: Option<focus::Return>,
    modal_focus: Vec<dialog::FocusReturn>,
    restore_focus: Option<dialog::WeakTarget>,
    physical_path: Vec<dialog::Scope>,
    focus_before_flush: Option<dialog::WeakTarget>,
    jobs: FuturesUnordered<Job>,
    surface: SurfaceState,
    reported: std::collections::BTreeSet<String>,
    /// One bounded physical clipboard intent, flushed before the next OS event.
    clipboard: Option<std::rc::Rc<str>>,
}
impl Presenter {
    pub fn new(
        input: &Input,
        packages: Vec<Package>,
        scheduler: tokio::runtime::Handle,
        native: NativeRegistry<dyn Widget>,
        props: Value,
        execution: maka_ui_runtime::ExecutionOptions,
    ) -> maka_ui_runtime::Result<Self> {
        if input.environment.presentation != Some(PresentationTarget::Tui) {
            return Err(Error::Invalid("terminal deployment required".into()));
        }
        let application = Application::new(
            input,
            packages,
            scheduler,
            execution,
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
        registrations: Vec<maka_ui_runtime::actions::RemoteService>,
        execution: maka_ui_runtime::ExecutionOptions,
    ) -> maka_ui_runtime::Result<Self> {
        let application = Application::start_from_store(
            store,
            source,
            PresentationTarget::Tui,
            scheduler,
            execution,
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
    ) -> maka_ui_runtime::Result<
        impl Future<Output = maka_ui_runtime::Result<maka_ui_runtime::CommittedUpdate>>
        + Send
        + 'static
        + use<>,
    > {
        self.application.load_committed_update(source)
    }
    /// Acknowledge desired input only. Native retirement is synchronous; actual
    /// terminal restoration and replacement remain the surface owner's work.
    pub fn adopt_update(
        &mut self,
        update: maka_ui_runtime::CommittedUpdate,
        registrations: Vec<maka_ui_runtime::actions::RemoteService>,
    ) -> maka_ui_runtime::Result<()> {
        let prepared = self.application.prepare_update(update)?;
        self.adopt_prepared(prepared, registrations)
    }
    pub fn adopt_prepared(
        &mut self,
        update: maka_ui_runtime::PreparedUpdate,
        registrations: Vec<maka_ui_runtime::actions::RemoteService>,
    ) -> maka_ui_runtime::Result<()> {
        let was_live = self.surface.is_live() && !self.application.is_retiring(&self.root);
        let kind = was_live
            .then(|| self.application.root_kind(&self.root))
            .transpose()?;
        self.application.adopt_prepared(update, registrations)?;
        if was_live && self.application.is_retiring(&self.root) {
            // Capture the install's effect now, not a later, unrelated realm fault.
            self.retire_surface(Retirement::Replace(kind.expect("live root")))?;
        }
        self.resources.deactivate(&self.application);
        Ok(())
    }
    pub fn deployment_status(&self) -> maka_ui_runtime::DeploymentStatus {
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
    pub fn try_replace_surface(&mut self, props: Value) -> maka_ui_runtime::Result<bool> {
        let kind = match self.surface {
            SurfaceState::Closed {
                cause: Retirement::Replace(kind),
            } => kind,
            _ => {
                return Err(Error::Invalid(
                    "original terminal cleanup not confirmed for replacement".into(),
                ));
            }
        };
        let Some(root) = self.application.mount_replacement_root(kind, props)? else {
            return Ok(false);
        };
        self.root = root;
        self.surface = SurfaceState::Live;
        self.sync_native()?;
        Ok(true)
    }

    fn mount(
        mut application: Application<dyn Widget>,
        props: Value,
    ) -> maka_ui_runtime::Result<Self> {
        let root = match application.mount_bootstrap(props) {
            Ok(root) => root,
            Err(error) => {
                let error = match application.deployment_status().error {
                    Some(initial) => {
                        Error::Invalid(format!("application activation failed: {initial}; {error}"))
                    }
                    None => error,
                };
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
            animation_origin: std::time::Instant::now(),
            window_focused: true,
            application,
            root,
            scene: Vec::new(),
            keyboard: None,
            resources: Default::default(),
            focus: None,
            drag: None,
            hover: None,
            native_return: None,
            modal_focus: Vec::new(),
            restore_focus: None,
            physical_path: Vec::new(),
            focus_before_flush: None,
            jobs: FuturesUnordered::new(),
            surface: SurfaceState::Live,
            reported: Default::default(),
            clipboard: None,
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
    /// The embedding may reserve a surface-wide exit key only when the actual
    /// admitted native focus does not own exclusive keyboard interaction.
    pub fn captures_keyboard(&mut self) -> maka_ui_runtime::Result<bool> {
        let Some(target) = self.focus.clone() else {
            return Ok(false);
        };
        if !self.surface.is_live()
            || !self.target_admitted(&target)
            || self.application.is_retiring(&target.mount)
            || !self.application.mount_visible(&target.mount)
        {
            return Ok(false);
        }
        self.resources.captures_keyboard(&target)
    }
    /// Resolve raw coordinates immediately against the last successfully flushed
    /// scene. Crossterm supplies no screen epoch; never re-resolve a captured key.
    /// 输入立即绑定到最后成功输出的场景和真实 mount/revision。
    pub fn input(&mut self, event: Event) -> maka_ui_runtime::Result<()> {
        let result = self.input_inner(event);
        self.recover_result(result)
    }
    fn input_inner(&mut self, event: Event) -> maka_ui_runtime::Result<()> {
        match event {
            Event::FocusLost => self.window_focused = false,
            Event::FocusGained => self.window_focused = true,
            _ => {}
        }
        if matches!(
            event,
            Event::Key(_) | Event::Paste(_) | Event::FocusLost | Event::Resize(..)
        ) {
            self.clear_hover()?;
        }
        if matches!(event, Event::FocusLost | Event::Resize(..)) {
            self.resources.cancel_pointer()?;
            self.drag = None;
        }
        if !self.surface.is_live() || !self.resources.ready(&self.application, &self.root)? {
            return Ok(());
        }
        if !self.physical_admitted() {
            self.clear_hover()?;
            return Ok(());
        }
        self.cancel_reading_focus_requests(&event);
        self.cancel_reading_keyboard_reveal(&event);
        let raw_pointer_target = if let Event::Mouse(mouse) = &event {
            let target = self
                .scene
                .iter()
                .rev()
                .find(|t| {
                    !matches!(t.kind, paint::TargetKind::ContextMenu { button: false })
                        && t.area.contains((mouse.column, mouse.row).into())
                })
                .cloned();
            match target {
                Some(target)
                    if self.target_admitted(&target) && self.resources.raw_pointer(&target)? =>
                {
                    Some(target)
                }
                _ => None,
            }
        } else {
            None
        };
        if raw_pointer_target.is_none() && self.open_context_menu(&event)? {
            return Ok(());
        }
        if let Event::Mouse(mouse) = &event
            && mouse.kind == MouseEventKind::Moved
            && raw_pointer_target.is_none()
        {
            return self.move_pointer((mouse.column, mouse.row).into());
        }
        if matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down(_) | MouseEventKind::Drag(_)))
        {
            self.clear_hover()?;
        }
        if matches!(self.native_return, Some(focus::Return::Exited)) {
            if matches!(event, Event::Key(key) if key.kind != KeyEventKind::Press) {
                return Ok(());
            }
            if matches!(
                event,
                Event::Key(_)
                    | Event::Mouse(crossterm::event::MouseEvent {
                        kind: MouseEventKind::Down(_),
                        ..
                    })
            ) {
                self.native_return = None;
            }
        }
        self.track_focus_return()?;
        let returnable = match self.focus.as_ref() {
            Some(target) => self.resources.returns_focus(target)?,
            None => false,
        };
        let native_tab = match self.focus.as_ref() {
            Some(target) => self.resources.handles_tab(target)?,
            None => false,
        };
        if self.extend_item_selection(&event)? || self.navigate_items(&event)? {
            return Ok(());
        }
        let target = if raw_pointer_target.is_some() {
            raw_pointer_target
        } else {
            match &event {
                Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                    KeyCode::Tab | KeyCode::BackTab if !returnable && !native_tab => {
                        if self.dispatch_focused_shortcut(&event)? {
                            return Ok(());
                        }
                        let reverse = key.code == KeyCode::BackTab
                            || key.modifiers.contains(KeyModifiers::SHIFT);
                        let stops = self.tab_stops();
                        if !stops.is_empty() {
                            let index = self.focus.as_ref().and_then(|focus| {
                                stops.iter().position(|target| target.same(focus))
                            });
                            let n = stops.len();
                            let next = match (index, reverse) {
                                (Some(i), true) => (i + n - 1) % n,
                                (Some(i), false) => (i + 1) % n,
                                (None, true) => n - 1,
                                (None, false) => 0,
                            };
                            let target = stops[next];
                            if !self.focus.as_ref().is_some_and(|old| old.same(target)) {
                                self.focus_item(target.clone())?;
                            }
                        }
                        self.track_focus_return()?;
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
                        MouseEventKind::Down(MouseButton::Left | MouseButton::Right)
                            | MouseEventKind::ScrollUp
                            | MouseEventKind::ScrollDown
                    ) =>
                {
                    self.scene
                        .iter()
                        .rev()
                        .find(|t| {
                            !matches!(t.kind, paint::TargetKind::ContextMenu { button: false })
                                && t.area.contains((mouse.column, mouse.row).into())
                        })
                        .cloned()
                }
                Event::Mouse(mouse)
                    if matches!(
                        mouse.kind,
                        MouseEventKind::Drag(MouseButton::Left)
                            | MouseEventKind::Up(MouseButton::Left)
                    ) =>
                {
                    self.drag.clone()
                }
                _ => None,
            }
        };
        if let Some(target) = target {
            if self.application.is_retiring(&target.mount) || !self.target_admitted(&target) {
                return Ok(());
            }
            if let paint::TargetKind::Backdrop { body } = &target.kind {
                if matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left | MouseButton::Right)) && !body.contains((mouse.column, mouse.row).into()))
                {
                    self.dismiss(&target, maka_ui_runtime::dismiss::Reason::Outside)?;
                } else {
                    self.dismiss_escape(&event)?;
                }
                return Ok(());
            }
            if let paint::TargetKind::SelectionCopy { text } = &target.kind {
                if !self.resources.admits(&target)
                    || self.application.frame(&target.mount)?.revision != target.revision
                {
                    return Ok(());
                }
                if matches!(event, Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left))
                    || matches!(event, Event::Key(key) if key.kind != KeyEventKind::Release && key.modifiers.is_empty() && matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')))
                {
                    self.clipboard = Some(text.clone());
                    self.close_context_action(target.scope.as_ref());
                    self.sync_native()?;
                } else {
                    self.dismiss_escape(&event)?;
                }
                return Ok(());
            }
            // Logical offscreen controls remain in Tab order, but cannot edit
            // or activate until their revealed geometry has actually flushed.
            let native = if target.area.is_empty() {
                Some(WidgetInput::Unhandled)
            } else {
                self.resources.input(&target, &event)?
            };
            let Some(native) = native else {
                return Ok(());
            };
            let (native, submit_copy) = match native {
                WidgetInput::SubmitCopy(input, text) => (WidgetInput::Emitted(input), Some(text)),
                other => (other, None),
            };
            let native = match native {
                WidgetInput::ExitFocus => {
                    self.exit_native_focus()?;
                    return Ok(());
                }
                WidgetInput::Emitted(emitted) => {
                    let revision = self.application.frame(&target.mount)?.revision;
                    let decision = self.application.dispatch(
                        &target.mount,
                        revision,
                        &emitted.key,
                        emitted.value,
                    )?;
                    if !decision.prevent_default
                        && !self.application.is_retiring(&target.mount)
                        && self.resources.admits(&target)
                        && self.target_admitted(&target)
                        && let Some(text) = submit_copy
                    {
                        self.clipboard = Some(text.into());
                    }
                    if !matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown))
                    {
                        self.focus = Some(target);
                    }
                    if !decision.prevent_default {
                        self.close_context_action(
                            self.focus
                                .clone()
                                .and_then(|target| target.scope.clone())
                                .as_ref(),
                        );
                    }
                    self.track_focus_return()?;
                    self.sync_native()?;
                    return Ok(());
                }
                other => other,
            };
            let preserve_focus = matches!(native, WidgetInput::PreserveFocus);
            let native = if preserve_focus {
                WidgetInput::Consumed(None)
            } else {
                native
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
                if !preserve_focus
                    && !matches!(event,Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown | MouseEventKind::Drag(_) | MouseEventKind::Up(_) | MouseEventKind::Moved))
                {
                    self.focus = Some(target);
                }
                self.track_focus_return()?;
                self.sync_native()?;
                return Ok(());
            }
            if self.dispatch_shortcut(
                &target.ancestors,
                &event,
                matches!(native, WidgetInput::Navigate),
            )? {
                return Ok(());
            }
            // Bubble unconsumed navigation through the original flushed scroll ancestors.
            for ancestor in target.ancestors.iter().rev() {
                if self.application.is_retiring(&ancestor.mount) {
                    continue;
                }
                let observed = match self.resources.input(ancestor, &event)? {
                    Some(WidgetInput::Emitted(emitted)) => {
                        let revision = self.application.frame(&ancestor.mount)?.revision;
                        self.application.dispatch(
                            &ancestor.mount,
                            revision,
                            &emitted.key,
                            emitted.value,
                        )?;
                        self.sync_native()?;
                        return Ok(());
                    }
                    Some(WidgetInput::Consumed(observed)) => observed,
                    Some(WidgetInput::PreserveFocus) => None,
                    _ => continue,
                };
                if let Some(observed) = observed {
                    self.resources.observe(&ancestor.mount, observed);
                }
                self.sync_native()?;
                return Ok(());
            }
            if self.dismiss_escape(&event)? {
                return Ok(());
            }
            if target.area.is_empty() {
                return Ok(());
            }
            if !matches!(
                target.kind,
                paint::TargetKind::Action { .. }
                    | paint::TargetKind::Focus(_)
                    | paint::TargetKind::Item { .. }
            ) {
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
            if let paint::TargetKind::Focus(binding) = &target.kind {
                if self.application.frame(&target.mount)?.revision == target.revision
                    && self.application.mount_visible(&target.mount)
                    && !matches!(event, Event::Key(key) if key.kind != KeyEventKind::Press)
                {
                    self.focus_child(binding)?;
                }
                return Ok(());
            }
            if matches!(target.kind, paint::TargetKind::Item { .. })
                && (self.application.frame(&target.mount)?.revision != target.revision
                    || matches!(event, Event::Key(key) if key.kind != KeyEventKind::Press))
            {
                return Ok(());
            }
            self.clear_item_selection(&target);
            let kind = match target.kind {
                paint::TargetKind::Item {
                    submit,
                    submit_space,
                    ..
                } if matches!(event, Event::Key(key) if (submit && key.code == KeyCode::Enter) || (submit_space && key.code == KeyCode::Char(' ')))
                    || (submit
                        && matches!(event, Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left))) =>
                {
                    "submit"
                }
                paint::TargetKind::Item { .. } => "activate",
                _ => "click",
            };
            let decision = self.application.dispatch(
                &target.mount,
                target.revision,
                &target.key,
                json!({"kind":kind}),
            )?;
            if !decision.prevent_default {
                if let paint::TargetKind::Action {
                    copy_text: Some(text),
                } = &target.kind
                    && !self.application.is_retiring(&target.mount)
                    && self.resources.admits(&target)
                    && self.target_admitted(&target)
                {
                    self.clipboard = Some(text.clone());
                }
                self.close_context_action(target.scope.as_ref());
                self.focus = Some(target);
            }
            self.sync_native()?;
        }
        if self.dismiss_escape(&event)? {
            return Ok(());
        }
        if self.focus.is_none()
            && let Some(scope) = self.keyboard.clone()
        {
            self.dispatch_shortcut(&[scope], &event, false)?;
        }
        Ok(())
    }
    /// Allow a live declared command to own an otherwise reserved embedding key.
    fn dispatch_focused_shortcut(&mut self, event: &Event) -> maka_ui_runtime::Result<bool> {
        if !self.surface.is_live()
            || !self.physical_admitted()
            || !self.resources.ready(&self.application, &self.root)?
        {
            return Ok(false);
        }
        let ancestors = match self.focus.clone() {
            Some(target)
                if self.target_admitted(&target)
                    && self.resources.admits(&target)
                    && self.application.mount_visible(&target.mount)
                    && !self.application.is_retiring(&target.mount) =>
            {
                target.ancestors
            }
            Some(_) => return Ok(false),
            None => self.keyboard.clone().into_iter().collect(),
        };
        self.dispatch_shortcut(&ancestors, event, false)
    }
    fn dispatch_shortcut(
        &mut self,
        ancestors: &[std::rc::Rc<paint::Ancestor>],
        event: &Event,
        navigation: bool,
    ) -> maka_ui_runtime::Result<bool> {
        let Event::Key(key) = event else {
            return Ok(false);
        };
        let Some((key, modifiers)) = shortcut_chord(key) else {
            return Ok(false);
        };
        for ancestor in ancestors.iter().rev() {
            if navigation
                && let Some(binding) = ancestor
                    .focus_shortcuts
                    .iter()
                    .find(|binding| binding.declaration.matches(&key, modifiers))
            {
                if binding.declaration.enabled
                    && self.target_admitted(ancestor)
                    && self.resources.admits(ancestor)
                    && !self.application.is_retiring(&ancestor.mount)
                    && self.application.mount_visible(&ancestor.mount)
                    && self.application.frame(&ancestor.mount)?.revision == ancestor.revision
                {
                    self.focus_child(&binding.target)?;
                }
                return Ok(true);
            }
            if let Some(shortcut) = ancestor
                .shortcuts
                .iter()
                .find(|shortcut| shortcut.matches(&key, modifiers, false))
            {
                if shortcut.enabled
                    && self.target_admitted(ancestor)
                    && self.resources.admits(ancestor)
                    && self.application.mount_visible(&ancestor.mount)
                {
                    self.application.dispatch(
                        &ancestor.mount,
                        ancestor.revision,
                        &shortcut.input,
                        json!({"kind":"click"}),
                    )?;
                    self.sync_native()?;
                }
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// Copy only the focused original live native scope through the bounded
    /// terminal writer. An OSC52 write is not an OS clipboard acknowledgement.
    pub fn copy(&mut self, writer: &mut impl std::io::Write) -> maka_ui_runtime::Result<bool> {
        if !self.physical_admitted() {
            return Ok(false);
        }
        let Some(target) = self.focus.clone() else {
            return Ok(false);
        };
        if self.application.is_retiring(&target.mount) {
            return Ok(false);
        }
        if !self.target_admitted(&target) {
            return Ok(false);
        }
        let text = match self.resources.copy(&target)? {
            Some(text) => Some(text),
            None => match self.item_reading_scope(&target) {
                Some((scope, _)) => scope.copy()?,
                None => None,
            },
        };
        let Some(text) = text else {
            return Ok(false);
        };
        crate::terminal::copy(writer, &text).map_err(|error| Error::Invalid(error.to_string()))?;
        Ok(true)
    }
    /// Complete the last accepted physical clipboard default action. The
    /// runner calls this immediately after input, before reading another event.
    /// OSC52 delivery does not prove that the terminal accepted the clipboard.
    pub fn flush_clipboard(
        &mut self,
        writer: &mut impl std::io::Write,
    ) -> maka_ui_runtime::Result<bool> {
        let Some(text) = self.clipboard.take() else {
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
    pub fn pump(&mut self) -> maka_ui_runtime::Result<()> {
        let result = self.pump_inner();
        self.recover_result(result)
    }
    fn recover_result(
        &mut self,
        result: maka_ui_runtime::Result<()>,
    ) -> maka_ui_runtime::Result<()> {
        if self.surface.is_live()
            && self.application.is_retiring(&self.root)
            && self.application.can_recover(&self.root)
        {
            if let Err(error) = &result {
                self.report(error);
            }
            self.retire_surface(Retirement::Replace(RootKind::Recovery))?;
            return Ok(());
        }
        if result.is_ok()
            && self.surface.is_live()
            && self.application.can_resume_launch(&self.root)
        {
            self.retire_surface(Retirement::Replace(RootKind::Launch))?;
        }
        result
    }
    fn pump_inner(&mut self) -> maka_ui_runtime::Result<()> {
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
        if self
            .hover
            .as_ref()
            .is_some_and(|target| retired.contains(&target.mount))
        {
            self.clear_hover()?;
        }
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
        // A native child can still own a worker after its parent's resources
        // close. Keep both mounts until the original child actually settles.
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
                        Err(Error::Core(maka_plugin_core::Error::CleanupPending)) => {}
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
                match self.application.close(&mount, cleanup) {
                    Ok(()) | Err(Error::Core(maka_plugin_core::Error::CleanupPending)) => {}
                    Err(error) => {
                        failure.get_or_insert(error);
                    }
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
    pub fn retire(&mut self) -> maka_ui_runtime::Result<()> {
        self.retire_surface(Retirement::Exit)
    }
    fn retire_surface(&mut self, cause: Retirement) -> maka_ui_runtime::Result<()> {
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
        self.clear_hover()?;
        self.native_return = None;
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
    ) -> maka_ui_runtime::Result<()> {
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
    pub fn surface_recovered(&mut self, surface: &Mount) -> maka_ui_runtime::Result<()> {
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
    pub fn shutdown(&mut self) -> maka_ui_runtime::Result<()> {
        self.application.shutdown()
    }
}

fn shortcut_chord(
    key: &crossterm::event::KeyEvent,
) -> Option<(String, maka_ui_runtime::shortcut::Modifiers)> {
    if key.kind != KeyEventKind::Press
        || !(key.modifiers
            - (KeyModifiers::CONTROL
                | KeyModifiers::ALT
                | KeyModifiers::SHIFT
                | KeyModifiers::SUPER))
            .is_empty()
    {
        return None;
    }
    // Crossterm's Windows AltGr text events carry both Control and Alt.
    if cfg!(target_os = "windows")
        && matches!(key.code, KeyCode::Char(_))
        && key
            .modifiers
            .contains(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }
    let name = match key.code {
        KeyCode::Char(' ') => "space".into(),
        KeyCode::Char(c) => c.to_ascii_lowercase().to_string(),
        KeyCode::Esc => "escape".into(),
        KeyCode::Enter => "enter".into(),
        KeyCode::Tab | KeyCode::BackTab => "tab".into(),
        KeyCode::Delete => "delete".into(),
        KeyCode::Up => "up".into(),
        KeyCode::Down => "down".into(),
        KeyCode::Left => "left".into(),
        KeyCode::Right => "right".into(),
        KeyCode::Home => "home".into(),
        KeyCode::End => "end".into(),
        KeyCode::PageUp => "pageup".into(),
        KeyCode::PageDown => "pagedown".into(),
        KeyCode::F(n) => format!("f{n}"),
        _ => return None,
    };
    Some((
        name,
        maka_ui_runtime::shortcut::Modifiers {
            control: key.modifiers.contains(KeyModifiers::CONTROL),
            alt: key.modifiers.contains(KeyModifiers::ALT),
            shift: key.code == KeyCode::BackTab || key.modifiers.contains(KeyModifiers::SHIFT),
            super_key: key.modifiers.contains(KeyModifiers::SUPER),
        },
    ))
}

#[cfg(test)]
#[path = "application/tests.rs"]
mod tests;
