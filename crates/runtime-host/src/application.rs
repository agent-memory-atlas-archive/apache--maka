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

//! Host service execution over the shared plugin lifecycle kernel.
pub mod abi;
mod context;
mod owner;
pub use context::Context;
mod script;

use futures_util::future::BoxFuture;
use maka_js_runtime::plugin::Limits;
use maka_plugin_core::{
    composition::{ExecutionLocation, Input, InstanceId},
    contract::{Contract, Interface, MethodMode},
    deployment::DeploymentRef,
    operation::{
        OperationCleanup, OperationObservation, OperationOutcome, OperationReply, Receipt,
        RecoveryRef,
    },
    package::{Implementation, PackageRef, Packages},
    plan::{Plan, RemoteExports},
    runtime::{Activation, Capability, Published, Status},
};
use maka_plugins::{
    call::{Attachments, Issuer, Scope, Ticket},
    package::Package,
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::Write,
    sync::{Arc, Weak},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, String>;

/// Runs only after final lifetime and Host policy admission. A handler error does
/// not prove failed cleanup; the owner separately settles Scope resources.
pub trait Handler: Send + Sync + 'static {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value>>;
}

/// Trusted Host policy, checked in the same owner turn as core admission.
/// Scope observation, package identity and a binding are not authorization.
pub trait Policy: Send + Sync + 'static {
    fn admit(&self, invocation: Invocation<'_>) -> Result<()>;
    /// Mutation admission never grants access to historical operation records.
    fn recover(&self, _invocation: RecoveryInvocation<'_>) -> Result<()> {
        Err("operation recovery is not authorized".into())
    }
}

/// Borrowed facts at final admission. Exact execution identities and actual
/// payloads stay distinct from the Scope's private authorization evidence.
pub struct Invocation<'a> {
    pub scope: &'a Scope,
    pub consumer: &'a Activation,
    pub provider: &'a Activation,
    pub contract: &'a Contract,
    pub method: &'a str,
    pub input: &'a Value,
    pub attachments: &'a Attachments,
}

pub struct RecoveryInvocation<'a> {
    pub scope: &'a Scope,
    pub consumer: &'a Activation,
    pub reference: &'a RecoveryRef,
}

/// The actual native owner survives rejected exports and failed close futures.
/// Successful close confirms its resources have settled; dropping a close
/// observer or future is not that confirmation.
pub trait NativeOwner: Send + Sync + 'static {
    fn close(&self) -> BoxFuture<'static, Result<()>>;
}
pub struct NativeActivation {
    pub exports: Result<BTreeMap<String, Published<Arc<dyn Handler>>>>,
    pub owner: Arc<dyn NativeOwner>,
}
type Factory = Arc<dyn Fn(Value) -> BoxFuture<'static, NativeActivation> + Send + Sync>;
#[derive(Default)]
pub struct NativeRegistry(BTreeMap<String, Factory>);
impl NativeRegistry {
    pub fn insert(&mut self, name: String, factory: Factory) -> Result<()> {
        if self.0.contains_key(&name) {
            return Err("duplicate native implementation".into());
        }
        self.0.insert(name, factory);
        Ok(())
    }
}

pub(super) struct Method {
    input: jsonschema::Validator,
    output: jsonschema::Validator,
}
pub(super) enum ImplementationValue {
    JavaScript(maka_js_runtime::plugin::Captured),
    Native(Arc<dyn Handler>),
}
pub(super) struct Service {
    contract: Contract,
    methods: BTreeMap<String, Method>,
    implementation: ImplementationValue,
}
impl Service {
    fn new(contract: Contract, implementation: ImplementationValue) -> Result<Self> {
        let Interface::Service { methods } = &contract.interface else {
            return Err("Host export must declare a service".into());
        };
        let compiled = methods
            .iter()
            .map(|(name, method)| {
                if method.mode == MethodMode::Subscription
                    || method.mode == MethodMode::Mutation && !method.resources.is_empty()
                    || method.resources.len() > 16

                {
                    return Err(
                        "subscriptions and mutation resource snapshots require their executors; maximum 16 resource slots"
                            .into(),
                    );
                }
                Ok((
                    name.clone(),
                    Method {
                        input: jsonschema::validator_for(&method.input)
                            .map_err(|e| e.to_string())?,
                        output: jsonschema::validator_for(&method.output)
                            .map_err(|e| e.to_string())?,
                    },
                ))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            contract,
            methods: compiled,
            implementation,
        })
    }
}

/// An exact consumer/provider registration, never a JSON or name-based grant.
pub struct Binding(Capability<Service>);
impl Clone for Binding {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl Binding {
    pub fn provider(&self) -> &Activation {
        self.0.provider()
    }
    pub fn registration_epoch(&self) -> Uuid {
        self.0.registration_epoch()
    }
}

pub struct Accepted {
    pub id: Uuid,
    result: oneshot::Receiver<QueryCompletion>,
}
/// Independent business and executor-cleanup observations from the original Host owner.
pub struct QueryCompletion {
    pub result: Result<Value>,
    pub cleanup: Result<()>,
}
impl Accepted {
    /// Observe business only. This does not acknowledge executor cleanup.
    pub async fn result(self) -> Result<Value> {
        self.completion().await.result
    }
    /// Dropping this observer never cancels execution or acknowledges cleanup.
    pub async fn completion(self) -> QueryCompletion {
        self.result.await.unwrap_or_else(|_| QueryCompletion {
            result: Err("Host executor owner disappeared".into()),
            cleanup: Err("Host executor cleanup unconfirmed: owner disappeared".into()),
        })
    }
}

/// An accepted mutation always retains its original reference, even when its
/// result observer loses the owner. Only a recorded receipt confirms journal facts.
pub struct Mutation {
    pub id: Uuid,
    pub reference: RecoveryRef,
    result: oneshot::Receiver<OperationReply>,
}
impl Mutation {
    pub async fn result(self) -> OperationReply {
        self.result.await.unwrap_or_else(|_| OperationReply {
            reference: self.reference,
            receipt: Receipt::Unavailable {
                diagnostic: "Host executor owner disappeared; read original receipt".into(),
            },
        })
    }
}

#[derive(Clone, Default)]
pub struct Snapshot {
    pub starting: bool,
    pub startup_error: Option<String>,
    pub closing: bool,
    pub closed: bool,
    pub cleanup_error: Option<String>,
    pub instances: Vec<Status>,
}

/// The deployment root owns its stop signal. Ordinary clients cannot keep that
/// root alive; root Drop requests retirement, never acknowledges cleanup.
pub struct Application {
    client: Client,
    stop: CancellationToken,
    source: Option<DeploymentRef>,
}
impl Drop for Application {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
#[derive(Clone)]
pub struct Client {
    dispatch: Arc<Dispatch>,
    control: mpsc::Sender<Control>,
    updates: watch::Receiver<Snapshot>,
}

struct Dispatch {
    commands: mpsc::Sender<Command>,
    slots: Arc<Semaphore>,
    bytes: Arc<Semaphore>,
    resources: Arc<Semaphore>,
}

pub(super) enum Command {
    Bind {
        consumer: Activation,
        import: String,
        reply: oneshot::Sender<Result<Vec<Binding>>>,
    },
    Call(Box<Call>),
    Recover {
        consumer: Activation,
        reference: Box<RecoveryRef>,
        scope: Scope,
        parent: Option<Ticket>,
        reply: oneshot::Sender<Result<Option<OperationObservation>>>,
        _slot: OwnedSemaphorePermit,
        _bytes: OwnedSemaphorePermit,
    },
}
pub(super) struct Call {
    parent: Option<Ticket>,
    kind: CallKind,
    binding: Binding,
    method: String,
    input: Value,
    scope: Scope,
    attachments: Attachments,
    slot: OwnedSemaphorePermit,
    bytes: OwnedSemaphorePermit,
    resources: OwnedSemaphorePermit,
}
pub(super) enum CallKind {
    Query(oneshot::Sender<Result<Accepted>>),
    Mutation {
        operation: Uuid,
        reply: oneshot::Sender<Result<Mutation>>,
    },
}
impl CallKind {
    fn operation(&self) -> Option<Uuid> {
        match self {
            Self::Query(_) => None,
            Self::Mutation { operation, .. } => Some(*operation),
        }
    }
    fn closed(&self) -> bool {
        match self {
            Self::Query(reply) => reply.is_closed(),
            Self::Mutation { reply, .. } => reply.is_closed(),
        }
    }
    fn reject(self, error: String) {
        match self {
            Self::Query(reply) => {
                let _ = reply.send(Err(error));
            }
            Self::Mutation { reply, .. } => {
                let _ = reply.send(Err(error));
            }
        }
    }
    fn accept(self, id: Uuid, reference: Option<RecoveryRef>) -> Reply {
        match self {
            Self::Query(reply) => {
                let (output, result) = oneshot::channel();
                let _ = reply.send(Ok(Accepted { id, result }));
                Reply::Query(output)
            }
            Self::Mutation { reply, .. } => {
                let reference = reference.expect("admitted mutation reference");
                let (output, result) = oneshot::channel();
                let _ = reply.send(Ok(Mutation {
                    id,
                    reference: reference.clone(),
                    result,
                }));
                Reply::Mutation {
                    reference: Box::new(reference),
                    reply: output,
                }
            }
        }
    }
}
pub(super) enum Reply {
    Query(oneshot::Sender<QueryCompletion>),
    Mutation {
        reference: Box<RecoveryRef>,
        reply: oneshot::Sender<OperationReply>,
    },
}
impl Reply {
    fn send(
        self,
        result: Result<Value>,
        cleanup: Result<()>,
        receipt: Option<Result<OperationObservation>>,
    ) {
        match self {
            Self::Query(reply) => {
                let _ = reply.send(QueryCompletion { result, cleanup });
            }
            Self::Mutation { reference, reply } => {
                let receipt = match receipt.expect("mutation completion carries receipt knowledge")
                {
                    Ok(observation) => {
                        assert_eq!(*reference, observation.reference);
                        Receipt::Recorded {
                            outcome: observation.outcome,
                            cleanup: observation.cleanup,
                        }
                    }
                    Err(error) => Receipt::Unavailable {
                        diagnostic: owner::diagnostic(&error),
                    },
                };
                let _ = reply.send(OperationReply {
                    reference: *reference,
                    receipt,
                });
            }
        }
    }
}
pub(super) enum Control {
    Retire {
        activation: Activation,
        reply: oneshot::Sender<Result<()>>,
    },
}

impl Application {
    pub fn start(
        input: Input,
        packages: Vec<Package>,
        native: NativeRegistry,
        issuer: Issuer,
        policy: Arc<dyn Policy>,
        limits: Limits,
    ) -> Result<Self> {
        Self::start_owned(input, packages, native, issuer, policy, limits, None)
    }

    /// Loads the original source and exact bytes in one storage snapshot.
    /// The execution owner retains the storage lease through actual cleanup.
    pub async fn start_from_store(
        store: Arc<maka_event_log::EventLog>,
        deployment: Uuid,
        native: NativeRegistry,
        issuer: Issuer,
        policy: Arc<dyn Policy>,
        limits: Limits,
    ) -> Result<Self> {
        let loaded = store
            .load_application(deployment)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("deployment not found")?;
        // Persisted remote declarations are compatibility observations. This
        // executor has no trusted remote transport and cannot turn them into grants.
        if !loaded.source.remote_exports.is_empty() {
            return Err("remote Host executor is unavailable".into());
        }
        Self::start_owned(
            loaded.source.input,
            loaded.packages,
            native,
            issuer,
            policy,
            limits,
            Some((loaded.source.reference, store)),
        )
    }

    pub fn source(&self) -> Option<&DeploymentRef> {
        self.source.as_ref()
    }

    #[allow(clippy::too_many_arguments)]
    fn start_owned(
        input: Input,
        packages: Vec<Package>,
        native: NativeRegistry,
        issuer: Issuer,
        policy: Arc<dyn Policy>,
        limits: Limits,
        persisted: Option<(DeploymentRef, Arc<maka_event_log::EventLog>)>,
    ) -> Result<Self> {
        if input.environment.location != ExecutionLocation::Host || input.launch.is_some() {
            return Err("Host service deployment required".into());
        }
        let mut declarations = Packages::default();
        let mut sources = BTreeMap::<PackageRef, Package>::new();
        for package in packages {
            let declaration = package
                .manifest()
                .application
                .as_ref()
                .ok_or("application package declaration required")?;
            declarations
                .insert(package.application_ref(), declaration.clone())
                .map_err(|e| e.to_string())?;
            sources.insert(package.application_ref(), package);
        }
        let plan = Plan::prepare(&input, &declarations, &RemoteExports::new())
            .map_err(|e| e.to_string())?;
        for instance in plan.instances().values().filter(|i| !i.disabled) {
            check_json(&instance.configuration)?;
            if let Some(entry) = &instance.entry {
                for export in entry.exports.values() {
                    let Interface::Service { methods } = &export.contract.interface else {
                        return Err("Host service executor requires service exports".into());
                    };
                    if methods.values().any(|m| {
                        m.mode == MethodMode::Subscription
                            || m.mode == MethodMode::Mutation
                                && (persisted.is_none() || !m.resources.is_empty())
                            || m.resources.len() > 16
                    }) {
                        return Err("subscriptions and mutation resource snapshots require their executors; maximum 16 resource slots".into());
                    }
                }
                if let Implementation::Native { implementation } = &entry.implementation
                    && !native.0.contains_key(implementation)
                {
                    return Err(format!(
                        "native implementation unavailable: {implementation}"
                    ));
                }
            }
        }
        let scheduler = tokio::runtime::Handle::try_current().map_err(|e| e.to_string())?;
        let (commands, receiver) = mpsc::channel(128);
        let (control, controls) = mpsc::channel(16);
        let (updates, observations) = watch::channel(Snapshot {
            starting: true,
            ..Default::default()
        });
        let stop = CancellationToken::new();
        let dispatch = Arc::new(Dispatch {
            commands,
            slots: Arc::new(Semaphore::new(128)),
            bytes: Arc::new(Semaphore::new(32 * 1024 * 1024)),
            resources: Arc::new(Semaphore::new(64 * 1024 * 1024)),
        });
        let client = Client {
            dispatch: dispatch.clone(),
            control,
            updates: observations,
        };
        let source = persisted.as_ref().map(|(source, _)| source.clone());
        scheduler.spawn(
            owner::Owner::new(
                plan,
                input,
                sources,
                native,
                issuer,
                policy,
                limits,
                updates,
                Arc::downgrade(&dispatch),
                persisted,
            )
            .run(receiver, controls, stop.clone()),
        );
        Ok(Self {
            client,
            stop,
            source,
        })
    }
    pub fn client(&self) -> Client {
        self.client.clone()
    }
    pub async fn ready(&self) -> Result<()> {
        self.client.ready().await
    }
    pub async fn shutdown(&self) -> Result<()> {
        self.stop.cancel();
        let mut updates = self.client.updates.clone();
        loop {
            let snapshot = updates.borrow().clone();
            if let Some(error) = snapshot.cleanup_error {
                return Err(error);
            }
            if snapshot.closed {
                return Ok(());
            }
            updates
                .changed()
                .await
                .map_err(|_| "Host owner disappeared before cleanup".to_string())?;
        }
    }
}
impl Client {
    pub fn snapshot(&self) -> Snapshot {
        self.updates.borrow().clone()
    }
    pub async fn ready(&self) -> Result<()> {
        let mut updates = self.updates.clone();
        loop {
            let snapshot = updates.borrow().clone();
            if let Some(error) = snapshot.startup_error {
                return Err(error);
            }
            if snapshot.closing {
                return Err("Host deployment is closing".into());
            }
            if !snapshot.starting {
                return Ok(());
            }
            updates
                .changed()
                .await
                .map_err(|_| "Host owner disappeared during startup".to_string())?;
        }
    }
    pub async fn bind(&self, consumer: Activation, import: String) -> Result<Vec<Binding>> {
        let (reply, result) = oneshot::channel();
        self.dispatch
            .commands
            .try_send(Command::Bind {
                consumer,
                import,
                reply,
            })
            .map_err(|e| e.to_string())?;
        result
            .await
            .map_err(|_| "Host deployment is closed".to_string())?
    }
    pub async fn start(
        &self,
        binding: Binding,
        method: String,
        input: Value,
        scope: Scope,
        attachments: Attachments,
    ) -> Result<Accepted> {
        let (reply, result) = oneshot::channel();
        self.dispatch.enqueue(
            binding,
            method,
            input,
            scope,
            attachments,
            None,
            CallKind::Query(reply),
        )?;
        result
            .await
            .map_err(|_| "Host deployment is closed".to_string())?
    }
    pub async fn mutate(
        &self,
        binding: Binding,
        method: String,
        input: Value,
        scope: Scope,
        operation: Uuid,
    ) -> Result<Mutation> {
        let (reply, result) = oneshot::channel();
        self.dispatch.enqueue(
            binding,
            method,
            input,
            scope,
            Attachments::new(),
            None,
            CallKind::Mutation { operation, reply },
        )?;
        result
            .await
            .map_err(|_| "Host deployment is closed".to_string())?
    }
    /// A fixed-root observation; no current provider is resolved or invoked.
    pub async fn recover(
        &self,
        consumer: Activation,
        reference: RecoveryRef,
        scope: Scope,
    ) -> Result<Option<OperationObservation>> {
        self.dispatch
            .recover(consumer, reference, scope, None)?
            .await
            .map_err(|_| "Host deployment is closed".to_string())?
    }
    pub async fn retire(&self, activation: Activation) -> Result<()> {
        let (reply, result) = oneshot::channel();
        self.control
            .try_send(Control::Retire { activation, reply })
            .map_err(|e| e.to_string())?;
        result
            .await
            .map_err(|_| "Host deployment is closed".to_string())?
    }
    pub fn activation(&self, id: &InstanceId) -> Result<Activation> {
        self.updates
            .borrow()
            .instances
            .iter()
            .find(|s| s.activation.instance() == id)
            .map(|s| s.activation.clone())
            .ok_or_else(|| "activation unavailable".into())
    }
}

fn check_attachments(attachments: &Attachments) -> Result<usize> {
    if attachments.len() > 16 {
        return Err("maximum 16 resource slots".into());
    }
    let mut bytes = 0usize;
    for (slot, resource) in attachments {
        if slot.is_empty() || slot.len() > 128 {
            return Err("invalid resource slot".into());
        }
        check_json(&resource.metadata())?;
        bytes = bytes
            .checked_add(resource.bytes())
            .filter(|&n| n <= 64 * 1024 * 1024)
            .ok_or("Host attachments exceed 64 MiB")?;
    }
    Ok(bytes)
}

fn check_json(value: &Value) -> Result<usize> {
    check_json_limit(value, 1024 * 1024)
}

// Leave room for receipt status and bounded diagnostics in the 1 MiB JS envelope.
fn check_reference(reference: &RecoveryRef) -> Result<usize> {
    maka_js_runtime::plugin::check_json_delivery(
        &serde_json::to_value(reference).map_err(|e| e.to_string())?,
        512 * 1024,
    )
    .map_err(|e| e.to_string())
}

fn check_json_limit(value: &Value, limit: usize) -> Result<usize> {
    struct Count(usize, usize);
    impl Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            if self.0 > self.1 {
                return Err(std::io::Error::other("service JSON exceeds byte budget"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0, limit);
    serde_json::to_writer(&mut count, value).map_err(|e| e.to_string())?;
    Ok(count.0)
}

impl Dispatch {
    // Transfer each actual work owner explicitly; none is encoded as a grant.
    #[allow(clippy::too_many_arguments)]
    fn enqueue(
        &self,
        binding: Binding,
        method: String,
        input: Value,
        scope: Scope,
        attachments: Attachments,
        mut parent: Option<Ticket>,
        kind: CallKind,
    ) -> Result<()> {
        let size = check_json(&input)?;
        let resource_bytes = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            check_attachments(&attachments)
        })) {
            Ok(result) => result?,
            Err(_) => {
                let error = "Host attachment descriptor panicked".to_string();
                if let Some(ticket) = parent.take() {
                    ticket.complete(Err(error.clone()));
                }
                return Err(error);
            }
        };
        if method.is_empty() || method.len() > 128 {
            return Err("invalid service method".into());
        }
        let slot = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| "Host call capacity exhausted")?;
        let bytes = self
            .bytes
            .clone()
            .try_acquire_many_owned(size.max(1) as u32)
            .map_err(|_| "Host input budget exhausted")?;
        let resources = self
            .resources
            .clone()
            .try_acquire_many_owned(resource_bytes as u32)
            .map_err(|_| "Host attachment budget exhausted")?;
        self.commands
            .try_send(Command::Call(Box::new(Call {
                parent,
                kind,
                binding,
                method,
                input,
                scope,
                attachments,
                slot,
                bytes,
                resources,
            })))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn recover(
        &self,
        consumer: Activation,
        reference: RecoveryRef,
        scope: Scope,
        parent: Option<Ticket>,
    ) -> Result<oneshot::Receiver<Result<Option<OperationObservation>>>> {
        let size = check_reference(&reference)?;
        let slot = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| "Host recovery capacity exhausted")?;
        let bytes = self
            .bytes
            .clone()
            .try_acquire_many_owned(size.max(1) as u32)
            .map_err(|_| "Host recovery budget exhausted")?;
        let (reply, result) = oneshot::channel();
        self.commands
            .try_send(Command::Recover {
                consumer,
                reference: Box::new(reference),
                scope,
                parent,
                reply,
                _slot: slot,
                _bytes: bytes,
            })
            .map_err(|e| e.to_string())?;
        Ok(result)
    }
}
