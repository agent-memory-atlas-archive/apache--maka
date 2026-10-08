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
    Presenter,
    dialog::{self, WeakTarget},
};

pub(super) struct Exit {
    pub mount: uuid::Uuid,
    pub owner: std::rc::Weak<()>,
}
#[derive(Clone)]
pub(super) enum Return {
    Active {
        source: WeakTarget,
        target: Option<WeakTarget>,
        scope: Option<dialog::Scope>,
    },
    /// A completed return suppresses ordinal fallback and the held key's repeats.
    Exited,
}
impl Presenter {
    pub(super) fn track_focus_return(&mut self) -> maka_ui::Result<()> {
        let Some(focus) = self.focus.as_ref() else {
            return Ok(());
        };
        if !self.resources.returns_focus(focus)? {
            if matches!(self.native_return, Some(Return::Active { .. })) {
                self.native_return = None;
            }
            return Ok(());
        }
        if matches!(&self.native_return, Some(Return::Active { source, .. }) if source.matches(focus))
        {
            return Ok(());
        }
        let target = self
            .scene
            .iter()
            .filter(|target| {
                target.tab_index >= 0
                    && target.mount != focus.mount
                    && dialog::same_scope(&target.scope, &focus.scope)
            })
            .min_by_key(|target| (target.tab_index == 0, target.tab_index));
        self.native_return = Some(Return::Active {
            source: WeakTarget::new(focus),
            target: target.map(WeakTarget::new),
            scope: focus.scope.clone(),
        });
        Ok(())
    }
    pub(super) fn exit_native_focus(&mut self) -> maka_ui::Result<()> {
        self.clear_hover()?;
        let destination = match self.native_return.take() {
            Some(Return::Active {
                source,
                target,
                scope,
            }) if self
                .focus
                .as_ref()
                .is_some_and(|focus| source.matches(focus)) =>
            {
                target
                    .and_then(|target| target.resolve(&self.scene))
                    .filter(|target| {
                        dialog::same_scope(&scope, &target.scope)
                            && self.target_admitted(target)
                            && self.resources.owns(target)
                            && self.application.mount_visible(&target.mount)
                    })
            }
            _ => None,
        };
        self.focus = destination;
        self.native_return = Some(Return::Exited);
        Ok(())
    }
    pub(super) fn apply_focus_exits(&mut self) -> maka_ui::Result<()> {
        // Drain every owner, so a loss while unfocused cannot exit a later entry.
        let exits = self.resources.take_focus_exits()?;
        self.apply_modal_focus_exits(&exits);
        if self
            .focus
            .as_ref()
            .is_some_and(|focus| WeakTarget::new(focus).exited(&exits))
        {
            self.exit_native_focus()?;
        }
        Ok(())
    }
}

impl Presenter {
    pub(super) fn freeze_focus_child(
        &self,
        parent: &maka_plugin_core::runtime::Mount,
        name: &str,
    ) -> maka_ui::Result<super::paint::FocusChild> {
        let child = self.application.child_mount(parent, name)?.clone();
        let owner = self.resources.owners.get(&child.id()).ok_or_else(|| {
            maka_ui::Error::Invalid("focus child native owner unavailable".into())
        })?;
        Ok(super::paint::FocusChild {
            child,
            owner: std::rc::Rc::downgrade(&owner.identity),
        })
    }
    pub(super) fn freeze_focus_shortcuts(
        &self,
        parent: &maka_plugin_core::runtime::Mount,
        bindings: &[maka_ui::shortcut::FocusShortcut],
    ) -> maka_ui::Result<Vec<super::paint::FocusBinding>> {
        bindings
            .iter()
            .map(|declaration| {
                Ok(super::paint::FocusBinding {
                    declaration: declaration.clone(),
                    target: self.freeze_focus_child(parent, &declaration.child)?,
                })
            })
            .collect()
    }
    pub(super) fn focus_child(
        &mut self,
        binding: &super::paint::FocusChild,
    ) -> maka_ui::Result<()> {
        if self.application.is_retiring(&binding.child)
            || !self.application.mount_visible(&binding.child)
            || !self
                .resources
                .owners
                .get(&binding.child.id())
                .is_some_and(|owner| {
                    owner.live
                        && binding
                            .owner
                            .ptr_eq(&std::rc::Rc::downgrade(&owner.identity))
                })
        {
            return Ok(());
        }
        let mounts = self.application.mounted_subtree(&binding.child)?;
        let target = self
            .focus_stops(|target| mounts.contains(&target.mount))
            .into_iter()
            .filter(|target| {
                target.tab_index >= 0
                    && mounts.contains(&target.mount)
                    && self.target_admitted(target)
                    && self.resources.owns(target)
                    && !self.application.is_retiring(&target.mount)
                    && self.application.mount_visible(&target.mount)
                    && self
                        .application
                        .frame(&target.mount)
                        .is_ok_and(|frame| frame.revision == target.revision)
            })
            .min_by_key(|target| (target.tab_index == 0, target.tab_index))
            .cloned();
        if let Some(target) = target {
            self.focus_item(target)?;
        }
        Ok(())
    }
}

// Original Surface grouping: one Tab stop, declaration-order arrow navigation,
// and focus-following activation. There is no second native selection cursor.
impl super::paint::Target {
    fn group(&self) -> Option<&super::paint::Ancestor> {
        self.ancestors
            .iter()
            .rev()
            .find(|ancestor| ancestor.focus_group.is_some())
            .map(|a| a.as_ref())
    }
    fn same_group(&self, other: &Self) -> bool {
        self.group().zip(other.group()).is_some_and(|(a, b)| {
            a.mount == b.mount
                && a.key == b.key
                && std::rc::Rc::ptr_eq(&a.owner, &b.owner)
                && dialog::same_scope(&a.scope, &b.scope)
        })
    }
}
impl Presenter {
    pub(super) fn tab_stops(&self) -> Vec<&super::paint::Target> {
        self.focus_stops(|_| true)
    }
    fn focus_stops(
        &self,
        include: impl Fn(&super::paint::Target) -> bool,
    ) -> Vec<&super::paint::Target> {
        use super::paint::TargetKind;
        let priority = |target: &super::paint::Target| {
            (
                self.focus.as_ref().is_some_and(|focus| focus.same(target)),
                matches!(target.kind, TargetKind::Item { current: true, .. }),
                !target.area.is_empty(),
            )
        };
        let mut stops: Vec<&super::paint::Target> = Vec::new();
        for target in self
            .scene
            .iter()
            .filter(|target| target.tab_index >= 0 && include(target))
        {
            if let Some(stop) = stops.iter_mut().find(|stop| stop.same_group(target)) {
                if priority(target) > priority(stop) {
                    *stop = target;
                }
            } else {
                stops.push(target);
            }
        }
        stops.sort_by_key(|target| (target.tab_index == 0, target.tab_index));
        stops
    }
    pub(super) fn focus_item(&mut self, target: super::paint::Target) -> maka_ui::Result<()> {
        use super::paint::TargetKind;
        if self.application.is_retiring(&target.mount)
            || !self.application.mount_visible(&target.mount)
            || !self.target_admitted(&target)
            || (matches!(target.kind, TargetKind::Item { .. })
                && self.application.frame(&target.mount)?.revision != target.revision)
            || !(if matches!(target.kind, TargetKind::Item { .. }) {
                self.resources.owns(&target)
            } else {
                self.resources.enter_focus(&target)?
            })
        {
            return Ok(());
        }
        let changed = !self.focus.as_ref().is_some_and(|old| old.same(&target));
        let mut bounds = target.area.bounds;
        for ancestor in target.ancestors.iter().rev() {
            if self.target_admitted(ancestor)
                && self.resources.owns(ancestor)
                && self.application.frame(&ancestor.mount)?.revision == ancestor.revision
                && let Some(viewport) = &ancestor.scroll
                && let Some(state) = self
                    .resources
                    .owners
                    .get_mut(&ancestor.mount.id())
                    .and_then(|owner| owner.state.downcast_mut::<super::scroll::SceneState>())
            {
                state.reveal(&ancestor.key, viewport, &mut bounds);
            }
        }
        self.focus = Some(target.clone());
        if let TargetKind::Item {
            current,
            follow_focus: true,
            ..
        } = target.kind
            && (changed || !current)
        {
            self.application.dispatch(
                &target.mount,
                target.revision,
                &target.key,
                serde_json::json!({"kind":"activate"}),
            )?;
            self.sync_native()?;
        }
        self.track_focus_return()
    }
    pub(super) fn navigate_items(
        &mut self,
        event: &crossterm::event::Event,
    ) -> maka_ui::Result<bool> {
        use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
        let Event::Key(key) = event else {
            return Ok(false);
        };
        if key.kind == KeyEventKind::Release || key.modifiers != KeyModifiers::NONE {
            return Ok(false);
        }
        let Some(focus) = self
            .focus
            .as_ref()
            .filter(|target| matches!(target.kind, super::paint::TargetKind::Item { .. }))
        else {
            return Ok(false);
        };
        use maka_ui::scene::terminal::Direction;
        let along = match key.code {
            KeyCode::Left | KeyCode::Right => {
                focus.group().and_then(|group| group.focus_group) == Some(Direction::Row)
            }
            KeyCode::Up | KeyCode::Down => {
                focus.group().and_then(|group| group.focus_group) == Some(Direction::Column)
            }
            KeyCode::Home | KeyCode::End => true,
            _ => false,
        };
        if !along {
            return Ok(false);
        }
        let peers: Vec<_> = self
            .scene
            .iter()
            .filter(|target| target.tab_index >= 0 && target.same_group(focus))
            .collect();
        let Some(at) = peers.iter().position(|target| target.same(focus)) else {
            return Ok(false);
        };
        let next = match key.code {
            KeyCode::Up | KeyCode::Left => at.saturating_sub(1),
            KeyCode::Down | KeyCode::Right => (at + 1).min(peers.len() - 1),
            KeyCode::Home => 0,
            _ => peers.len() - 1,
        };
        self.focus_item(peers[next].clone())?;
        Ok(true)
    }
}
