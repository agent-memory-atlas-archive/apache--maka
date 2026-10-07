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
use super::{NativeContext, NativeInput, Placement, Slots, Widget, WidgetInput};
use crate::editor::{Editor, Geometry};
use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use maka_ui::{
    ComponentCall, ComponentOutput, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, budget::Kind, file::*,
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

struct Owner {
    context: NativeContext,
    editor: Editor,
    geometry: Option<Geometry>,
    selection: FileSelection,
    path: Option<PathBuf>,
    receiver: Option<mpsc::Receiver<Result<Option<Arc<FileSnapshot>>, FileError>>>,
    visible: bool,
    announced: bool,
}
impl Owner {
    fn observation(&self) -> NativeInput {
        NativeInput {
            key: "observe".into(),
            value: serde_json::to_value(self.selection.observation()).expect("file observation"),
        }
    }
    fn start(&mut self) {
        let path = self.path.take().expect("original file gesture path");
        let budget = self.context.budget.clone();
        let job = match budget.reserve(Kind::Job, 1) {
            Ok(job) => job,
            Err(error) => {
                self.selection.settle(Err(error.into()));
                return;
            }
        };
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
                    read_selected(&path, &budget)
                }));
                let (result, cleanup) = match result {
                    Ok(result) => (result.map(Some), Ok(())),
                    Err(_) => (
                        Err(FileError::WorkerFailed),
                        Err("native file worker panicked".into()),
                    ),
                };
                drop(job);
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
            receiver: None,
            visible: true,
            announced: false,
        })
    }
    fn commit(&self, state: &mut dyn Any) -> maka_ui::Result<Option<NativeInput>> {
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
    fn observe(&self, state: &mut dyn Any) -> Option<NativeInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        let receiver = owner.receiver.as_ref()?;
        match receiver.try_recv() {
            Ok(result) => {
                owner.receiver = None;
                owner.selection.settle(result);
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
    fn input(&self, state: &mut dyn Any, key: &str, event: &Event) -> maka_ui::Result<WidgetInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        if self.0.props.disabled || !owner.visible || owner.selection.busy() {
            return Ok(WidgetInput::Unhandled);
        }
        let activate = matches!(event, Event::Key(k) if (k.code == KeyCode::Enter || key == "choose" && k.code == KeyCode::Char(' ')) && k.modifiers.is_empty())
            || key == "choose"
                && matches!(event, Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left));
        if activate {
            owner.selection.cancel_intent();
            if let Some(choose) = owner.selection.stage() {
                owner.path = Some(PathBuf::from(owner.editor.text()));
                return Ok(WidgetInput::Emitted(NativeInput {
                    key: "choose".into(),
                    value: serde_json::to_value(choose).unwrap(),
                }));
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
    fn pointer_capture(&self, state: &dyn Any) -> bool {
        state.downcast_ref::<Owner>().unwrap().editor.dragging()
    }
    fn capture(
        &self,
        state: &dyn Any,
        target: &maka_ui::actions::CaptureTarget,
    ) -> maka_ui::Result<Arc<dyn maka_plugins::call::ReadResource>> {
        let capture = serde_json::from_value(target.input().clone())
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        state
            .downcast_ref::<Owner>()
            .unwrap()
            .selection
            .capture(&capture)
    }
    fn measure(&self, _: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u32)> {
        Ok((width, 3))
    }
    fn paint(
        &self,
        slots: &mut Slots<'_>,
        area: Placement,
        buffer: &mut Buffer,
    ) -> maka_ui::Result<()> {
        let owner = slots.native_state::<Owner>()?;
        let a = &self.0.props.appearance;
        let color = |c: maka_ui::appearance::Color| {
            Color::Rgb((c.0 >> 24) as u8, (c.0 >> 16) as u8, (c.0 >> 8) as u8)
        };
        let style = Style::default()
            .fg(color(a.text.foreground))
            .bg(color(a.background));
        let row = |y| {
            Placement::new(
                super::CellRect::new(area.x, area.y + y, area.width, 1),
                area.visible(),
            )
        };
        super::paint::text(&self.0.props.path_label, row(0), buffer, style);
        let path = row(1);
        let geometry = owner.editor.paint_prepared(
            path,
            buffer,
            slots.focused_key() == Some("path"),
            style,
            style.bg(color(a.selection)),
            style.add_modifier(Modifier::REVERSED),
        );
        if owner.editor.text().is_empty() {
            super::paint::text(
                &self.0.props.placeholder,
                path,
                buffer,
                style.fg(color(a.muted_foreground)),
            );
        }
        super::paint::text(
            &self.0.props.label,
            row(2),
            buffer,
            if slots.focused_key() == Some("choose") {
                style.add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                style.add_modifier(Modifier::BOLD)
            },
        );
        let enabled = !self.0.props.disabled && !owner.selection.busy();
        slots.native_geometry(Box::new(geometry))?;
        if enabled {
            slots.native_target("path", path, self.0.props.tab_index)?;
            slots.native_target("choose", row(2), self.0.props.tab_index)?;
        }
        Ok(())
    }
    fn present(&self, state: &mut dyn Any, geometry: Box<dyn Any>) -> Option<NativeInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        let geometry = *geometry.downcast::<Geometry>().unwrap();
        owner.editor.present_geometry(geometry.clone());
        owner.geometry = Some(geometry);
        None
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool) {
        let owner = state.downcast_mut::<Owner>().unwrap();
        owner.visible = visible;
        if !visible {
            owner.editor.invalidate_geometry();
            owner.geometry = None;
            owner.selection.cancel_intent();
            owner.path = None;
        }
    }
    fn deactivate(&self, state: &mut dyn Any) {
        self.presentation(state, false);
    }
}
struct Component;
impl NativeComponent<dyn Widget> for Component {
    fn capture(
        &self,
        _: maka_ui::actions::CaptureCall<'_>,
    ) -> maka_ui::Result<maka_ui::actions::CapturePlan> {
        Ok(maka_ui::actions::CapturePlan::Native)
    }
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
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
        .register("maka.ui.file", |_, _| {
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
    fn props() -> FilePickerProps {
        serde_json::from_value(serde_json::json!({
        "appearance": {"text":{"font":{"family":".SystemUIFont","fallbacks":[],"size":14,"lineHeight":1.5,"weight":400},"foreground":0x242724ffu32},
            "mutedForeground":0x656a63ffu32,"background":0xfaf9f6ffu32,"selection":0xa7c4b0ffu32,"caret":0x242724ffu32,
            "border":0x656a63ffu32,"focusedBorder":0x416653ffu32,"borderWidth":1,"radius":8,"padding":{"top":8,"right":12,"bottom":8,"left":12}},
        "label":"Choose plugin file","pathLabel":"Local file path","placeholder":"/absolute/path","disabled":false,"tabIndex":0
    })).unwrap()
    }

    #[test]
    fn native_path_is_private_and_one_committed_gesture_reads_an_immutable_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("selected.maka-extension");
        std::fs::write(&path, b"original").unwrap();
        let props = props();
        let resources = Arc::new(maka_plugins::call::Resources::default());
        let mut state = Recipe(FileView {
            props: props.clone(),
            request: Some("forged".into()),
        })
        .create(NativeContext {
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
            origin: maka_ui::EventOrigin::Raw,
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
        )
        .unwrap();
        let draw = |tui: &mut Presenter| {
            let area = Rect::new(0, 0, 30, 4);
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
                row: 1,
                modifiers: KeyModifiers::NONE,
            })
        };
        tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 1))
            .unwrap();
        assert!(tui.drag.is_some());
        tui.input(mouse(MouseEventKind::Drag(MouseButton::Left), 4))
            .unwrap();
        tui.input(mouse(MouseEventKind::Up(MouseButton::Left), 4))
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
        let root = tui.root.clone();
        tui.surface_closed(&root, Ok(())).unwrap();
        tui.shutdown().unwrap();
    }
}
