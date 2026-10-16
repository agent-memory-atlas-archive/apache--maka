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

//! Object menus reuse dialog focus custody and ordinary physical action targets.
use super::{
    CellRect, PaintedScene, Placement, Presenter, dialog, paint, scene::Node, scroll::SceneState,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Position,
    style::{Modifier, Style},
};
use std::rc::Rc;

impl Presenter {
    pub(super) fn open_context_menu(&mut self, event: &Event) -> maka_ui_runtime::Result<bool> {
        if self
            .physical_path
            .last()
            .and_then(|scope| scope.upgrade())
            .is_some_and(|opening| opening.anchor.is_some())
            && let Event::Key(key) = event
            && key.kind != KeyEventKind::Release
            && key.modifiers.is_empty()
            && matches!(
                key.code,
                KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End
            )
        {
            let peers: Vec<_> = self
                .scene
                .iter()
                .filter(|target| target.tab_index >= 0 && self.target_admitted(target))
                .collect();
            if !peers.is_empty() {
                let at = self
                    .focus
                    .as_ref()
                    .and_then(|focus| peers.iter().position(|target| target.same(focus)))
                    .unwrap_or(0);
                let next = match key.code {
                    KeyCode::Up => at.saturating_sub(1),
                    KeyCode::Down => (at + 1).min(peers.len() - 1),
                    KeyCode::Home => 0,
                    _ => peers.len() - 1,
                };
                self.focus_item(peers[next].clone())?;
            }
            return Ok(true);
        }
        let target = match event {
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Right) => {
                self.scene.iter().rev().find(|target| {
                    matches!(target.kind, paint::TargetKind::ContextMenu { .. })
                        && target.area.contains((mouse.column, mouse.row).into())
                        && self.target_admitted(target)
                })
            }
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => self
                .scene
                .iter()
                .rev()
                .find(|target| target.area.contains((mouse.column, mouse.row).into()))
                .filter(|target| {
                    matches!(target.kind, paint::TargetKind::ContextMenu { button: true })
                }),
            Event::Key(key)
                if key.kind != KeyEventKind::Release
                    && key.code == KeyCode::F(10)
                    && key.modifiers == KeyModifiers::SHIFT =>
            {
                self.focus.as_ref().and_then(|focus| {
                    self.scene.iter().rev().find(|target| {
                        matches!(
                            target.kind,
                            paint::TargetKind::ContextMenu { button: false }
                        ) && target.area.contains(focus.area.visible().as_position())
                            && self.target_admitted(target)
                    })
                })
            }
            Event::Key(key)
                if key.kind != KeyEventKind::Release
                    && key.modifiers.is_empty()
                    && matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) =>
            {
                self.focus.as_ref().filter(|target| {
                    matches!(target.kind, paint::TargetKind::ContextMenu { button: true })
                })
            }
            _ => None,
        }
        .cloned();
        let Some(target) = target else {
            return Ok(false);
        };
        if !self.target_admitted(&target)
            || !self.resources.admits(&target)
            || self.application.frame(&target.mount)?.revision != target.revision
        {
            return Ok(true);
        }
        let mut opening = dialog::Opening::new();
        opening.anchor = Some(match event {
            Event::Mouse(mouse) => Position::new(mouse.column, mouse.row),
            _ => target.area.visible().as_position(),
        });
        if self
            .application
            .frame(&target.mount)?
            .view
            .view
            .scene()
            .and_then(|node| node.overlay(&target.key))
            .is_some_and(|node| {
                matches!(
                    node,
                    Node::ContextMenu {
                        copy_selection_label: Some(_),
                        ..
                    }
                )
            })
            && let Some(focus) = self
                .focus
                .clone()
                .filter(|focus| self.target_admitted(focus))
        {
            opening.selection = match self.resources.copy(&focus, false)? {
                Some(text) => Some(text.into()),
                None => self
                    .item_reading_scope(&focus)
                    .map(|(scope, _)| scope.copy())
                    .transpose()?
                    .flatten()
                    .map(Into::into),
            };
        }
        self.clear_hover()?;
        self.resources.cancel_pointer()?;
        self.drag = None;
        let state = self
            .resources
            .owners
            .get_mut(&target.mount.id())
            .unwrap()
            .state
            .downcast_mut::<SceneState>()
            .unwrap();
        state.dialogs.insert(target.key.clone(), Rc::new(opening));
        self.sync_native()?;
        Ok(true)
    }
    /// Close only the originating menu, including when the action opens another dialog.
    pub(super) fn close_context_action(&mut self, scope: Option<&dialog::Scope>) {
        let Some(scope) = scope else { return };
        for owner in self.resources.owners.values_mut() {
            if let Some(state) = owner.state.downcast_mut::<SceneState>() {
                state.dialogs.retain(|_, opening| {
                    opening.anchor.is_none() || !scope.ptr_eq(&Rc::downgrade(opening))
                });
            }
        }
    }
    pub(super) fn paint_context_menu(
        &self,
        mount: &maka_plugin_core::runtime::Mount,
        node: &Node,
        area: Placement,
        buffer: &mut Buffer,
        scene: &mut PaintedScene,
        pass: &mut paint::NodePaint<'_>,
    ) -> maka_ui_runtime::Result<()> {
        let Node::ContextMenu {
            key,
            background,
            appearance,
            ..
        } = node
        else {
            unreachable!()
        };
        let target = paint::Target {
            identity: paint::TargetIdentity {
                mount: mount.clone(),
                key: key.clone(),
                owner: self.resources.owners[&mount.id()].identity.clone(),
                scope: scene.input_scope.clone(),
            },
            revision: self.application.frame(mount)?.revision,
            area,
            tab_index: -1,
            autofocus: false,
            kind: paint::TargetKind::ContextMenu { button: false },
            ancestors: scene.ancestry.clone(),
        };
        if !area.is_empty() {
            scene.targets.push(target.clone());
        }
        let width = area.width.saturating_sub(3);
        self.paint_node(
            mount,
            background,
            Placement::new(CellRect::new(area.x, area.y, width, area.height), area.clip),
            buffer,
            scene,
            pass,
        )?;
        // A clipped menu still carries logical Item targets for keyboard
        // navigation. Only its pointer targets and overlay require placement.
        if area.is_empty() {
            return Ok(());
        }
        let button = Placement::new(
            CellRect::new(area.x + i64::from(width), area.y, area.width - width, 1),
            area.clip,
        );
        let button_target = paint::Target {
            area: button,
            tab_index: 0,
            kind: paint::TargetKind::ContextMenu { button: true },
            ..target
        };
        let focused = self
            .focus
            .as_ref()
            .is_some_and(|focus| focus.same(&button_target));
        let mut style = Style::default().fg(paint::color(
            appearance.title_color.unwrap_or(appearance.border_color),
        ));
        if focused
            || self
                .hover
                .as_ref()
                .is_some_and(|hover| hover.same(&button_target))
        {
            style = style.add_modifier(Modifier::UNDERLINED);
        } else {
            style = style.add_modifier(Modifier::DIM);
        }
        paint::text(
            if appearance.ascii { "..." } else { " ⋯ " },
            button,
            buffer,
            style,
        );
        if !button.is_empty() {
            scene.targets.push(button_target);
        }
        let state = self.resources.owners[&mount.id()]
            .state
            .downcast_ref::<SceneState>()
            .unwrap();
        if let Some(opening) = state.dialogs.get(key) {
            scene.dialogs.push_back(dialog::Deferred {
                mount: mount.clone(),
                key: key.clone(),
                opening: Rc::downgrade(opening),
                parent_path: scene.input_path.clone(),
                anchor: None,
            });
        }
        Ok(())
    }
}

/// A physical editor menu retains the original native owner and snapshot. Its
/// actions use the ordinary input and clipboard paths, never guest commands.
pub(super) struct EditorMenu {
    target: paint::Target,
    snapshot: maka_ui_runtime::editor::EditorSnapshot,
    menu: maka_ui_runtime::textarea::TextareaMenu,
    enabled: [bool; 4],
    anchor: Position,
    selected: usize,
    identity: Rc<()>,
    presented: bool,
}
impl EditorMenu {
    fn labels(&self) -> [&str; 4] {
        let labels = &self.menu.labels;
        [&labels.copy, &labels.cut, &labels.paste, &labels.select_all]
    }
    fn area(&self, viewport: ratatui::layout::Rect) -> ratatui::layout::Rect {
        use unicode_width::UnicodeWidthStr;
        let width = (self
            .labels()
            .iter()
            .map(|label| label.width())
            .max()
            .unwrap_or(0)
            + 4)
        .min(usize::from(viewport.width)) as u16;
        let height = 6.min(viewport.height);
        ratatui::layout::Rect::new(
            self.anchor
                .x
                .min(viewport.right().saturating_sub(width))
                .max(viewport.x),
            self.anchor
                .y
                .min(viewport.bottom().saturating_sub(height))
                .max(viewport.y),
            width,
            height,
        )
    }
}
impl Presenter {
    fn editor_menu_current(&self, menu: &EditorMenu) -> bool {
        self.target_admitted(&menu.target)
            && self.resources.admits(&menu.target)
            && self.application.mount_visible(&menu.target.mount)
            && self
                .focus
                .as_ref()
                .is_some_and(|focus| focus.same(&menu.target))
            && self.scene.iter().any(|target| {
                target.same(&menu.target)
                    && target.area == menu.target.area
                    && !target.area.is_empty()
            })
            && !self.application.is_retiring(&menu.target.mount)
            && self
                .application
                .frame(&menu.target.mount)
                .is_ok_and(|frame| frame.revision == menu.target.revision)
            && self
                .resources
                .owners
                .get(&menu.target.mount.id())
                .is_some_and(|owner| {
                    owner.recipe.editing_menu(owner.state.as_ref()).is_some_and(
                        |(style, snapshot, enabled)| {
                            style == menu.menu
                                && snapshot == menu.snapshot
                                && enabled == menu.enabled
                        },
                    )
                })
    }
    pub(super) fn editor_context_menu(&mut self, event: &Event) -> maka_ui_runtime::Result<bool> {
        if self
            .editor_menu
            .as_ref()
            .is_some_and(|menu| !self.editor_menu_current(menu))
        {
            self.editor_menu = None;
        }
        if let Some(mut menu) = self.editor_menu.take() {
            if !menu.presented
                && !matches!(
                    event,
                    Event::FocusLost
                        | Event::Resize(..)
                        | Event::Key(crossterm::event::KeyEvent {
                            code: KeyCode::Esc,
                            ..
                        })
                )
            {
                self.editor_menu = Some(menu);
                return Ok(true);
            }
            let mut activate = None;
            match event {
                Event::FocusLost | Event::Resize(..) => return Ok(false),
                Event::Key(key) if key.kind == KeyEventKind::Release => {
                    self.editor_menu = Some(menu);
                    return Ok(true);
                }
                Event::Key(key) => match key.code {
                    KeyCode::Esc | KeyCode::Tab | KeyCode::BackTab => return Ok(true),
                    KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End => {
                        menu.selected = match key.code {
                            KeyCode::Up => (menu.selected + 3) % 4,
                            KeyCode::Down => (menu.selected + 1) % 4,
                            KeyCode::Home => 0,
                            _ => 3,
                        };
                        self.editor_menu = Some(menu);
                        return Ok(true);
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => activate = Some(menu.selected),
                    _ => return Ok(true),
                },
                Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                    let area = menu.area(self.viewport);
                    if area.contains((mouse.column, mouse.row).into())
                        && mouse.row > area.y
                        && mouse.row < area.bottom().saturating_sub(1)
                    {
                        activate = Some(usize::from(mouse.row - area.y - 1));
                    } else {
                        return Ok(true);
                    }
                }
                Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Right) => {}
                _ => {
                    self.editor_menu = Some(menu);
                    return Ok(true);
                }
            }
            if let Some(index) = activate {
                if index >= 4 || !menu.enabled[index] {
                    self.editor_menu = Some(menu);
                    return Ok(true);
                }
                if index <= 1 {
                    self.clipboard = self.resources.copy(&menu.target, true)?.map(Into::into);
                    if self.clipboard.is_none() {
                        return Ok(true);
                    }
                }
                let key = match index {
                    1 => Some(crossterm::event::KeyEvent::new(
                        KeyCode::Backspace,
                        KeyModifiers::NONE,
                    )),
                    2 => Some(crossterm::event::KeyEvent::new(
                        KeyCode::Char('v'),
                        KeyModifiers::CONTROL,
                    )),
                    3 => Some(crossterm::event::KeyEvent::new(
                        KeyCode::Char('a'),
                        KeyModifiers::CONTROL,
                    )),
                    _ => None,
                };
                if let Some(key) = key {
                    match self.resources.input(&menu.target, &Event::Key(key))? {
                        Some(super::WidgetInput::Consumed(Some(observed))) => {
                            self.resources.observe(&menu.target.mount, observed)?;
                        }
                        Some(super::WidgetInput::Emitted(input)) => {
                            self.accept_native_input(&menu.target.mount, input, None)?;
                        }
                        _ => {}
                    }
                }
                self.sync_native()?;
                return Ok(true);
            }
        }
        let target = match event {
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Right) => self
                .scene
                .iter()
                .rev()
                .find(|target| {
                    !matches!(
                        target.kind,
                        paint::TargetKind::ContextMenu { .. } | paint::TargetKind::Hint
                    ) && target.area.contains((mouse.column, mouse.row).into())
                })
                .cloned(),
            Event::Key(key)
                if key.kind == KeyEventKind::Press
                    && key.code == KeyCode::F(10)
                    && key.modifiers == KeyModifiers::SHIFT =>
            {
                self.focus.clone()
            }
            _ => None,
        };
        let Some(target) = target else {
            return Ok(false);
        };
        if !self.target_admitted(&target)
            || !self.resources.admits(&target)
            || self.application.is_retiring(&target.mount)
        {
            return Ok(false);
        }
        let Some((menu, snapshot, enabled)) = self.resources.editing_menu(&target)? else {
            return Ok(false);
        };
        let anchor = match event {
            Event::Mouse(mouse) => Position::new(mouse.column, mouse.row),
            _ => target.area.visible().as_position(),
        };
        self.clear_hover()?;
        self.resources.cancel_pointer()?;
        self.drag = None;
        self.focus = Some(target.clone());
        self.editor_menu = Some(EditorMenu {
            target,
            snapshot,
            menu,
            enabled,
            anchor,
            selected: 0,
            identity: Rc::new(()),
            presented: false,
        });
        Ok(true)
    }
    pub(super) fn present_editor_menu(&mut self, presented: Option<&std::rc::Weak<()>>) {
        if let Some(menu) = &mut self.editor_menu {
            menu.presented =
                presented.is_some_and(|identity| identity.ptr_eq(&Rc::downgrade(&menu.identity)));
        }
    }
    pub(super) fn paint_editor_menu(
        &self,
        area: ratatui::layout::Rect,
        buffer: &mut Buffer,
        scene: &mut PaintedScene,
    ) {
        let Some(menu) = self
            .editor_menu
            .as_ref()
            .filter(|menu| self.editor_menu_current(menu))
        else {
            return;
        };
        if !dialog::same_scope(&menu.target.scope, &scene.foreground_path.last().cloned()) {
            return;
        }
        scene.editor_menu = Some(Rc::downgrade(&menu.identity));
        let area = menu.area(area);
        let appearance = &menu.menu.appearance;
        let color = |value: maka_ui_runtime::appearance::Color| {
            ratatui::style::Color::Rgb(
                (value.0 >> 24) as u8,
                (value.0 >> 16) as u8,
                (value.0 >> 8) as u8,
            )
        };
        let base = Style::default()
            .fg(color(appearance.text.foreground))
            .bg(color(appearance.background));
        for row in area.y..area.bottom() {
            buffer.set_stringn(
                area.x,
                row,
                " ".repeat(usize::from(area.width)),
                usize::from(area.width),
                base,
            );
        }
        super::border::paint(
            buffer,
            area.into(),
            color(appearance.border),
            maka_ui_runtime::scene::terminal::BorderStyle::Rounded,
        );
        for (index, label) in menu.labels().iter().enumerate() {
            let y = area.y + 1 + index as u16;
            if y >= area.bottom().saturating_sub(1) {
                break;
            }
            let style = if !menu.enabled[index] {
                base.fg(color(appearance.disabled_foreground))
            } else if index == menu.selected {
                base.fg(color(appearance.active_foreground))
                    .bg(color(appearance.active_background))
            } else {
                base
            };
            buffer.set_stringn(
                area.x + 1,
                y,
                format!(" {} ", paint::sanitize(label)),
                usize::from(area.width.saturating_sub(2)),
                style,
            );
        }
    }
}
