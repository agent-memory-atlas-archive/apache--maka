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
    focus::{Exit, Return},
    paint::{Ancestor, Target, TargetIdentity, TargetKind},
    scene::Node,
};
use std::rc::{Rc, Weak};
use unicode_width::UnicodeWidthStr;

pub(super) struct Opening {
    pub pending_flush: std::cell::Cell<bool>,
    pub anchor: Option<ratatui::layout::Position>,
    pub selection: Option<Rc<str>>,
}
impl Opening {
    pub(super) fn new() -> Self {
        Self {
            pending_flush: std::cell::Cell::new(true),
            anchor: None,
            selection: None,
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
    role: (std::mem::Discriminant<TargetKind>, bool),
}
impl WeakTarget {
    pub(super) fn new(target: &Target) -> Self {
        Self {
            mount: target.mount.clone(),
            key: target.key.clone(),
            owner: Rc::downgrade(&target.owner),
            role: target.role(),
        }
    }
    pub(super) fn matches(&self, target: &Target) -> bool {
        target.mount == self.mount
            && target.key == self.key
            && self.owner.ptr_eq(&Rc::downgrade(&target.owner))
            && self.role == target.role()
    }
    pub(super) fn same(&self, other: &Self) -> bool {
        self.mount == other.mount
            && self.key == other.key
            && self.owner.ptr_eq(&other.owner)
            && self.role == other.role
    }
    pub(super) fn exited(&self, exits: &[Exit]) -> bool {
        exits
            .iter()
            .any(|exit| exit.mount == self.mount.id() && exit.owner.ptr_eq(&self.owner))
    }
    pub fn resolve(&self, targets: &[Target]) -> Option<Target> {
        targets.iter().find(|target| self.matches(target)).cloned()
    }
}
pub(super) struct FocusReturn {
    opening: Scope,
    target: Option<WeakTarget>,
    native_return: Option<Return>,
}

impl super::scroll::SceneState {
    pub fn prepare_dialogs(&mut self, node: &Node, visible: bool) {
        fn visit(
            node: &Node,
            visible: bool,
            keys: &mut Vec<String>,
            choices: &mut Vec<String>,
            openings: &std::collections::BTreeMap<String, Rc<Opening>>,
        ) {
            match node {
                Node::Item { content, .. } => visit(content, visible, keys, choices, openings),
                Node::Choice(choice) => {
                    if visible && !choice.disabled && !choice.options.is_empty() {
                        choices.push(choice.key.clone());
                        if openings.contains_key(&choice.key) {
                            keys.push(choice.key.clone());
                        }
                    }
                }
                Node::ContextMenu {
                    key,
                    background,
                    content,
                    ..
                }
                | Node::AdaptivePane {
                    key,
                    background,
                    content,
                    ..
                } => {
                    visit(background, visible, keys, choices, openings);
                    if visible && openings.contains_key(key) {
                        keys.push(key.clone());
                    }
                    // Layout visibility is acknowledged by the complete paint
                    // batch, including openings declared in an unplaced pane.
                    visit(content, visible, keys, choices, openings);
                }
                Node::Dialog {
                    key,
                    background,
                    content,
                    open,
                    ..
                } => {
                    visit(background, visible, keys, choices, openings);
                    if visible && *open {
                        keys.push(key.clone());
                    }
                    visit(content, visible && *open, keys, choices, openings);
                }
                Node::Stack {
                    children,
                    visible: local,
                    ..
                } => {
                    for child in children {
                        visit(child, visible && *local, keys, choices, openings);
                    }
                }
                _ => {}
            }
        }
        let mut keys = Vec::new();
        let mut choices = Vec::new();
        visit(node, visible, &mut keys, &mut choices, &self.dialogs);
        self.choices.retain(|key, _| choices.contains(key));
        for key in choices {
            self.choices.entry(key).or_default();
        }
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
    /// Consume invalidation against the captured native owner, including a
    /// background selector covered by a modal. Its foreground keeps focus.
    pub(super) fn apply_modal_focus_exits(&mut self, exits: &[Exit]) {
        for memory in &mut self.modal_focus {
            if memory
                .target
                .as_ref()
                .is_some_and(|target| target.exited(exits))
            {
                memory.target = match memory.native_return.take() {
                    Some(Return::Active { source, target, .. }) if source.exited(exits) => target,
                    _ => None,
                };
                memory.native_return = Some(Return::Exited);
            }
        }
        // The opening may not have flushed yet: the original target still
        // resides in pre-flush custody rather than in a modal memory.
        if self
            .focus_before_flush
            .as_ref()
            .is_some_and(|target| target.exited(exits))
        {
            self.focus_before_flush = match self.native_return.take() {
                Some(Return::Active { source, target, .. }) if source.exited(exits) => target,
                _ => None,
            };
            self.native_return = Some(Return::Exited);
        }
    }
    /// Mechanical focus memory follows actual published ancestry, not mount order.
    pub(super) fn publish_modal_focus(&mut self, path: &[Scope], targets: &[Target]) {
        let prior = self
            .focus_before_flush
            .take()
            .or_else(|| self.focus.as_ref().map(WeakTarget::new));
        let retained = prior
            .as_ref()
            // A pane can reappear inline with the same target identity after
            // its drawer closes. That must not defeat the modal focus return.
            .filter(|_| same_scope(&self.physical_path.last().cloned(), &path.last().cloned()))
            .and_then(|target| target.resolve(targets))
            .map(|target| (Some(target), self.native_return.clone()));
        let restore = |memory: &FocusReturn| {
            memory
                .target
                .as_ref()
                .and_then(|target| target.resolve(targets))
                .map(|target| (Some(target), memory.native_return.clone()))
        };
        let exited = self.modal_focus.iter().any(|memory| {
            memory.opening.strong_count() == 0
                && matches!(memory.native_return, Some(Return::Exited))
        });
        let departing_memory = self.physical_path.last().and_then(|scope| {
            self.modal_focus
                .iter()
                .find(|memory| memory.opening.ptr_eq(scope))
        });
        // An action can close its menu and open a dialog in the same frame.
        // That replacement inherits the original return, not the retired action.
        let replacement = departing_memory
            .filter(|memory| memory.opening.strong_count() == 0)
            .map(|memory| (memory.target.clone(), memory.native_return.clone()));
        let departing = departing_memory.and_then(restore);
        let closed = self
            .modal_focus
            .iter()
            .rev()
            .filter(|memory| memory.opening.strong_count() == 0)
            .find_map(restore);
        self.modal_focus
            .retain(|memory| memory.opening.strong_count() != 0);
        let mut opened = false;
        for (index, opening) in path.iter().enumerate() {
            if self
                .modal_focus
                .iter()
                .any(|memory| memory.opening.ptr_eq(opening))
            {
                continue;
            }
            opened = true;
            // A wrapper inherits the original target and its exact native return
            // together. A covering sibling captures the actual foreground.
            let inherited = path[index + 1..].iter().find_map(|descendant| {
                self.modal_focus
                    .iter()
                    .find(|memory| memory.opening.ptr_eq(descendant))
                    .map(|memory| (memory.target.clone(), memory.native_return.clone()))
            });
            let (target, native_return) =
                inherited
                    .or_else(|| replacement.clone())
                    .unwrap_or_else(|| {
                        let native_return = match &self.native_return {
                            Some(Return::Active { source, .. })
                                if prior.as_ref().is_some_and(|prior| source.same(prior)) =>
                            {
                                self.native_return.clone()
                            }
                            Some(Return::Exited) => Some(Return::Exited),
                            _ => None,
                        };
                        (prior.clone(), native_return)
                    });
            self.modal_focus.push(FocusReturn {
                opening: opening.clone(),
                target,
                native_return,
            });
        }
        // A newly opened foreground must receive its own focus. Any background
        // exit marker now belongs to that opening's captured return, not its input.
        if opened {
            self.native_return = None;
        }
        // Resolve every surviving original ancestor before suppressing fallback
        // for an invalidated inner return. Simultaneous closes cannot hide it.
        let restore = retained.or(if opened {
            None
        } else {
            departing
                .or(closed)
                .or(exited.then_some((None, Some(Return::Exited))))
        });
        if self.focus.is_none()
            && let Some((target, native_return)) = restore
        {
            self.restore_focus = target.as_ref().map(WeakTarget::new);
            self.native_return = native_return.or((!opened && exited).then_some(Return::Exited));
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
        if self
            .application
            .frame(&target.mount)?
            .view
            .view
            .scene()
            .and_then(|node| node.overlay(&target.key))
            .is_some_and(|node| {
                matches!(node, Node::AdaptivePane { .. } | Node::ContextMenu { .. })
            })
        {
            let state = self
                .resources
                .owners
                .get_mut(&target.mount.id())
                .unwrap()
                .state
                .downcast_mut::<super::scroll::SceneState>()
                .unwrap();
            if state.dialogs.get(&target.key).is_some_and(|opening| {
                target
                    .scope
                    .as_ref()
                    .is_some_and(|scope| scope.ptr_eq(&Rc::downgrade(opening)))
            }) {
                state.dialogs.remove(&target.key);
            }
            return self.sync_native();
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
    pub anchor: Option<super::Placement>,
}
impl Presenter {
    pub(super) fn paint_dialogs(
        &self,
        buffer: &mut ratatui::buffer::Buffer,
        scene: &mut super::PaintedScene,
        measurements: super::paint::MeasureContext<'_>,
    ) -> maka_ui::Result<()> {
        use ratatui::{
            style::Style,
            widgets::{Block, Borders, Widget as _},
        };
        while let Some(deferred) = scene.dialogs.pop_front() {
            let recipe = &self.application.frame(&deferred.mount)?.view.view;
            let node = recipe
                .scene()
                .and_then(|node| node.overlay(&deferred.key))
                .expect("original deferred overlay");
            let full = scene.viewport;
            let (body, padding) = match node {
                Node::Dialog { .. } | Node::ContextMenu { .. } | Node::AdaptivePane { .. } => {
                    let (appearance, content) = node.dialog_content().unwrap();
                    let maka_ui::scene::terminal::DialogAppearance {
                        width,
                        height,
                        padding,
                        ..
                    } = appearance;
                    let margin = if full.width >= 4 && full.height >= 4 {
                        1
                    } else {
                        0
                    };
                    let mut width = (*width).min(full.width.saturating_sub(margin * 2));
                    let maximum_height = (*height).min(full.height.saturating_sub(margin * 2));
                    let padding = (*padding)
                        .min(width.saturating_sub(3) / 2)
                        .min(maximum_height.saturating_sub(3) / 2);
                    let inset = padding.saturating_add(1).saturating_mul(2);
                    let (content_width, content_height) = self.measure(
                        &deferred.mount,
                        content,
                        width.saturating_sub(inset),
                        measurements,
                    )?;
                    if matches!(node, Node::ContextMenu { .. }) {
                        let copy_width = match node {
                            Node::ContextMenu {
                                copy_selection_label: Some(label),
                                ..
                            } if deferred
                                .opening
                                .upgrade()
                                .is_some_and(|opening| opening.selection.is_some()) =>
                            {
                                super::paint::text_size(label, width).0
                            }
                            _ => 0,
                        };
                        width =
                            width.min(content_width.max(copy_width).saturating_add(inset).max(4));
                    }
                    // Growing content needs a bounded viewport before flex allocation;
                    // its zero-basis children intentionally have no intrinsic height.
                    let selection_row = u32::from(
                        deferred
                            .opening
                            .upgrade()
                            .is_some_and(|opening| opening.selection.is_some()),
                    );
                    let height = if self
                        .node_layout(&deferred.mount, content, measurements)?
                        .grow
                        > 0
                    {
                        maximum_height
                    } else {
                        maximum_height.min(
                            content_height
                                .saturating_add(selection_row)
                                .saturating_add(u32::from(inset))
                                .min(u32::from(u16::MAX)) as u16,
                        )
                    }
                    .max(1);
                    let mut body = ratatui::layout::Rect::new(
                        full.x + full.width.saturating_sub(width) / 2,
                        full.y + full.height.saturating_sub(height) / 2,
                        width,
                        height,
                    );

                    if let Some(anchor) = deferred
                        .opening
                        .upgrade()
                        .and_then(|opening| opening.anchor)
                    {
                        body.x = anchor.x.clamp(full.x, full.right().saturating_sub(width));
                        body.y = anchor.y.clamp(full.y, full.bottom().saturating_sub(height));
                    }
                    (body, padding)
                }
                Node::Choice(choice) => {
                    let Some(body) = super::choice::bounds(
                        choice,
                        deferred.anchor.expect("choice anchor"),
                        full,
                    ) else {
                        // Absence in this successful batch closes the local opening.
                        continue;
                    };
                    (body, 0)
                }
                _ => unreachable!(),
            };
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
                focus_group: None,
                scroll: None,
                focus_shortcuts: Vec::new(),
            }));
            scene.targets.push(Target {
                identity,
                revision: self.application.frame(&deferred.mount)?.revision,
                area: full.into(),
                tab_index: -1,
                autofocus: false,
                kind: if matches!(node, Node::Choice(_)) {
                    TargetKind::Native
                } else {
                    TargetKind::Backdrop { body: body.into() }
                },
                ancestors: Vec::new(),
            });
            for y in body.y..body.bottom() {
                // A background glyph may start outside the overlay but occupy
                // its first column. Retaining that lead makes terminal diffing
                // skip the new border, so erase the exposed fragment as well.
                let mut x = buffer.area.x;
                while x < body.x {
                    let width = buffer[(x, y)].symbol().width().max(1);
                    let end = usize::from(x) + width;
                    if end > usize::from(body.x) {
                        for column in x..body.x {
                            buffer[(column, y)].set_symbol(" ");
                        }
                        break;
                    }
                    x = end as u16;
                }
                for x in body.x..body.right() {
                    buffer[(x, y)].reset();
                }
            }
            let remaining = std::mem::take(&mut scene.dialogs);
            let mut viewports = Vec::new();
            let mut choices = Vec::new();
            let mut panes = Vec::new();
            match node {
                Node::Choice(choice) => {
                    buffer.set_style(
                        body,
                        Style::default().bg(super::paint::color(choice.background)),
                    );
                    choices.push(self.paint_choice_menu(
                        &deferred.mount,
                        choice,
                        body,
                        buffer,
                        scene,
                    ));
                }
                Node::Dialog { .. } | Node::ContextMenu { .. } | Node::AdaptivePane { .. } => {
                    let (appearance, content) = node.dialog_content().unwrap();
                    let maka_ui::scene::terminal::DialogAppearance {
                        label,
                        background_color,
                        border_color,
                        ..
                    } = appearance;
                    Block::new()
                        .borders(Borders::ALL)
                        .border_set(if appearance.ascii {
                            ratatui::symbols::border::Set {
                                top_left: "+",
                                top_right: "+",
                                bottom_left: "+",
                                bottom_right: "+",
                                horizontal_top: "-",
                                horizontal_bottom: "-",
                                vertical_left: "|",
                                vertical_right: "|",
                            }
                        } else {
                            ratatui::symbols::border::ROUNDED
                        })
                        .title(ratatui::text::Span::styled(
                            if matches!(node, Node::ContextMenu { .. }) {
                                String::new()
                            } else {
                                super::sanitize(label)
                            },
                            Style::default().fg(super::paint::color(
                                appearance.title_color.unwrap_or(*border_color),
                            )),
                        ))
                        .style(Style::default().bg(super::paint::color(*background_color)))
                        .border_style(Style::default().fg(super::paint::color(*border_color)))
                        .render(body, buffer);
                    let mut content_area =
                        super::Placement::from(body).inner(ratatui::layout::Margin::new(
                            padding.saturating_add(1),
                            padding.saturating_add(1),
                        ));
                    if let Node::ContextMenu {
                        copy_selection_label: Some(label),
                        ..
                    } = node
                        && let Some(text) = deferred
                            .opening
                            .upgrade()
                            .and_then(|opening| opening.selection.clone())
                    {
                        let row = super::Placement::new(
                            super::CellRect::new(
                                content_area.x,
                                content_area.y,
                                content_area.width,
                                1,
                            ),
                            content_area.clip,
                        );
                        let target = Target {
                            identity: TargetIdentity {
                                mount: deferred.mount.clone(),
                                key: deferred.key.clone(),
                                owner: self.resources.owners[&deferred.mount.id()].identity.clone(),
                                scope: scene.input_scope.clone(),
                            },
                            revision: self.application.frame(&deferred.mount)?.revision,
                            area: row,
                            tab_index: 0,
                            autofocus: false,
                            kind: TargetKind::SelectionCopy { text },
                            ancestors: Vec::new(),
                        };
                        let mut style = Style::default().fg(super::paint::color(
                            appearance.title_color.unwrap_or(appearance.border_color),
                        ));
                        if self.focus.as_ref().is_some_and(|focus| focus.same(&target))
                            || self.hover.as_ref().is_some_and(|hover| hover.same(&target))
                        {
                            style = style.add_modifier(ratatui::style::Modifier::REVERSED);
                        }
                        buffer.set_style(row.visible(), style);
                        super::paint::text(label, row, buffer, style);
                        if !row.is_empty() {
                            scene.targets.push(target);
                        }
                        content_area = super::Placement::new(
                            super::CellRect::new(
                                content_area.x,
                                content_area.y + 1,
                                content_area.width,
                                content_area.height.saturating_sub(1),
                            ),
                            content_area.clip,
                        );
                    }
                    self.paint_node(
                        &deferred.mount,
                        content,
                        content_area,
                        buffer,
                        scene,
                        &mut super::paint::NodePaint {
                            measurements,
                            viewports: &mut viewports,
                            choices: &mut choices,
                            panes: &mut panes,
                        },
                    )?;
                }
                _ => unreachable!(),
            }
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
                    choices,
                    panes,
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
