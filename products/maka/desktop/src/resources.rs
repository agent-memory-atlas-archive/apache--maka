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

use super::widgets::LiveInput;
use maka_plugin_core::{Error as CoreError, runtime::Mount};
use maka_ui_runtime::{Application, Error};
use std::any::{Any, TypeId};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use tokio::sync::mpsc::Sender;

/// Actual outstanding renderer resources, not a frame-count or logical-close
/// heuristic. A lease must be held by every retained native object/callback.
struct Scope {
    mount: Mount,
    leases: Cell<usize>,
    detached: Cell<bool>,
    wake: Sender<()>,
    workers: std::sync::Arc<maka_capabilities::call::Resources>,
}
impl Scope {
    fn acquire(self: &Rc<Self>) -> Lease {
        assert!(
            !self.detached.get(),
            "detached native scope cannot acquire resources"
        );
        self.leases.set(
            self.leases
                .get()
                .checked_add(1)
                .expect("native lease count"),
        );
        Lease(Rc::new(Resource(self.clone())))
    }
    fn detach(&self) {
        let _ = self.workers.seal();
        if !self.detached.replace(true) && self.leases.get() == 0 {
            let _ = self.wake.try_send(());
        }
    }
    fn ready(&self) -> bool {
        self.detached.get() && self.leases.get() == 0
    }
}
struct Resource(Rc<Scope>);
impl Drop for Resource {
    fn drop(&mut self) {
        self.0.leases.set(
            self.0
                .leases
                .get()
                .checked_sub(1)
                .expect("native lease count"),
        );
        if self.0.ready() {
            // The owner thread processes the receipt later, after the native
            // destructor returns. No Application/GPUI mutation from Drop.
            let _ = self.0.wake.try_send(());
        }
    }
}
#[derive(Clone)]
pub(super) struct Lease(Rc<Resource>);
impl Lease {
    pub fn is_attached(&self) -> bool {
        !self.0.0.detached.get()
    }
    pub fn resources(&self) -> std::sync::Arc<maka_capabilities::call::Resources> {
        self.0.0.workers.clone()
    }
    pub fn mount(&self) -> &Mount {
        &self.0.0.mount
    }
}

pub(super) struct NativeInstance {
    pub recipe: Rc<dyn super::widgets::Widget>,
    pub kind: TypeId,
    pub identity: Rc<()>,
    pub revision: Option<u64>,
    pub visible: bool,
    pub state: Box<dyn Any>,
    pub input: LiveInput,
}
struct OwnerWitness {
    identity: std::rc::Weak<()>,
    value: maka_ui_runtime::actions::ContinuationWitness,
}
impl NativeInstance {
    pub fn deactivate(&mut self, window: &mut gpui_kit::Window, cx: &mut gpui_kit::App) {
        self.input.observations.close();
        if self.input.current.borrow_mut().take().is_some() {
            self.recipe.deactivate(self.state.as_mut(), window, cx);
        }
    }
}
impl Drop for NativeInstance {
    fn drop(&mut self) {
        self.input.current.borrow_mut().take();
    }
}

pub(super) struct NativeResources {
    pub budget: super::widgets::NativeBudget,
    pub owners: BTreeMap<String, NativeInstance>,
    // The root also owns a platform window. Renderer-resource release alone
    // cannot confirm its cleanup; retain that fence pending a platform receipt.
    surface: Mount,
    binding: Option<(
        gpui_kit::AnyWindowHandle,
        gpui_kit::WeakEntity<super::Surface>,
    )>,
    removed: bool,
    platform_confirmed: bool,
    pub surface_closed: bool,
    scopes: BTreeMap<String, Rc<Scope>>,
    preparation_failed: BTreeMap<String, String>,
    wake: Sender<()>,
}
impl NativeResources {
    #[cfg(test)]
    pub fn cleanup_diagnostics(&self) -> String {
        format!(
            "removed={} platform={} scopes={:?}",
            self.removed,
            self.platform_confirmed,
            self.scopes
                .iter()
                .map(|(id, scope)| (id.clone(), scope.leases.get(), scope.ready()))
                .collect::<Vec<_>>()
        )
    }
    pub fn capture(
        &mut self,
        target: &maka_ui_runtime::actions::CaptureTarget,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui_runtime::Result<maka_ui_runtime::actions::NativeCapture> {
        use maka_ui_runtime::actions::{ContinuationWitness, NativeCapture};
        let owner = self
            .owners
            .get_mut(&target.mount().id().to_string())
            .filter(|owner| owner.input.current.borrow().is_some() && !self.removed)
            .ok_or_else(|| Error::Invalid("native capture owner unavailable".into()))?;
        match target.output() {
            maka_plugin_core::contract::CaptureOutput::Resource { .. } => owner
                .recipe
                .capture(owner.state.as_mut(), target, window, cx)
                .map(NativeCapture::Resource),
            maka_plugin_core::contract::CaptureOutput::Continuation => {
                let value =
                    owner
                        .recipe
                        .capture_continuation(owner.state.as_ref(), target, window, cx)?;
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
    }
    pub fn validate_continuation(
        &self,
        target: &maka_ui_runtime::actions::CaptureTarget,
        recipe: &dyn super::widgets::Widget,
        witness: &maka_ui_runtime::actions::ContinuationWitness,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui_runtime::Result<bool> {
        let Some(witness) = witness.downcast_ref::<OwnerWitness>() else {
            return Ok(false);
        };
        let Some(identity) = witness.identity.upgrade() else {
            return Ok(false);
        };
        let Some(owner) = self
            .owners
            .get(&target.mount().id().to_string())
            .filter(|owner| {
                !self.removed
                    && owner.input.current.borrow().is_some()
                    && Rc::ptr_eq(&owner.identity, &identity)
                    && owner.kind == recipe.type_id()
            })
        else {
            return Ok(false);
        };
        recipe.validate_continuation(owner.state.as_ref(), target, &witness.value, window, cx)
    }
    pub fn bind_surface(
        &mut self,
        window: gpui_kit::AnyWindowHandle,
        surface: gpui_kit::WeakEntity<super::Surface>,
    ) -> Result<(), Error> {
        if self.removed {
            return Err(Error::Invalid("original native surface removed".into()));
        }
        match &self.binding {
            Some((original, entity))
                if original.window_id() != window.window_id()
                    || entity.entity_id() != surface.entity_id() =>
            {
                Err(Error::Invalid(
                    "native surface binding owner mismatch".into(),
                ))
            }
            Some(_) => Ok(()),
            None => {
                self.binding = Some((window, surface));
                Ok(())
            }
        }
    }
    pub fn surface_binding(
        &self,
    ) -> Option<(
        gpui_kit::AnyWindowHandle,
        gpui_kit::WeakEntity<super::Surface>,
    )> {
        self.binding.clone()
    }
    pub fn new(surface: Mount, wake: Sender<()>) -> Self {
        Self {
            budget: super::widgets::NativeBudget::default(),
            surface,
            binding: None,
            removed: false,
            platform_confirmed: false,
            surface_closed: false,
            owners: BTreeMap::new(),
            scopes: BTreeMap::new(),
            preparation_failed: BTreeMap::new(),
            wake,
        }
    }
    fn scope(&mut self, mount: &Mount) -> &Rc<Scope> {
        self.scopes
            .entry(mount.id().to_string())
            .or_insert_with(|| {
                Rc::new(Scope {
                    mount: mount.clone(),
                    leases: Cell::new(0),
                    detached: Cell::new(false),
                    wake: self.wake.clone(),
                    workers: {
                        let wake = self.wake.clone();
                        std::sync::Arc::new(maka_capabilities::call::Resources::with_wake(
                            move || {
                                let _ = wake.try_send(());
                            },
                        ))
                    },
                })
            })
    }
    pub fn preparation_failed(&mut self, mount: &Mount, reason: String) {
        self.preparation_failed
            .insert(mount.id().to_string(), reason);
    }
    pub fn prepare_scope(&mut self, mount: &Mount) {
        self.scope(mount);
    }
    pub fn acquire(&self, mount: &Mount) -> Lease {
        self.scopes
            .get(&mount.id().to_string())
            .expect("scope prepared before render")
            .acquire()
    }

    /// Logical tree removal closes native mechanical input immediately, before
    /// old scene leases and resources finish their independent destruction.
    pub fn deactivate_unmounted<V: ?Sized + 'static>(
        &mut self,
        application: &Application<V>,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> Result<(), Error> {
        let visible: BTreeSet<_> = application
            .mounted_subtree(&self.surface)
            .or_else(|error| {
                if application.is_retiring(&self.surface) {
                    Ok(vec![])
                } else {
                    Err(error)
                }
            })?
            .into_iter()
            .map(|mount| mount.id().to_string())
            .collect();
        let retiring: BTreeSet<_> = application
            .retiring_mounts()
            .into_iter()
            .map(|mount| mount.id().to_string())
            .collect();
        for (id, owner) in &mut self.owners {
            if !visible.contains(id) || retiring.contains(id) {
                owner.deactivate(window, cx);
            }
        }
        Ok(())
    }
    pub fn surface(&self) -> &Mount {
        &self.surface
    }
    pub fn is_removed(&self) -> bool {
        self.removed
    }

    /// Logical removal revokes all inputs and seals resource scopes. It is not
    /// an OS cleanup acknowledgement; the independent window owner waits for it.
    pub fn window_removed<V: ?Sized + 'static>(
        &mut self,
        application: &mut Application<V>,
    ) -> Result<(), Error> {
        let result = application.retire(&self.surface);
        self.binding = None;
        self.removed = true;
        self.owners.clear();
        for scope in self.scopes.values() {
            scope.detach();
        }
        let _ = self.wake.try_send(());
        result
    }
    pub fn confirm_platform_cleanup(&mut self) {
        self.platform_confirmed = true;
    }
    pub fn fail_platform_cleanup<V: ?Sized + 'static>(
        &mut self,
        application: &mut Application<V>,
        error: String,
    ) {
        // Persist the failed fence, without removing its component/frame.
        let _ = application.close(&self.surface, Err(error));
    }

    /// Only scopes absent from the committed display tree can detach. A faulted
    /// but still displayed component retains its renderer resources and frame.
    pub fn collect<V: ?Sized + 'static>(
        &mut self,
        application: &mut Application<V>,
    ) -> Result<usize, Error> {
        if self.surface_closed {
            return Ok(0);
        }
        let visible: BTreeSet<_> = if self.removed {
            BTreeSet::new()
        } else {
            application
                .mounted_subtree(&self.surface)?
                .into_iter()
                .map(|mount| mount.id().to_string())
                .collect()
        };
        let retired = application.retiring_mounts();
        let mut closed = 0;
        // The common adapter orders leaves before parents. Parents with still
        // outstanding children remain fenced even if their own leases released.
        for mount in retired {
            if let Some(reason) = self.preparation_failed.get(&mount.id().to_string()) {
                application.close(&mount, Err(reason.clone()))?;
                continue;
            }
            if (mount == self.surface && !(self.removed && self.platform_confirmed))
                || visible.contains(&mount.id().to_string())
                || application.mount_root(&mount)? != self.surface
            {
                continue;
            }
            // Release resource owners independently of retained frames. Their
            // entities/input handlers still hold real leases until destruction.
            self.owners.remove(&mount.id().to_string());
            let scope = self.scope(&mount);
            scope.detach();
            if !scope.ready() {
                continue;
            }
            let cleanup = match scope.workers.seal() {
                Ok(false) => continue,
                Ok(true) => Ok(()),
                Err(error) => Err(error.to_string()),
            };
            match application.close(&mount, cleanup) {
                Ok(()) => {
                    self.scopes.remove(&mount.id().to_string());
                    if mount == self.surface {
                        self.surface_closed = true;
                    }
                    closed += 1;
                }
                Err(Error::Core(CoreError::CleanupPending)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(closed)
    }
}
