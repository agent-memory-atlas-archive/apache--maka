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

use super::Placement;
use super::editor::Owner;
use super::{NativeInput, Slots, Widget, WidgetInput};
use crate::editor::{Editor, Geometry};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use maka_ui_runtime::{
    ComponentCall, ComponentOutput, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, editor::*, input::*,
};
use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
};
use std::{any::Any, collections::BTreeMap, rc::Rc, sync::Arc};

fn color(color: maka_ui_runtime::appearance::Color) -> Color {
    let n = color.0;
    Color::Rgb((n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8)
}
struct Recipe {
    props: InputProps,
    secret: bool,
}
impl Widget for Recipe {
    fn publication_capacity(&self, state: &dyn Any) -> Vec<(&'static str, usize)> {
        let owner = state.downcast_ref::<Owner>().unwrap();
        if !owner.announced
            || self
                .props
                .command
                .as_ref()
                .is_some_and(|command| owner.last_request.as_ref() != Some(command))
        {
            vec![("observe", 4096)]
        } else {
            Vec::new()
        }
    }

    fn cancel_pointer(&self, state: &mut dyn Any) {
        state
            .downcast_mut::<Owner>()
            .unwrap()
            .editor
            .cancel_pointer();
    }
    fn create(&self, _: super::NativeContext) -> Box<dyn Any> {
        Box::new(Owner::new(
            Editor::single_line(self.props.max_bytes as usize, self.secret),
            "",
        ))
    }
    fn commit(&self, state: &mut dyn Any) -> maka_ui_runtime::Result<Option<NativeInput>> {
        let owner = state
            .downcast_mut::<Owner>()
            .expect("terminal editor owner");
        assert_eq!(owner.editor.secret(), self.secret, "input export changed");
        let changed = self
            .props
            .command
            .as_ref()
            .is_some_and(|command| owner.last_request.as_ref() != Some(command));
        if changed {
            owner.apply(self.props.command.as_ref().unwrap());
        }
        if changed || !owner.announced {
            owner.announced = true;
            Ok(Some(owner.observation()))
        } else {
            Ok(None)
        }
    }
    fn present_focus(&self, state: &mut dyn Any) -> bool {
        state
            .downcast_mut::<Owner>()
            .expect("terminal editor owner")
            .focus
            .presented()
    }
    fn prepare_focus(&self, state: &mut dyn Any, visible: bool) {
        state
            .downcast_mut::<Owner>()
            .expect("terminal editor owner")
            .focus
            .reconcile(visible && !self.props.disabled && self.props.auto_focus);
    }
    fn input(
        &self,
        state: &mut dyn Any,
        _: &str,
        event: &Event,
    ) -> maka_ui_runtime::Result<WidgetInput> {
        let owner = state
            .downcast_mut::<Owner>()
            .expect("terminal editor owner");
        if self.props.disabled {
            return Ok(if matches!(event, Event::Key(_)) {
                WidgetInput::Unhandled
            } else {
                WidgetInput::Consumed(None)
            });
        }
        let handled = match event {
            Event::Key(key) => {
                if key.modifiers.is_empty() && matches!(key.code, KeyCode::Up | KeyCode::Down) {
                    return Ok(WidgetInput::Navigate);
                }
                if key.code == KeyCode::Enter && key.modifiers.is_empty() && !self.props.read_only {
                    let input = NativeInput {
                        replaceable: false,
                        key: "submit".into(),
                        value: serde_json::json!({}),
                    };
                    owner.refresh();
                    return Ok(match &self.props.submit_copy {
                        Some(copy) if owner.snapshot().text == copy.basis => {
                            WidgetInput::SubmitCopy(input, copy.clone())
                        }
                        Some(_) => WidgetInput::Consumed(Some(owner.observation())),
                        None => WidgetInput::Emitted(input),
                    });
                }
                let navigation = matches!(
                    key.code,
                    KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Up
                        | KeyCode::Down
                        | KeyCode::Home
                        | KeyCode::End
                ) || key.code == KeyCode::Char('a')
                    && key.modifiers.contains(KeyModifiers::CONTROL);
                if self.props.read_only && !navigation {
                    false
                } else {
                    owner.editor.key(*key)
                }
            }
            Event::Paste(text) if !self.props.read_only => match self.props.terminal_paste {
                TerminalPaste::RejectControls => owner.editor.insert(text),
                TerminalPaste::Spaces => owner
                    .editor
                    .insert(&text.replace("\r\n", " ").replace(['\r', '\n', '\t'], " ")),
            },
            Event::Mouse(mouse) => owner
                .presented
                .as_ref()
                .is_some_and(|geometry| owner.editor.mouse_presented(*mouse, geometry)),
            _ => false,
        };
        let observed = handled && owner.refresh();
        Ok(if handled {
            WidgetInput::Consumed(observed.then(|| owner.observation()))
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
        target: &maka_ui_runtime::actions::CaptureTarget,
    ) -> maka_ui_runtime::Result<Arc<dyn maka_capabilities::call::ReadResource>> {
        state
            .downcast_ref::<Owner>()
            .expect("terminal editor owner")
            .capture(target)
    }

    fn measure(&self, slots: &Slots<'_>, width: u16) -> maka_ui_runtime::Result<(u16, u32)> {
        let _ = slots.native_state::<Owner>()?;
        Ok((
            width,
            if self.props.appearance.border_width > 0.0 {
                3
            } else {
                1
            },
        ))
    }

    fn paint(
        &self,
        slots: &mut Slots<'_>,
        area: Placement,
        buffer: &mut Buffer,
    ) -> maka_ui_runtime::Result<()> {
        if area.is_empty() {
            if !self.props.disabled {
                slots.native_target("observe", area, self.props.tab_index, false)?;
            }
            return Ok(());
        }
        let owner = slots.native_state::<Owner>()?;
        let focused = slots.focused();
        let appearance = &self.props.appearance;
        let terminal = appearance.terminal.as_ref();
        let foreground = terminal.map_or_else(
            || color(appearance.text.foreground),
            |p| super::paint::color(p.foreground),
        );
        let background = terminal.map_or_else(
            || color(appearance.background),
            |p| super::paint::color(p.background),
        );
        let selection = terminal.map_or_else(
            || color(appearance.selection),
            |p| super::paint::color(p.selection),
        );
        let selection_text = terminal.map_or(foreground, |p| super::paint::color(p.selection_text));
        let muted = terminal.map_or_else(
            || color(appearance.muted_foreground),
            |p| super::paint::color(p.subtle),
        );
        let border = terminal.map_or_else(
            || {
                color(if focused {
                    appearance.focused_border
                } else {
                    appearance.border
                })
            },
            |p| super::paint::color(if focused { p.accent } else { p.border }),
        );
        buffer.set_style(
            area.visible(),
            Style::default().bg(background).fg(foreground),
        );
        let inner = if appearance.border_width > 0.0 {
            super::border::paint(
                buffer,
                area,
                border,
                if appearance.ascii {
                    maka_ui_runtime::scene::terminal::BorderStyle::Ascii
                } else if appearance.radius > 0.0 {
                    maka_ui_runtime::scene::terminal::BorderStyle::Rounded
                } else {
                    maka_ui_runtime::scene::terminal::BorderStyle::Square
                },
            );
            super::border::inner(area)
        } else {
            area
        };
        let geometry = owner.editor.paint_prepared(
            inner,
            buffer,
            focused,
            Style::default().fg(foreground).bg(background),
            Style::default().bg(selection).fg(selection_text),
            Style::default().add_modifier(Modifier::REVERSED),
        );
        if owner.editor.text().is_empty() {
            super::paint::text(
                &self.props.placeholder,
                inner,
                buffer,
                Style::default().fg(muted),
            );
        }
        let autofocus = owner.focus.pending();
        slots.native_geometry(Box::new(geometry))?;
        if !self.props.disabled {
            slots.native_target("observe", area, self.props.tab_index, autofocus)?;
        }
        Ok(())
    }
    fn present(&self, state: &mut dyn Any, geometry: Box<dyn Any>) -> Option<NativeInput> {
        let owner = state
            .downcast_mut::<Owner>()
            .expect("terminal editor owner");
        let geometry = *geometry.downcast::<Geometry>().expect("editor geometry");
        owner.editor.present_geometry(geometry.clone());
        owner.presented = Some(geometry);
        None
    }
    fn deactivate(&self, state: &mut dyn Any) {
        let owner = state
            .downcast_mut::<Owner>()
            .expect("terminal editor owner");
        owner.invalidate_geometry();
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool) {
        if !visible {
            state
                .downcast_mut::<Owner>()
                .expect("terminal editor owner")
                .invalidate_geometry();
        }
    }
}
struct Component {
    secret: bool,
}
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
        let output = maka_ui_runtime::input::reduce(call)?;
        let props = output.frame.view.clone();
        let view = NativeView::<dyn Widget>::new(
            Rc::new(Recipe {
                props,
                secret: self.secret,
            }),
            BTreeMap::from([
                ("observe".into(), observation_schema()),
                ("submit".into(), submitted_schema()),
            ]),
            vec![],
        )?;
        Ok(output.map_view(|_| view))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register(maka_ui_runtime::input::IMPLEMENTATION, |_, _| {
            Ok(BTreeMap::from([
                (
                    "TextInput".into(),
                    NativeExport {
                        contract: text_input_contract(),
                        value: NativeCapability::Component(Rc::new(Component { secret: false })),
                    },
                ),
                (
                    "SecretInput".into(),
                    NativeExport {
                        contract: secret_input_contract(),
                        value: NativeCapability::Component(Rc::new(Component { secret: true })),
                    },
                ),
            ]))
        })
        .expect("unique terminal input");
}
