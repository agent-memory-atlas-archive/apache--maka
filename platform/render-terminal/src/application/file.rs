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

//! Native path input and original asynchronous selected-file custody.
mod browser;
pub(super) mod paths;
use super::{NativeContext, NativeInput, Placement, Slots, Widget, WidgetInput};
use crate::editor::{Editor, Geometry};
use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use maka_ui_runtime::{
    ComponentCall, ComponentOutput, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, file::*,
};
use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
};
use std::{
    any::Any,
    collections::BTreeMap,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, mpsc},
};

use unicode_width::UnicodeWidthStr;

struct Owner {
    context: NativeContext,
    editor: Editor,
    geometry: Option<Geometry>,
    selection: FileSelection,
    path: Option<browser::Request>,
    browser: Option<browser::Browser>,
    receiver: Option<mpsc::Receiver<Result<browser::Selected, FileError>>>,
    visible: bool,
    announced: bool,
    focus_browser: bool,
}
impl Owner {
    fn observation(&self) -> NativeInput {
        NativeInput {
            key: "observe".into(),
            value: serde_json::to_value(self.selection.observation()).expect("file observation"),
        }
    }
    fn stage(&mut self, path: PathBuf, directory: bool) -> WidgetInput {
        self.selection.cancel_intent();
        match self.selection.stage() {
            Some(choose) => {
                self.path = Some(browser::Request { path, directory });
                WidgetInput::Emitted(NativeInput {
                    key: "choose".into(),
                    value: serde_json::to_value(choose).unwrap(),
                })
            }
            None => WidgetInput::Consumed(None),
        }
    }
    fn settle(&mut self, result: Result<browser::Selected, FileError>) {
        match result {
            Ok(browser::Selected::File { path, snapshot }) => {
                self.browser = None;
                self.editor.clear();
                self.editor
                    .insert(path.to_str().expect("validated local path"));
                self.selection.settle(Ok(Some(snapshot)));
            }
            Ok(browser::Selected::Directory(directory)) => {
                self.selection.settle(Ok(None));
                let browser =
                    browser::Browser::new(directory, self.selection.observation().revision);
                self.editor.clear();
                self.editor
                    .insert(browser.path().to_str().expect("validated local path"));
                self.browser = Some(browser);
                self.focus_browser = self.visible;
            }
            Err(error) => {
                self.browser = None;
                self.selection.settle(Err(error));
            }
        }
    }
    fn start(&mut self) {
        let path = self.path.take().expect("original file gesture path");
        let budget = self.context.budget.clone();
        let mut ticket = match self.context.resources.reserve() {
            Ok(ticket) => ticket,
            Err(_) => {
                self.selection.settle(Err(FileError::WorkerFailed));
                return;
            }
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        match std::thread::Builder::new()
            .name("native-file-read".into())
            .spawn(move || {
                ticket.start();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    browser::select(path, &budget)
                }));
                let (result, cleanup) = match result {
                    Ok(result) => (result, Ok(())),
                    Err(_) => (
                        Err(FileError::WorkerFailed),
                        Err("native file worker panicked".into()),
                    ),
                };
                let _ = sender.send(result);
                ticket.complete(cleanup);
            }) {
            Ok(_) => self.receiver = Some(receiver),
            Err(_) => self.selection.settle(Err(FileError::WorkerFailed)),
        }
    }
}
struct Recipe(FileView);
impl Widget for Recipe {
    fn create(&self, context: NativeContext) -> Box<dyn Any> {
        Box::new(Owner {
            context,
            editor: Editor::single_line(8192, false),
            geometry: None,
            selection: FileSelection::default(),
            path: None,
            browser: None,
            receiver: None,
            visible: true,
            announced: false,
            focus_browser: false,
        })
    }
    fn commit(&self, state: &mut dyn Any) -> maka_ui_runtime::Result<Option<NativeInput>> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        if self.0.props.disabled || !owner.visible {
            owner.selection.cancel_intent();
            owner.path = None;
        }
        let accepted = owner.selection.accept(self.0.request.as_deref());
        if accepted {
            owner.start();
        }
        let announce = !owner.announced || accepted;
        owner.announced = true;
        Ok(announce.then(|| owner.observation()))
    }
    fn present_focus(&self, state: &mut dyn Any) -> bool {
        std::mem::take(&mut state.downcast_mut::<Owner>().unwrap().focus_browser)
    }
    fn observe(&self, state: &mut dyn Any) -> Option<NativeInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        let receiver = owner.receiver.as_ref()?;
        match receiver.try_recv() {
            Ok(result) => {
                owner.receiver = None;
                owner.settle(result);
                Some(owner.observation())
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                owner.receiver = None;
                owner.selection.settle(Err(FileError::WorkerFailed));
                Some(owner.observation())
            }
            Err(mpsc::TryRecvError::Empty) => None,
        }
    }
    fn input(
        &self,
        state: &mut dyn Any,
        key: &str,
        event: &Event,
    ) -> maka_ui_runtime::Result<WidgetInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        if self.0.props.disabled || !owner.visible || owner.selection.busy() {
            return Ok(WidgetInput::Unhandled);
        }
        if key == "entries" {
            return Ok(
                match owner.browser.as_mut().map(|browser| browser.input(event)) {
                    Some(browser::Input::Select(path)) => owner.stage(path, false),
                    Some(browser::Input::Handled) => WidgetInput::Consumed(None),
                    Some(browser::Input::PreserveFocus) => WidgetInput::PreserveFocus,
                    _ => WidgetInput::Unhandled,
                },
            );
        }
        let control = matches!(key, "choose" | "browse" | "parent");
        let activate = matches!(event, Event::Key(k) if k.kind != crossterm::event::KeyEventKind::Release && (k.code == KeyCode::Enter || control && k.code == KeyCode::Char(' ')) && k.modifiers.is_empty())
            || control
                && matches!(event, Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left));
        if activate {
            if key == "parent" {
                return Ok(owner
                    .browser
                    .as_ref()
                    .and_then(browser::Browser::parent)
                    .map_or(WidgetInput::Consumed(None), |path| owner.stage(path, true)));
            }
            if key == "path" || control {
                return Ok(owner.stage(PathBuf::from(owner.editor.text()), key == "browse"));
            }
        }
        if key != "path" {
            return Ok(WidgetInput::Unhandled);
        }
        let handled = match event {
            Event::Key(key) => owner.editor.key(*key),
            Event::Paste(text) => owner.editor.insert(text),
            Event::Mouse(mouse) => owner
                .geometry
                .as_ref()
                .is_some_and(|g| owner.editor.mouse_presented(*mouse, g)),
            _ => false,
        };
        Ok(if handled {
            WidgetInput::Consumed(None)
        } else {
            WidgetInput::Unhandled
        })
    }
    fn cancel_pointer(&self, state: &mut dyn Any) {
        let owner = state.downcast_mut::<Owner>().unwrap();
        owner.editor.cancel_pointer();
        if let Some(browser) = &mut owner.browser {
            browser.cancel_pointer();
        }
    }
    fn pointer_capture(&self, state: &dyn Any) -> bool {
        let owner = state.downcast_ref::<Owner>().unwrap();
        owner.editor.dragging()
            || owner
                .browser
                .as_ref()
                .is_some_and(browser::Browser::dragging)
    }
    fn capture(
        &self,
        state: &dyn Any,
        target: &maka_ui_runtime::actions::CaptureTarget,
    ) -> maka_ui_runtime::Result<Arc<dyn maka_capabilities::call::ReadResource>> {
        let capture = serde_json::from_value(target.input().clone())
            .map_err(|e| maka_ui_runtime::Error::Invalid(e.to_string()))?;
        state
            .downcast_ref::<Owner>()
            .unwrap()
            .selection
            .capture(&capture)
    }
    fn layout(&self, slots: &Slots<'_>) -> maka_ui_runtime::Result<super::TerminalLayout> {
        Ok(super::TerminalLayout {
            grow: u16::from(slots.native_state::<Owner>()?.browser.is_some()),
            shrink: true,
            ..Default::default()
        })
    }
    fn measure(&self, slots: &Slots<'_>, width: u16) -> maka_ui_runtime::Result<(u16, u32)> {
        let owner = slots.native_state::<Owner>()?;
        let (_, controls) = controls(
            &self.0.props,
            width,
            owner
                .browser
                .as_ref()
                .is_some_and(browser::Browser::has_parent),
        );
        Ok((
            width,
            2 + u32::from(self.0.props.appearance.border_width > 0.0) * 2
                + controls
                + owner
                    .browser
                    .as_ref()
                    .map_or(0, |browser| browser.height(2)),
        ))
    }
    fn paint(
        &self,
        slots: &mut Slots<'_>,
        area: Placement,
        buffer: &mut Buffer,
    ) -> maka_ui_runtime::Result<()> {
        let owner = slots.native_state::<Owner>()?;
        let a = &self.0.props.appearance;
        let color = |c: maka_ui_runtime::appearance::Color| {
            Color::Rgb((c.0 >> 24) as u8, (c.0 >> 16) as u8, (c.0 >> 8) as u8)
        };
        let terminal = a.terminal.as_ref();
        let foreground = terminal.map_or_else(
            || color(a.text.foreground),
            |p| super::paint::color(p.foreground),
        );
        let background = terminal.map_or_else(
            || color(a.background),
            |p| super::paint::color(p.background),
        );
        let muted = terminal.map_or_else(
            || color(a.muted_foreground),
            |p| super::paint::color(p.subtle),
        );
        let selected =
            terminal.map_or_else(|| color(a.selection), |p| super::paint::color(p.selection));
        let style = Style::default().fg(foreground).bg(background);
        let path_focused = slots.focused_key() == Some("path");
        let inset = u32::from(a.border_width > 0.0) * 2;
        let chrome = 2 + inset;
        let row = |y| {
            Placement::new(
                super::CellRect::new(area.x, area.y + y, area.width, 1),
                area.visible(),
            )
        };
        super::paint::text(&self.0.props.path_label, row(0), buffer, style.fg(muted));
        let field = Placement::new(
            super::CellRect::new(area.x, area.y + 1, area.width, 1 + inset),
            area.visible(),
        );
        let path = if inset > 0 {
            let border = terminal.map_or_else(
                || {
                    color(if path_focused {
                        a.focused_border
                    } else {
                        a.border
                    })
                },
                |p| super::paint::color(if path_focused { p.accent } else { p.border }),
            );
            super::border::paint(
                buffer,
                field,
                border,
                if a.ascii {
                    maka_ui_runtime::scene::terminal::BorderStyle::Ascii
                } else {
                    maka_ui_runtime::scene::terminal::BorderStyle::Rounded
                },
            );
            super::border::inner(field)
        } else {
            field
        };
        let geometry = owner.editor.paint_prepared(
            path,
            buffer,
            path_focused,
            style,
            style.bg(selected),
            style.add_modifier(Modifier::REVERSED),
        );
        if owner.editor.text().is_empty() {
            super::paint::text(&self.0.props.placeholder, path, buffer, style.fg(muted));
        }
        let enabled = !self.0.props.disabled && !owner.selection.busy();
        let (controls, control_rows) = controls(
            &self.0.props,
            area.width,
            owner
                .browser
                .as_ref()
                .is_some_and(browser::Browser::has_parent),
        );
        let mut targets = Vec::new();
        for control in controls {
            let bounds = Placement::new(
                super::CellRect::new(
                    area.x + i64::from(control.x),
                    area.y + i64::from(chrome + control.y),
                    control.width,
                    1,
                ),
                area.clip,
            );
            let tone = if !enabled {
                style.fg(muted)
            } else if slots.focused_key() == Some(control.key) {
                style.add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                style.add_modifier(Modifier::BOLD)
            };
            super::paint::text(&control.label, bounds, buffer, tone);
            targets.push((control.key, bounds));
        }
        let browser_geometry = owner.browser.as_ref().map(|browser| {
            browser.paint(
                &self.0.props,
                Placement::new(
                    super::CellRect::new(
                        area.x,
                        area.y + i64::from(chrome + control_rows),
                        area.width,
                        area.height.saturating_sub(chrome + control_rows),
                    ),
                    area.clip,
                ),
                area.height.saturating_sub(chrome + 1 + control_rows) as usize,
                buffer,
                style,
                enabled && slots.focused_key() == Some("entries"),
            )
        });
        let entries = browser_geometry.as_ref().map(|geometry| geometry.area);
        let focus_browser = owner.focus_browser;
        slots.native_geometry(Box::new((geometry, browser_geometry)))?;
        // A pending local read keeps keyboard focus; input remains gated above.
        if !self.0.props.disabled {
            slots.native_target("path", field, self.0.props.tab_index, false)?;
            for (key, bounds) in targets {
                slots.native_target(key, bounds, self.0.props.tab_index, false)?;
            }
            if let Some(entries) = entries {
                slots.native_target("entries", entries, self.0.props.tab_index, focus_browser)?;
            }
        }
        Ok(())
    }
    fn present(&self, state: &mut dyn Any, geometry: Box<dyn Any>) -> Option<NativeInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        let (geometry, browser_geometry) = *geometry
            .downcast::<(Geometry, Option<browser::Geometry>)>()
            .unwrap();
        owner.editor.present_geometry(geometry.clone());
        owner.geometry = Some(geometry);
        if let (Some(browser), Some(geometry)) = (owner.browser.as_mut(), browser_geometry) {
            browser.present(geometry);
        }
        None
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool) {
        let owner = state.downcast_mut::<Owner>().unwrap();
        owner.visible = visible;
        if !visible {
            owner.focus_browser = false;
            owner.editor.invalidate_geometry();
            owner.geometry = None;
            if let Some(browser) = &mut owner.browser {
                browser.suspend();
            }
            owner.selection.cancel_intent();
            owner.path = None;
        }
    }
    fn deactivate(&self, state: &mut dyn Any) {
        self.presentation(state, false);
    }
}
struct Control {
    key: &'static str,
    label: String,
    x: u16,
    y: u32,
    width: u16,
}
fn controls(props: &FilePickerProps, width: u16, parent: bool) -> (Vec<Control>, u32) {
    if width == 0 {
        return (vec![], 0);
    }
    let mut controls = Vec::new();
    let mut x = 0u16;
    let mut y = 0u32;
    for (key, label) in [("choose", &props.label), ("browse", &props.browse_label)]
        .into_iter()
        .chain(parent.then_some(("parent", &props.parent_label)))
    {
        let label = format!(" {} ", super::paint::sanitize(label));
        let size = label.width().min(usize::from(width)) as u16;
        if x > 0 && x.saturating_add(size) > width {
            x = 0;
            y += 1;
        }
        controls.push(Control {
            key,
            label,
            x,
            y,
            width: size,
        });
        x = x.saturating_add(size).saturating_add(1);
    }
    (controls, y + 1)
}

struct Component;
impl NativeComponent<dyn Widget> for Component {
    fn capture(
        &self,
        _: maka_ui_runtime::actions::CaptureCall<'_>,
    ) -> maka_ui_runtime::Result<maka_ui_runtime::actions::CapturePlan> {
        Ok(maka_ui_runtime::actions::CapturePlan::Native)
    }
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui_runtime::abi::ValueBindings,
    ) -> maka_ui_runtime::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let output = reduce(call)?;
        let view = NativeView::<dyn Widget>::new(
            Rc::new(Recipe(output.frame.view.clone())),
            BTreeMap::from([
                ("choose".into(), choose_schema()),
                ("observe".into(), observation_schema()),
            ]),
            vec![],
        )?;
        Ok(output.map_view(|_| view))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register(maka_ui_runtime::file::IMPLEMENTATION, |_, _| {
            Ok(BTreeMap::from([(
                "FilePicker".into(),
                NativeExport {
                    contract: file_picker_contract(),
                    value: NativeCapability::Component(Rc::new(Component)),
                },
            )]))
        })
        .expect("unique native file picker");
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn props() -> FilePickerProps {
        serde_json::from_value(serde_json::json!({
        "appearance": {"text":{"font":{"family":".SystemUIFont","fallbacks":[],"size":14,"lineHeight":1.5,"weight":400},"foreground":0x242724ffu32},
            "mutedForeground":0x656a63ffu32,"background":0xfaf9f6ffu32,"selection":0xa7c4b0ffu32,"caret":0x242724ffu32,
            "border":0x656a63ffu32,"focusedBorder":0x416653ffu32,"borderWidth":1,"radius":8,"padding":{"top":8,"right":12,"bottom":8,"left":12}},
        "label":"Choose plugin file","pathLabel":"Local file path","browseLabel":"Browse","parentLabel":"Parent","emptyLabel":"No files or folders","truncatedLabel":"First 2048 entries","placeholder":"/absolute/path","disabled":false,"tabIndex":0
    })).unwrap()
    }

    #[test]
    fn native_path_is_private_and_one_committed_gesture_reads_an_immutable_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("selected.maka-extension");
        std::fs::write(&path, b"original").unwrap();
        let props = props();
        let resources = Arc::new(maka_capabilities::call::Resources::default());
        let mut state = Recipe(FileView {
            props: props.clone(),
            request: Some("forged".into()),
        })
        .create(NativeContext {
            checkpoint: None,
            resources: resources.clone(),
            budget: Default::default(),
        });
        let initial = FileView {
            props: props.clone(),
            request: None,
        };
        Recipe(initial.clone()).commit(state.as_mut()).unwrap();
        let owner = state.downcast_ref::<Owner>().unwrap();
        assert!(owner.receiver.is_none());
        let native = serde_json::to_value(&props).unwrap();
        let mut forged = native.clone();
        forged["path"] = serde_json::json!(path);
        assert!(reduce(ComponentCall::Mount { props: &forged }).is_err());
        Recipe(initial.clone())
            .input(
                state.as_mut(),
                "path",
                &Event::Paste(path.to_string_lossy().into()),
            )
            .unwrap();
        assert_eq!(
            state
                .downcast_ref::<Owner>()
                .unwrap()
                .selection
                .observation()
                .phase,
            FilePhase::Empty
        );
        let WidgetInput::Emitted(choose) = Recipe(initial.clone())
            .input(
                state.as_mut(),
                "path",
                &Event::Key(crossterm::event::KeyEvent::new(
                    KeyCode::Enter,
                    crossterm::event::KeyModifiers::NONE,
                )),
            )
            .unwrap()
        else {
            panic!()
        };
        assert!(state.downcast_ref::<Owner>().unwrap().receiver.is_none());
        let mounted = reduce(ComponentCall::Mount { props: &native }).unwrap();
        let committed = reduce(ComponentCall::Event {
            state: &mounted.frame.state,
            props: &native,
            key: "choose",
            input: &choose.value,
            origin: maka_ui_runtime::EventOrigin::Raw,
        })
        .unwrap();
        let recipe = Recipe(committed.frame.view);
        recipe.commit(state.as_mut()).unwrap();
        recipe.commit(state.as_mut()).unwrap();
        let timer = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        timer.block_on(resources.finish()).unwrap();
        assert!(recipe.observe(state.as_mut()).is_some());
        let owner = state.downcast_ref::<Owner>().unwrap();
        let observed = owner.selection.observation();
        assert_eq!(observed.phase, FilePhase::Ready);
        assert_eq!(observed.file.unwrap().name, "selected.maka-extension");
        let captured = owner
            .selection
            .capture(&FileCapture {
                owner: observed.owner,
                revision: observed.revision,
            })
            .unwrap();
        std::fs::write(&path, b"replacement").unwrap();
        drop(state);
        assert_eq!(captured.read(0..8).unwrap(), b"original");
    }
    #[test]
    fn actual_presenter_routes_original_path_drag_and_space_activation() {
        use super::super::Presenter;
        use crossterm::event::{KeyEvent, KeyModifiers, MouseEvent};
        use ratatui::layout::Rect;
        let (runtime, input, packages) = super::super::tests::fixture(
            "return frame({count:state.count+(event.key==='choose'?1:0),revision:state.revision+1});",
        );
        let p = props();
        let native =
            NativeRegistry::with_surface(maka_plugin_core::composition::PresentationTarget::Tui)
                .with_scene_compiler(super::super::scene::language(), move |_, _| {
                    NativeView::<dyn Widget>::new(
                        Rc::new(Recipe(FileView {
                            props: p.clone(),
                            request: None,
                        })),
                        BTreeMap::from([
                            ("choose".into(), choose_schema()),
                            ("observe".into(), observation_schema()),
                        ]),
                        vec![],
                    )
                })
                .unwrap();
        let mut tui = Presenter::new(
            &input,
            packages,
            runtime.handle().clone(),
            native,
            serde_json::json!({}),
            maka_ui_js::execution(Default::default()),
        )
        .unwrap();
        let draw = |tui: &mut Presenter| {
            let area = Rect::new(0, 0, 30, 6);
            let scene = tui.paint(area, &mut Buffer::empty(area)).unwrap();
            tui.present(scene);
        };
        draw(&mut tui);
        tui.input(Event::Paste("abcdef".into())).unwrap();
        draw(&mut tui);
        let mouse = |kind, x| {
            Event::Mouse(MouseEvent {
                kind,
                column: x,
                row: 2,
                modifiers: KeyModifiers::NONE,
            })
        };
        tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 2))
            .unwrap();
        assert!(tui.drag.is_some());
        tui.input(mouse(MouseEventKind::Drag(MouseButton::Left), 5))
            .unwrap();
        tui.input(mouse(MouseEventKind::Up(MouseButton::Left), 5))
            .unwrap();
        assert!(tui.drag.is_none());
        let owner = tui.resources.owners[&tui.root.id()]
            .state
            .downcast_ref::<Owner>()
            .unwrap();
        assert_eq!(owner.editor.selected_text(), "bcd");
        assert!(!owner.editor.dragging());
        draw(&mut tui);
        tui.input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        tui.input(Event::Key(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
        )))
        .unwrap();
        assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 1);
        // A full-page browser uses its allocated height, including after a resize.
        // The native file remains the same owner and does not recreate its selection.
        let directory = tempfile::tempdir().unwrap();
        for i in 0..40 {
            std::fs::write(
                directory.path().join(format!("file-{i:02}.maka-plugin")),
                b"package",
            )
            .unwrap();
        }
        let owner = tui
            .resources
            .owners
            .get_mut(&tui.root.id())
            .unwrap()
            .state
            .downcast_mut::<Owner>()
            .unwrap();
        let selected = browser::select(
            browser::Request {
                path: directory.path().to_path_buf(),
                directory: true,
            },
            &owner.context.budget,
        )
        .unwrap();
        owner.settle(Ok(selected));
        for (height, last_visible) in [(40, "file-33"), (12, "file-05"), (30, "file-23")] {
            let area = Rect::new(0, 0, 65, height);
            let mut buffer = Buffer::empty(area);
            let scene = tui.paint(area, &mut buffer).unwrap();
            tui.present(scene);
            let rendered = buffer
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(
                rendered.contains(last_visible),
                "browser did not fill {height} rows: {rendered}"
            );
        }
        let root = tui.root.clone();
        tui.surface_closed(&root, Ok(())).unwrap();
        tui.shutdown().unwrap();
    }
}
