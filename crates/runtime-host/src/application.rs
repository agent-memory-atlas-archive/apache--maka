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
mod owner;
mod script;

use futures_util::future::BoxFuture;
use maka_js_runtime::plugin::Limits;
use maka_plugin_core::{
    composition::{ExecutionLocation, Input, InstanceId},
    contract::{Contract, Interface, MethodMode},
    package::{Implementation, PackageRef, Packages},
    plan::{Plan, RemoteExports},
    runtime::{Activation, Capability, Published, Status},
};
use maka_plugins::{
    call::{Issuer, Scope},
    package::Package,
};
use serde_json::Value;
use std::{collections::BTreeMap, io::Write, sync::Arc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, String>;

/// Runs only after final lifetime and Host policy admission. A query error does
/// not prove failed cleanup; the owner separately settles Scope resources.
pub trait Query: Send + Sync + 'static {
    fn call(&self, method: String, input: Value, scope: Scope)
    -> BoxFuture<'static, Result<Value>>;
}

/// Trusted Host policy, checked in the same owner turn as core admission.
/// Scope observation, package identity and a binding are not authorization.
pub trait Policy: Send + Sync + 'static {
    fn admit(
        &self,
        scope: &Scope,
        consumer: &Activation,
        provider: &Activation,
        contract: &Contract,
        method: &str,
        input: &Value,
    ) -> Result<()>;
}

/// The actual native owner survives rejected exports and failed close futures.
/// Successful close confirms its resources have settled; dropping a close
/// observer or future is not that confirmation.
pub trait NativeOwner: Send + Sync + 'static {
    fn close(&self) -> BoxFuture<'static, Result<()>>;
}
pub struct NativeActivation {
    pub exports: Result<BTreeMap<String, Published<Arc<dyn Query>>>>,
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
    Native(Arc<dyn Query>),
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
                if method.mode != MethodMode::Query || !method.resources.is_empty() {
                    return Err(
                        "mutation, subscription and resource attachments require their executors"
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
    result: oneshot::Receiver<Result<Value>>,
}
impl Accepted {
    /// This is only an observer. Drop never cancels the executor or its cleanup.
    pub async fn result(self) -> Result<Value> {
        self.result
            .await
            .map_err(|_| "Host executor owner disappeared".to_string())?
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
}
impl Drop for Application {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
#[derive(Clone)]
pub struct Client {
    commands: mpsc::Sender<Command>,
    control: mpsc::Sender<Control>,
    updates: watch::Receiver<Snapshot>,
    slots: Arc<Semaphore>,
    bytes: Arc<Semaphore>,
}

pub(super) enum Command {
    Bind {
        consumer: Activation,
        import: String,
        reply: oneshot::Sender<Result<Vec<Binding>>>,
    },
    Call(Box<Call>),
}
pub(super) struct Call {
    binding: Binding,
    method: String,
    input: Value,
    scope: Scope,
    slot: OwnedSemaphorePermit,
    bytes: OwnedSemaphorePermit,
    reply: oneshot::Sender<Result<Accepted>>,
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
                    if methods
                        .values()
                        .any(|m| m.mode != MethodMode::Query || !m.resources.is_empty())
                    {
                        return Err("mutation, subscription and resource attachments require their executors".into());
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
        let client = Client {
            commands,
            control,
            updates: observations,
            slots: Arc::new(Semaphore::new(128)),
            bytes: Arc::new(Semaphore::new(32 * 1024 * 1024)),
        };
        scheduler.spawn(
            owner::Owner::new(
                plan, input, sources, native, issuer, policy, limits, updates,
            )
            .run(receiver, controls, stop.clone()),
        );
        Ok(Self { client, stop })
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
        self.commands
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
    ) -> Result<Accepted> {
        let size = check_json(&input)?;
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
        let (reply, result) = oneshot::channel();
        self.commands
            .try_send(Command::Call(Box::new(Call {
                binding,
                method,
                input,
                scope,
                slot,
                bytes,
                reply,
            })))
            .map_err(|e| e.to_string())?;
        result
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

fn check_json(value: &Value) -> Result<usize> {
    struct Count(usize);
    impl Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            if self.0 > 1024 * 1024 {
                return Err(std::io::Error::other("service JSON exceeds 1 MiB"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    serde_json::to_writer(&mut count, value).map_err(|e| e.to_string())?;
    Ok(count.0)
}
