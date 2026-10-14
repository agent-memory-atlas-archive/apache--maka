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

use super::{
    Placement,
    scene::{Node, ScrollPolicy},
};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct SceneState {
    pub generation: u64,
    pub choices: BTreeMap<String, super::choice::State>,
    pub panes: BTreeMap<String, super::adaptive::State>,
    pub dialogs: BTreeMap<String, std::rc::Rc<super::dialog::Opening>>,
    pub reading: Option<super::reading::ReadingState>,
    pub scrolls: BTreeMap<String, ScrollState>,
    pub scrollbar_drag: Option<(String, Placement)>,
    pub focus: BTreeMap<String, maka_ui_runtime::focus::FocusOnReveal>,
}
#[derive(Default)]
pub(super) struct ScrollState {
    pub offset: u32,
    pub intent: u64,
    pub initialized: bool,
    pub presented: Option<Viewport>,
}
#[derive(Clone)]
pub(super) struct Viewport {
    pub wheel: Option<maka_ui_runtime::scene::terminal::WheelCommands>,
    pub area: Placement,
    pub content: u32,
    pub offset: u32,
    pub intent: u64,
    pub anchor: Option<(String, i64)>,
    pub reading_revision: Option<u64>,
    pub reading_intent: Option<u64>,
    pub command: Option<(u64, bool)>,
    pub visible_child_keys: Vec<String>,
}
pub(super) struct Geometry {
    pub generation: u64,
    pub choices: Vec<super::choice::Geometry>,
    pub panes: Vec<super::adaptive::Geometry>,
    pub viewports: Vec<(String, Viewport)>,
}
impl Viewport {
    pub fn end(&self) -> u32 {
        self.content.saturating_sub(self.area.height)
    }
    pub fn reveal_offset(&self, row: &mut super::CellRect) -> u32 {
        row.height = row.height.min(self.area.height);
        let above = self.area.y - row.y;
        let below = row.bottom() - self.area.bottom();
        let delta = if above > 0 { -above } else { below.max(0) };
        let next = (i64::from(self.offset) + delta).clamp(0, i64::from(self.end())) as u32;
        row.y += i64::from(self.offset) - i64::from(next);
        next
    }
    pub fn at_end(&self) -> bool {
        let visible_rows = (i64::from(self.area.visible().bottom()) - self.area.y).max(0) as u32;
        self.offset.saturating_add(visible_rows) >= self.content
    }
}
impl SceneState {
    pub fn prepare_focus_scopes(&mut self, node: &Node, visible: bool) {
        fn visit(node: &Node, visible: bool, state: &mut SceneState, keys: &mut Vec<String>) {
            match node {
                Node::Item { content, .. } => visit(content, visible, state, keys),
                Node::Dialog {
                    background,
                    content,
                    open,
                    ..
                } => {
                    visit(background, visible, state, keys);
                    visit(content, visible && *open, state, keys);
                }
                Node::ContextMenu {
                    key,
                    background,
                    content,
                    ..
                } => {
                    visit(background, visible, state, keys);
                    visit(
                        content,
                        visible && state.dialogs.contains_key(key),
                        state,
                        keys,
                    );
                }
                Node::AdaptivePane {
                    background,
                    content,
                    ..
                } => {
                    visit(background, visible, state, keys);
                    // Placement decides inline/overlay admission. Resizing alone
                    // must not re-arm an already presented focus offer.
                    visit(content, visible, state, keys);
                }
                Node::Stack {
                    key,
                    auto_focus,
                    visible: local,
                    children,
                    ..
                } => {
                    let visible = visible && *local;
                    if *auto_focus {
                        keys.push(key.clone());
                        state
                            .focus
                            .entry(key.clone())
                            .or_default()
                            .reconcile(visible);
                    }
                    for child in children {
                        visit(child, visible, state, keys);
                    }
                }
                _ => {}
            }
        }
        let mut keys = Vec::new();
        visit(node, visible, self, &mut keys);
        self.focus.retain(|key, _| keys.contains(key));
    }
    pub fn commit(&mut self, node: &Node) {
        fn visit(node: &Node, keys: &mut Vec<String>) {
            if let Node::Item { content, .. } = node {
                visit(content, keys);
            }
            if let Node::Dialog {
                background,
                content,
                ..
            }
            | Node::ContextMenu {
                background,
                content,
                ..
            }
            | Node::AdaptivePane {
                background,
                content,
                ..
            } = node
            {
                visit(background, keys);
                visit(content, keys);
            }
            if let Node::Stack {
                key,
                scroll,
                wide,
                children,
                ..
            } = node
            {
                if *scroll != ScrollPolicy::None
                    || wide
                        .as_ref()
                        .and_then(|wide| wide.scroll)
                        .is_some_and(|policy| policy != ScrollPolicy::None)
                {
                    keys.push(key.clone());
                }
                for child in children {
                    visit(child, keys);
                }
            }
        }
        self.commit_panes(node);
        let mut keys = Vec::new();
        visit(node, &mut keys);
        self.scrolls.retain(|key, _| keys.contains(key));
        for key in keys {
            self.scrolls.entry(key).or_default();
        }
        self.generation += 1;
    }
    pub fn candidate(
        &self,
        key: &str,
        policy: ScrollPolicy,
        wheel: Option<&maka_ui_runtime::scene::terminal::WheelCommands>,
        area: Placement,
        content: u32,
        children: &[(String, u32, u32)],
    ) -> Viewport {
        let state = &self.scrolls[key];
        let end = content.saturating_sub(area.height);
        let mut offset = state.offset.min(end);
        if !state.initialized && policy == ScrollPolicy::FollowEnd {
            offset = end;
        }
        if let Some(old) = &state.presented {
            if old.intent == state.intent && policy == ScrollPolicy::FollowEnd && old.at_end() {
                offset = end;
            } else if matches!(policy, ScrollPolicy::Anchor | ScrollPolicy::FollowEnd)
                && let Some((key, delta)) = &old.anchor
                && let Some((_, start, _)) =
                    children.iter().find(|(candidate, _, _)| candidate == key)
            {
                // A history page can prepend rows while a wheel gesture is
                // awaiting paint. Preserve its movement relative to the last
                // visible child, rather than reusing an offset in the old page.
                let movement = i64::from(state.offset) - i64::from(old.offset);
                offset = (i64::from(*start) + *delta + movement).clamp(0, i64::from(end)) as u32;
            }
        }
        let reading = self
            .reading
            .as_ref()
            .filter(|reading| reading.root_key == key);
        let command = reading
            .and_then(|reading| {
                reading
                    .pending
                    .as_ref()
                    .filter(|pending| pending.intent == reading.scope.intent())
            })
            .filter(|pending| {
                !matches!(
                    pending.command.target,
                    maka_ui_runtime::reading::ReadingTarget::FocusItem { .. }
                        | maka_ui_runtime::reading::ReadingTarget::Find { .. }
                        | maka_ui_runtime::reading::ReadingTarget::ClearFind
                        | maka_ui_runtime::reading::ReadingTarget::RevealSource { .. }
                )
            })
            .map(|pending| {
                let command = &pending.command;
                use maka_ui_runtime::reading::ReadingTarget;
                let target = match &command.target {
                    ReadingTarget::FocusItem { .. }
                    | ReadingTarget::Find { .. }
                    | ReadingTarget::ClearFind
                    | ReadingTarget::RevealSource { .. } => unreachable!(),
                    ReadingTarget::Start => Some(0),
                    ReadingTarget::End => Some(end),
                    ReadingTarget::ScrollRows { rows } => {
                        Some((i64::from(offset) + i64::from(*rows)).clamp(0, i64::from(end)) as u32)
                    }
                    ReadingTarget::RevealChild { key } => children
                        .iter()
                        .find(|(candidate, _, _)| candidate == key)
                        .map(|(_, start, _)| (*start).min(end)),
                };
                if let Some(target) = target {
                    offset = target;
                }
                (command.sequence, target.is_some())
            });
        let visible_start =
            offset.saturating_add((i64::from(area.visible().y) - area.y).max(0) as u32);
        let visible_end = visible_start.saturating_add(u32::from(area.visible().height));
        let visible_child_keys = children
            .iter()
            .filter(|(_, start, height)| {
                *start < visible_end && start.saturating_add(*height) > visible_start
            })
            .map(|(key, _, _)| key.clone())
            .collect();
        let anchor = children
            .iter()
            .find(|(_, start, height)| start.saturating_add(*height) > offset)
            .map(|(key, start, _)| (key.clone(), i64::from(offset) - i64::from(*start)));
        Viewport {
            wheel: wheel.cloned(),
            area,
            content,
            offset,
            intent: state.intent,
            anchor,
            reading_revision: reading.map(|reading| reading.scope.revision()),
            reading_intent: reading.map(|reading| reading.scope.intent()),
            command,
            visible_child_keys,
        }
    }
    pub fn present(&mut self, geometry: Geometry) {
        // The whole batch already accepted this exact presentation recipe,
        // including panes, even if a guest refresh advanced the generation.
        for pane in geometry.panes {
            if let Some(state) = self.panes.get_mut(&pane.key) {
                state.present(pane);
            }
        }
        if geometry.generation != self.generation {
            return;
        }
        for choice in geometry.choices {
            if let Some(state) = self.choices.get_mut(&choice.choice.key) {
                state.geometry = Some(choice);
            }
        }
        for (key, viewport) in geometry.viewports {
            if let Some(state) = self.scrolls.get_mut(&key) {
                if viewport.area.is_empty() || state.intent != viewport.intent {
                    continue;
                }
                if let Some(reading) = self
                    .reading
                    .as_mut()
                    .filter(|reading| reading.root_key == key)
                {
                    reading.accept(&viewport);
                    if viewport.reading_revision != Some(reading.scope.revision()) {
                        continue;
                    }
                }
                state.offset = viewport.offset;
                state.initialized = true;
                state.presented = Some(viewport);
            }
        }
    }
    pub fn input(&mut self, key: &str, event: &crossterm::event::Event) -> bool {
        use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
        let Some(state) = self.scrolls.get_mut(key) else {
            return false;
        };
        let Some(viewport) = &state.presented else {
            return false;
        };
        if let Event::Mouse(mouse) = event {
            use crossterm::event::MouseButton;
            if mouse.kind == MouseEventKind::Up(MouseButton::Left) {
                return self.scrollbar_drag.take().is_some();
            }
            let captured = self
                .scrollbar_drag
                .as_ref()
                .is_some_and(|(drag_key, area)| drag_key == key && *area == viewport.area);
            let hit = viewport.end() > 0
                && viewport.area.width > 1
                && viewport.area.contains((mouse.column, mouse.row).into())
                && i64::from(mouse.column) == viewport.area.right() - 1;
            if (mouse.kind == MouseEventKind::Down(MouseButton::Left) && hit)
                || (mouse.kind == MouseEventKind::Drag(MouseButton::Left) && captured)
            {
                let row = (i64::from(mouse.row) - viewport.area.y)
                    .clamp(0, i64::from(viewport.area.height.saturating_sub(1)))
                    as u64;
                let next = (row * u64::from(viewport.end())
                    / u64::from(viewport.area.height.saturating_sub(1).max(1)))
                    as u32;
                self.scrollbar_drag = Some((key.to_owned(), viewport.area));
                let delta = i64::from(next) - i64::from(state.offset);
                state.offset = next;
                state.intent += 1;
                if let Some(reading) = self
                    .reading
                    .as_mut()
                    .filter(|reading| reading.root_key == key)
                {
                    reading.scope.touch();
                    reading.scrolled(delta);
                    reading.cancel();
                }
                return true;
            }
        }
        let delta = match event {
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => -1,
                MouseEventKind::ScrollDown => 1,
                _ => return false,
            },
            Event::Key(key)
                if key.kind != KeyEventKind::Release && key.modifiers == KeyModifiers::NONE =>
            {
                match key.code {
                    KeyCode::PageUp => -i64::from(viewport.area.height.max(1)),
                    KeyCode::PageDown => i64::from(viewport.area.height.max(1)),
                    KeyCode::Home => -i64::from(u32::MAX),
                    KeyCode::End => i64::from(u32::MAX),
                    _ => return false,
                }
            }
            _ => return false,
        };
        let next = (i64::from(state.offset) + delta).clamp(0, i64::from(viewport.end())) as u32;
        state.intent += 1;
        let reading_scroll = if let Some(reading) = self
            .reading
            .as_mut()
            .filter(|reading| reading.root_key == key)
        {
            reading.scope.touch();
            reading.scrolled(delta);
            reading.cancel();
            true
        } else {
            false
        };
        let changed = state.offset != next;
        state.offset = next;
        changed || reading_scroll
    }
}

impl SceneState {
    /// Original Surface reveal_offsets: expose the complete logical row with
    /// the least scroll, preserving the same native offset and intent owner.
    pub fn reveal(
        &mut self,
        key: &str,
        viewport: &Viewport,
        row: &mut super::CellRect,
        physical: bool,
    ) -> bool {
        let Some(state) = self.scrolls.get_mut(key) else {
            return false;
        };
        // The allocated viewport, not its physical clip: an outer scroller can
        // hide this entire group. Carry the adjusted logical row outward.
        let next = viewport.reveal_offset(row);
        let changed = next != viewport.offset;
        if physical || changed {
            state.intent += 1;
            state.offset = next;
            if let Some(reading) = self
                .reading
                .as_mut()
                .filter(|reading| reading.root_key == key)
            {
                if physical {
                    reading.scope.touch();
                    reading.cancel();
                } else {
                    reading.scope.0.borrow_mut().revision += 1;
                }
            }
        }
        changed
    }
}
