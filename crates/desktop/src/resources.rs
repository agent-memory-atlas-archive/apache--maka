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

use maka_plugin_core::{Error as CoreError, runtime::Mount};
use maka_ui::{Application, Error};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use tokio::sync::mpsc::UnboundedSender;

/// Actual outstanding renderer resources, not a frame-count or logical-close
/// heuristic. A lease must be held by every retained native object/callback.
struct Scope {
    mount: Mount,
    leases: Cell<usize>,
    detached: Cell<bool>,
    wake: UnboundedSender<()>,
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
        if !self.detached.replace(true) && self.leases.get() == 0 {
            let _ = self.wake.send(());
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
            let _ = self.0.wake.send(());
        }
    }
}
#[derive(Clone)]
pub(super) struct Lease(Rc<Resource>);
impl Lease {
    pub fn mount(&self) -> &Mount {
        &self.0.0.mount
    }
}

pub(super) struct NativeResources {
    // The root also owns a platform window. Renderer-resource release alone
    // cannot confirm its cleanup; retain that fence pending a platform receipt.
    surface: Mount,
    scopes: BTreeMap<String, Rc<Scope>>,
    wake: UnboundedSender<()>,
}
impl NativeResources {
    pub fn new(surface: Mount, wake: UnboundedSender<()>) -> Self {
        Self {
            surface,
            scopes: BTreeMap::new(),
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
                })
            })
    }
    pub fn acquire(&mut self, mount: &Mount) -> Lease {
        self.scope(mount).acquire()
    }

    /// Only scopes absent from the committed display tree can detach. A faulted
    /// but still displayed component retains its renderer resources and frame.
    pub fn collect(&mut self, application: &mut Application) -> Result<usize, Error> {
        let visible: BTreeSet<_> = application
            .mounted_subtree(&self.surface)?
            .into_iter()
            .map(|mount| mount.id().to_string())
            .collect();
        let retired = application.retiring_mounts();
        let mut closed = 0;
        // The common adapter orders leaves before parents. Parents with still
        // outstanding children remain fenced even if their own leases released.
        for mount in retired {
            if mount == self.surface
                || visible.contains(&mount.id().to_string())
                || application.mount_root(&mount)? != self.surface
            {
                continue;
            }
            let scope = self.scope(&mount);
            scope.detach();
            if !scope.ready() {
                continue;
            }
            match application.close(&mount, Ok(())) {
                Ok(()) => {
                    self.scopes.remove(&mount.id().to_string());
                    closed += 1;
                }
                Err(Error::Core(CoreError::CleanupPending)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(closed)
    }
}
