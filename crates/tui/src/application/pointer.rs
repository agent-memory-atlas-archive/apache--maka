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

use super::{Presenter, paint};
use ratatui::layout::Position;

impl Presenter {
    pub(super) fn clear_hover(&mut self) -> maka_ui::Result<()> {
        if let Some(target) = self.hover.take() {
            self.resources.hover(&target, None)?;
        }
        Ok(())
    }
    pub(super) fn move_pointer(&mut self, position: Position) -> maka_ui::Result<()> {
        let target = self
            .scene
            .iter()
            .rev()
            .find(|target| target.area.contains(position))
            .filter(|target| {
                self.drag.is_none()
                    && !self.application.is_retiring(&target.mount)
                    && self.target_admitted(target)
                    && self.resources.admits(target)
            })
            .cloned();
        if !self
            .hover
            .as_ref()
            .zip(target.as_ref())
            .is_some_and(|(old, new)| old.same(new))
        {
            self.clear_hover()?;
        }
        if let Some(target) = target {
            self.resources.hover(&target, Some(position))?;
            self.hover = Some(target);
        }
        Ok(())
    }
    /// A successful flush may move, hide or replace the hovered target. Never
    /// transfer feedback to a new target at the same physical coordinates.
    pub(super) fn reconcile_hover(&mut self, targets: &[paint::Target]) -> maka_ui::Result<()> {
        if let Some(old) = self.hover.as_ref() {
            if let Some(new) = targets.iter().find(|new| {
                new.same(old)
                    && new.area == old.area
                    && super::dialog::same_scope(&new.scope, &old.scope)
            }) {
                self.hover = Some(new.clone());
            } else {
                self.clear_hover()?;
            }
        }
        Ok(())
    }
}
