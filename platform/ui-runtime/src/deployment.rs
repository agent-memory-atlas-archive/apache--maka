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

//! Owner-thread reconciliation of exact committed UI sources.
use super::*;
use maka_plugin_core::{
    deployment::DeploymentRef,
    plan::{Instance, RemoteCapability, RemoteExports},
    runtime::Status,
};
use std::collections::BTreeSet;

pub struct DeploymentStatus {
    pub desired: Option<DeploymentRef>,
    /// Last source whose activation plan fully converged, not platform readiness.
    pub effective: Option<DeploymentRef>,
    pub converging: bool,
    pub error: Option<String>,
    pub closing: bool,
    pub instances: Vec<Status>,
}

/// Exact bytes from one owned storage snapshot. This is input evidence, not a grant.
pub struct CommittedUpdate {
    loaded: maka_event_log::applications::LoadedDeployment,
}
impl CommittedUpdate {
    pub fn source(&self) -> &DeploymentRef {
        &self.loaded.source.reference
    }
}

/// Immutable candidate preflighted by its original foreground owner. This is
/// not an editable plan, connection approval or convergence acknowledgement.
pub struct PreparedUpdate {
    owner: Uuid,
    source: maka_event_log::applications::SourceSnapshot,
    candidate: Candidate,
}
impl PreparedUpdate {
    pub fn source(&self) -> &DeploymentRef {
        self.source.reference()
    }
    /// Immutable projection already checked by this foreground owner.
    pub fn plan(&self) -> &Plan {
        &self.candidate.plan
    }
    pub fn target(&self) -> PresentationTarget {
        self.candidate.target
    }
}

pub(super) struct Candidate {
    pub plan: Plan,
    pub sources: BTreeMap<PackageRef, Package>,
    pub instances: Vec<Instance>,
    pub target: PresentationTarget,
}

pub(super) fn prepare<V: ?Sized + 'static>(
    input: &Input,
    bytes: Vec<Package>,
    native: &NativeRegistry<V>,
    declarations: &RemoteExports,
) -> Result<Candidate> {
    if input.environment.location != ExecutionLocation::Ui {
        return Err(Error::Invalid("UI deployment required".into()));
    }
    let target = input
        .environment
        .presentation
        .ok_or_else(|| Error::Invalid("presentation target required".into()))?;
    let mut packages = Packages::default();
    let mut sources: BTreeMap<PackageRef, Package> = BTreeMap::new();
    for package in bytes {
        let declaration = package
            .manifest()
            .application
            .as_ref()
            .ok_or_else(|| Error::Invalid("application declaration required".into()))?;
        packages.insert(package.application_ref(), declaration.clone())?;
        sources.insert(package.application_ref(), package);
    }
    let plan = Plan::prepare(input, &packages, declarations)?;
    let pending: Vec<_> = plan
        .instances()
        .values()
        .filter(|instance| !instance.disabled)
        .cloned()
        .collect();
    // Reject known unavailable implementations before any guest or factory
    // activation. Runtime factory/script failures remain execution failures.
    for instance in &pending {
        if let Some(entry) = &instance.entry {
            match &entry.implementation {
                Implementation::Native { implementation } => {
                    native.validate_implementation(implementation)?
                }
                Implementation::JavaScript { module, .. } => {
                    if let Some(language) = &entry.scene {
                        let compiler = native.scene_compiler.as_ref().ok_or_else(|| {
                            Error::Invalid("presenter scene compiler required".into())
                        })?;
                        if compiler.language != *language {
                            return Err(Error::Invalid(format!(
                                "unsupported scene language: {} v{}",
                                language.name, language.version
                            )));
                        }
                    }
                    std::str::from_utf8(
                        sources[&instance.package]
                            .file(module)
                            .ok_or_else(|| Error::Invalid("entry bytes missing".into()))?,
                    )
                    .map_err(|error| Error::Invalid(error.to_string()))?;
                }
            }
        }
    }
    Ok(Candidate {
        plan,
        sources,
        instances: pending,
        target,
    })
}

impl<V: ?Sized + 'static> Application<V> {
    fn plan_active(&self) -> bool {
        self.runtime
            .desired()
            .instances()
            .values()
            .filter(|i| !i.disabled)
            .all(|instance| {
                self.activations.get(&instance.identity).is_some_and(|a| {
                    self.runtime
                        .phase(a)
                        .is_ok_and(|phase| phase == Phase::Active)
                })
            })
    }

    pub fn is_closing(&self) -> bool {
        self.closing
    }

    pub(crate) fn deployment_converging(&self) -> bool {
        !self.plan_active()
            || !self.pending.is_empty()
            || !self.pending_remotes.is_empty()
            || self.runtime.activations().any(|a| {
                self.runtime
                    .phase(a)
                    .is_ok_and(|phase| phase != Phase::Active)
            })
    }

    pub fn deployment_status(&self) -> DeploymentStatus {
        DeploymentStatus {
            desired: self.source().cloned(),
            effective: self.effective.clone(),
            converging: self.deployment_converging(),
            error: self.deployment_error.clone(),
            closing: self.closing,
            instances: self
                .runtime
                .activations()
                .map(|a| self.runtime.status(a).expect("live UI activation"))
                .collect(),
        }
    }

    /// Original executable origins remain necessary through actual retirement
    /// and cleanup. Structural origins confer no consumer approval; the current
    /// desired revision also covers its not-yet-started executable instances.
    pub fn live_source_revisions(&self) -> BTreeSet<u64> {
        let mut revisions = BTreeSet::new();
        if let Some(source) = &self.source {
            revisions.insert(source.reference().revision);
        }
        for activation in self.activations.values() {
            if self
                .runtime
                .instance(activation)
                .is_ok_and(|instance| instance.entry.is_some())
                && let Some(source) = self.origins.get(&activation.epoch())
            {
                revisions.insert(source.reference().revision);
            }
        }
        revisions
    }

    /// Root-owner operation. Remote registrations are trusted endpoints, never grants derived from JSON.
    /// Acknowledges desired input only; the renderer still owns actual native close acknowledgements.
    pub async fn adopt_committed(
        &mut self,
        source: DeploymentRef,
        registrations: Vec<actions::RemoteService>,
    ) -> Result<()> {
        let update = self.load_committed_update(source)?.await?;
        self.adopt_update(update, registrations)
    }

    /// Storage work does not borrow the foreground owner. Render/input may continue
    /// while an embedding awaits this bounded immutable snapshot off its event loop.
    pub fn load_committed_update(
        &self,
        source: DeploymentRef,
    ) -> Result<impl std::future::Future<Output = Result<CommittedUpdate>> + Send + 'static + use<V>>
    {
        self.check_update_source(&source)?;
        let store = self.store.as_ref().expect("paired UI store").clone();
        Ok(async move {
            let loaded = store
                .load_application_at(&source)
                .await
                .map_err(|e| Error::Invalid(e.to_string()))?
                .ok_or_else(|| Error::Invalid("committed UI source not found".into()))?;
            Ok(CommittedUpdate { loaded })
        })
    }

    fn check_update_source(&self, source: &DeploymentRef) -> Result<()> {
        if self.closing {
            return Err(Error::Invalid("UI deployment is closing".into()));
        }
        let current = self
            .source()
            .ok_or_else(|| Error::Invalid("persisted UI source required".into()))?;
        if source.state_domain != current.state_domain
            || source.deployment != current.deployment
            || source.revision < current.revision
        {
            return Err(Error::Invalid(
                "foreign or stale UI deployment source".into(),
            ));
        }
        Ok(())
    }

    /// Refresh external registrations against the already adopted source. No
    /// source edit, package reload, or change to native availability occurs.
    pub fn prepare_current_update(&self) -> Result<PreparedUpdate> {
        let source = self
            .source
            .as_ref()
            .ok_or_else(|| Error::Invalid("persisted UI source required".into()))?;
        self.check_update_source(source.reference())?;
        let plan = self.runtime.desired().clone();
        let instances = plan
            .instances()
            .values()
            .filter(|i| !i.disabled)
            .cloned()
            .collect();
        Ok(PreparedUpdate {
            owner: self.update_owner,
            source: source.clone(),
            candidate: Candidate {
                plan,
                sources: self.sources.clone(),
                instances,
                target: self.target,
            },
        })
    }

    /// Preflight before the trusted launcher publishes any new connection grant.
    pub fn prepare_update(&self, update: CommittedUpdate) -> Result<PreparedUpdate> {
        self.check_update_source(update.source())?;
        let loaded = update.loaded;
        let source = loaded.snapshot();
        let declarations = loaded
            .source
            .remote_exports
            .into_iter()
            .map(|r| (r.capability, r.declaration))
            .collect();
        let candidate = prepare(
            &loaded.source.input,
            loaded.packages,
            self.native.as_ref().expect("open UI registry"),
            &declarations,
        )?;
        if candidate.target != self.target {
            return Err(Error::Invalid("stored presentation target mismatch".into()));
        }
        Ok(PreparedUpdate {
            owner: self.update_owner,
            source,
            candidate,
        })
    }

    pub fn adopt_update(
        &mut self,
        update: CommittedUpdate,
        registrations: Vec<actions::RemoteService>,
    ) -> Result<()> {
        let prepared = self.prepare_update(update)?;
        self.adopt_prepared(prepared, registrations)
    }

    /// Final owner turn rechecks provenance and revision after physical approval.
    pub fn adopt_prepared(
        &mut self,
        update: PreparedUpdate,
        registrations: Vec<actions::RemoteService>,
    ) -> Result<()> {
        if update.owner != self.update_owner {
            return Err(Error::Invalid("foreign foreground update owner".into()));
        }
        self.check_update_source(update.source())?;
        let PreparedUpdate {
            source, candidate, ..
        } = update;
        let Candidate {
            plan,
            sources,
            instances,
            ..
        } = candidate;
        // Validate all endpoint shapes and their exact desired references before install.
        let mut check: Runtime<Export<V>> = Runtime::new(plan.clone());
        let mut supplied = BTreeMap::<RemoteCapability, Uuid>::new();
        for remote in &registrations {
            if supplied
                .insert(remote.target.clone(), remote.registration_epoch)
                .is_some()
            {
                return Err(Error::Invalid("duplicate remote registration".into()));
            }
            let value = service_export(remote.implementation.clone(), &remote.contract)?;
            check
                .attach_remote(
                    remote.target.clone(),
                    remote.registration_epoch,
                    Published {
                        contract: remote.contract.clone(),
                        value,
                    },
                )
                .map_err(|rejected| Error::Core(rejected.error))?;
        }
        self.runtime.install(plan);
        for activation in self.runtime.activations().cloned().collect::<Vec<_>>() {
            if let Some(target) = activation.remote_target()
                && self.remote_epochs.get(target) != supplied.get(target)
            {
                self.runtime.retire(&activation)?;
            }
        }
        self.pending_remotes = registrations
            .into_iter()
            .filter(|remote| {
                !self.runtime.activations().any(|a| {
                    a.remote_target() == Some(&remote.target)
                        && self.remote_epochs.get(&remote.target)
                            == Some(&remote.registration_epoch)
                        && self
                            .runtime
                            .status(a)
                            .is_ok_and(|s| s.phase == Phase::Active)
                })
            })
            .collect();
        self.pending = instances
            .into_iter()
            .filter(|i| {
                !self.activations.get(&i.identity).is_some_and(|a| {
                    self.runtime
                        .phase(a)
                        .is_ok_and(|phase| phase == Phase::Active)
                })
            })
            .collect();
        self.source = Some(source);
        self.sources = sources;
        self.deployment_error = None;
        self.invalidate_pending_captures();
        self.sync_subscription_demand();
        Ok(())
    }

    pub(super) fn dependencies_ready(&self, instance: &Instance) -> bool {
        let active = |id: &InstanceId| {
            self.activations.get(id).is_some_and(|a| {
                self.runtime
                    .status(a)
                    .is_ok_and(|s| s.phase == Phase::Active)
            })
        };
        !instance.parent.as_ref().is_some_and(|id| !active(id))
            && (instance.entry.is_none()
                || instance
                    .bindings
                    .values()
                    .flatten()
                    .all(|b| match &b.provider {
                        Provider::Local { instance, .. } => active(instance),
                        Provider::Remote { capability } => !self
                            .pending_remotes
                            .iter()
                            .any(|remote| &remote.target == capability),
                    }))
    }

    /// Call after original task settlement and renderer acknowledgements. Never closes native resources.
    pub fn advance_deployment(&mut self) -> Result<()> {
        if self.closing {
            return Err(Error::Invalid("UI deployment is closing".into()));
        }
        // Foreground realms share their actual failure fate, while already
        // retiring original domains do not cancel a newer candidate.
        for (id, loaded) in &self.modules {
            if let Some(error) = loaded.realm.failure() {
                let activation = &self.activations[id];
                if self.runtime.phase(activation)? != Phase::Retiring {
                    self.deployment_error.get_or_insert(error.to_string());
                }
                self.runtime.retire(activation)?;
            }
        }
        self.invalidate_pending_captures();
        self.sync_subscription_demand();
        let mut failure = None;
        loop {
            let mut progressed = self.poll_script_activations()?;
            for id in self.modules.keys().cloned().collect::<Vec<_>>() {
                let activation = &self.activations[&id];
                if self.runtime.status(activation).is_ok_and(|s| {
                    s.phase == Phase::Retiring && s.calls == 0 && s.failures.is_empty()
                }) && !self
                    .runtime
                    .mount_statuses()
                    .iter()
                    .any(|m| m.provider == *activation)
                {
                    match self.close_module(&id) {
                        Ok(closed) => progressed |= closed,
                        Err(error) => {
                            self.deployment_error.get_or_insert(error.to_string());
                            failure.get_or_insert(error);
                        }
                    }
                }
            }
            for activation in self.runtime.activations().cloned().collect::<Vec<_>>() {
                if self.runtime.phase(&activation)? != Phase::Retiring {
                    continue;
                }
                match self.runtime.finish_cleanup(&activation) {
                    Ok(()) => {
                        if let Some(target) = activation.remote_target() {
                            self.remote_epochs.remove(target);
                        } else {
                            self.activations.remove(activation.instance());
                            self.values.remove(&activation.epoch());
                            self.value_bytes.remove(&activation.epoch());
                            self.origins.remove(&activation.epoch());
                        }
                        progressed = true;
                    }
                    Err(maka_plugin_core::Error::CleanupPending) => {}
                    Err(error @ maka_plugin_core::Error::CleanupFailed(_)) => {
                        failure.get_or_insert(Error::from(error));
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            for remote in std::mem::take(&mut self.pending_remotes) {
                if self
                    .runtime
                    .activations()
                    .any(|a| a.remote_target() == Some(&remote.target))
                {
                    self.pending_remotes.push(remote);
                    continue;
                }
                let value = service_export(remote.implementation, &remote.contract)?;
                self.runtime
                    .attach_remote(
                        remote.target.clone(),
                        remote.registration_epoch,
                        Published {
                            contract: remote.contract,
                            value,
                        },
                    )
                    .map_err(|rejected| Error::Core(rejected.error))?;
                self.remote_epochs
                    .insert(remote.target, remote.registration_epoch);
                progressed = true;
            }
            for instance in std::mem::take(&mut self.pending) {
                if self.activations.contains_key(&instance.identity)
                    || !self.dependencies_ready(&instance)
                {
                    self.pending.push(instance);
                    continue;
                }
                let id = instance.identity.clone();
                if let Err(error) = self.activate_instance(instance) {
                    if let Some(activation) = self.activations.get(&id) {
                        self.runtime.retire(activation)?;
                    }
                    self.deployment_error.get_or_insert(error.to_string());
                    failure.get_or_insert(error);
                }
                progressed = true;
            }
            if !progressed {
                break;
            }
        }
        if self.deployment_error.is_none() && !self.deployment_converging() {
            self.effective = self.source().cloned();
        }
        failure.map_or(Ok(()), Err)
    }
}
