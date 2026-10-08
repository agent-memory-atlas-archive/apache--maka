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

use super::*;
use std::{
    any::{Any, TypeId},
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

pub(super) struct Owner {
    pub identity: Rc<()>,
    pub recipe: Rc<dyn Widget>,
    pub state: Box<dyn Any>,
    resources: std::sync::Arc<maka_plugins::call::Resources>,
    kind: TypeId,
    revision: Option<u64>,
    pub live: bool,
    presented: bool,
    pending: Option<NativeInput>,
}
struct OwnerWitness {
    identity: std::rc::Weak<()>,
    value: maka_ui::actions::ContinuationWitness,
}
#[derive(Default)]
pub(super) struct Resources {
    pub owners: BTreeMap<Uuid, Owner>,
    failed: BTreeMap<Uuid, String>,
    budget: maka_ui::budget::NativeBudget,
}
impl Resources {
    pub fn cancel_pointer(&mut self) -> maka_ui::Result<()> {
        for (id, owner) in &mut self.owners {
            if owner.live
                && catch_unwind(AssertUnwindSafe(|| {
                    owner.recipe.cancel_pointer(owner.state.as_mut())
                }))
                .is_err()
            {
                owner.live = false;
                let error = "terminal pointer interruption cleanup unconfirmed".to_owned();
                self.failed.insert(*id, error.clone());
                return Err(Error::Invalid(error));
            }
        }
        Ok(())
    }
    pub fn prepare(
        &mut self,
        app: &Application<dyn Widget>,
        root: &Mount,
    ) -> maka_ui::Result<bool> {
        for mount in app.mounted_subtree(root)? {
            if app.is_retiring(&mount) {
                continue;
            }
            if let Some(reason) = self.failed.get(&mount.id()) {
                return Err(Error::Invalid(reason.clone()));
            }
            let frame = app.frame(&mount)?;
            let recipe = frame.view.view.clone();
            let kind = recipe.as_ref().type_id();
            if self
                .owners
                .get(&mount.id())
                .is_some_and(|owner| owner.kind != kind || !owner.live)
                && !self.close(&mount).map_err(Error::Invalid)?
            {
                return Ok(false);
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = self.owners.entry(mount.id())
            {
                let resources = std::sync::Arc::new(maka_plugins::call::Resources::default());
                let context = NativeContext {
                    resources: resources.clone(),
                    budget: self.budget.clone(),
                };
                match catch_unwind(AssertUnwindSafe(|| recipe.create(context))) {
                    Ok(state) => {
                        entry.insert(Owner {
                            identity: Rc::new(()),
                            recipe: recipe.clone(),
                            state,
                            resources,
                            kind,
                            revision: None,
                            live: true,
                            presented: false,
                            pending: None,
                        });
                    }
                    Err(_) => {
                        let error = "terminal native creation cleanup unconfirmed".to_owned();
                        self.failed.insert(mount.id(), error.clone());
                        return Err(Error::Invalid(error));
                    }
                }
            }
            let owner = self.owners.get_mut(&mount.id()).unwrap();
            owner.recipe = recipe;
            if catch_unwind(AssertUnwindSafe(|| {
                owner
                    .recipe
                    .prepare_focus(owner.state.as_mut(), app.mount_visible(&mount))
            }))
            .is_err()
            {
                owner.live = false;
                let error = "terminal native focus preparation cleanup unconfirmed".to_owned();
                self.failed.insert(mount.id(), error.clone());
                return Err(Error::Invalid(error));
            }
        }
        Ok(true)
    }
    /// Derived compatibility with the committed recipe, never a cleanup receipt.
    pub fn ready(&self, app: &Application<dyn Widget>, root: &Mount) -> maka_ui::Result<bool> {
        for mount in app.mounted_subtree(root)? {
            if app.is_retiring(&mount) {
                continue;
            }
            let kind = app.frame(&mount)?.view.view.as_ref().type_id();
            if !self
                .owners
                .get(&mount.id())
                .is_some_and(|owner| owner.live && owner.kind == kind)
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn commit(
        &mut self,
        app: &Application<dyn Widget>,
        root: &Mount,
    ) -> maka_ui::Result<Vec<(Mount, NativeInput)>> {
        let mut observations = vec![];
        for mount in app.mounted_subtree(root)? {
            if app.is_retiring(&mount) {
                continue;
            }
            let frame = app.frame(&mount)?;
            let owner = self
                .owners
                .get_mut(&mount.id())
                .expect("prepared terminal native owner");
            if owner.revision != Some(frame.revision) {
                let observed = match catch_unwind(AssertUnwindSafe(|| {
                    owner.recipe.commit(owner.state.as_mut())
                })) {
                    Ok(result) => result?,
                    Err(_) => {
                        let error = "terminal native commit cleanup unconfirmed".to_owned();
                        self.failed.insert(mount.id(), error.clone());
                        return Err(Error::Invalid(error));
                    }
                };
                owner.revision = Some(frame.revision);
                if let Some(observed) = observed {
                    owner.pending = Some(observed);
                }
            }
            match catch_unwind(AssertUnwindSafe(|| {
                owner.recipe.observe(owner.state.as_mut())
            })) {
                Ok(Some(observed)) => owner.pending = Some(observed),
                Ok(None) => {}
                Err(_) => {
                    owner.live = false;
                    let error = "terminal native observation cleanup unconfirmed".to_owned();
                    self.failed.insert(mount.id(), error.clone());
                    return Err(Error::Invalid(error));
                }
            }
            if let Some(observed) = &owner.pending {
                observations.push((mount, observed.clone()));
            }
        }
        Ok(observations)
    }
    pub fn deactivate(&mut self, app: &Application<dyn Widget>) {
        for mount in app.retiring_mounts() {
            if let Some(owner) = self.owners.get_mut(&mount.id())
                && owner.live
            {
                owner.live = false;
                if catch_unwind(AssertUnwindSafe(|| {
                    owner.recipe.deactivate(owner.state.as_mut())
                }))
                .is_err()
                {
                    self.failed.insert(
                        mount.id(),
                        "terminal native deactivation cleanup unconfirmed".into(),
                    );
                }
            }
        }
    }
    /// Keep original native custody while its sealed worker group is pending.
    pub fn close(&mut self, mount: &Mount) -> Result<bool, String> {
        if let Some(error) = self.failed.get(&mount.id()) {
            return Err(error.clone());
        }
        if let Some(owner) = self.owners.get_mut(&mount.id()) {
            if owner.live {
                owner.live = false;
                if catch_unwind(AssertUnwindSafe(|| {
                    owner.recipe.deactivate(owner.state.as_mut());
                }))
                .is_err()
                {
                    let error = "terminal native deactivation cleanup unconfirmed".to_owned();
                    self.failed.insert(mount.id(), error.clone());
                    return Err(error);
                }
            }
            match owner.resources.seal() {
                Ok(false) => return Ok(false),
                Ok(true) => {}
                Err(error) => {
                    let error = error.to_string();
                    self.failed.insert(mount.id(), error.clone());
                    return Err(error);
                }
            }
        }
        if let Some(owner) = self.owners.remove(&mount.id())
            && catch_unwind(AssertUnwindSafe(|| drop(owner))).is_err()
        {
            let error = "terminal native destruction cleanup unconfirmed".to_owned();
            self.failed.insert(mount.id(), error.clone());
            return Err(error);
        }
        Ok(true)
    }
    pub fn observe(&mut self, mount: &Mount, observed: NativeInput) {
        if let Some(owner) = self.owners.get_mut(&mount.id()) {
            owner.pending = Some(observed);
        }
    }
    fn published(&mut self, mount: &Mount, observed: &NativeInput) {
        if let Some(owner) = self.owners.get_mut(&mount.id())
            && owner.pending.as_ref() == Some(observed)
        {
            owner.pending = None;
        }
    }
    pub fn admits(&self, target: &paint::TargetIdentity) -> bool {
        self.owners.get(&target.mount.id()).is_some_and(|owner| {
            owner.live && owner.presented && Rc::ptr_eq(&owner.identity, &target.owner)
        })
    }
    pub fn input(
        &mut self,
        target: &paint::TargetIdentity,
        event: &Event,
    ) -> maka_ui::Result<Option<WidgetInput>> {
        let Some(owner) = self.owners.get_mut(&target.mount.id()).filter(|owner| {
            owner.live && owner.presented && Rc::ptr_eq(&owner.identity, &target.owner)
        }) else {
            return Ok(None);
        };
        match catch_unwind(AssertUnwindSafe(|| {
            owner.recipe.input(owner.state.as_mut(), &target.key, event)
        })) {
            Ok(result) => result.map(Some),
            Err(_) => {
                let error = "terminal native input cleanup unconfirmed".to_owned();
                owner.live = false;
                self.failed.insert(target.mount.id(), error.clone());
                Err(Error::Invalid(error))
            }
        }
    }
    pub(super) fn returns_focus(
        &mut self,
        target: &paint::TargetIdentity,
    ) -> maka_ui::Result<bool> {
        Ok(self
            .read_native(target, |recipe, _| recipe.returns_focus())?
            .unwrap_or(false))
    }
    pub(super) fn take_focus_exits(&mut self) -> maka_ui::Result<Vec<super::focus::Exit>> {
        let mut exits = Vec::new();
        for (id, owner) in &mut self.owners {
            if !owner.live {
                continue;
            }
            match catch_unwind(AssertUnwindSafe(|| {
                owner.recipe.take_focus_exit(owner.state.as_mut())
            })) {
                Ok(true) => exits.push(super::focus::Exit {
                    mount: *id,
                    owner: Rc::downgrade(&owner.identity),
                }),
                Ok(false) => {}
                Err(_) => {
                    owner.live = false;
                    let error = "terminal native focus cleanup unconfirmed".to_owned();
                    self.failed.insert(*id, error.clone());
                    return Err(Error::Invalid(error));
                }
            }
        }
        Ok(exits)
    }
    fn read_native<T>(
        &mut self,
        target: &paint::TargetIdentity,
        read: impl FnOnce(&dyn Widget, &dyn Any) -> T,
    ) -> maka_ui::Result<Option<T>> {
        let Some(owner) = self.owners.get_mut(&target.mount.id()).filter(|owner| {
            owner.live && owner.presented && Rc::ptr_eq(&owner.identity, &target.owner)
        }) else {
            return Ok(None);
        };
        match catch_unwind(AssertUnwindSafe(|| {
            read(owner.recipe.as_ref(), owner.state.as_ref())
        })) {
            Ok(value) => Ok(Some(value)),
            Err(_) => {
                owner.live = false;
                let error = "terminal native read cleanup unconfirmed".to_owned();
                self.failed.insert(target.mount.id(), error.clone());
                Err(Error::Invalid(error))
            }
        }
    }
    pub fn pointer_capture(&mut self, target: &paint::TargetIdentity) -> maka_ui::Result<bool> {
        Ok(self
            .read_native(target, |recipe, state| recipe.pointer_capture(state))?
            .unwrap_or(false))
    }
    pub fn copy(&mut self, target: &paint::TargetIdentity) -> maka_ui::Result<Option<String>> {
        self.read_native(target, |recipe, state| recipe.copy(state))?
            .unwrap_or(Ok(None))
    }
    pub fn present_focus(&mut self, target: &paint::TargetIdentity) -> bool {
        let Some(owner) = self.owners.get_mut(&target.mount.id()).filter(|owner| {
            owner.live && owner.presented && Rc::ptr_eq(&owner.identity, &target.owner)
        }) else {
            return false;
        };
        match catch_unwind(AssertUnwindSafe(|| {
            owner.recipe.present_focus(owner.state.as_mut())
        })) {
            Ok(offered) => offered,
            Err(_) => {
                owner.live = false;
                self.failed.insert(
                    target.mount.id(),
                    "terminal native focus presentation cleanup unconfirmed".into(),
                );
                false
            }
        }
    }
    pub fn present(&mut self, mount: &Mount, identity: &Rc<()>, geometry: Box<dyn Any>) {
        let Some(owner) = self
            .owners
            .get_mut(&mount.id())
            .filter(|owner| owner.live && owner.presented && Rc::ptr_eq(&owner.identity, identity))
        else {
            return;
        };
        match catch_unwind(AssertUnwindSafe(|| {
            owner.recipe.present(owner.state.as_mut(), geometry)
        })) {
            Ok(Some(observed)) => owner.pending = Some(observed),
            Ok(None) => {}
            Err(_) => {
                owner.live = false;
                self.failed.insert(
                    mount.id(),
                    "terminal native presentation cleanup unconfirmed".into(),
                );
            }
        }
    }
    pub fn presentation(&mut self, placed: &std::collections::BTreeSet<Uuid>) {
        for (id, owner) in &mut self.owners {
            let visible = placed.contains(id);
            if !owner.live || owner.presented == visible {
                continue;
            }
            owner.presented = visible;
            if catch_unwind(AssertUnwindSafe(|| {
                owner.recipe.presentation(owner.state.as_mut(), visible)
            }))
            .is_err()
            {
                owner.live = false;
                self.failed.insert(
                    *id,
                    "terminal native presentation cleanup unconfirmed".into(),
                );
            }
        }
    }
    pub fn capture(
        &mut self,
        target: &maka_ui::actions::CaptureTarget,
    ) -> maka_ui::Result<maka_ui::actions::NativeCapture> {
        let owner = self
            .owners
            .get(&target.mount().id())
            .filter(|owner| owner.live)
            .ok_or_else(|| Error::Invalid("terminal native owner unavailable".into()))?;
        match catch_unwind(AssertUnwindSafe(|| {
            use maka_ui::actions::{ContinuationWitness, NativeCapture};
            match target.output() {
                maka_plugin_core::contract::CaptureOutput::Resource { .. } => owner
                    .recipe
                    .capture(owner.state.as_ref(), target)
                    .map(NativeCapture::Resource),
                maka_plugin_core::contract::CaptureOutput::Continuation => {
                    let value = owner
                        .recipe
                        .capture_continuation(owner.state.as_ref(), target)?;
                    let bytes = value
                        .bytes()
                        .checked_add(std::mem::size_of::<OwnerWitness>())
                        .ok_or_else(|| Error::Invalid("continuation byte overflow".into()))?;
                    Ok(NativeCapture::Continuation(ContinuationWitness::new(
                        OwnerWitness {
                            identity: Rc::downgrade(&owner.identity),
                            value,
                        },
                        bytes,
                    )))
                }
            }
        })) {
            Ok(result) => result,
            Err(_) => {
                let error = "terminal native capture cleanup unconfirmed".to_owned();
                self.failed.insert(target.mount().id(), error.clone());
                Err(Error::Invalid(error))
            }
        }
    }
    pub fn validate_continuation(
        &mut self,
        target: &maka_ui::actions::CaptureTarget,
        recipe: &dyn Widget,
        witness: &maka_ui::actions::ContinuationWitness,
    ) -> maka_ui::Result<bool> {
        let Some(witness) = witness.downcast_ref::<OwnerWitness>() else {
            return Ok(false);
        };
        let Some(identity) = witness.identity.upgrade() else {
            return Ok(false);
        };
        let Some(owner) = self.owners.get_mut(&target.mount().id()).filter(|owner| {
            owner.live && Rc::ptr_eq(&owner.identity, &identity) && owner.kind == recipe.type_id()
        }) else {
            return Ok(false);
        };
        match catch_unwind(AssertUnwindSafe(|| {
            recipe.validate_continuation(owner.state.as_ref(), target, &witness.value)
        })) {
            Ok(result) => result,
            Err(_) => {
                owner.live = false;
                let error = "terminal native continuation cleanup unconfirmed".to_owned();
                self.failed.insert(target.mount().id(), error.clone());
                Err(Error::Invalid(error))
            }
        }
    }
}
impl Presenter {
    pub(super) fn sync_native(&mut self) -> maka_ui::Result<()> {
        self.resources.deactivate(&self.application);
        if !self.surface.is_live() || self.application.is_retiring(&self.root) {
            return Ok(());
        }
        if !self.resources.prepare(&self.application, &self.root)? {
            self.clear_hover()?;
            self.scene.clear();
            self.focus = None;
            self.drag = None;
            return Ok(());
        }
        let resources = &mut self.resources;
        self.application
            .capture_actions(&self.root, |target| resources.capture(target))?;
        let observed = self.resources.commit(&self.application, &self.root)?;
        for (mount, input) in observed {
            if !self.application.is_retiring(&mount) {
                let revision = self.application.frame(&mount)?.revision;
                match self
                    .application
                    .dispatch(&mount, revision, &input.key, input.value.clone())
                {
                    Ok(_) => self.resources.published(&mount, &input),
                    Err(error) => self.report(&error),
                }
            }
        }
        if !self.resources.prepare(&self.application, &self.root)? {
            self.clear_hover()?;
            self.scene.clear();
            self.focus = None;
            self.drag = None;
            return Ok(());
        }
        self.sync_modal_focus()?;
        self.apply_focus_exits()?;
        if let Some(target) = &self.drag
            && !self.resources.pointer_capture(target)?
        {
            self.drag = None;
        }
        Ok(())
    }
}

impl Resources {
    pub(super) fn hover(
        &mut self,
        target: &paint::Target,
        position: Option<ratatui::layout::Position>,
    ) -> maka_ui::Result<()> {
        let Some(owner) = self
            .owners
            .get_mut(&target.mount.id())
            .filter(|owner| owner.live && std::rc::Rc::ptr_eq(&owner.identity, &target.owner))
        else {
            return Ok(());
        };
        if catch_unwind(AssertUnwindSafe(|| {
            owner
                .recipe
                .hover(owner.state.as_mut(), &target.key, position);
        }))
        .is_err()
        {
            owner.live = false;
            self.failed.insert(
                target.mount.id(),
                "terminal native hover cleanup unconfirmed".into(),
            );
            return Err(maka_ui::Error::Invalid(
                "terminal native hover cleanup unconfirmed".into(),
            ));
        }
        Ok(())
    }
}
