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
}
impl Resources {
    pub fn prepare(&mut self, app: &Application<dyn Widget>, root: &Mount) -> maka_ui::Result<()> {
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
                .is_some_and(|owner| owner.kind != kind)
            {
                self.close(&mount).map_err(Error::Invalid)?;
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = self.owners.entry(mount.id())
            {
                match catch_unwind(AssertUnwindSafe(|| recipe.create())) {
                    Ok(state) => {
                        entry.insert(Owner {
                            identity: Rc::new(()),
                            recipe: recipe.clone(),
                            state,
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
        }
        Ok(())
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
    pub fn close(&mut self, mount: &Mount) -> Result<(), String> {
        if let Some(error) = self.failed.get(&mount.id()) {
            return Err(error.clone());
        }
        if let Some(mut owner) = self.owners.remove(&mount.id())
            && catch_unwind(AssertUnwindSafe(|| {
                if owner.live {
                    owner.live = false;
                    owner.recipe.deactivate(owner.state.as_mut());
                }
                drop(owner);
            }))
            .is_err()
        {
            let error = "terminal native destruction cleanup unconfirmed".to_owned();
            self.failed.insert(mount.id(), error.clone());
            return Err(error);
        }
        Ok(())
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
    pub fn input(
        &mut self,
        target: &paint::Target,
        event: &Event,
    ) -> maka_ui::Result<Option<WidgetInput>> {
        let Some(owner) = self.owners.get_mut(&target.mount.id()).filter(|owner| {
            owner.live && owner.presented && Rc::ptr_eq(&owner.identity, &target.owner)
        }) else {
            return Ok(None);
        };
        match catch_unwind(AssertUnwindSafe(|| {
            owner.recipe.input(owner.state.as_mut(), event)
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
        if self.closing || self.application.is_retiring(&self.root) {
            return Ok(());
        }
        self.resources.prepare(&self.application, &self.root)?;
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
        self.resources.prepare(&self.application, &self.root)?;
        Ok(())
    }
}
