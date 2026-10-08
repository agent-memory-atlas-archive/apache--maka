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
                            && self.resources.admits(target)
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
        if self
            .focus
            .as_ref()
            .is_some_and(|focus| exits.contains(&focus.mount.id()))
        {
            self.exit_native_focus()?;
        }
        Ok(())
    }
}
