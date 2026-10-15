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

//! Retain one physical activation across a logical commit and its output receipt.
use super::{
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind, Presenter, dialog,
    paint,
};
use maka_ui_runtime::shortcut::Shortcut;
use ratatui::layout::Position;
use std::rc::Rc;

pub(super) struct CapturedActivation {
    binding: Binding,
    event: Event,
    point: Option<Position>,
    openings: Vec<dialog::Scope>,
}
enum Binding {
    Control(paint::Target),
    Shortcut {
        ancestor: Rc<paint::Ancestor>,
        declaration: Shortcut,
        navigation: bool,
    },
}
impl CapturedActivation {
    fn identity(&self) -> &paint::TargetIdentity {
        match &self.binding {
            Binding::Control(target) => target,
            Binding::Shortcut { ancestor, .. } => ancestor,
        }
    }

    pub(super) fn event(&self) -> Event {
        self.event.clone()
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Admission {
    Pending,
    Ready,
    Discard,
}
fn same_action(original: &paint::TargetKind, current: &paint::TargetKind) -> bool {
    use paint::TargetKind::*;
    match (original, current) {
        (Action { copy_text: a }, Action { copy_text: b }) => a == b,
        (SelectionCopy { text: a }, SelectionCopy { text: b }) => a == b,
        (
            Item {
                submit: a,
                submit_space: sa,
                ..
            },
            Item {
                submit: b,
                submit_space: sb,
                ..
            },
        ) => a == b && sa == sb,
        (Focus(a), Focus(b)) => a.child == b.child && a.owner.ptr_eq(&b.owner),
        _ => false,
    }
}
impl Presenter {
    pub(super) fn release_activation(
        &mut self,
        input: CapturedActivation,
    ) -> maka_ui_runtime::Result<Option<Event>> {
        debug_assert_eq!(self.activation_admission(&input), Admission::Ready);
        match input.binding {
            Binding::Control(original) => {
                if input.point.is_none() {
                    self.focus = self
                        .scene
                        .iter()
                        .find(|target| target.same(&original))
                        .cloned();
                }
                Ok(Some(input.event))
            }
            Binding::Shortcut {
                ancestor,
                declaration,
                ..
            } => {
                let revision = self.application.frame(&ancestor.mount)?.revision;
                self.application.dispatch(
                    &ancestor.mount,
                    revision,
                    &declaration.input,
                    serde_json::json!({"kind":"click"}),
                )?;
                self.sync_native()?;
                Ok(None)
            }
        }
    }
    pub(super) fn capture_shortcut(
        &self,
        ancestor: Rc<paint::Ancestor>,
        declaration: Shortcut,
        event: Event,
        navigation: bool,
    ) -> maka_ui_runtime::Result<CapturedActivation> {
        Ok(CapturedActivation {
            binding: Binding::Shortcut {
                ancestor,
                declaration,
                navigation,
            },
            event,
            point: None,
            openings: self.opening_witnesses()?,
        })
    }
    fn current_shortcut(
        &self,
        event: &Event,
        navigation: bool,
    ) -> Option<(&Rc<paint::Ancestor>, &Shortcut)> {
        let Event::Key(key) = event else {
            return None;
        };
        let (key, modifiers) = super::shortcut_chord(key)?;
        let ancestors = match &self.focus {
            Some(target) => target.ancestors.as_slice(),
            None => self.keyboard.as_slice(),
        };
        for ancestor in ancestors.iter().rev() {
            if navigation
                && ancestor
                    .focus_shortcuts
                    .iter()
                    .any(|binding| binding.declaration.matches(&key, modifiers))
            {
                return None;
            }
            if let Some(declaration) = ancestor
                .shortcuts
                .iter()
                .find(|shortcut| shortcut.matches(&key, modifiers, false))
            {
                return Some((ancestor, declaration));
            }
        }
        None
    }
    /// Native editors retain their own input path. Only explicit scene actions
    /// need this fence; their labels and selection styling may legitimately change.
    pub(super) fn capture_activation(&self, event: &Event) -> Option<CapturedActivation> {
        if !self.surface.is_live() || !self.physical_admitted() {
            return None;
        }
        let (target, point) = match event {
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                let point = (mouse.column, mouse.row).into();
                (
                    self.scene.iter().rev().find(|target| {
                        !matches!(
                            target.kind,
                            paint::TargetKind::ContextMenu { button: false }
                                | paint::TargetKind::Hint
                        ) && target.area.contains(point)
                    })?,
                    Some(point),
                )
            }
            Event::Key(key)
                if key.kind == KeyEventKind::Press
                    && key.modifiers == KeyModifiers::NONE
                    && matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) =>
            {
                (self.focus.as_ref()?, None)
            }
            _ => return None,
        };
        if !matches!(
            target.kind,
            paint::TargetKind::Action { .. }
                | paint::TargetKind::Item { .. }
                | paint::TargetKind::Focus(_)
                | paint::TargetKind::SelectionCopy { .. }
        ) || target.area.is_empty()
            || !self.target_admitted(target)
            || !self.resources.admits(target)
            || self.application.is_retiring(&target.mount)
            || !self.application.mount_visible(&target.mount)
        {
            return None;
        }
        if !self.application.ui_pending()
            && self
                .application
                .frame(&target.mount)
                .is_ok_and(|frame| frame.revision == target.revision)
        {
            return None;
        }
        // A declared shortcut owns this key before an ordinary scene action.
        if let Event::Key(event) = event
            && let Some((key, modifiers)) = super::shortcut_chord(event)
            && target.ancestors.iter().rev().any(|ancestor| {
                ancestor
                    .shortcuts
                    .iter()
                    .any(|shortcut| shortcut.matches(&key, modifiers, false))
            })
        {
            return None;
        }
        Some(CapturedActivation {
            binding: Binding::Control(target.clone()),
            event: event.clone(),
            point,
            openings: self.opening_witnesses().ok()?,
        })
    }
    pub(super) fn activation_admission(&self, input: &CapturedActivation) -> Admission {
        let original = input.identity();
        if !self.surface.is_live()
            || self.application.is_retiring(&original.mount)
            || !self.application.mount_visible(&original.mount)
            || !self.resources.owns(original)
            || !self.scene_openings_current(&input.openings)
        {
            return Admission::Discard;
        }
        if self.application.ui_pending() || !self.physical_admitted() {
            return Admission::Pending;
        }
        if let Binding::Shortcut {
            ancestor,
            declaration,
            navigation,
        } = &input.binding
        {
            let Some((current, current_declaration)) =
                self.current_shortcut(&input.event, *navigation)
            else {
                return Admission::Discard;
            };
            if !self
                .application
                .frame(&current.mount)
                .is_ok_and(|frame| frame.revision == current.revision)
            {
                return Admission::Pending;
            }
            return if current.mount == ancestor.mount
                && current.key == ancestor.key
                && Rc::ptr_eq(&current.owner, &ancestor.owner)
                && dialog::same_scope(&current.scope, &ancestor.scope)
                && current_declaration == declaration
                && self.target_admitted(current)
                && self.resources.admits(current)
            {
                Admission::Ready
            } else {
                Admission::Discard
            };
        }
        let Binding::Control(original) = &input.binding else {
            unreachable!()
        };
        let Some(current) = self.scene.iter().find(|target| {
            target.same(original) && dialog::same_scope(&target.scope, &original.scope)
        }) else {
            return Admission::Discard;
        };
        if !self
            .application
            .frame(&current.mount)
            .is_ok_and(|frame| frame.revision == current.revision)
        {
            return Admission::Pending;
        }
        if !same_action(&original.kind, &current.kind)
            || !self.target_admitted(current)
            || !self.resources.admits(current)
            || current.area.is_empty()
        {
            return Admission::Discard;
        }
        if let Some(point) = input.point {
            let hit = self.scene.iter().rev().find(|target| {
                !matches!(
                    target.kind,
                    paint::TargetKind::ContextMenu { button: false } | paint::TargetKind::Hint
                ) && target.area.contains(point)
            });
            if !hit.is_some_and(|target| target.same(current)) {
                return Admission::Discard;
            }
        } else {
            if !self
                .focus
                .as_ref()
                .is_some_and(|target| target.same(current))
            {
                return Admission::Discard;
            }
            if let Event::Key(event) = &input.event
                && let Some((key, modifiers)) = super::shortcut_chord(event)
                && current.ancestors.iter().rev().any(|ancestor| {
                    ancestor
                        .shortcuts
                        .iter()
                        .any(|shortcut| shortcut.matches(&key, modifiers, false))
                })
            {
                return Admission::Discard;
            }
        }
        Admission::Ready
    }
}
