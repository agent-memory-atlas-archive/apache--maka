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

//! Original Chrome sidebar mechanics over one retained native pane.
use super::{dialog::Opening, scene::Node, scroll::SceneState};
use maka_ui::scene::terminal::PaneOperation;
use std::{rc::Rc, time::Instant};

#[derive(Default)]
pub(super) struct State {
    expanded: Option<bool>,
    sequence: u64,
    transition: Option<Transition>,
    pub geometry: Option<Geometry>,
}
struct Transition {
    from: u16,
    to: u16,
    started: Instant,
}
#[derive(Clone)]
pub(super) struct Geometry {
    pub key: String,
    pub width: u16,
    pub inline_width: u16,
    pub target: u16,
    pub time: Instant,
}
impl State {
    pub fn target(&self, node: &Node, width: u16) -> u16 {
        let Node::AdaptivePane {
            pane_width,
            inline_min_width,
            auto_inline_min_width,
            ..
        } = node
        else {
            unreachable!()
        };
        if width >= *inline_min_width && self.expanded.unwrap_or(width >= *auto_inline_min_width) {
            *pane_width
        } else {
            0
        }
    }
    pub fn width(&self, node: &Node, width: u16, time: Instant) -> u16 {
        let Node::AdaptivePane { motion, .. } = node else {
            unreachable!()
        };
        let target = self.target(node, width);
        match &self.transition {
            Some(transition) if transition.to == target && *motion => {
                let t = (time
                    .saturating_duration_since(transition.started)
                    .as_secs_f32()
                    / 0.16)
                    .min(1.0);
                let eased = 1.0 - (1.0 - t).powi(3);
                (transition.from as f32 + (target as f32 - transition.from as f32) * eased).round()
                    as u16
            }
            _ => target,
        }
        .min(width)
    }
    pub fn present(&mut self, geometry: Geometry) {
        if self.transition.as_ref().is_some_and(|transition| {
            geometry
                .time
                .saturating_duration_since(transition.started)
                .as_secs_f32()
                >= 0.16
                || geometry.target != transition.to
        }) {
            self.transition = None;
        }
        self.geometry = Some(geometry);
    }
}
impl SceneState {
    pub fn commit_panes(&mut self, node: &Node) {
        fn visit(state: &mut SceneState, node: &Node, keys: &mut Vec<String>) {
            match node {
                Node::AdaptivePane {
                    key,
                    background,
                    content,
                    pane_width,
                    inline_min_width,
                    auto_inline_min_width,
                    motion,
                    command,
                    ..
                } => {
                    keys.push(key.clone());
                    let pane = state.panes.entry(key.clone()).or_default();
                    if !motion {
                        pane.transition = None;
                    }
                    if let Some(command) = command
                        .as_ref()
                        .filter(|command| command.sequence > pane.sequence)
                    {
                        pane.sequence = command.sequence;
                        match command.op {
                            PaneOperation::DismissTransient => {
                                state.dialogs.remove(key);
                            }
                            PaneOperation::Toggle if state.dialogs.contains_key(key) => {
                                state.dialogs.remove(key);
                            }
                            PaneOperation::Toggle => {
                                if let Some(geometry) = &pane.geometry {
                                    if geometry.width < *inline_min_width {
                                        state.dialogs.insert(key.clone(), Rc::new(Opening::new()));
                                    } else {
                                        let expanded = !pane
                                            .expanded
                                            .unwrap_or(geometry.width >= *auto_inline_min_width);
                                        pane.expanded = Some(expanded);
                                        pane.transition = motion.then_some(Transition {
                                            from: geometry.inline_width,
                                            to: if expanded { *pane_width } else { 0 },
                                            started: Instant::now(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    visit(state, background, keys);
                    visit(state, content, keys);
                }
                Node::Dialog {
                    background,
                    content,
                    ..
                } => {
                    visit(state, background, keys);
                    visit(state, content, keys);
                }
                Node::Item { content, .. } => visit(state, content, keys),
                Node::Stack { children, .. } => {
                    for child in children {
                        visit(state, child, keys);
                    }
                }
                _ => {}
            }
        }
        let mut keys = Vec::new();
        visit(self, node, &mut keys);
        self.panes.retain(|key, _| keys.contains(key));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn original_sidebar_transition_reverses_from_published_width_and_stops_on_resize() {
        let now = Instant::now();
        let mut node: Node = serde_json::from_value(serde_json::json!({
            "kind":"adaptive_pane","key":"pane","pane_width":32,"inline_min_width":60,"auto_inline_min_width":90,"motion":true,"command":null,
            "overlay":{"label":"Pane","width":40,"height":10,"background_color":0,"border_color":0},
            "background":{"kind":"text","key":"body","text":"","color":0},
            "content":{"kind":"text","key":"content","text":"","color":0}
        })).unwrap();
        let mut state = State {
            expanded: Some(false),
            transition: Some(Transition {
                from: 32,
                to: 0,
                started: now,
            }),
            ..Default::default()
        };
        let midway = now + Duration::from_millis(60);
        let width = state.width(&node, 120, midway);
        assert!((1..32).contains(&width));
        state.present(Geometry {
            key: "pane".into(),
            width: 120,
            inline_width: width,
            target: 0,
            time: midway,
        });
        state.expanded = Some(true);
        state.transition = Some(Transition {
            from: state.geometry.as_ref().unwrap().inline_width,
            to: 32,
            started: midway,
        });
        assert_eq!(state.width(&node, 120, midway), width);
        assert_eq!(
            state.width(&node, 120, now + Duration::from_millis(220)),
            32
        );
        state.present(Geometry {
            key: "pane".into(),
            width: 59,
            inline_width: 0,
            target: 0,
            time: midway,
        });
        assert!(
            state.transition.is_none(),
            "original resize cancels a transition to another target"
        );
        assert_eq!(state.width(&node, 120, midway), 32);
        state.expanded = Some(false);
        state.transition = Some(Transition {
            from: 32,
            to: 0,
            started: now,
        });
        if let Node::AdaptivePane { motion, .. } = &mut node {
            *motion = false;
        }
        assert_eq!(
            state.width(&node, 120, now),
            0,
            "reduced motion goes directly to the target"
        );
    }
}
