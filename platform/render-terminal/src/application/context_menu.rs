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
            opening.selection = match self.resources.copy(&focus)? {
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
