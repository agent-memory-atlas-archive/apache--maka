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

use super::{NativeInput, Slots, Widget, WidgetInput};
use crate::editor::{Editor, Geometry};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use maka_ui::{
    ComponentCall, ComponentOutput, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, textarea::*,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};
use serde_json::{Value, json};
use std::{any::Any, collections::BTreeMap, ops::Range, rc::Rc, sync::Arc};

struct Owner {
    editor: Editor,
    presented: Option<Geometry>,
    id: String,
    revision: u64,
    last_edit: u64,
    last_selection: (Range<usize>, bool),
    last_request: Option<TextareaCommand>,
    sequence: u64,
    receipt: Option<TextareaReceipt>,
    announced: bool,
}
impl Owner {
    fn new(props: &TextareaProps) -> Self {
        let mut editor = Editor::bounded(64 * 1024, "composer-too-large");
        editor.insert(&props.initial_text);
        editor.clear_history();
        let last_edit = editor.revision();
        let last_selection = (editor.selection_range(), editor.selection_reversed());
        Self {
            editor,
            presented: None,
            id: uuid::Uuid::new_v4().to_string(),
            revision: 0,
            last_edit,
            last_selection,
            last_request: None,
            sequence: 0,
            receipt: None,
            announced: false,
        }
    }
    fn refresh(&mut self) -> bool {
        let edit = self.editor.revision();
        let selection = (
            self.editor.selection_range(),
            self.editor.selection_reversed(),
        );
        if edit != self.last_edit || selection != self.last_selection {
            self.last_edit = edit;
            self.last_selection = selection;
            self.revision += 1;
            return true;
        }
        false
    }
    fn observation(&self) -> NativeInput {
        let range = self.editor.selection_range();
        let selection = TextRange {
            start: self.editor.text()[..range.start].encode_utf16().count(),
            end: self.editor.text()[..range.end].encode_utf16().count(),
        };
        let observation = TextareaObservation {
            revision: self.revision,
            snapshot: TextareaSnapshot {
                text: TextareaTextRef {
                    owner: self.id.clone(),
                    revision: self.revision,
                    bytes: self.editor.text().len(),
                },
                selection,
                reversed: self.editor.selection_reversed(),
                marked: None,
            },
            receipt: self.receipt.clone(),
        };
        NativeInput {
            key: "observe".into(),
            value: serde_json::to_value(observation).expect("native textarea observation"),
        }
    }
    fn apply(&mut self, command: &TextareaCommand) {
        self.last_request = Some(command.clone());
        let outcome = if command.owner != self.id {
            TextareaOutcome::Conflict {
                reason: TextareaConflict::Owner,
            }
        } else if command.sequence <= self.sequence {
            TextareaOutcome::Conflict {
                reason: TextareaConflict::Sequence,
            }
        } else {
            self.sequence = command.sequence;
            if command.based_on != self.revision {
                TextareaOutcome::Conflict {
                    reason: TextareaConflict::Revision,
                }
            } else {
                let (TextareaEdit::Replace { text } | TextareaEdit::Reset { text }) = &command.edit;
                if !representable(text) {
                    TextareaOutcome::Conflict {
                        reason: TextareaConflict::Representation,
                    }
                } else {
                    match &command.edit {
                        TextareaEdit::Replace { text } => {
                            self.editor.select_all();
                            self.editor.insert(text);
                        }
                        TextareaEdit::Reset { text } => {
                            self.editor.clear();
                            self.editor.insert(text);
                            self.editor.clear_history();
                        }
                    }
                    self.refresh();
                    TextareaOutcome::Applied
                }
            }
        };
        self.receipt = Some(TextareaReceipt {
            owner: command.owner.clone(),
            sequence: command.sequence,
            revision: self.revision,
            outcome,
        });
    }
}
fn representable(text: &str) -> bool {
    text.len() <= 64 * 1024
        && !text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
}
fn color(color: maka_ui::appearance::Color) -> Color {
    let n = color.0;
    Color::Rgb((n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8)
}
struct Recipe {
    props: TextareaProps,
}
impl Widget for Recipe {
    fn create(&self) -> Box<dyn Any> {
        Box::new(Owner::new(&self.props))
    }
    fn commit(&self, state: &mut dyn Any) -> maka_ui::Result<Option<NativeInput>> {
        let owner = state
            .downcast_mut::<Owner>()
            .expect("terminal editor owner");
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
    fn input(&self, state: &mut dyn Any, event: &Event) -> maka_ui::Result<WidgetInput> {
        let owner = state
            .downcast_mut::<Owner>()
            .expect("terminal editor owner");
        if self.props.disabled {
            return Ok(WidgetInput::Consumed(None));
        }
        let handled = match event {
            Event::Key(key) => {
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
            Event::Paste(text) if !self.props.read_only => owner.editor.insert(text),
            Event::Mouse(mouse) => owner
                .presented
                .as_ref()
                .is_some_and(|geometry| owner.editor.mouse_presented(*mouse, geometry)),
            _ => false,
        };
        let observed = handled && owner.refresh();
        Ok(WidgetInput::Consumed(observed.then(|| owner.observation())))
    }
    fn capture(
        &self,
        state: &dyn Any,
        target: &maka_ui::actions::CaptureTarget,
    ) -> maka_ui::Result<Arc<dyn maka_plugins::call::ReadResource>> {
        if target.method() != "snapshot" {
            return Err(maka_ui::Error::Invalid(
                "unknown terminal editor capture".into(),
            ));
        }
        let owner = state
            .downcast_ref::<Owner>()
            .expect("terminal editor owner");
        let capture: TextareaCapture = serde_json::from_value(target.input().clone())
            .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
        if matches!(capture,TextareaCapture::AtRevision {owner:ref expected,revision} if expected!=&owner.id || revision!=owner.revision)
        {
            return Err(maka_ui::Error::Invalid("editor revision changed".into()));
        }
        Ok(Arc::new(Snapshot {
            owner: owner.id.clone(),
            text: owner.editor.text().to_owned(),
            revision: owner.revision,
        }))
    }
    fn measure(&self, slots: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u16)> {
        let owner = slots.native_state::<Owner>()?;
        Ok((
            width,
            owner
                .editor
                .rows(width.saturating_sub(2))
                .clamp(3, 8)
                .saturating_add(2),
        ))
    }
    fn paint(&self, slots: &mut Slots<'_>, area: Rect, buffer: &mut Buffer) -> maka_ui::Result<()> {
        let owner = slots.native_state::<Owner>()?;
        let focused = slots.focused();
        let appearance = &self.props.appearance;
        buffer.set_style(
            area,
            Style::default()
                .bg(color(appearance.background))
                .fg(color(appearance.text.foreground)),
        );
        let inner = area.inner(ratatui::layout::Margin::new(1, 1));
        let geometry = owner.editor.paint_prepared(
            inner,
            buffer,
            focused,
            Style::default()
                .fg(color(appearance.text.foreground))
                .bg(color(appearance.background)),
            Style::default()
                .bg(color(appearance.selection))
                .fg(color(appearance.text.foreground)),
            Style::default().add_modifier(Modifier::REVERSED),
        );
        if owner.editor.text().is_empty() {
            super::paint::text(
                &self.props.placeholder,
                inner,
                buffer,
                Style::default().fg(color(appearance.muted_foreground)),
            );
        }
        slots.native_geometry(Box::new(geometry))?;
        if !self.props.disabled {
            slots.native_target("observe", area, self.props.tab_index)?;
        }
        Ok(())
    }
    fn present(&self, state: &mut dyn Any, geometry: Box<dyn Any>) {
        let owner = state
            .downcast_mut::<Owner>()
            .expect("terminal editor owner");
        let geometry = *geometry.downcast::<Geometry>().expect("editor geometry");
        owner.editor.present_geometry(geometry.clone());
        owner.presented = Some(geometry);
    }
    fn deactivate(&self, state: &mut dyn Any) {
        let owner = state
            .downcast_mut::<Owner>()
            .expect("terminal editor owner");
        owner.editor.invalidate_geometry();
        owner.presented = None;
    }
}
struct Snapshot {
    owner: String,
    text: String,
    revision: u64,
}
impl maka_plugins::call::ReadResource for Snapshot {
    fn resource_type(&self) -> maka_plugin_core::contract::ResourceType {
        text_resource_type()
    }
    fn bytes(&self) -> usize {
        self.text.len()
    }
    fn metadata(&self) -> Value {
        json!({"owner":self.owner,"revision":self.revision,"bytes":self.text.len()})
    }
    fn read(&self, range: Range<usize>) -> Result<Vec<u8>, String> {
        if range
            .end
            .checked_sub(range.start)
            .is_none_or(|n| n > 64 * 1024)
        {
            return Err("invalid text range or chunk exceeds 64 KiB".into());
        }
        self.text
            .get(range)
            .map(|text| text.as_bytes().to_vec())
            .ok_or_else(|| "invalid editor UTF-8 range".into())
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
        let output = maka_ui::textarea::reduce(call)?;
        let props = output.frame.view.clone();
        if !props.soft_wrap {
            return Err(maka_ui::Error::Invalid(
                "terminal editor requires soft wrapping".into(),
            ));
        }
        if !representable(&props.initial_text) {
            return Err(maka_ui::Error::Invalid(
                "initial text cannot be represented in terminal editor".into(),
            ));
        }
        let view = NativeView::<dyn Widget>::new(
            Rc::new(Recipe { props }),
            BTreeMap::from([("observe".into(), observation_schema())]),
            vec![],
        )?;
        Ok(output.map_view(|_| view))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register("maka.ui.textarea", |_, _| {
            Ok(BTreeMap::from([(
                "Textarea".into(),
                NativeExport {
                    contract: textarea_contract(),
                    value: NativeCapability::Component(Rc::new(Component)),
                },
            )]))
        })
        .expect("unique terminal textarea");
}
