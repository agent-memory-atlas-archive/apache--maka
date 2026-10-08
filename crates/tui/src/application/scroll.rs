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
    pub dialogs: BTreeMap<String, std::rc::Rc<super::dialog::Opening>>,
    pub reading: Option<super::reading::ReadingState>,
    pub scrolls: BTreeMap<String, ScrollState>,
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
    pub viewports: Vec<(String, Viewport)>,
}
impl Viewport {
    pub fn end(&self) -> u32 {
        self.content.saturating_sub(self.area.height)
    }
    pub fn at_end(&self) -> bool {
        let visible_rows = (i64::from(self.area.visible().bottom()) - self.area.y).max(0) as u32;
        self.offset.saturating_add(visible_rows) >= self.content
    }
}
impl SceneState {
    pub fn commit(&mut self, node: &Node) {
        fn visit(node: &Node, keys: &mut Vec<String>) {
            if let Node::Dialog {
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
                children,
                ..
            } = node
            {
                if *scroll != ScrollPolicy::None {
                    keys.push(key.clone());
                }
                for child in children {
                    visit(child, keys);
                }
            }
        }
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
        if let Some(old) = state
            .presented
            .as_ref()
            .filter(|old| old.intent == state.intent)
        {
            if policy == ScrollPolicy::FollowEnd && old.at_end() {
                offset = end;
            } else if matches!(policy, ScrollPolicy::Anchor | ScrollPolicy::FollowEnd)
                && let Some((key, delta)) = &old.anchor
                && let Some((_, start, _)) =
                    children.iter().find(|(candidate, _, _)| candidate == key)
            {
                offset = (i64::from(*start) + *delta).clamp(0, i64::from(end)) as u32;
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
            .map(|pending| {
                let command = &pending.command;
                use maka_ui::reading::ReadingTarget;
                let target = match &command.target {
                    ReadingTarget::Start => Some(0),
                    ReadingTarget::End => Some(end),
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
        if geometry.generation != self.generation {
            return;
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
        let delta = match event {
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => -3,
                MouseEventKind::ScrollDown => 3,
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
        if let Some(reading) = self
            .reading
            .as_mut()
            .filter(|reading| reading.root_key == key)
        {
            reading.scope.touch();
            reading.cancel();
        }
        let changed = state.offset != next;
        state.offset = next;
        changed
    }
}
