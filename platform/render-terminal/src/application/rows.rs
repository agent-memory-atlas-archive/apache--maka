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

//! Compact row interaction extracted from the original Surface and queue view.
//! Selection/hover/scroll remain local; only explicit stable actions leave Rust.
use super::{CellRect, NativeInput, Placement, Slots, Widget, WidgetInput, paint};
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use maka_ui_runtime::{
    ComponentCall, ComponentOutput, Emission, Frame, NativeCapability, NativeComponent,
    NativeExport, NativeRegistry, NativeView, rows::*,
};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
};
use serde_json::json;
use std::{any::Any, collections::BTreeMap, rc::Rc};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Default)]
struct Owner {
    selected: Option<String>,
    hover: Option<String>,
    offset: usize,
    reveal: bool,
    exit: bool,
    geometry: Option<Geometry>,
    drag: Option<Rect>,
}
struct Hit {
    row: String,
    action: Option<String>,
    area: Placement,
}
struct Geometry {
    props: Rc<RowsProps>,
    area: Placement,
    hits: Vec<Hit>,
    offset: usize,
    rows: usize,
    bar: Option<Rect>,
    intent: (usize, bool, Option<String>),
}
struct Rows {
    props: Rc<RowsProps>,
}
impl Rows {
    fn max_rows(&self, slots: &Slots<'_>) -> usize {
        usize::from(
            self.props
                .compact_below
                .filter(|compact| slots.viewport().height < compact.height)
                .map_or(self.props.max_rows, |compact| compact.max_rows),
        )
    }
    fn action(&self, row: &str, key: &str) -> WidgetInput {
        let Some(action) = self
            .props
            .rows
            .iter()
            .find(|item| item.key == row)
            .and_then(|item| {
                item.actions
                    .iter()
                    .find(|action| action.key == key && action.enabled)
            })
        else {
            return WidgetInput::Consumed(None);
        };
        WidgetInput::Emitted(NativeInput {
            replaceable: false,
            key: "activate".into(),
            value: json!({"revision":self.props.revision,"row":row,"action":action.key}),
        })
    }
}
impl Widget for Rows {
    fn cancel_pointer(&self, state: &mut dyn Any) {
        state.downcast_mut::<Owner>().unwrap().drag = None;
    }
    fn pointer_capture(&self, state: &dyn Any) -> bool {
        state.downcast_ref::<Owner>().unwrap().drag.is_some()
    }
    fn create(&self, _: super::NativeContext) -> Box<dyn Any> {
        Box::new(Owner {
            selected: self.props.rows.first().map(|row| row.key.clone()),
            ..Default::default()
        })
    }
    fn same_presentation(&self, other: &dyn Any) -> bool {
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| self.props == other.props)
    }
    fn returns_focus(&self) -> bool {
        true
    }
    fn enter_focus(&self, state: &mut dyn Any, _: &str) -> bool {
        let owner = state.downcast_mut::<Owner>().unwrap();
        if owner
            .geometry
            .as_ref()
            .is_none_or(|geometry| geometry.props != self.props)
        {
            return false;
        }
        owner.selected = self
            .props
            .entry
            .clone()
            .or_else(|| owner.selected.clone())
            .or_else(|| self.props.rows.first().map(|row| row.key.clone()));
        owner.reveal = true;
        owner.drag = None;
        owner.hover = None;
        owner.selected.is_some()
    }
    fn take_focus_exit(&self, state: &mut dyn Any) -> bool {
        std::mem::take(&mut state.downcast_mut::<Owner>().unwrap().exit)
    }
    fn commit(&self, state: &mut dyn Any) -> maka_ui_runtime::Result<Option<NativeInput>> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        if owner
            .selected
            .as_ref()
            .is_some_and(|key| !self.props.rows.iter().any(|row| &row.key == key))
        {
            owner.selected = None;
            owner.exit = true;
            owner.drag = None;
        }
        if owner
            .hover
            .as_ref()
            .is_some_and(|key| !self.props.rows.iter().any(|row| &row.key == key))
        {
            owner.hover = None;
        }
        owner.offset = owner.offset.min(self.props.rows.len().saturating_sub(1));
        Ok(None)
    }
    fn hover(&self, state: &mut dyn Any, _: &str, point: Option<Position>) -> bool {
        let owner = state.downcast_mut::<Owner>().unwrap();
        let hover = point.and_then(|point| {
            if owner
                .geometry
                .as_ref()?
                .bar
                .is_some_and(|bar| bar.contains(point))
            {
                return None;
            }
            owner
                .geometry
                .as_ref()?
                .hits
                .iter()
                .rev()
                .find(|hit| hit.area.contains(point))
                .map(|hit| hit.row.clone())
        });
        let changed = owner.hover != hover;
        owner.hover = hover;
        changed
    }
    fn input(
        &self,
        state: &mut dyn Any,
        _: &str,
        event: &Event,
    ) -> maka_ui_runtime::Result<WidgetInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        if matches!(event, Event::Mouse(mouse) if mouse.kind == MouseEventKind::Up(MouseButton::Left))
            && owner.drag.take().is_some()
        {
            return Ok(WidgetInput::PreserveFocus);
        }
        let Some(geometry) = &owner.geometry else {
            return Ok(WidgetInput::Consumed(None));
        };
        if owner.exit {
            return Ok(WidgetInput::ExitFocus);
        }
        // New props can arrive before the next successful flush. They cannot
        // authorize a control that the user has not seen at that revision.
        if geometry.props != self.props {
            return Ok(WidgetInput::PreserveFocus);
        }
        match event {
            Event::Mouse(mouse) => {
                let point = (mouse.column, mouse.row).into();
                let bar = match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        geometry.bar.filter(|bar| bar.contains(point))
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        owner.drag.filter(|bar| Some(*bar) == geometry.bar)
                    }
                    _ => None,
                };
                if let Some(bar) = bar {
                    // Original Surface mapping: the track endpoints are the
                    // content endpoints, including captured drags outside it.
                    let maximum = self.props.rows.len().saturating_sub(geometry.rows);
                    let row = mouse
                        .row
                        .saturating_sub(bar.y)
                        .min(bar.height.saturating_sub(1));
                    owner.offset = usize::from(row) * maximum
                        / usize::from(bar.height.saturating_sub(1).max(1));
                    owner.reveal = false;
                    owner.hover = None;
                    owner.drag = Some(bar);
                    return Ok(WidgetInput::PreserveFocus);
                }
                if !geometry.area.contains(point) {
                    return Ok(WidgetInput::Unhandled);
                }
                match mouse.kind {
                    MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                        owner.offset = if mouse.kind == MouseEventKind::ScrollDown {
                            owner
                                .offset
                                .saturating_add(3)
                                .min(self.props.rows.len().saturating_sub(geometry.rows))
                        } else {
                            owner.offset.saturating_sub(3)
                        };
                        owner.reveal = false;
                        owner.hover = None;
                        Ok(WidgetInput::Consumed(None))
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        let Some(hit) = geometry
                            .hits
                            .iter()
                            .rev()
                            .find(|hit| hit.area.contains(point))
                        else {
                            return Ok(WidgetInput::Consumed(None));
                        };
                        owner.selected = Some(hit.row.clone());
                        owner.reveal = true;
                        Ok(match &hit.action {
                            Some(key) => self.action(&hit.row, key),
                            None => WidgetInput::Consumed(None),
                        })
                    }
                    _ => Ok(WidgetInput::Unhandled),
                }
            }
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                owner.drag = None;
                if matches!(key.code, KeyCode::Esc | KeyCode::Tab | KeyCode::BackTab) {
                    return Ok(WidgetInput::ExitFocus);
                }
                if let Some((key, modifiers)) = super::shortcut_chord(key)
                    && let Some(shortcut) = self
                        .props
                        .shortcuts
                        .iter()
                        .find(|shortcut| shortcut.matches(&key, modifiers, false))
                {
                    return Ok(if shortcut.enabled {
                        owner
                            .selected
                            .as_ref()
                            .map_or(WidgetInput::Consumed(None), |row| {
                                self.action(row, &shortcut.input)
                            })
                    } else {
                        WidgetInput::Consumed(None)
                    });
                }
                if key.modifiers.is_empty()
                    && matches!(
                        key.code,
                        KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End
                    )
                {
                    let index = owner
                        .selected
                        .as_ref()
                        .and_then(|key| self.props.rows.iter().position(|row| &row.key == key));
                    let index = match key.code {
                        KeyCode::Home => 0,
                        KeyCode::End => self.props.rows.len().saturating_sub(1),
                        KeyCode::Up => index.unwrap_or(self.props.rows.len()).saturating_sub(1),
                        _ => index.map_or(0, |index| {
                            (index + 1).min(self.props.rows.len().saturating_sub(1))
                        }),
                    };
                    owner.selected = self.props.rows.get(index).map(|row| row.key.clone());
                    owner.reveal = true;
                    return Ok(WidgetInput::Consumed(None));
                }
                Ok(if key.kind == KeyEventKind::Repeat {
                    WidgetInput::Consumed(None)
                } else {
                    WidgetInput::Unhandled
                })
            }
            _ => Ok(WidgetInput::Unhandled),
        }
    }
    fn presentation(&self, state: &mut dyn Any, visible: bool) {
        if !visible {
            let owner = state.downcast_mut::<Owner>().unwrap();
            owner.geometry = None;
            owner.hover = None;
            owner.drag = None;
        }
    }
    fn present(&self, state: &mut dyn Any, geometry: Box<dyn Any>) -> Option<NativeInput> {
        let owner = state.downcast_mut::<Owner>().unwrap();
        let geometry = *geometry.downcast::<Geometry>().unwrap();
        if owner.drag.is_some_and(|bar| Some(bar) != geometry.bar) {
            owner.drag = None;
        }
        if (owner.offset, owner.reveal, owner.selected.clone()) == geometry.intent {
            owner.offset = geometry.offset;
            owner.reveal = false;
        }
        owner.geometry = Some(geometry);
        None
    }
    fn measure(&self, slots: &Slots<'_>, width: u16) -> maka_ui_runtime::Result<(u16, u32)> {
        Ok((
            width,
            self.props.rows.len().min(self.max_rows(slots)) as u32,
        ))
    }
    fn paint(
        &self,
        slots: &mut Slots<'_>,
        area: Placement,
        buffer: &mut Buffer,
    ) -> maka_ui_runtime::Result<()> {
        let owner = slots.native_state::<Owner>()?;
        let rows = self
            .props
            .rows
            .len()
            .min(area.height as usize)
            .min(self.max_rows(slots));
        let mut offset = owner.offset.min(self.props.rows.len().saturating_sub(rows));
        let focused = slots.focused();
        let resized = focused
            && owner
                .geometry
                .as_ref()
                .is_some_and(|geometry| geometry.rows != rows);
        if (owner.reveal || resized)
            && let Some(index) = owner
                .selected
                .as_ref()
                .and_then(|key| self.props.rows.iter().position(|row| &row.key == key))
        {
            offset = offset.min(index).max((index + 1).saturating_sub(rows));
        }
        let intent = (owner.offset, owner.reveal, owner.selected.clone());
        let color = |color: maka_ui_runtime::appearance::Color| {
            Color::Rgb(
                (color.0 >> 24) as u8,
                (color.0 >> 16) as u8,
                (color.0 >> 8) as u8,
            )
        };
        let viewport = Placement::new(
            CellRect {
                height: rows as u32,
                ..area.bounds
            },
            area.clip,
        )
        .clipped(buffer.area)
        .visible();
        let bar = (self.props.rows.len() > rows && viewport.height > 1 && viewport.width > 0)
            .then(|| Rect::new(viewport.right() - 1, viewport.y, 1, viewport.height));
        // Reserve the original trailing scrollbar column, even when content fits.
        let terminal = self.props.terminal.as_ref();
        let foreground = terminal.map_or_else(
            || color(self.props.foreground),
            |p| super::paint::color(p.foreground),
        );
        let muted =
            terminal.map_or_else(|| color(self.props.muted), |p| super::paint::color(p.muted));
        let selected_fill = terminal.map_or_else(
            || color(self.props.selected),
            |p| super::paint::color(p.selection),
        );
        let selected_text = terminal.map_or(foreground, |p| super::paint::color(p.selection_text));
        let hovered_fill = terminal.map_or_else(
            || color(self.props.hovered),
            |p| super::paint::color(p.surface),
        );
        let width = area.width.saturating_sub(1);
        let mut hits = vec![];
        for (line, row) in self.props.rows.iter().skip(offset).take(rows).enumerate() {
            let selected = owner.selected.as_ref() == Some(&row.key);
            let hovered = owner.hover.as_ref() == Some(&row.key);
            let bounds = CellRect::new(area.x, area.y + line as i64, width, 1);
            let row_area = Placement::new(bounds, area.clip).clipped(buffer.area);
            let mut style = Style::default().fg(muted);
            if selected && focused {
                style = style.bg(selected_fill).fg(selected_text);
            } else if hovered {
                style = style.bg(hovered_fill);
            }
            for y in row_area.visible().y..row_area.visible().bottom() {
                for x in row_area.visible().x..row_area.visible().right() {
                    buffer[(x, y)].set_style(style);
                }
            }
            hits.push(Hit {
                row: row.key.clone(),
                action: None,
                area: row_area,
            });
            // Original row composition: clipped preview fills the space before
            // trailing controls, which appear only on focused/hovered rows.
            let actions = if selected && focused || hovered {
                row.actions.as_slice()
            } else {
                &[]
            };
            let widths: Vec<_> = actions
                .iter()
                .map(|action| {
                    paint::sanitize(&action.label)
                        .width()
                        .saturating_add(2)
                        .min(u16::MAX as usize) as u16
                })
                .collect();
            let total = widths
                .iter()
                .copied()
                .fold(0u16, u16::saturating_add)
                .min(width);
            let preview = Placement::new(
                CellRect {
                    width: width - total,
                    ..bounds
                },
                area.clip,
            );
            clipped_text(&row.text, preview, buffer, style);
            let mut x = bounds.x + i64::from(preview.width);
            for (action, width) in actions.iter().zip(widths) {
                let action_area = Placement::new(
                    CellRect::new(
                        x,
                        bounds.y,
                        width.min((bounds.right() - x).max(0) as u16),
                        1,
                    ),
                    area.clip,
                );
                let action_style = if action.enabled {
                    style.fg(foreground)
                } else {
                    style.add_modifier(Modifier::DIM)
                };
                clipped_text(
                    &format!(" {} ", action.label),
                    action_area,
                    buffer,
                    action_style,
                );
                hits.push(Hit {
                    row: row.key.clone(),
                    action: action.enabled.then(|| action.key.clone()),
                    area: action_area,
                });
                x += i64::from(width);
            }
        }
        if let Some(bar) = bar {
            // Reuse the original Surface's proportional thumb calculation.
            let maximum = self.props.rows.len() - rows;
            let visible = usize::from(bar.height);
            let thumb = (visible * visible / (maximum + visible)).max(1);
            let top = offset * (visible - thumb) / maximum;
            for line in 0..visible {
                let active = (top..top + thumb).contains(&line);
                let style = &self.props.scrollbar;
                buffer[(bar.x, bar.y + line as u16)]
                    .set_char(if active { style.thumb } else { style.track })
                    .set_style(Style::default().fg(terminal.map_or_else(
                        || {
                            color(if active {
                                style.thumb_color
                            } else {
                                style.track_color
                            })
                        },
                        |p| super::paint::color(if active { p.scrollbar } else { p.border }),
                    )));
            }
        }
        slots.native_target("rows", area, self.props.tab_index, false)?;
        slots.native_geometry(Box::new(Geometry {
            props: self.props.clone(),
            area,
            hits,
            offset,
            rows,
            bar,
            intent,
        }))
    }
}
fn clipped_text(text: &str, area: Placement, buffer: &mut Buffer, style: Style) {
    if area.is_empty() {
        return;
    }
    let text: String = paint::sanitize(text)
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    // Reuse the original text painter's grapheme/clip rules; one row discards
    // wrapped overflow without ever publishing half of a wide character.
    paint::text(&text, area, buffer, style);
}
pub(super) struct Component;
impl NativeComponent<dyn Widget> for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui_runtime::abi::ValueBindings,
    ) -> maka_ui_runtime::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let (revision, props, events) = match call {
            ComponentCall::Mount { props } => (0, props, vec![]),
            ComponentCall::Update { state, props } => {
                (state["revision"].as_u64().unwrap() + 1, props, vec![])
            }
            ComponentCall::Event {
                state,
                props,
                key: "activate",
                input,
                ..
            } => (
                state["revision"].as_u64().unwrap() + 1,
                props,
                vec![Emission {
                    name: "activated".into(),
                    value: input.clone(),
                }],
            ),
            ComponentCall::Event { .. } => {
                return Err(maka_ui_runtime::Error::Invalid("unknown rows input".into()));
            }
        };
        let props: RowsProps = serde_json::from_value(props.clone())
            .map_err(|error| maka_ui_runtime::Error::Invalid(error.to_string()))?;
        props.validate()?;
        if [props.scrollbar.thumb, props.scrollbar.track]
            .iter()
            .any(|symbol| symbol.width() != Some(1))
        {
            return Err(maka_ui_runtime::Error::Invalid(
                "scrollbar symbols must occupy one terminal cell".into(),
            ));
        }
        Ok(Frame {
            revision,
            state: json!({"revision":revision}),
            surface: None,
            view: NativeView::<dyn Widget>::new(
                Rc::new(Rows {
                    props: Rc::new(props),
                }),
                BTreeMap::from([("activate".into(), activated_schema())]),
                vec![],
            )?,
        }
        .output(events))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register(maka_ui_runtime::rows::IMPLEMENTATION, |_, _| {
            Ok(BTreeMap::from([(
                "Rows".into(),
                NativeExport {
                    contract: rows_contract(),
                    value: NativeCapability::Component(Rc::new(Component)),
                },
            )]))
        })
        .expect("unique terminal rows");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_preview_never_paints_half_a_wide_character_across_its_clip() {
        let mut buffer = Buffer::empty(ratatui::layout::Rect::new(0, 0, 4, 1));
        buffer.set_string(0, 0, "----", Style::default());
        let area = Placement::new(
            CellRect::new(0, 0, 2, 1),
            ratatui::layout::Rect::new(0, 0, 1, 1),
        );
        clipped_text("界", area, &mut buffer, Style::default());
        assert_eq!(buffer[(0, 0)].symbol(), "-");
        assert_eq!(buffer[(1, 0)].symbol(), "-");
        clipped_text(
            "界",
            ratatui::layout::Rect::new(0, 0, 2, 1).into(),
            &mut buffer,
            Style::default(),
        );
        assert_eq!(buffer[(0, 0)].symbol(), "界");
        assert_eq!(buffer[(2, 0)].symbol(), "-");
    }
}
