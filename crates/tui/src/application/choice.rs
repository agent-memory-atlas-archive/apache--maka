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

//! Original Surface choice menu, using the presenter's existing modal custody.
use super::{
    CellRect, NativeInput, PaintedScene, Placement, Presenter, WidgetInput, dialog, paint,
    scroll::SceneState,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use maka_ui::scene::terminal::{BorderStyle, Choice};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Modifier, Style},
};
use serde_json::json;
use std::rc::Rc;
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub(super) struct State {
    highlighted: Option<String>,
    pub geometry: Option<Geometry>,
}
pub(super) struct Geometry {
    pub choice: Choice,
    pub menu: Option<Menu>,
}
pub(super) struct Menu {
    pub rect: Rect,
    first: usize,
    rows: Vec<Rect>,
}
fn safe(text: &str) -> String {
    paint::sanitize(text).replace('\n', " ")
}
fn value(choice: &Choice) -> &str {
    choice
        .current
        .as_ref()
        .and_then(|key| choice.options.iter().find(|option| &option.key == key))
        .map_or("", |option| &option.label)
}
pub(super) fn size(choice: &Choice, width: u16) -> (u16, u32) {
    (
        (safe(&choice.label).width() + safe(value(choice)).width() + 3).min(usize::from(width))
            as u16,
        u32::from(width > 0),
    )
}
// Original Surface::choice_start keeps a clipped list's highlight visible.
fn first(previous: Option<usize>, highlighted: usize, rows: usize, len: usize) -> usize {
    let first = previous.unwrap_or(0).min(len.saturating_sub(rows));
    if highlighted < first {
        highlighted
    } else {
        first.max((highlighted + 1).saturating_sub(rows))
    }
}
pub(super) fn bounds(choice: &Choice, anchor: Placement, area: Rect) -> Option<Rect> {
    let anchor = anchor.visible();
    if anchor.is_empty() {
        return None;
    }
    let width = choice
        .options
        .iter()
        .map(|option| safe(&option.label).width())
        .max()
        .unwrap_or(0)
        .saturating_add(6)
        .min(usize::from(area.width)) as u16;
    let rows = (choice.options.len() as u16).min(area.height.saturating_sub(2));
    if rows == 0 || width < 7 {
        return None;
    }
    let height = rows + 2;
    let y = if anchor.bottom().saturating_add(height) <= area.bottom() {
        anchor.bottom()
    } else {
        anchor.y.saturating_sub(height).max(area.y)
    };
    let x = anchor
        .right()
        .saturating_sub(width)
        .clamp(area.x, area.right().saturating_sub(width));
    Some(Rect::new(x, y, width, height).intersection(area))
}
impl SceneState {
    pub fn choice_input(&mut self, choice: &Choice, event: &Event) -> WidgetInput {
        let key = &choice.key;
        let Some(state) = self.choices.get_mut(key) else {
            return WidgetInput::Consumed(None);
        };
        let Some(geometry) = &state.geometry else {
            return WidgetInput::Consumed(None);
        };
        if geometry.choice != *choice || choice.disabled || choice.options.is_empty() {
            return WidgetInput::Consumed(None);
        }
        if !self.dialogs.contains_key(key) {
            if matches!(event, Event::Key(key) if key.kind != KeyEventKind::Release && key.modifiers.is_empty() && matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')))
                || matches!(event, Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left))
            {
                state.highlighted = choice
                    .current
                    .clone()
                    .or_else(|| choice.options.first().map(|option| option.key.clone()));
                self.dialogs
                    .insert(key.clone(), Rc::new(dialog::Opening::new()));
                return WidgetInput::Consumed(None);
            }
            return WidgetInput::Unhandled;
        }
        let Some(menu) = &geometry.menu else {
            return WidgetInput::Consumed(None);
        };
        let mut chosen = None;
        let mut close = false;
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                let index = choice
                    .options
                    .iter()
                    .position(|option| Some(&option.key) == state.highlighted.as_ref())
                    .unwrap_or(0);
                match key.code {
                    KeyCode::Up | KeyCode::Down => {
                        let index = if key.code == KeyCode::Up {
                            index.saturating_sub(1)
                        } else {
                            (index + 1).min(choice.options.len() - 1)
                        };
                        state.highlighted = Some(choice.options[index].key.clone());
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        chosen = Some(choice.options[index].key.clone())
                    }
                    KeyCode::Esc | KeyCode::Tab | KeyCode::BackTab => close = true,
                    _ => {}
                }
            }
            Event::Mouse(mouse) => {
                let point = Position::new(mouse.column, mouse.row);
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Right) => close = true,
                    MouseEventKind::Down(MouseButton::Left) => {
                        if let Some(index) = menu.rows.iter().position(|row| row.contains(point)) {
                            chosen = Some(choice.options[menu.first + index].key.clone());
                        } else if !menu.rect.contains(point) {
                            close = true;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        if close || chosen.is_some() {
            self.dialogs.remove(&choice.key);
        }
        chosen.map_or(WidgetInput::Consumed(None), |key| {
            WidgetInput::Emitted(NativeInput {
                key: choice.key.clone(),
                value: json!({"kind":"selected","key":key}),
            })
        })
    }
    pub fn choice_hover(&mut self, choice: &Choice, point: Option<Position>) {
        if !self.dialogs.contains_key(&choice.key) {
            return;
        }
        let Some(state) = self.choices.get_mut(&choice.key) else {
            return;
        };
        let Some(geometry) = state
            .geometry
            .as_ref()
            .filter(|geometry| geometry.choice == *choice)
        else {
            return;
        };
        if let Some((menu, point)) = geometry.menu.as_ref().zip(point)
            && let Some(index) = menu.rows.iter().position(|row| row.contains(point))
        {
            state.highlighted = Some(choice.options[menu.first + index].key.clone());
        }
    }
}
impl Presenter {
    pub(super) fn paint_choice(
        &self,
        mount: &maka_plugin_core::runtime::Mount,
        choice: &Choice,
        area: Placement,
        buffer: &mut Buffer,
        scene: &mut PaintedScene,
        choices: &mut Vec<Geometry>,
    ) -> maka_ui::Result<()> {
        let area = Placement::new(CellRect::new(area.x, area.y, area.width, 1), area.clip);
        let mut style = Style::default().fg(paint::color(choice.foreground));
        let target = paint::Target {
            identity: paint::TargetIdentity {
                mount: mount.clone(),
                key: choice.key.clone(),
                owner: self.resources.owners[&mount.id()].identity.clone(),
                scope: scene.input_scope.clone(),
            },
            revision: self.application.frame(mount)?.revision,
            area,
            tab_index: 0,
            autofocus: false,
            kind: paint::TargetKind::Native,
            ancestors: scene.ancestry.clone(),
        };
        if choice.disabled {
            style = style.add_modifier(Modifier::DIM);
        }
        let mut value_style = style.bg(paint::color(choice.background));
        if !choice.disabled && self.focus.as_ref().is_some_and(|focus| focus.same(&target)) {
            value_style = value_style
                .fg(paint::color(choice.accent))
                .add_modifier(Modifier::UNDERLINED);
        }
        let tail = format!(
            "{} {}",
            safe(value(choice)),
            if choice.ascii { "v" } else { "▾" }
        );
        let tail_width = tail.width().min(usize::from(area.width)) as u16;
        let label_width = area.width.saturating_sub(tail_width.saturating_add(1));
        paint::text(
            &safe(&choice.label),
            Placement::new(CellRect::new(area.x, area.y, label_width, 1), area.clip),
            buffer,
            style,
        );
        paint::text(
            &tail,
            Placement::new(
                CellRect::new(area.right() - i64::from(tail_width), area.y, tail_width, 1),
                area.clip,
            ),
            buffer,
            value_style,
        );
        if !choice.disabled && !choice.options.is_empty() && !area.is_empty() {
            scene.targets.push(target);
        }
        let state = self.resources.owners[&mount.id()]
            .state
            .downcast_ref::<SceneState>()
            .unwrap();
        if let Some(opening) = state.dialogs.get(&choice.key) {
            scene.dialogs.push_back(dialog::Deferred {
                mount: mount.clone(),
                key: choice.key.clone(),
                opening: Rc::downgrade(opening),
                parent_path: scene.input_path.clone(),
                anchor: Some(area),
            });
        } else {
            choices.push(Geometry {
                choice: choice.clone(),
                menu: None,
            });
        }
        Ok(())
    }
    pub(super) fn paint_choice_menu(
        &self,
        mount: &maka_plugin_core::runtime::Mount,
        choice: &Choice,
        rect: Rect,
        buffer: &mut Buffer,
        scene: &mut PaintedScene,
    ) -> Geometry {
        let state = self.resources.owners[&mount.id()]
            .state
            .downcast_ref::<SceneState>()
            .unwrap();
        let state = &state.choices[&choice.key];
        let highlighted = choice
            .options
            .iter()
            .position(|option| Some(&option.key) == state.highlighted.as_ref())
            .unwrap_or(0);
        let rows = usize::from(rect.height.saturating_sub(2));
        let first = first(
            state
                .geometry
                .as_ref()
                .and_then(|geometry| geometry.menu.as_ref())
                .map(|menu| menu.first),
            highlighted,
            rows,
            choice.options.len(),
        );
        super::border::paint(
            buffer,
            rect.into(),
            paint::color(choice.border),
            if choice.ascii {
                BorderStyle::Ascii
            } else {
                BorderStyle::Rounded
            },
        );
        let mut painted = Vec::new();
        for (row, index) in (first..choice.options.len()).take(rows).enumerate() {
            let line = Rect::new(rect.x + 1, rect.y + 1 + row as u16, rect.width - 2, 1);
            let option = &choice.options[index];
            let current = choice.current.as_ref() == Some(&option.key);
            let marker = match (current, choice.ascii) {
                (true, false) => "● ",
                (false, false) => "○ ",
                (true, true) => "* ",
                (false, true) => "  ",
            };
            let mut style = Style::default()
                .fg(paint::color(if current {
                    choice.accent
                } else {
                    choice.foreground
                }))
                .bg(paint::color(choice.background));
            if highlighted == index {
                style = style.bg(paint::color(choice.selected));
            }
            buffer.set_style(line, style);
            paint::text(
                &format!(" {marker}{}", safe(&option.label)),
                line.into(),
                buffer,
                style,
            );
            painted.push(line);
        }
        scene.targets.push(paint::Target {
            identity: paint::TargetIdentity {
                mount: mount.clone(),
                key: choice.key.clone(),
                owner: self.resources.owners[&mount.id()].identity.clone(),
                scope: scene.input_scope.clone(),
            },
            revision: self.application.frame(mount).unwrap().revision,
            area: rect.into(),
            tab_index: 0,
            autofocus: false,
            kind: paint::TargetKind::Native,
            ancestors: Vec::new(),
        });
        Geometry {
            choice: choice.clone(),
            menu: Some(Menu {
                rect,
                first,
                rows: painted,
            }),
        }
    }
    /// Reconcile local openings only from a fully validated successful batch.
    /// A clipped/unplaced trigger cannot resurrect a menu when it later returns.
    pub(super) fn reconcile_choices(&mut self, scene: &PaintedScene) {
        for (id, owner) in &mut self.resources.owners {
            let Some(state) = owner.state.downcast_mut::<SceneState>() else {
                continue;
            };
            for key in state.choices.keys() {
                let shown = scene
                    .geometries
                    .iter()
                    .filter(|geometry| geometry.mount.id() == *id)
                    .filter_map(|geometry| {
                        geometry.geometry.downcast_ref::<super::scroll::Geometry>()
                    })
                    .flat_map(|geometry| &geometry.choices)
                    .any(|geometry| geometry.choice.key == *key && geometry.menu.is_some());
                if !shown {
                    state.dialogs.remove(key);
                }
            }
        }
    }
}
