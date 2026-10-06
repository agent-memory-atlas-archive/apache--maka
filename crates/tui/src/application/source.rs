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

//! Literal source with a standalone native byte-selection scope.
use super::{NativeInput, Placement, Slots, Widget, WidgetInput};
use crate::editor::layout::Layout;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use maka_ui::{
    ComponentCall, ComponentOutput, Frame, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, source::*,
};
use ratatui::{
    buffer::Buffer,
    layout::Position,
    style::{Color, Modifier, Style},
};
use serde_json::json;
use std::{any::Any, collections::BTreeMap, ops::Range, rc::Rc, sync::Arc};
use unicode_width::UnicodeWidthStr;

#[derive(Clone)]
struct Geometry {
    source: Arc<str>,
    layout: Layout,
    area: Placement,
}
struct Owner {
    source: Arc<str>,
    anchor: Option<usize>,
    cursor: usize,
    dragging: bool,
    disabled: bool,
    geometry: Option<Geometry>,
}
impl Owner {
    fn range(&self) -> Range<usize> {
        let anchor = self.anchor.unwrap_or(self.cursor);
        anchor.min(self.cursor)..anchor.max(self.cursor)
    }
    fn clear(&mut self) {
        self.anchor = None;
        self.cursor = 0;
        self.dragging = false;
    }
    fn apply(&mut self, props: &SourceTextProps) {
        if self.source.as_ref() != props.source {
            if !props.source.starts_with(self.source.as_ref()) {
                self.clear();
                self.geometry = None;
            }
            self.source = Arc::from(props.source.as_str());
        }
        self.disabled = props.disabled;
        if self.disabled {
            self.clear();
            self.geometry = None;
        }
    }
    fn input(&mut self, event: &Event) -> bool {
        if self.disabled {
            return false;
        }
        let Some(geometry) = &self.geometry else {
            return false;
        };
        match event {
            Event::Mouse(mouse) => {
                if geometry.source.as_ref() != self.source.as_ref() {
                    self.dragging = false;
                    return false;
                }
                let point = Position::new(mouse.column, mouse.row);
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) if geometry.area.contains(point) => {
                        let byte = geometry.layout.pointer(geometry.area, 0, point).0;
                        if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                            self.anchor.get_or_insert(self.cursor);
                        } else {
                            self.anchor = Some(byte);
                        }
                        self.cursor = byte;
                        self.dragging = true;
                    }
                    MouseEventKind::Drag(MouseButton::Left) if self.dragging => {
                        self.cursor = geometry.layout.pointer(geometry.area, 0, point).0;
                    }
                    MouseEventKind::Up(MouseButton::Left) if self.dragging => {
                        self.dragging = false;
                    }
                    _ => return false,
                }
            }
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if matches!(key.code, KeyCode::Char('a' | 'A'))
                    && key.modifiers == KeyModifiers::CONTROL
                {
                    self.anchor = Some(0);
                    self.cursor = self.source.len();
                    self.dragging = false;
                    return true;
                }
                if key.code == KeyCode::Esc {
                    self.clear();
                    return true;
                }
                if !key.modifiers.intersects(KeyModifiers::SHIFT) {
                    return false;
                }
                let layout = Layout::literal(&self.source, geometry.area.width);
                let (row, column) = layout.cursor(self.cursor, false);
                let next = match key.code {
                    KeyCode::Left => layout.previous(self.cursor),
                    KeyCode::Right => layout.next(self.cursor),
                    KeyCode::Up => layout.byte_at(row.saturating_sub(1), column),
                    KeyCode::Down => layout.byte_at(row.saturating_add(1), column),
                    KeyCode::Home => layout.rows[row].start,
                    KeyCode::End => layout.rows[row].end,
                    _ => return false,
                };
                self.anchor.get_or_insert(self.cursor);
                self.cursor = next;
                self.dragging = false;
            }
            _ => return false,
        }
        true
    }
}
struct Recipe(SourceTextProps);
fn color(rgba: u32) -> Color {
    Color::Rgb((rgba >> 24) as u8, (rgba >> 16) as u8, (rgba >> 8) as u8)
}
impl Widget for Recipe {
    fn create(&self) -> Box<dyn Any> {
        Box::new(Owner {
            source: Arc::from(self.0.source.as_str()),
            anchor: None,
            cursor: 0,
            dragging: false,
            disabled: self.0.disabled,
            geometry: None,
        })
    }
    fn commit(&self, state: &mut dyn Any) -> maka_ui::Result<Option<NativeInput>> {
        state.downcast_mut::<Owner>().unwrap().apply(&self.0);
        Ok(None)
    }
    fn input(&self, state: &mut dyn Any, _: &str, event: &Event) -> maka_ui::Result<WidgetInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        Ok(
            if !self.0.disabled && self.0.source == owner.source.as_ref() && owner.input(event) {
                WidgetInput::Consumed(None)
            } else {
                WidgetInput::Unhandled
            },
        )
    }
    fn pointer_capture(&self, state: &dyn Any) -> bool {
        let owner = state.downcast_ref::<Owner>().unwrap();
        !self.0.disabled
            && self.0.source == owner.source.as_ref()
            && owner.dragging
            && owner
                .geometry
                .as_ref()
                .is_some_and(|geometry| geometry.source.as_ref() == owner.source.as_ref())
    }
    fn copy(&self, state: &dyn Any) -> Option<String> {
        let owner = state.downcast_ref::<Owner>().unwrap();
        (!self.0.disabled
            && !owner.disabled
            && self.0.source.starts_with(owner.source.as_ref())
            && !owner.range().is_empty())
        .then(|| owner.source[owner.range()].to_owned())
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool) {
        if !visible {
            let owner = state.downcast_mut::<Owner>().unwrap();
            owner.geometry = None;
            owner.dragging = false;
        }
    }
    fn deactivate(&self, state: &mut dyn Any) {
        let owner = state.downcast_mut::<Owner>().unwrap();
        owner.clear();
        owner.geometry = None;
    }
    fn measure(&self, _: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u32)> {
        let layout = Layout::literal(&self.0.source, width);
        Ok((
            layout
                .rows
                .iter()
                .map(|row| row.cells.last().map_or(0, |cell| cell.column + cell.width))
                .max()
                .unwrap_or(0)
                .min(usize::from(width)) as u16,
            layout.rows.len() as u32,
        ))
    }
    fn paint(
        &self,
        slots: &mut Slots<'_>,
        area: Placement,
        buffer: &mut Buffer,
    ) -> maka_ui::Result<()> {
        let owner = slots.native_state::<Owner>()?;
        let layout = Layout::literal(&owner.source, area.width);
        let range = owner.range();
        let mut normal = Style::default().fg(color(self.0.appearance.foreground.0));
        if self.0.appearance.font.weight >= 600 {
            normal = normal.add_modifier(Modifier::BOLD);
        }
        let selected = normal.bg(color(self.0.selection.0));
        let visible = area.visible();
        let first = (i64::from(visible.y) - area.y).max(0) as usize;
        for (row_index, row) in layout
            .rows
            .iter()
            .enumerate()
            .skip(first)
            .take(usize::from(visible.height))
        {
            for cell in &row.cells {
                let Some(point) = area.point(cell.column, row_index) else {
                    continue;
                };
                if area
                    .point(cell.column + cell.width - 1, row_index)
                    .is_none()
                {
                    continue;
                }
                let literal = &owner.source[cell.bytes.clone()];
                let style = if cell.bytes.start < range.end && cell.bytes.end > range.start {
                    selected
                } else {
                    normal
                };
                if literal == "\t" {
                    for column in 0..cell.width {
                        buffer[(point.x + column as u16, point.y)]
                            .set_symbol(" ")
                            .set_style(style);
                    }
                } else {
                    let symbol =
                        if literal.chars().any(char::is_control) || literal.width() > cell.width {
                            "�".to_owned()
                        } else if literal.width() == 0 {
                            format!("◌{literal}")
                        } else {
                            literal.to_owned()
                        };
                    buffer.set_stringn(point.x, point.y, symbol, cell.width, style);
                }
            }
            let break_end = layout
                .rows
                .get(row_index + 1)
                .map_or(row.end, |next| next.start);
            if row.end < range.end && break_end > range.start && break_end > row.end {
                let column = row.cells.last().map_or(0, |cell| cell.column + cell.width);
                if let Some(point) = area.point(
                    column.min(usize::from(area.width.saturating_sub(1))),
                    row_index,
                ) {
                    buffer[(point.x, point.y)].set_style(selected);
                }
            }
        }
        if slots.focused() && range.is_empty() {
            let (row, column) = layout.cursor(owner.cursor, false);
            if let Some(point) = area.point(column, row) {
                buffer[(point.x, point.y)].set_style(normal.add_modifier(Modifier::UNDERLINED));
            }
        }
        let geometry = Geometry {
            source: owner.source.clone(),
            layout,
            area,
        };
        slots.native_geometry(Box::new(geometry))?;
        if !self.0.disabled {
            slots.native_target("source", area, 0)?;
        }
        Ok(())
    }
    fn present(&self, state: &mut dyn Any, geometry: Box<dyn Any>) -> Option<NativeInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        let geometry = *geometry.downcast::<Geometry>().unwrap();
        if !owner.disabled && owner.source.starts_with(geometry.source.as_ref()) {
            owner.geometry = Some(geometry);
        }
        None
    }
}
struct Component;
impl NativeComponent<dyn Widget> for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let (props, revision) = match call {
            ComponentCall::Mount { props } => (props, 0),
            ComponentCall::Update { props, state } => {
                (props, state["revision"].as_u64().unwrap() + 1)
            }
            ComponentCall::Event { .. } => {
                return Err(maka_ui::Error::Invalid(
                    "SourceText has no component events".into(),
                ));
            }
        };
        Ok(Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: compile(props)?,
        }
        .output(vec![]))
    }
}
pub(super) fn compile(props: &serde_json::Value) -> maka_ui::Result<NativeView<dyn Widget>> {
    let declaration = props.clone();
    let props: SourceTextProps = serde_json::from_value(props.clone())
        .map_err(|error| maka_ui::Error::Invalid(error.to_string()))?;
    if props.source.len() > 65536 {
        return Err(maka_ui::Error::Invalid("source byte budget".into()));
    }
    NativeView::<dyn Widget>::new(Rc::new(Recipe(props)), BTreeMap::new(), vec![])?
        .with_declaration(declaration)
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register("maka.ui.source", |_, _| {
            Ok(BTreeMap::from([(
                "SourceText".into(),
                NativeExport {
                    contract: source_contract(),
                    value: NativeCapability::Component(Rc::new(Component)),
                },
            )]))
        })
        .expect("unique source implementation");
}
