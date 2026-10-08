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

//! Controlled terminal dialogs retain only original opening and weak focus custody.
use super::{
    Presenter,
    paint::{Ancestor, Target, TargetIdentity, TargetKind},
    scene::Node,
};
use std::rc::{Rc, Weak};

pub(super) struct Opening {
    pub pending_flush: std::cell::Cell<bool>,
}
impl Opening {
    fn new() -> Self {
        Self {
            pending_flush: std::cell::Cell::new(true),
        }
    }
}
pub(super) type Scope = Weak<Opening>;

pub(super) fn same_scope(a: &Option<Scope>, b: &Option<Scope>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.ptr_eq(b) && a.strong_count() != 0,
        _ => false,
    }
}
pub(super) fn same_path(a: &[Scope], b: &[Scope]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.ptr_eq(b) && a.strong_count() != 0)
}

#[derive(Clone)]
pub(super) struct WeakTarget {
    mount: maka_plugin_core::runtime::Mount,
    key: String,
    owner: Weak<()>,
}
impl WeakTarget {
    pub(super) fn new(target: &Target) -> Self {
        Self {
            mount: target.mount.clone(),
            key: target.key.clone(),
            owner: Rc::downgrade(&target.owner),
        }
    }
    pub fn resolve(&self, targets: &[Target]) -> Option<Target> {
        targets
            .iter()
            .find(|target| {
                target.mount == self.mount
                    && target.key == self.key
                    && self.owner.ptr_eq(&Rc::downgrade(&target.owner))
            })
            .cloned()
    }
}
pub(super) struct FocusReturn {
    opening: Scope,
    target: Option<WeakTarget>,
}

impl super::scroll::SceneState {
    pub fn prepare_dialogs(&mut self, node: &Node, visible: bool) {
        fn visit(node: &Node, visible: bool, keys: &mut Vec<String>) {
            match node {
                Node::Dialog {
                    key,
                    background,
                    content,
                    open,
                    ..
                } => {
                    visit(background, visible, keys);
                    if visible && *open {
                        keys.push(key.clone());
                    }
                    visit(content, visible && *open, keys);
                }
                Node::Stack {
                    children,
                    visible: local,
                    ..
                } => {
                    for child in children {
                        visit(child, visible && *local, keys);
                    }
                }
                _ => {}
            }
        }
        let mut keys = Vec::new();
        visit(node, visible, &mut keys);
        self.dialogs.retain(|key, _| keys.contains(key));
        for key in keys {
            self.dialogs
                .entry(key)
                .or_insert_with(|| Rc::new(Opening::new()));
        }
    }
}

impl Presenter {
    /// Original opening witnesses include unplaced declarations. Their first
    /// successful batch acknowledges omission as well as actual placement.
    pub(super) fn opening_witnesses(&self) -> maka_ui::Result<Vec<Scope>> {
        let mut openings = Vec::new();
        for mount in self.application.mounted_subtree(&self.root)? {
            if self.application.is_retiring(&mount) {
                continue;
            }
            if let Some(state) = self
                .resources
                .owners
                .get(&mount.id())
                .filter(|owner| owner.live)
                .and_then(|owner| owner.state.downcast_ref::<super::scroll::SceneState>())
            {
                openings.extend(state.dialogs.values().map(Rc::downgrade));
            }
        }
        Ok(openings)
    }
    pub(super) fn physical_admitted(&self) -> bool {
        self.resources
            .owners
            .values()
            .filter(|owner| owner.live)
            .all(|owner| {
                owner
                    .state
                    .downcast_ref::<super::scroll::SceneState>()
                    .is_none_or(|state| {
                        state
                            .dialogs
                            .values()
                            .all(|opening| !opening.pending_flush.get())
                    })
            })
            && self
                .physical_path
                .iter()
                .all(|scope| scope.strong_count() != 0)
    }
    pub(super) fn target_admitted(&self, target: &TargetIdentity) -> bool {
        self.physical_admitted() && same_scope(&target.scope, &self.physical_path.last().cloned())
    }
    pub(super) fn scene_openings_current(&self, openings: &[Scope]) -> bool {
        self.opening_witnesses()
            .is_ok_and(|current| same_path(openings, &current))
    }
    pub(super) fn sync_modal_focus(&mut self) -> maka_ui::Result<()> {
        if !self.physical_admitted() {
            self.resources.cancel_pointer()?;
            self.drag = None;
            if self.focus_before_flush.is_none() {
                self.focus_before_flush = self.focus.as_ref().map(WeakTarget::new);
            }
            self.focus = None;
        }
        Ok(())
    }
    /// Mechanical focus memory follows actual published ancestry, not mount order.
    pub(super) fn publish_modal_focus(&mut self, path: &[Scope], targets: &[Target]) {
        let prior = self
            .focus_before_flush
            .take()
            .or_else(|| self.focus.as_ref().map(WeakTarget::new));
        let retained = prior.as_ref().and_then(|target| target.resolve(targets));
        let departing = self
            .physical_path
            .last()
            .and_then(|scope| {
                self.modal_focus
                    .iter()
                    .find(|memory| memory.opening.ptr_eq(scope))
            })
            .and_then(|memory| memory.target.as_ref())
            .and_then(|target| target.resolve(targets));
        let closed = self
            .modal_focus
            .iter()
            .rev()
            .filter(|memory| memory.opening.strong_count() == 0)
            .filter_map(|memory| memory.target.as_ref())
            .find_map(|target| target.resolve(targets));
        self.modal_focus
            .retain(|memory| memory.opening.strong_count() != 0);
        for (index, opening) in path.iter().enumerate() {
            if self
                .modal_focus
                .iter()
                .any(|memory| memory.opening.ptr_eq(opening))
            {
                continue;
            }
            // A new outer wrapping an existing foreground inherits that original
            // return target. A new covering sibling remembers the actual focus.
            let inherited = path[index + 1..].iter().find_map(|descendant| {
                self.modal_focus
                    .iter()
                    .find(|memory| memory.opening.ptr_eq(descendant))
                    .and_then(|memory| memory.target.clone())
            });
            self.modal_focus.push(FocusReturn {
                opening: opening.clone(),
                target: inherited.or_else(|| prior.clone()),
            });
        }
        if self.focus.is_none() {
            self.restore_focus = retained
                .or(departing)
                .or(closed)
                .as_ref()
                .map(WeakTarget::new);
        }
    }
    pub(super) fn dismiss(
        &mut self,
        target: &Target,
        reason: maka_ui::dismiss::Reason,
    ) -> maka_ui::Result<()> {
        if !self.target_admitted(target) || !self.resources.admits(target) {
            return Ok(());
        }
        self.application.dispatch(
            &target.mount,
            target.revision,
            &target.key,
            serde_json::to_value(maka_ui::dismiss::Input::Dismiss { reason })
                .expect("dismiss input"),
        )?;
        self.sync_native()
    }
    pub(super) fn dismiss_escape(
        &mut self,
        event: &crossterm::event::Event,
    ) -> maka_ui::Result<bool> {
        use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
        if !matches!(event, Event::Key(key) if key.code == KeyCode::Esc && key.kind != KeyEventKind::Release && key.modifiers == KeyModifiers::NONE)
        {
            return Ok(false);
        }
        if let Some(target) = self
            .scene
            .iter()
            .rev()
            .find(|target| matches!(target.kind, TargetKind::Backdrop { .. }))
            .cloned()
        {
            self.dismiss(&target, maka_ui::dismiss::Reason::Escape)?;
            return Ok(true);
        }
        Ok(false)
    }
}

/// Paint-local deferred surface; neither this nor its weak opening retains authority.
pub(super) struct Deferred {
    pub mount: maka_plugin_core::runtime::Mount,
    pub key: String,
    pub opening: Scope,
    pub parent_path: Vec<Scope>,
}
impl Node {
    fn dialog(&self, key: &str) -> Option<&Node> {
        match self {
            Self::Dialog {
                key: own,
                background,
                content,
                ..
            } => {
                if own == key {
                    Some(self)
                } else {
                    background.dialog(key).or_else(|| content.dialog(key))
                }
            }
            Self::Stack { children, .. } => children.iter().find_map(|node| node.dialog(key)),
            _ => None,
        }
    }
}
impl Presenter {
    pub(super) fn paint_dialogs(
        &self,
        buffer: &mut ratatui::buffer::Buffer,
        scene: &mut super::PaintedScene,
        measurements: &super::paint::Measurements,
    ) -> maka_ui::Result<()> {
        use ratatui::{
            style::Style,
            widgets::{Block, Borders, Widget as _},
        };
        while let Some(deferred) = scene.dialogs.pop_front() {
            let recipe = &self.application.frame(&deferred.mount)?.view.view;
            let Node::Dialog {
                label,
                content,
                width,
                height,
                background_color,
                border_color,
                padding,
                ..
            } = recipe
                .scene()
                .and_then(|node| node.dialog(&deferred.key))
                .expect("original deferred dialog")
            else {
                unreachable!()
            };
            let full = scene.viewport;
            let margin = if full.width >= 4 && full.height >= 4 {
                1
            } else {
                0
            };
            let width = (*width).min(full.width.saturating_sub(margin * 2));
            let maximum_height = (*height).min(full.height.saturating_sub(margin * 2));
            let padding = (*padding)
                .min(width.saturating_sub(3) / 2)
                .min(maximum_height.saturating_sub(3) / 2);
            let inset = padding.saturating_add(1).saturating_mul(2);
            let (_, content_height) = self.measure(
                &deferred.mount,
                content,
                width.saturating_sub(inset),
                measurements,
            )?;
            let height = maximum_height
                .min(
                    content_height
                        .saturating_add(u32::from(inset))
                        .min(u32::from(u16::MAX)) as u16,
                )
                .max(1);
            let body = ratatui::layout::Rect::new(
                full.x + full.width.saturating_sub(width) / 2,
                full.y + full.height.saturating_sub(height) / 2,
                width,
                height,
            );
            let previous_scope = scene.scope.take();
            let previous_input = scene.input_scope.replace(deferred.opening.clone());
            let previous_path = std::mem::replace(&mut scene.input_path, deferred.parent_path);
            scene.input_path.push(deferred.opening.clone());
            scene.foreground_path = scene.input_path.clone();
            let previous_ancestry = std::mem::take(&mut scene.ancestry);
            let identity = TargetIdentity {
                mount: deferred.mount.clone(),
                key: deferred.key.clone(),
                owner: self.resources.owners[&deferred.mount.id()].identity.clone(),
                scope: scene.input_scope.clone(),
            };
            scene.keyboard = Some(Rc::new(Ancestor {
                identity: identity.clone(),
                revision: self.application.frame(&deferred.mount)?.revision,
                shortcuts: Vec::new(),
            }));
            scene.targets.push(Target {
                identity,
                revision: self.application.frame(&deferred.mount)?.revision,
                area: full.into(),
                tab_index: -1,
                autofocus: false,
                kind: TargetKind::Backdrop { body: body.into() },
                ancestors: Vec::new(),
            });
            for y in body.y..body.bottom() {
                for x in body.x..body.right() {
                    buffer[(x, y)].reset();
                }
            }
            Block::new()
                .borders(Borders::ALL)
                .title(super::sanitize(label))
                .style(Style::default().bg(super::paint::color(*background_color)))
                .border_style(Style::default().fg(super::paint::color(*border_color)))
                .render(body, buffer);
            let remaining = std::mem::take(&mut scene.dialogs);
            let mut viewports = Vec::new();
            self.paint_node(
                &deferred.mount,
                content,
                super::Placement::from(body).inner(ratatui::layout::Margin::new(
                    padding.saturating_add(1),
                    padding.saturating_add(1),
                )),
                buffer,
                scene,
                &mut super::paint::NodePaint {
                    measurements,
                    viewports: &mut viewports,
                },
            )?;
            scene.geometries.push(super::paint::NativeGeometry {
                mount: deferred.mount.clone(),
                owner: self.resources.owners[&deferred.mount.id()].identity.clone(),
                geometry: Box::new(super::scroll::Geometry {
                    generation: self.resources.owners[&deferred.mount.id()]
                        .state
                        .downcast_ref::<super::scroll::SceneState>()
                        .unwrap()
                        .generation,
                    viewports,
                }),
            });
            self.paint_dialogs(buffer, scene, measurements)?;
            scene.dialogs = remaining;
            scene.scope = previous_scope;
            scene.input_scope = previous_input;
            scene.input_path = previous_path;
            scene.ancestry = previous_ancestry;
        }
        Ok(())
    }
}
