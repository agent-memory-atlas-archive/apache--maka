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
use futures_util::{FutureExt, StreamExt, stream::FuturesUnordered};
use maka_event_log::applications::operations::{Dispatch as OperationDispatch, OperationRequest};
use maka_js_runtime::plugin::Vm;
use maka_plugin_core::{
    package::Realm,
    plan::{Instance, Provider},
    runtime::{Execution, Phase, Resource, Runtime},
};
use std::panic::AssertUnwindSafe;

type Close = BoxFuture<'static, Result<()>>;
type Jobs = FuturesUnordered<BoxFuture<'static, Done>>;
fn next_change(
    mut cursor: maka_event_log::applications::DeploymentChanges,
) -> BoxFuture<
    'static,
    (
        maka_event_log::applications::DeploymentChanges,
        Result<DeploymentRef>,
    ),
> {
    Box::pin(async move {
        let source = cursor.next().await.map_err(|error| error.to_string());
        (cursor, source)
    })
}

struct Failed {
    _execution: Execution<Service>,
    _scope: Scope,
    _slot: OwnedSemaphorePermit,
    _bytes: OwnedSemaphorePermit,
    _attachments: Attachments,
    _resources: OwnedSemaphorePermit,
    _confirmation: Option<Confirmation>,
}
struct Owned {
    imports: Arc<BTreeMap<String, Vec<Binding>>>,
    resource: Resource,
    close: Option<Close>,
    initializing: bool,
    closing: bool,
    native_owner: Option<Arc<dyn NativeOwner>>,
    vm: Option<usize>,
}
enum Done {
    Loaded {
        loaded: Result<maka_event_log::applications::LoadedDeployment>,
        reply: Option<oneshot::Sender<Result<()>>>,
    },
    IntentRecorded {
        call: Box<Call>,
        recorded: Result<Box<maka_event_log::applications::intents::RecordedIntent>>,
    },
    Initialized {
        activation: Activation,
        exports: Result<BTreeMap<String, Published<Service>>>,
        close: Option<Close>,
        native_owner: Option<Arc<dyn NativeOwner>>,
    },
    Called {
        parent: Option<Ticket>,
        execution: Execution<Service>,
        result: Result<Value>,
        historical: Option<Box<OperationObservation>>,
        consumer: Activation,
        caller: Box<Scope>,
        scope: Box<Scope>,
        cleanup: Result<()>,
        receipt: Option<Box<Result<OperationObservation>>>,
        reply: Reply,
        _slot: OwnedSemaphorePermit,
        _bytes: OwnedSemaphorePermit,
        attachments: Attachments,
        _resources: OwnedSemaphorePermit,
        confirmation: Option<Confirmation>,
    },
    Recovered {
        consumer: Activation,
        reference: Box<RecoveryRef>,
        caller: Box<Scope>,
        resource: Resource,
        parent: Option<Ticket>,
        result: Box<Result<Option<OperationObservation>>>,
        reply: oneshot::Sender<Result<Option<OperationObservation>>>,
        _slot: OwnedSemaphorePermit,
        _bytes: OwnedSemaphorePermit,
    },
    Closed {
        activation: Activation,
        result: Result<()>,
    },
    VmsClosed,
}

pub(super) struct Owner {
    dispatch: Weak<Dispatch>,
    runtime: Runtime<Service>,
    // The source input remains the only composition authority; the plan is derived.
    _input: Input,
    // This lease belongs to accepted work, including sticky cleanup failures.
    persisted: Option<(DeploymentRef, Arc<maka_event_log::EventLog>)>,
    loading_adoption: Option<DeploymentRef>,
    effective: Option<DeploymentRef>,
    origins: BTreeMap<InstanceId, DeploymentRef>,
    pending: Vec<Instance>,
    activations: BTreeMap<InstanceId, Activation>,
    owned: BTreeMap<InstanceId, Owned>,
    sources: BTreeMap<PackageRef, Package>,
    native: NativeRegistry,
    issuer: Issuer,
    policy: Arc<dyn Policy>,
    limits: Limits,
    shared: Option<usize>,
    vms: BTreeMap<usize, Vm>,
    next_vm: usize,
    jobs: Jobs,
    terminals: FuturesUnordered<BoxFuture<'static, (usize, String)>>,
    failed: Vec<Failed>,
    updates: watch::Sender<Snapshot>,
    starting: bool,
    startup_error: Option<String>,
    cleanup_error: Option<String>,
    closing: bool,
    closing_vms: bool,
    closed: bool,
}
impl Owner {
    // These are independently owned deployment dependencies, not a new config
    // authority. Keep their transfer explicit at this private constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plan: Plan,
        input: Input,
        sources: BTreeMap<PackageRef, Package>,
        native: NativeRegistry,
        issuer: Issuer,
        policy: Arc<dyn Policy>,
        limits: Limits,
        updates: watch::Sender<Snapshot>,
        dispatch: Weak<Dispatch>,
        persisted: Option<(DeploymentRef, Arc<maka_event_log::EventLog>)>,
    ) -> Self {
        Self {
            dispatch,
            pending: plan
                .instances()
                .values()
                .filter(|i| !i.disabled)
                .cloned()
                .collect(),
            runtime: Runtime::new(plan),
            _input: input,
            persisted,
            loading_adoption: None,
            effective: None,
            origins: BTreeMap::new(),
            activations: BTreeMap::new(),
            owned: BTreeMap::new(),
            sources,
            native,
            issuer,
            policy,
            limits,
            shared: None,
            vms: BTreeMap::new(),
            next_vm: 0,
            jobs: Jobs::new(),
            terminals: FuturesUnordered::new(),
            failed: Vec::new(),
            updates,
            starting: true,
            startup_error: None,
            cleanup_error: None,
            closing: false,
            closing_vms: false,
            closed: false,
        }
    }
    pub async fn run(
        mut self,
        mut commands: mpsc::Receiver<Command>,
        mut control: mpsc::Receiver<Retirement>,
        mut adoptions: mpsc::Receiver<Adoption>,
        stop: CancellationToken,
    ) {
        let mut commands_open = true;
        let mut control_open = true;
        let mut adoptions_open = true;
        let mut changes = match self
            .persisted
            .as_ref()
            .map(|(source, store)| store.subscribe_application(source.clone()))
            .transpose()
        {
            Ok(changes) => changes.map(next_change),
            Err(error) => {
                self.startup_error = Some(error.to_string());
                self.stop();
                None
            }
        };
        loop {
            self.drive();
            if self.closed {
                let snapshot = self.snapshot();
                let updates = self.updates.clone();
                // The closing acknowledgement follows all of this owner's drops,
                // including native factories that may retain a store themselves.
                drop(commands);
                drop(control);
                drop(adoptions);
                drop(stop);
                drop(changes);
                drop(self);
                updates.send_replace(snapshot);
                return;
            }
            self.publish();
            tokio::select! {
                _ = stop.cancelled(), if !self.closing => {
                    self.stop();
                    commands.close();
                },
                Some(done) = self.jobs.next(), if !self.jobs.is_empty() => self.complete(done),
                Some((vm, reason)) = self.terminals.next(), if !self.terminals.is_empty() => self.retire_vm(vm, reason),
                // Desired-source reads serialize independently of immediate
                // lifecycle revocation. Only the application root owns ingress.
                adoption = adoptions.recv(), if adoptions_open && self.loading_adoption.is_none() => match adoption {
                    Some(Adoption { source, reply }) => {
                        match self.check_adoption_source(&source) {
                            Err(error) => { let _ = reply.send(Err(error)); }
                            Ok(false) => { let _ = reply.send(Ok(())); }
                            Ok(true) => self.load_adoption(source, Some(reply)),
                        }
                    }
                    None => adoptions_open = false,
                },
                command = control.recv(), if control_open => match command {
                    Some(Retirement { activation, reply }) => {
                        let result = self.runtime.retire(&activation).map(|_| ()).map_err(|e| e.to_string());
                        let _ = reply.send(result);
                    }
                    None => control_open = false,
                },
                change = async {
                    match &mut changes {
                        Some(next) => next.await,
                        None => std::future::pending().await,
                    }
                }, if !self.closing && self.loading_adoption.is_none() => {
                    let (cursor, source) = change;
                    match source {
                        Ok(source) => {
                            changes = Some(next_change(cursor));
                            self.observe_source(source);
                        }
                        Err(error) => {
                            changes = None;
                            self.startup_error = Some(error);
                        }
                    }
                },
                command = commands.recv(), if commands_open => match command {
                    Some(command) => self.command(command),
                    None => { commands_open = false; self.stop(); },
                },
                else => std::future::pending::<()>().await,
            }
        }
    }
    fn check_adoption_source(&self, source: &DeploymentRef) -> Result<bool> {
        if self.closing {
            return Err("Host deployment is closing".into());
        }
        let (current, _) = self
            .persisted
            .as_ref()
            .ok_or("persisted deployment required")?;
        if source.state_domain != current.state_domain || source.deployment != current.deployment {
            return Err("foreign deployment source".into());
        }
        if source.revision < current.revision {
            return Err("stale deployment source".into());
        }
        Ok(source != current)
    }
    fn observe_source(&mut self, source: DeploymentRef) {
        // A retained SQL observation can arrive after a newer explicit adoption.
        // It is not a failed candidate and must not poison healthy readiness.
        if self
            .persisted
            .as_ref()
            .is_some_and(|(current, _)| source.revision <= current.revision)
        {
            return;
        }
        match self.check_adoption_source(&source) {
            Ok(true) => self.load_adoption(source, None),
            Ok(false) => {}
            Err(error) => self.startup_error = Some(error),
        }
    }
    fn load_adoption(&mut self, source: DeploymentRef, reply: Option<oneshot::Sender<Result<()>>>) {
        let store = self
            .persisted
            .as_ref()
            .expect("validated persisted source")
            .1
            .clone();
        self.loading_adoption = Some(source.clone());
        self.jobs.push(Box::pin(async move {
            let loaded = store
                .load_application_at(&source)
                .await
                .map_err(|e| e.to_string())
                .and_then(|loaded| loaded.ok_or("committed deployment not found".into()));
            Done::Loaded { loaded, reply }
        }));
    }
    fn adopt(&mut self, loaded: maka_event_log::applications::LoadedDeployment) -> Result<()> {
        if self.closing {
            return Err("Host deployment is closing".into());
        }
        let (current, _) = self
            .persisted
            .as_ref()
            .ok_or("persisted deployment required")?;
        let source = loaded.source.reference;
        if source.revision < current.revision {
            return Err("stale deployment source".into());
        }
        if source == *current {
            return Ok(());
        }
        if !loaded.source.remote_exports.is_empty() {
            return Err("remote Host executor is unavailable".into());
        }
        let input = loaded.source.input;
        let (plan, sources) = prepare(&input, loaded.packages, &self.native, true)?;
        let instances = plan
            .instances()
            .values()
            .filter(|i| !i.disabled)
            .cloned()
            .collect::<Vec<_>>();
        self.runtime.install(plan);
        self.pending = instances
            .into_iter()
            .filter(|i| {
                !self.activations.get(&i.identity).is_some_and(|a| {
                    self.runtime
                        .status(a)
                        .is_ok_and(|s| s.phase == Phase::Active || s.phase == Phase::Loading)
                })
            })
            .collect();
        self.persisted.as_mut().expect("persisted deployment").0 = source;
        self._input = input;
        self.sources = sources;
        self.startup_error = None;
        self.starting = true;
        Ok(())
    }
    fn stop(&mut self) {
        self.closing = true;
        self.starting = false;
        self.pending.clear();
        for activation in self.activations.values() {
            self.runtime
                .retire(activation)
                .expect("activation belongs to this owner");
        }
    }
    fn fail_startup(&mut self, error: String) {
        self.startup_error.get_or_insert(error);
        if self.effective.is_none() {
            self.stop();
        } else {
            self.starting = false;
            self.pending.clear();
            for activation in self.activations.values() {
                if self
                    .runtime
                    .status(activation)
                    .is_ok_and(|s| s.phase == Phase::Loading)
                {
                    self.runtime.retire(activation).expect("owned activation");
                }
            }
        }
    }
    fn drive(&mut self) {
        if self.starting {
            let mut progressed = true;
            while progressed {
                progressed = false;
                for instance in std::mem::take(&mut self.pending) {
                    if self.closing || !self.starting {
                        break;
                    }
                    if self.activations.contains_key(&instance.identity) {
                        self.pending.push(instance);
                        continue;
                    }
                    let active = |id: &InstanceId| {
                        self.activations
                            .get(id)
                            .and_then(|a| self.runtime.status(a).ok())
                            .is_some_and(|s| s.phase == Phase::Active)
                    };
                    if instance.parent.as_ref().is_some_and(|id| !active(id))
                        || instance.entry.is_some()
                            && instance
                                .bindings
                                .values()
                                .flatten()
                                .any(|b| match &b.provider {
                                    Provider::Local { instance, .. } => !active(instance),
                                    Provider::Remote { .. } => true,
                                })
                    {
                        self.pending.push(instance);
                        continue;
                    }
                    progressed = true;
                    if let Err(error) = self.initialize(instance) {
                        self.fail_startup(error);
                    }
                }
            }
            if self.starting
                && self
                    .runtime
                    .desired()
                    .instances()
                    .values()
                    .filter(|i| !i.disabled)
                    .all(|instance| {
                        self.activations.get(&instance.identity).is_some_and(|a| {
                            self.runtime
                                .status(a)
                                .is_ok_and(|s| s.phase == Phase::Active)
                        })
                    })
                && self.pending.is_empty()
                && self.activations.values().all(|a| {
                    self.runtime
                        .status(a)
                        .is_ok_and(|s| s.phase == Phase::Active)
                })
            {
                self.starting = false;
                self.effective = self.persisted.as_ref().map(|(source, _)| source.clone());
            } else if !self.pending.is_empty()
                && self.jobs.is_empty()
                && !self.activations.values().any(|a| {
                    self.runtime
                        .status(a)
                        .is_ok_and(|s| s.phase == Phase::Retiring)
                })
            {
                self.fail_startup("Host dependencies did not become ready".into());
            }
        }
        for (id, activation) in &self.activations {
            if self
                .runtime
                .status(activation)
                .is_ok_and(|s| s.phase == Phase::Retiring && s.calls == 0)
                && let Some(owned) = self.owned.get_mut(id)
                && !owned.initializing
                && !owned.closing
                && let Some(close) = owned.close.take()
            {
                owned.closing = true;
                let activation = activation.clone();
                self.jobs.push(Box::pin(async move {
                    let result = AssertUnwindSafe(close)
                        .catch_unwind()
                        .await
                        .unwrap_or_else(|_| Err("native/module close panicked".into()));
                    Done::Closed { activation, result }
                }));
            }
        }
        let before_cleanup = self.activations.len();
        loop {
            let before = self.activations.len();
            for id in self.activations.keys().cloned().collect::<Vec<_>>() {
                let activation = &self.activations[&id];
                if self
                    .runtime
                    .status(activation)
                    .is_ok_and(|s| s.phase == Phase::Retiring)
                {
                    match self.runtime.finish_cleanup(activation) {
                        Ok(()) => {
                            self.activations.remove(&id);
                            self.origins.remove(&id);
                        }
                        Err(maka_plugin_core::Error::CleanupPending) => {}
                        Err(error) => {
                            self.cleanup_error.get_or_insert(error.to_string());
                        }
                    }
                }
            }
            if before == self.activations.len() {
                break;
            }
        }
        if self.starting && before_cleanup != self.activations.len() {
            self.drive();
            return;
        }
        if self.closing && self.activations.is_empty() && self.jobs.is_empty() && !self.closing_vms
        {
            self.closing_vms = true;
            self.terminals.clear();
            let vms = std::mem::take(&mut self.vms);
            self.shared = None;
            self.jobs.push(Box::pin(async move {
                for vm in vms.into_values() {
                    vm.shutdown().await;
                }
                Done::VmsClosed
            }));
        }
    }
    fn initialize(&mut self, instance: Instance) -> Result<()> {
        let activation = self
            .runtime
            .begin(&instance.identity)
            .map_err(|e| e.to_string())?;
        self.activations
            .insert(instance.identity.clone(), activation.clone());
        if let Some((source, _)) = &self.persisted {
            self.origins
                .insert(instance.identity.clone(), source.clone());
        }
        let Some(entry) = instance.entry else {
            return self
                .runtime
                .publish(&activation, BTreeMap::new())
                .map_err(|e| e.error.to_string());
        };
        let imports = entry
            .imports
            .keys()
            .map(|name| {
                self.capture_bindings(&activation, name)
                    .map(|bindings| (name.clone(), bindings))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        // This fence precedes every asynchronous initialization step, including
        // module readiness. Retire cannot release an initializer's owner early.
        let resource = self
            .runtime
            .own_resource(&activation, "Host service initialization and code")
            .map_err(|e| e.to_string())?;
        self.owned.insert(
            instance.identity.clone(),
            Owned {
                imports: Arc::new(imports),
                resource,
                close: None,
                initializing: true,
                closing: false,
                native_owner: None,
                vm: None,
            },
        );
        match &entry.implementation {
            Implementation::Native { implementation } => {
                let factory = self.native.0[implementation].clone();
                let configuration = instance.configuration;
                let context = NativeContext {
                    activation: activation.clone(),
                    package: instance.package.clone(),
                    services: NativeServices {
                        activation: activation.clone(),
                        imports: self.owned[activation.instance()].imports.clone(),
                        dispatch: self.dispatch.clone(),
                        issuer: self.issuer.clone(),
                    },
                    persistence: self
                        .persisted
                        .as_ref()
                        .map(|(source, store)| NativePersistence {
                            source: source.clone(),
                            store: store.clone(),
                        }),
                };
                self.jobs.push(Box::pin(async move {
                    let initialized =
                        AssertUnwindSafe(async move { factory(context, configuration).await })
                            .catch_unwind()
                            .await;
                    match initialized {
                        Ok(native) => {
                            let exports = native.exports.and_then(|actual| {
                                actual
                                    .into_iter()
                                    .map(|(name, actual)| {
                                        Ok((
                                            name,
                                            Published {
                                                value: Service::new(
                                                    actual.contract.clone(),
                                                    ImplementationValue::Native(actual.value),
                                                )?,
                                                contract: actual.contract,
                                            },
                                        ))
                                    })
                                    .collect()
                            });
                            let owner = native.owner.clone();
                            Done::Initialized {
                                activation,
                                exports,
                                close: Some(Box::pin(async move { owner.close().await })),
                                native_owner: Some(native.owner),
                            }
                        }
                        Err(_) => Done::Initialized {
                            activation,
                            exports: Err("native initializer panicked".into()),
                            close: None,
                            native_owner: None,
                        },
                    }
                }));
            }
            Implementation::JavaScript { module, realm } => {
                let load = (|| {
                    let vm_index = match realm {
                        Realm::Shared => match self.shared {
                            Some(index)
                                if self.vms.get(&index).is_some_and(|vm| !vm.is_terminated()) =>
                            {
                                index
                            }
                            _ => {
                                let index = self.new_vm()?;
                                self.shared = Some(index);
                                index
                            }
                        },
                        Realm::Dedicated => self.new_vm()?,
                    };
                    self.owned
                        .get_mut(&instance.identity)
                        .expect("initialization resource")
                        .vm = Some(vm_index);
                    let vm = &self.vms[&vm_index];
                    let source = std::str::from_utf8(
                        self.sources[&instance.package]
                            .file(module)
                            .ok_or("Host module bytes missing")?,
                    )
                    .map_err(|e| e.to_string())?;
                    vm.load(module.clone(), source.into())
                        .map_err(|e| e.to_string())
                })();
                match load {
                    Ok(module) => {
                        let close_module = module.clone();
                        let close: Close = Box::pin(async move {
                            close_module.close().await.map_err(|e| e.to_string())
                        });
                        self.jobs.push(Box::pin(async move {
                            let exports = AssertUnwindSafe(script::activate(
                                module,
                                entry,
                                instance.configuration,
                            ))
                            .catch_unwind()
                            .await
                            .unwrap_or_else(|_| Err("JavaScript initializer panicked".into()));
                            Done::Initialized {
                                activation,
                                exports,
                                close: Some(close),
                                native_owner: None,
                            }
                        }));
                    }
                    Err(error) => {
                        // No module was admitted; VM lifetime remains owned by
                        // this deployment and is acknowledged during shutdown.
                        self.jobs.push(Box::pin(async move {
                            Done::Initialized {
                                activation,
                                exports: Err(error),
                                close: Some(Box::pin(async { Ok(()) })),
                                native_owner: None,
                            }
                        }));
                    }
                }
            }
        }
        Ok(())
    }
    fn new_vm(&mut self) -> Result<usize> {
        let index = self.next_vm;
        let next = index.checked_add(1).ok_or("VM identity exhausted")?;
        let vm = Vm::scoped(self.limits.clone()).map_err(|e| e.to_string())?;
        self.next_vm = next;
        let observed = vm.clone();
        self.terminals.push(Box::pin(async move {
            (index, observed.failed().await.to_string())
        }));
        self.vms.insert(index, vm);
        Ok(index)
    }
    fn retire_vm(&mut self, vm: usize, reason: String) {
        let mut affected = false;
        for (id, owned) in &self.owned {
            if owned.vm == Some(vm) {
                affected |= self
                    .runtime
                    .status(&self.activations[id])
                    .is_ok_and(|s| s.phase != Phase::Retiring);
                self.runtime
                    .retire(&self.activations[id])
                    .expect("VM activation belongs to owner");
            }
        }
        if affected && self.starting {
            self.fail_startup(reason);
        }
    }
    fn check_vm(&mut self, provider: &Activation) -> Result<()> {
        if let Some(index) = self.owned.get(provider.instance()).and_then(|o| o.vm)
            && self.vms.get(&index).is_some_and(Vm::is_terminated)
        {
            self.retire_vm(index, "Host JavaScript execution domain terminated".into());
            return Err("Host JavaScript execution domain terminated".into());
        }
        Ok(())
    }
    fn capture_bindings(&self, consumer: &Activation, import: &str) -> Result<Vec<Binding>> {
        self.runtime
            .bind(consumer, import)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|capability| {
                let contract = self
                    .runtime
                    .inspect(&capability)
                    .map_err(|e| e.to_string())?
                    .contract
                    .clone();
                let instance = self
                    .runtime
                    .instance(capability.provider())
                    .map_err(|e| e.to_string())?;
                let owned = self
                    .owned
                    .get(capability.provider().instance())
                    .ok_or("provider has no captured imports")?;
                // Use the actual activation's retained plan and captured imports,
                // never the desired plan or a newly resolved dependency. Child
                // digests carry the dependency closure without retaining values.
                let imports: BTreeMap<_, Vec<_>> = owned
                    .imports
                    .iter()
                    .map(|(name, bindings)| {
                        (
                            name,
                            bindings.iter().map(Binding::execution_basis).collect(),
                        )
                    })
                    .collect();
                let native = owned
                    .native_owner
                    .as_ref()
                    .map(|owner| {
                        std::panic::catch_unwind(AssertUnwindSafe(|| owner.execution_basis()))
                            .map(|basis| match basis {
                                Some(facts) => serde_json::json!({"facts": facts}),
                                None => {
                                    serde_json::json!({"activation": capability.provider().epoch()})
                                }
                            })
                            .map_err(|_| "native execution basis panicked")
                    })
                    .transpose()?;
                let mut basis = serde_json::json!({
                    "identity": instance.identity, "parent": instance.parent,
                    "package": instance.package, "disabled": instance.disabled,
                    "configuration": instance.configuration, "entry": instance.entry,
                    "bindings": instance.bindings, "exports": instance.exports,
                    "imports": imports, "export": capability.export(), "contract": contract,
                    "native": native,
                });
                basis.sort_all_objects();
                let digest = maka_runtime::artifact::content_digest(
                    &serde_json::to_vec(&basis).map_err(|e| e.to_string())?,
                );
                Ok(Binding(capability, Arc::new(contract), digest.into()))
            })
            .collect()
    }
    fn command(&mut self, command: Command) {
        match command {
            Command::Bind {
                consumer,
                import,
                reply,
            } => {
                let result = if self.closing {
                    Err("Host deployment is closing".into())
                } else {
                    self.capture_bindings(&consumer, &import)
                };
                let _ = reply.send(result);
            }
            Command::Call(call) => {
                let Call {
                    mut parent,
                    binding,
                    mut kind,
                    method,
                    input,
                    scope,
                    attachments,
                    capture,
                    slot,
                    bytes,
                    resources,
                } = *call;
                let operation = kind.operation();
                if kind.closed() && parent.is_none() {
                    return;
                }
                let mut descriptor_failed = false;
                let admission = (|| {
                    if self.closing
                        || !self.issuer.owns(&scope)
                        || scope.cancellation.is_cancelled()
                    {
                        return Err("foreign or closed Host invocation".into());
                    }
                    self.check_native_origin(&scope)?;
                    self.check_vm(binding.0.consumer())?;
                    self.check_vm(binding.provider())?;
                    let service = self
                        .runtime
                        .inspect(&binding.0)
                        .map_err(|e| e.to_string())?;
                    service
                        .methods
                        .get(&method)
                        .ok_or_else(|| "undeclared service method".to_string())?
                        .input
                        .validate(&input)
                        .map_err(|e| e.to_string())?;
                    let Interface::Service { methods } = &service.contract.interface else {
                        unreachable!()
                    };
                    let declared_method = &methods[&method];
                    if operation.is_some_and(|id| id.is_nil())
                        || match declared_method.mode {
                            MethodMode::Query => {
                                operation.is_some() || matches!(kind, CallKind::Subscription { .. })
                            }
                            MethodMode::Mutation => operation.is_none(),
                            MethodMode::Subscription => {
                                !matches!(kind, CallKind::Subscription { .. })
                            }
                        }
                    {
                        return Err("service method mode or operation identity mismatch".into());
                    }
                    let request = operation
                        .map(|operation| {
                            let source = self
                                .origins
                                .get(binding.0.consumer().instance())
                                .ok_or("durable mutation requires original deployment source")?;
                            Ok::<OperationRequest, String>(OperationRequest {
                                source: source.clone(),
                                namespace: binding.provider().instance().clone(),
                                consumer: binding.0.consumer().instance().clone(),
                                export: binding.0.export().into(),
                                method: method.clone(),
                                contract: service.contract.clone(),
                                operation,
                                input: input.clone(),
                                capture,
                                origin: scope.identity.clone(),
                            })
                        })
                        .transpose()?;
                    let reference = request
                        .as_ref()
                        .map(OperationRequest::reference)
                        .transpose()
                        .map_err(|e| e.to_string())?;
                    if let Some(reference) = &reference {
                        check_reference(reference)?;
                        if kind
                            .expected()
                            .is_some_and(|expected| expected != reference)
                        {
                            return Err("prepared original reference no longer matches".into());
                        }
                    }
                    let authenticated_caller =
                        match std::panic::catch_unwind(AssertUnwindSafe(|| {
                            let Interface::Service { methods } = &service.contract.interface else {
                                unreachable!()
                            };
                            let declared = &methods[&method].resources;
                            if declared.len() != attachments.len()
                                || declared.iter().any(|(slot, resource_type)| {
                                    attachments.get(slot).is_none_or(|resource| {
                                        resource.resource_type() != *resource_type
                                    })
                                })
                            {
                                return Err("Host attachment slots or types mismatch".into());
                            }
                            let size = resource_ingress::attachment_size(&attachments)?;
                            if size.payload != resources.num_permits()
                                || size.descriptors + check_json(&input)?.max(1)
                                    > bytes.num_permits()
                            {
                                return Err(
                                    "Host attachment descriptor changed before admission".into()
                                );
                            }
                            let invocation = Invocation {
                                scope: &scope,
                                native_origin: self.native_origin(&scope),
                                consumer: binding.0.consumer(),
                                provider: binding.provider(),
                                export: binding.export(),
                                contract: &service.contract,
                                method: &method,
                                input: &input,
                                attachments: &attachments,
                            };
                            self.policy.admit(invocation)?;
                            if let Some(intent) = kind.peer_intent() {
                                self.policy.confirm_peer_mutation(invocation, intent)?;
                            }
                            Ok::<_, String>(self.policy.caller(&scope))
                        })) {
                            Ok(result) => result?,
                            Err(_) => {
                                let error =
                                    "Host admission policy or attachment descriptor panicked"
                                        .to_string();
                                descriptor_failed = true;
                                return Err(error);
                            }
                        };
                    if scope.cancellation.is_cancelled() {
                        return Err("Host invocation was revoked".into());
                    }
                    if matches!(
                        kind,
                        CallKind::PrepareMutation { .. }
                            | CallKind::Mutation {
                                confirmation: None,
                                ..
                            }
                    ) {
                        return Ok((None, None, request, reference, authenticated_caller));
                    }
                    let child = scope.child().map_err(|e| e.to_string())?;
                    let execution = self.runtime.admit(&binding.0).map_err(|e| e.to_string())?;
                    Ok((
                        Some(execution),
                        Some(child),
                        request,
                        reference,
                        authenticated_caller,
                    ))
                })();
                match admission {
                    Err(error) => {
                        drop(kind.take_confirmation());
                        drop((attachments, resources, slot, bytes, input, scope));
                        if let Some(ticket) = parent.take() {
                            ticket.complete(if descriptor_failed {
                                Err(error.clone())
                            } else {
                                Ok(())
                            });
                        }
                        kind.reject(error);
                    }
                    Ok((execution, child, request, reference, authenticated_caller)) => {
                        if let CallKind::PrepareMutation { reply, .. } = kind {
                            let _ = reply.send(Ok(PreparedMutation {
                                reference: reference.expect("mutation reference"),
                                invocation: Some(PreparedInvocation {
                                    binding,
                                    method,
                                    input,
                                    scope,
                                    attachments,
                                    capture,
                                    slot,
                                    bytes,
                                    resources,
                                    parent,
                                }),
                            }));
                            return;
                        }
                        if matches!(
                            kind,
                            CallKind::Mutation {
                                confirmation: None,
                                ..
                            }
                        ) {
                            let request = request.expect("checked original mutation");
                            let store = self
                                .persisted
                                .as_ref()
                                .expect("persisted mutation")
                                .1
                                .clone();
                            if let Some(ticket) = &mut parent {
                                ticket.start();
                            }
                            let call = Box::new(Call {
                                parent,
                                binding,
                                kind,
                                method,
                                input,
                                scope,
                                attachments,
                                capture,
                                slot,
                                bytes,
                                resources,
                            });
                            // Persistence owns no child Execution. Return the exact captured
                            // call to this owner for final admission after the SQL task ends.
                            self.jobs.push(Box::pin(async move {
                                let recorded = store
                                    .record_application_intent(
                                        request.source,
                                        request.consumer,
                                        reference.expect("checked original reference"),
                                    )
                                    .await
                                    .map(Box::new)
                                    .map_err(|e| format!("caller intent was not confirmed: {e}"));
                                Done::IntentRecorded { call, recorded }
                            }));
                            return;
                        }
                        let execution = execution.expect("accepted execution");
                        let child = child.expect("accepted child scope");
                        let confirmation = kind.take_confirmation();
                        let caller = scope;
                        let scope = child;
                        let consumer = binding.0.consumer().clone();
                        let store = self.persisted.as_ref().map(|(_, store)| store.clone());
                        if let Some(ticket) = &mut parent {
                            ticket.start();
                        }
                        let (output, stream) = kind.accept(
                            execution.id(),
                            reference.clone(),
                            execution.value().methods[&method].output.clone(),
                            scope.cancellation.clone(),
                        );
                        let context = Context {
                            stream: stream.clone(),
                            caller: authenticated_caller,
                            scope: scope.clone(),
                            consumer: execution.provider().clone(),
                            operation: reference.clone(),
                            attachments: attachments.clone(),
                            imports: self.owned[execution.provider().instance()].imports.clone(),
                            dispatch: self.dispatch.clone(),
                        };
                        let is_mutation = operation.is_some();
                        self.jobs.push(Box::pin(async move {
                            let mut historical = None;
                            let mut claim = None;
                            let mut receipt = None;
                            let dispatched = if let Some(request) = request {
                                let store = store.as_ref().expect("persisted mutation");
                                match store.dispatch_operation(request).await.map_err(|e| e.to_string()) {
                                    Ok(OperationDispatch::New(new)) => { claim = Some(new); Ok(()) },
                                    Ok(OperationDispatch::Existing(observation)) => { historical = Some(Box::new(observation)); Ok(()) },
                                    Err(error) => {
                                        let error=format!("operation dispatch was not confirmed: {error}");
                                        receipt=Some(Err(error.clone())); Err(error)
                                    },
                                }
                            } else { Ok(()) };
                            let invocation = async {
                                dispatched?;
                                if historical.is_some() { return Ok::<Value, String>(Value::Null); }
                                if scope.cancellation.is_cancelled() { return Err("Host invocation was revoked before callback".into()); }
                                let service = execution.value();
                                let value = match &service.implementation {
                                    ImplementationValue::Native(provider) => provider.call(method.clone(), input, context).await,
                                    ImplementationValue::JavaScript(callbacks) => callbacks.call_with_context(
                                        &method, vec![input, serde_json::to_value(abi::HostObservation {source:scope.identity.clone(), operation:context.operation().cloned()}).map_err(|e| e.to_string())?], Arc::new(context),
                                    ).await.map_err(|e| e.to_string()),
                                }?;
                                check_json(&value)?;
                                service.methods[&method].output.validate(&value).map_err(|e| e.to_string())?;
                                Ok(value)
                            };
                            let result = AssertUnwindSafe(invocation).catch_unwind().await;
                            if let Some(stream) = stream { stream.close(); }
                            let cleanup = scope.finish().await.map_err(|e| e.to_string());
                            let (result, cleanup) = match result {
                                Ok(result) => (result, cleanup),
                                Err(_) => (Err("Host service executor panicked".into()), Err("Host service executor disappeared".into())),
                            };
                            if let Some(claim) = claim {
                                let outcome = match &result {
                                    Ok(value) => OperationOutcome::Returned { value: value.clone() },
                                    Err(error) => OperationOutcome::Unknown { diagnostic: diagnostic(error) },
                                };
                                let settled_cleanup = match &cleanup {
                                    Ok(()) => OperationCleanup::Confirmed,
                                    Err(error) => OperationCleanup::Failed { diagnostic: diagnostic(error) },
                                };
                                receipt=Some(store.as_ref().expect("original operation store").finish_operation(&claim, outcome, settled_cleanup).await.map_err(|error|format!("operation settlement outcome unknown; read original receipt: {error}")));
                            }
                            if is_mutation && historical.is_none() && receipt.is_none() {
                                receipt=Some(Err("operation receipt unavailable; read original reference".into()));
                            }
                            Done::Called { parent, execution, result, historical, consumer, caller: Box::new(caller),
                                scope: Box::new(scope), cleanup, receipt: receipt.map(Box::new), reply: output, _slot: slot, _bytes: bytes, attachments, _resources: resources, confirmation }
                        }));
                    }
                }
            }
            Command::Recover {
                consumer,
                reference,
                scope,
                mut parent,
                reply,
                _slot,
                _bytes,
            } => {
                let admission = self
                    .recovery_gate(&consumer, &reference, &scope)
                    .and_then(|()| {
                        let store = self
                            .persisted
                            .as_ref()
                            .ok_or("operation recovery requires an owned State Root")?
                            .1
                            .clone();
                        let resource = self
                            .runtime
                            .own_resource(&consumer, "original operation receipt lookup")
                            .map_err(|e| e.to_string())?;
                        Ok((store, resource))
                    });
                match admission {
                    Err(error) => {
                        let _ = reply.send(Err(error));
                    }
                    Ok((store, resource)) => {
                        if let Some(ticket) = &mut parent {
                            ticket.start();
                        }
                        self.jobs.push(Box::pin(async move {
                            let result = store
                                .operation_observation(&reference)
                                .await
                                .map_err(|e| e.to_string());
                            Done::Recovered {
                                consumer,
                                reference,
                                caller: Box::new(scope),
                                resource,
                                parent,
                                result: Box::new(result),
                                reply,
                                _slot,
                                _bytes,
                            }
                        }));
                    }
                }
            }
        }
    }
    fn native_origin<'a>(&self, scope: &'a Scope) -> Option<&'a Activation> {
        self.issuer
            .evidence::<native_context::NativeOrigin>(scope)
            .map(|origin| &origin.0)
    }
    fn check_native_origin(&mut self, scope: &Scope) -> Result<()> {
        if let Some(origin) = self.native_origin(scope) {
            self.check_vm(origin)?;
            if self
                .runtime
                .status(origin)
                .map_err(|error| error.to_string())?
                .phase
                != Phase::Active
            {
                return Err("native work origin is not active".into());
            }
        }
        Ok(())
    }
    fn recovery_gate(
        &mut self,
        consumer: &Activation,
        reference: &RecoveryRef,
        scope: &Scope,
    ) -> Result<()> {
        if self.closing || !self.issuer.owns(scope) || scope.cancellation.is_cancelled() {
            return Err("foreign or closed Host recovery invocation".into());
        }
        self.check_native_origin(scope)?;
        self.check_vm(consumer)?;
        if self
            .runtime
            .status(consumer)
            .map_err(|e| e.to_string())?
            .phase
            != Phase::Active
        {
            return Err("Host recovery consumer is not active".into());
        }
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            self.policy.recover(RecoveryInvocation {
                scope,
                native_origin: self.native_origin(scope),
                consumer,
                reference,
            })
        }));
        result.map_err(|_| "Host recovery policy panicked".to_string())??;
        if scope.cancellation.is_cancelled() {
            return Err("Host recovery invocation was revoked".into());
        }
        Ok(())
    }
    fn complete(&mut self, done: Done) {
        match done {
            Done::Loaded { loaded, reply } => {
                self.loading_adoption = None;
                let result = loaded.and_then(|loaded| self.adopt(loaded));
                if let Err(error) = &result {
                    self.startup_error = Some(error.clone());
                }
                self.publish();
                if let Some(reply) = reply {
                    let _ = reply.send(result);
                }
            }
            Done::IntentRecorded { mut call, recorded } => match recorded {
                Ok(intent) => {
                    let CallKind::Mutation { confirmation, .. } = &mut call.kind else {
                        unreachable!("recording belongs to a mutation")
                    };
                    *confirmation = Some(Confirmation::Recorded(intent));
                    self.command(Command::Call(call));
                }
                Err(error) => {
                    let parent = call.parent.take();
                    let Call {
                        kind,
                        attachments,
                        resources,
                        slot,
                        bytes,
                        input,
                        scope,
                        ..
                    } = *call;
                    drop((attachments, resources, slot, bytes, input, scope));
                    if let Some(ticket) = parent {
                        ticket.complete(Ok(()));
                    }
                    kind.reject(error);
                }
            },
            Done::Initialized {
                activation,
                exports,
                close,
                native_owner,
            } => {
                let was_retiring = self
                    .runtime
                    .status(&activation)
                    .is_ok_and(|s| s.phase == Phase::Retiring);
                let owned = self
                    .owned
                    .get_mut(activation.instance())
                    .expect("initializer has a resource fence");
                owned.initializing = false;
                if close.is_none() {
                    let reason = "native initialization cleanup is unconfirmed".to_string();
                    self.runtime
                        .complete_resource(&owned.resource, Err(reason.clone()))
                        .expect("owned resource");
                    self.cleanup_error.get_or_insert(reason);
                }
                owned.close = close;
                owned.native_owner = native_owner;
                let result = exports.and_then(|batch| {
                    self.runtime
                        .publish(&activation, batch)
                        .map_err(|e| e.error.to_string())?;
                    if let Some(owner) = &owned.native_owner {
                        std::panic::catch_unwind(AssertUnwindSafe(|| owner.activated()))
                            .map_err(|_| "native activation hook panicked".to_string())?;
                    }
                    Ok(())
                });
                if let Err(error) = result {
                    if self.closing || was_retiring {
                        // A late successful initializer must still close under
                        // its original resource; retirement is not startup failure.
                    } else {
                        self.fail_startup(error);
                    }
                }
            }
            Done::Called {
                parent,
                execution,
                result,
                historical,
                consumer,
                caller,
                scope,
                cleanup,
                mut receipt,
                reply,
                _slot,
                _bytes,
                attachments,
                _resources,
                confirmation,
            } => {
                if let Some(observation) = historical {
                    receipt = Some(Box::new(
                        self.recovery_gate(&consumer, &observation.reference, &caller)
                            .map(|()| *observation),
                    ));
                }
                let _ = self.check_vm(execution.provider());
                let failed = cleanup.is_err();
                let cleanup_error = cleanup.as_ref().err().cloned();
                self.runtime
                    .settle(&execution, cleanup.clone())
                    .expect("accepted execution belongs to owner");
                if failed {
                    self.cleanup_error
                        .get_or_insert_with(|| cleanup_error.unwrap());
                    self.failed.push(Failed {
                        _execution: execution,
                        _scope: *scope,
                        _slot,
                        _bytes,
                        _attachments: attachments,
                        _resources,
                        _confirmation: confirmation,
                    });
                } else {
                    // Confirmed physical cleanup releases caller custody before waking
                    // an observer that may immediately reopen the caller Root.
                    drop((confirmation, attachments, _resources, _slot, _bytes, scope));
                }
                if let Some(ticket) = parent {
                    ticket.complete(cleanup.clone());
                }
                reply.send(result, cleanup, receipt.map(|receipt| *receipt));
            }
            Done::Recovered {
                consumer,
                reference,
                caller,
                resource,
                parent,
                result,
                reply,
                _slot,
                _bytes,
            } => {
                let result = self
                    .recovery_gate(&consumer, &reference, &caller)
                    .and(*result);
                self.runtime
                    .complete_resource(&resource, Ok(()))
                    .expect("owned receipt lookup");
                if let Some(ticket) = parent {
                    ticket.complete(Ok(()));
                }
                let _ = reply.send(result);
            }
            Done::Closed { activation, result } => {
                // The last module owns actual domain shutdown under its original
                // resource fence. An empty isolate is not settled code ownership.
                if result.is_ok()
                    && let Some(index) = self.owned[activation.instance()].vm
                    && self.owned.values().filter(|o| o.vm == Some(index)).count() == 1
                    && let Some(vm) = self.vms.remove(&index)
                {
                    self.owned
                        .get_mut(activation.instance())
                        .expect("owned module")
                        .vm = None;
                    if self.shared == Some(index) {
                        self.shared = None;
                    }
                    self.jobs.push(Box::pin(async move {
                        vm.shutdown().await;
                        Done::Closed { activation, result }
                    }));
                    return;
                }
                let owned = &self.owned[activation.instance()];
                let success = result.is_ok();
                if let Err(error) = &result {
                    self.cleanup_error.get_or_insert(error.clone());
                }
                self.runtime
                    .complete_resource(&owned.resource, result)
                    .expect("owned resource");
                if success {
                    self.owned.remove(activation.instance());
                }
            }
            Done::VmsClosed => self.closed = true,
        }
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            loading: self.loading_adoption.clone(),
            desired: self.persisted.as_ref().map(|(source, _)| source.clone()),
            effective: self.effective.clone(),
            starting: self.starting,
            startup_error: self.startup_error.clone(),
            closing: self.closing,
            closed: self.closed,
            cleanup_error: self.cleanup_error.clone(),
            instances: self
                .activations
                .values()
                .map(|a| self.runtime.status(a).expect("live activation"))
                .collect(),
        }
    }
    fn publish(&self) {
        self.updates.send_replace(self.snapshot());
    }
}

pub(super) fn diagnostic(error: &str) -> String {
    // Keep the encoded diagnostic within the journal budget as well as its
    // raw UTF-8 size. A control character may occupy six JSON bytes.
    let mut size = 0;
    let mut end = 0;
    for (offset, character) in error.char_indices() {
        let bytes = match character {
            '"' | '\\' | '\n' | '\r' | '\t' | '\u{08}' | '\u{0c}' => 2,
            '\u{00}'..='\u{1f}' => 6,
            _ => character.len_utf8(),
        };
        if size + bytes > 64 * 1024 - 64 {
            break;
        }
        size += bytes;
        end = offset + character.len_utf8();
    }
    let mut diagnostic = error[..end].to_owned();
    if end < error.len() {
        diagnostic.push('…');
    }
    diagnostic
}

#[cfg(test)]
mod tests;
