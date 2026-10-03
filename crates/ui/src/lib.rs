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

//! Local component execution. Renderers only read committed frames; JavaScript
//! executes on initialization or input, never during layout/paint.
pub mod abi;
pub mod actions;
pub mod appearance;
mod frame;
mod native;
pub mod operations;
pub mod text;
pub mod textarea;
mod tree;
mod values;
pub use frame::{ComponentOutput, ComponentSlot, Emission, Frame, InputDecision, Surface};
pub use native::{
    ComponentCall, EventOrigin, LabelProps, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, label_contract,
};

use maka_js_runtime::plugin::{Foreground, ForegroundCallbacks, ForegroundModule, Limits};
use maka_plugin_core::{
    composition::{ExecutionLocation, Input, InstanceId, PresentationTarget},
    contract::{Contract, Interface, Transport},
    package::{Implementation, PackageRef, Packages, Realm},
    plan::{Plan, Provider},
    runtime::{Activation, Mount, Phase, Published, Resource, Runtime},
};
use maka_plugins::package::Package;
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] maka_plugin_core::Error),
    #[error(transparent)]
    Script(#[from] maka_js_runtime::plugin::Error),
    #[error("invalid UI activation or batch: {0}")]
    Invalid(String),
}
pub type Result<T> = std::result::Result<T, Error>;
type ExecutionRealm = Rc<RefCell<Foreground>>;

enum Component<V: ?Sized> {
    JavaScript {
        realm: ExecutionRealm,
        callbacks: ForegroundCallbacks,
    },
    Native(Rc<dyn NativeComponent<V>>),
}
struct CaptureDefinition {
    input: jsonschema::Validator,
    resource: maka_plugin_core::contract::ResourceType,
}
struct ServiceMethod {
    mode: maka_plugin_core::contract::MethodMode,
    input: jsonschema::Validator,
    output: jsonschema::Validator,
    resources: BTreeMap<String, maka_plugin_core::contract::ResourceType>,
}
enum Export<V: ?Sized> {
    Component {
        implementation: Component<V>,
        props: jsonschema::Validator,
        events: BTreeMap<String, jsonschema::Validator>,
        captures: BTreeMap<String, CaptureDefinition>,
    },
    Surface(PresentationTarget),
    Value(Value),
    Service {
        implementation: std::sync::Arc<dyn actions::NativeService>,
        methods: BTreeMap<String, ServiceMethod>,
    },
}
struct Loaded {
    realm: ExecutionRealm,
    module: ForegroundModule,
    resource: Resource,
}

pub struct Application<V: ?Sized = ()> {
    runtime: Runtime<Export<V>>,
    activations: BTreeMap<InstanceId, Activation>,
    modules: BTreeMap<InstanceId, Loaded>,
    values: BTreeMap<Uuid, values::Imports<V>>,
    value_bytes: BTreeMap<Uuid, usize>,
    views: BTreeMap<Uuid, tree::Mounted<V>>,
    actions: actions::Actions<V>,
    slice: std::time::Duration,
    pub(crate) scheduler: tokio::runtime::Handle,
    target: PresentationTarget,
    scene_compiler: Option<Rc<native::SceneCompiler<V>>>,
    source: Option<maka_plugin_core::deployment::DeploymentRef>,
    store: Option<std::sync::Arc<maka_event_log::EventLog>>,
}

impl<V: ?Sized + 'static> Application<V> {
    /// Unsaved, query-only preview. Production executions load their owned source
    /// through `start_from_store`; this constructor has no DeploymentRef.
    pub fn new(
        input: &Input,
        bytes: Vec<Package>,
        timer: tokio::runtime::Handle,
        limits: Limits,
        native: NativeRegistry<V>,
        remotes: actions::RemoteServices,
    ) -> Result<Self> {
        Self::new_owned(input, bytes, timer, limits, native, remotes, None)
    }

    /// Loads the owned original source and exact bytes before executing any code.
    /// Static remote observations come only from that source; endpoints remain
    /// separate trusted registrations. Construct on the presentation owner thread.
    pub async fn start_from_store(
        store: std::sync::Arc<maka_event_log::EventLog>,
        source: maka_plugin_core::deployment::DeploymentRef,
        target: PresentationTarget,
        timer: tokio::runtime::Handle,
        limits: Limits,
        mut native: NativeRegistry<V>,
        registrations: Vec<actions::RemoteService>,
    ) -> Result<Self> {
        let loaded = store
            .load_application_at(&source)
            .await
            .map_err(|e| Error::Invalid(e.to_string()))?
            .ok_or_else(|| Error::Invalid("UI deployment not found".into()))?;
        if loaded.source.input.environment.location != ExecutionLocation::Ui
            || loaded.source.input.environment.presentation != Some(target)
        {
            return Err(Error::Invalid("stored presentation target mismatch".into()));
        }
        operations::provide(&mut native, loaded.source.reference.clone(), store.clone())?;
        let remotes = actions::RemoteServices {
            declarations: loaded
                .source
                .remote_exports
                .into_iter()
                .map(|r| (r.capability, r.declaration))
                .collect(),
            registrations,
        };
        Self::new_owned(
            &loaded.source.input,
            loaded.packages,
            timer,
            limits,
            native,
            remotes,
            Some((loaded.source.reference, store)),
        )
    }

    /// The immutable source this execution actually loaded, not the latest desire.
    pub fn source(&self) -> Option<&maka_plugin_core::deployment::DeploymentRef> {
        self.source.as_ref()
    }

    fn new_owned(
        input: &Input,
        bytes: Vec<Package>,
        timer: tokio::runtime::Handle,
        limits: Limits,
        native: NativeRegistry<V>,
        remotes: actions::RemoteServices,
        persisted: Option<(
            maka_plugin_core::deployment::DeploymentRef,
            std::sync::Arc<maka_event_log::EventLog>,
        )>,
    ) -> Result<Self> {
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
        let plan = Plan::prepare(input, &packages, &remotes.declarations)?;
        let mut pending: Vec<_> = plan
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
        let (source, store) = match persisted {
            Some((source, store)) => (Some(source), Some(store)),
            None => (None, None),
        };
        let mut app = Self {
            runtime: Runtime::new(plan),
            activations: BTreeMap::new(),
            modules: BTreeMap::new(),
            values: BTreeMap::new(),
            value_bytes: BTreeMap::new(),
            views: BTreeMap::new(),
            actions: actions::Actions::default(),
            slice: limits.synchronous_slice,
            target,
            scene_compiler: native.scene_compiler.clone(),
            scheduler: timer.clone(),
            source,
            store,
        };
        for remote in remotes.registrations {
            let value = service_export(remote.implementation, &remote.contract)?;
            app.runtime
                .attach_remote(
                    remote.target,
                    remote.registration_epoch,
                    Published {
                        contract: remote.contract,
                        value,
                    },
                )
                .map_err(|rejected| Error::Core(rejected.error))?;
        }
        let mut shared: Option<ExecutionRealm> = None;
        while !pending.is_empty() {
            let before = pending.len();
            for instance in std::mem::take(&mut pending) {
                if instance
                    .parent
                    .as_ref()
                    .is_some_and(|id| !app.activations.contains_key(id))
                    || (instance.entry.is_some()
                        && instance.bindings.values().flatten().any(|binding| {
                            match &binding.provider {
                                Provider::Local { instance, .. } => {
                                    !app.activations.contains_key(instance)
                                }
                                Provider::Remote { .. } => false,
                            }
                        }))
                {
                    pending.push(instance);
                    continue;
                }
                let activation = app.runtime.begin(&instance.identity)?;
                if let Some(entry) = &instance.entry {
                    app.capture_values(&activation, &entry.imports)?;
                }
                let mut batch = BTreeMap::new();
                if let Some(entry) = &instance.entry {
                    match &entry.implementation {
                        Implementation::Native { implementation } => {
                            let values =
                                app.values_snapshot(&activation, &instance.configuration)?;
                            for (name, actual) in
                                native.activate(implementation, &instance.configuration, &values)?
                            {
                                let value = match actual.value {
                                    NativeCapability::Surface(target) => Export::Surface(target),
                                    NativeCapability::Value(value) => {
                                        values::export(value, &actual.contract)?
                                    }
                                    NativeCapability::Service(implementation) => {
                                        service_export(implementation, &actual.contract)?
                                    }
                                    NativeCapability::Component(component) => {
                                        let Interface::Component {
                                            props,
                                            events,
                                            captures,
                                        } = &actual.contract.interface
                                        else {
                                            unreachable!()
                                        };
                                        Export::Component {
                                            implementation: Component::Native(component),
                                            props: jsonschema::validator_for(props).map_err(
                                                |error| Error::Invalid(error.to_string()),
                                            )?,
                                            events: event_schemas(events)?,
                                            captures: capture_schemas(captures)?,
                                        }
                                    }
                                };
                                batch.insert(
                                    name,
                                    Published {
                                        contract: actual.contract,
                                        value,
                                    },
                                );
                            }
                        }
                        Implementation::JavaScript { module, realm } => {
                            let realm = match realm {
                                Realm::Shared => shared.clone(),
                                Realm::Dedicated => None,
                            };
                            let realm = match realm {
                                Some(realm) => realm,
                                None => Rc::new(RefCell::new(Foreground::new(
                                    limits.clone(),
                                    timer.clone(),
                                )?)),
                            };
                            if matches!(
                                entry.implementation,
                                Implementation::JavaScript {
                                    realm: Realm::Shared,
                                    ..
                                }
                            ) {
                                shared = Some(realm.clone());
                            }
                            let package = &sources[&instance.package];
                            let source = std::str::from_utf8(
                                package
                                    .file(module)
                                    .ok_or_else(|| Error::Invalid("entry bytes missing".into()))?,
                            )
                            .map_err(|error| Error::Invalid(error.to_string()))?;
                            let loaded = realm.borrow_mut().load(module, source)?;
                            let values =
                                app.values_snapshot(&activation, &instance.configuration)?;
                            let args = vec![
                                instance.configuration.clone(),
                                serde_json::to_value(values)
                                    .map_err(|e| Error::Invalid(e.to_string()))?,
                            ];
                            maka_js_runtime::plugin::check_json_delivery(
                                &Value::Array(args.clone()),
                                1024 * 1024,
                            )?;
                            let value =
                                realm
                                    .borrow_mut()
                                    .call(&loaded, &["activate".into()], &args)?;
                            tree::check_call_bytes(&value)?;
                            let actual: BTreeMap<String, abi::UiExport> =
                                serde_json::from_value(value)
                                    .map_err(|error| Error::Invalid(error.to_string()))?;
                            for (name, actual) in actual {
                                let (contract, component) = match actual {
                                    abi::UiExport::Value { contract, value } => {
                                        let value = values::export(value, &contract)?;
                                        batch.insert(name, Published { contract, value });
                                        continue;
                                    }
                                    abi::UiExport::Component {
                                        contract,
                                        component,
                                    } => (contract, component),
                                };
                                if !matches!(contract.interface, Interface::Component { .. })
                                    || component.is_empty()
                                {
                                    return Err(Error::Invalid(
                                        "local JS export must declare a component".into(),
                                    ));
                                }
                                let Interface::Component {
                                    props,
                                    events,
                                    captures,
                                } = &contract.interface
                                else {
                                    unreachable!()
                                };
                                let props = jsonschema::validator_for(props)
                                    .map_err(|error| Error::Invalid(error.to_string()))?;
                                let events = event_schemas(events)?;
                                let callback_methods = if captures.is_empty() {
                                    vec!["mount", "event", "update"]
                                } else {
                                    vec!["mount", "event", "update", "capture"]
                                };
                                let compiled_captures = capture_schemas(captures)?;
                                batch.insert(
                                    name,
                                    Published {
                                        contract,
                                        value: Export::Component {
                                            props,
                                            events,
                                            captures: compiled_captures,
                                            implementation: Component::JavaScript {
                                                realm: realm.clone(),
                                                callbacks: realm.borrow_mut().capture(
                                                    &loaded,
                                                    &component,
                                                    &callback_methods,
                                                )?,
                                            },
                                        },
                                    },
                                );
                            }
                            let resource =
                                app.runtime.own_resource(&activation, "foreground module")?;
                            app.modules.insert(
                                instance.identity.clone(),
                                Loaded {
                                    realm,
                                    module: loaded,
                                    resource,
                                },
                            );
                        }
                    }
                }
                app.charge_values(&activation, &batch)?;
                app.runtime
                    .publish(&activation, batch)
                    .map_err(|rejected| Error::Core(rejected.error))?;
                app.activations.insert(instance.identity, activation);
            }
            if pending.len() == before {
                return Err(Error::Invalid("UI dependencies are not ready".into()));
            }
        }
        Ok(app)
    }

    fn invoke(
        &mut self,
        mount: &Mount,
        method: &str,
        args: &[Value],
        deadline: std::time::Instant,
    ) -> Result<ComponentOutput<NativeView<V>>> {
        let input = if method == "event" {
            Some(
                serde_json::from_value::<abi::ComponentInput>(args[1].clone())
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            )
        } else {
            None
        };
        let values = self.mount_values(mount, &args)?;
        let execution = self.runtime.admit_mount(mount)?;
        let Export::Component {
            implementation,
            events,
            ..
        } = execution.value()
        else {
            unreachable!()
        };
        let mut failed_realm = None;
        let result = match implementation {
            Component::JavaScript { realm, callbacks } => {
                let mut args = args.to_vec();
                args.push(
                    serde_json::to_value(&values).map_err(|e| Error::Invalid(e.to_string()))?,
                );
                maka_js_runtime::plugin::check_json_delivery(
                    &Value::Array(args.to_vec()),
                    1024 * 1024,
                )
                .map_err(Error::from)
                .and_then(|_| {
                    let result = realm
                        .borrow_mut()
                        .call_bound_until(callbacks, method, &args, deadline)
                        .and_then(|value| {
                            serde_json::from_value::<ComponentOutput>(value)
                                .map_err(|error| maka_js_runtime::plugin::Error(error.to_string()))
                        });
                    if result.is_err() {
                        failed_realm = Some(realm.clone());
                    }
                    result.map_err(Error::from).and_then(|output| {
                        // Charge the complete guest result before any translation.
                        tree::check_call_bytes(&output)?;
                        if std::time::Instant::now() >= deadline {
                            return Err(Error::Invalid("UI scene compilation deadline".into()));
                        }
                        let view =
                            (self
                                .scene_compiler
                                .as_ref()
                                .expect("preflight compiler")
                                .compile)(&output.frame.view, deadline)?;
                        if std::time::Instant::now() >= deadline {
                            return Err(Error::Invalid("UI scene compilation deadline".into()));
                        }
                        // The kernel, not the compiler, retains untrusted source
                        // for callback, transaction and mounted-frame accounting.
                        let view = view.with_declaration(output.frame.view.clone())?;
                        Ok(output.map_view(|_| view))
                    })
                })
            }
            Component::Native(component) => {
                let call = match method {
                    "mount" => ComponentCall::Mount { props: &args[0] },
                    "update" => ComponentCall::Update {
                        state: &args[0],
                        props: &args[1],
                    },
                    "event" => {
                        let (key, event, origin) = match input.as_ref().expect("event input") {
                            abi::ComponentInput::Raw { key, event } => {
                                (key, event, EventOrigin::Raw)
                            }
                            abi::ComponentInput::Action { key, event } => {
                                (key, event, EventOrigin::Action)
                            }
                            abi::ComponentInput::Component {
                                key,
                                child,
                                name,
                                event,
                            } => (key, event, EventOrigin::Component { child, name }),
                        };
                        ComponentCall::Event {
                            state: &args[0],
                            props: &args[2],
                            key,
                            input: event,
                            origin,
                        }
                    }
                    _ => unreachable!(),
                };
                component.call(call, &values)
            }
        };
        let result = result.and_then(|output| {
            if output.decision.is_some() && !matches!(input, Some(abi::ComponentInput::Raw { .. }))
            {
                return Err(Error::Invalid(
                    "input decision requires an original raw callback".into(),
                ));
            }
            // All declarative data is charged, including the original guest
            // scene. Only trusted opaque Rust recipe memory is excluded.
            let metadata = (
                output.frame.revision,
                &output.frame.state,
                &output.frame.surface,
                &output.events,
                &output.actions,
                &output.subscriptions,
                &output.decision,
            );
            tree::check_call_bytes(&(metadata, output.frame.view.metadata()))?;
            if output.events.len() > 64 {
                return Err(Error::Invalid("component emission limit".into()));
            }
            tree::check_call_bytes(&output.events)?;
            if output.actions.len() > 16 || output.subscriptions.len() > 16 {
                return Err(Error::Invalid("component action limit".into()));
            }
            tree::check_call_bytes(&output.actions)?;
            for event in &output.events {
                events
                    .get(&event.name)
                    .ok_or_else(|| Error::Invalid("undeclared component emission".into()))?
                    .validate(&event.value)
                    .map_err(|error| Error::Invalid(error.to_string()))?;
            }
            output.frame.validate_metadata().map_err(Error::Invalid)?;
            Ok(output)
        });
        self.runtime.settle(&execution, Ok(()))?; // Only declarative scene preparation.
        if let Some(realm) = failed_realm {
            for (id, loaded) in &self.modules {
                if Rc::ptr_eq(&loaded.realm, &realm) {
                    self.runtime.retire(&self.activations[id])?;
                }
            }
        }
        self.sync_subscription_demand();
        result
    }

    fn validate_surface(&mut self, mount: &Mount, frame: &Frame<NativeView<V>>) -> Result<()> {
        let provider = self
            .runtime
            .mount_statuses()
            .into_iter()
            .find(|status| status.mount == *mount)
            .ok_or(maka_plugin_core::Error::Retired)?
            .provider;
        let surface = frame
            .surface
            .as_ref()
            .ok_or_else(|| Error::Invalid("launch component must request a surface".into()))?;
        if surface.target() != self.target {
            return Err(Error::Invalid("surface variant mismatch".into()));
        }
        let handles = self.runtime.bind(&provider, surface.binding())?;
        if handles.len() != 1 {
            return Err(Error::Invalid(
                "one explicit surface binding required".into(),
            ));
        }
        let valid = matches!(self.runtime.inspect(&handles[0])?, Export::Surface(target) if *target == self.target);
        if !valid {
            return Err(Error::Invalid("surface target mismatch".into()));
        }
        Ok(())
    }

    pub fn can_shutdown(&self) -> bool {
        self.views.is_empty() && self.actions.accepted_count() == 0
    }

    pub fn scheduler(&self) -> &tokio::runtime::Handle {
        &self.scheduler
    }

    pub fn frame(&self, mount: &Mount) -> Result<&Frame<NativeView<V>>> {
        self.views
            .get(&mount.id())
            .map(|view| &view.frame)
            .ok_or_else(|| Error::Invalid("mount has no committed frame".into()))
    }
    pub fn close(
        &mut self,
        mount: &Mount,
        native_cleanup: std::result::Result<(), String>,
    ) -> Result<()> {
        self.runtime.retire_mount(mount)?;
        self.sync_subscription_demand();
        self.runtime.complete_mount(mount, native_cleanup)?;
        self.views.remove(&mount.id());
        Ok(())
    }
    /// Stops callbacks without asserting that the renderer/platform has closed
    /// any native resources. `close` requires a separate real acknowledgement.
    pub fn retire(&mut self, mount: &Mount) -> Result<()> {
        self.runtime.retire_mount(mount)?;
        self.sync_subscription_demand();
        Ok(())
    }
    /// Capture the exact attachment when installing a transport disconnect callback.
    /// The target identifies a source reference; only this token identifies its lifetime.
    pub fn remote_activation(
        &self,
        target: &maka_plugin_core::plan::RemoteCapability,
    ) -> Result<Activation> {
        self.runtime
            .activations()
            .find(|a| a.remote_target() == Some(target))
            .cloned()
            .ok_or(maka_plugin_core::Error::Retired.into())
    }
    /// Late notifications cannot retire a replacement attachment or another kernel.
    /// Accepted calls stay with the original attachment until actual cleanup.
    pub fn retire_remote(&mut self, attachment: &Activation) -> Result<()> {
        if attachment.remote_target().is_none() {
            return Err(Error::Invalid("remote attachment required".into()));
        }
        self.runtime.retire(attachment)?;
        self.sync_subscription_demand();
        Ok(())
    }

    pub fn is_retiring(&self, mount: &Mount) -> bool {
        self.runtime
            .mount_statuses()
            .iter()
            .find(|status| status.mount == *mount)
            .is_none_or(|status| status.phase == Phase::Retiring)
    }

    /// Stop admission and release module registrations. Native adapters must
    /// first confirm each mount's actual close through `close`; pending native
    /// cleanup remains an error rather than an implicit shutdown acknowledgement.
    pub fn shutdown(&mut self) -> Result<()> {
        for activation in self.runtime.activations().cloned().collect::<Vec<_>>() {
            self.runtime.retire(&activation)?;
        }
        self.sync_subscription_demand();
        self.actions.queue.clear();
        self.actions.outcomes.clear();
        for id in self.modules.keys().cloned().collect::<Vec<_>>() {
            let loaded = &self.modules[&id];
            let result = loaded.realm.borrow_mut().close(&loaded.module);
            match result {
                Ok(()) => {
                    self.runtime.complete_resource(&loaded.resource, Ok(()))?;
                    self.modules.remove(&id);
                }
                Err(error) => {
                    self.runtime
                        .complete_resource(&loaded.resource, Err(error.to_string()))?;
                    return Err(error.into());
                }
            }
        }
        // Children must finish before their parent. The checked plan has no
        // ownership cycles; a fixed point with owners left is genuine pending
        // work/resources/mounts, not a reason to discard them.
        loop {
            let pending = self.runtime.activations().cloned().collect::<Vec<_>>();
            let before = pending.len();
            for activation in pending {
                match self.runtime.finish_cleanup(&activation) {
                    Ok(()) => {
                        if activation.remote_target().is_none() {
                            self.activations.remove(activation.instance());
                            self.values.remove(&activation.epoch());
                            self.value_bytes.remove(&activation.epoch());
                        }
                    }
                    Err(maka_plugin_core::Error::CleanupPending) => {}
                    Err(error) => return Err(error.into()),
                }
            }
            let remaining = self.runtime.activations().count();
            if remaining == 0 {
                // EventLog's independent owner confirms its actual SQL close.
                // This only releases the UI execution's custody after cleanup.
                self.store.take();
                return Ok(());
            }
            if remaining == before {
                return Err(maka_plugin_core::Error::CleanupPending.into());
            }
        }
    }
}

fn service_export<V: ?Sized>(
    implementation: std::sync::Arc<dyn actions::NativeService>,
    contract: &Contract,
) -> Result<Export<V>> {
    let Interface::Service { methods } = &contract.interface else {
        return Err(Error::Invalid("service contract required".into()));
    };
    let mut compiled = BTreeMap::new();
    for (name, method) in methods {
        if method.mode == maka_plugin_core::contract::MethodMode::Mutation
            && !method.resources.is_empty()
        {
            return Err(Error::Invalid(
                "UI resource mutation requires its own executor".into(),
            ));
        }
        compiled.insert(
            name.clone(),
            ServiceMethod {
                mode: method.mode,
                input: jsonschema::validator_for(&method.input)
                    .map_err(|e| Error::Invalid(e.to_string()))?,
                output: jsonschema::validator_for(&method.output)
                    .map_err(|e| Error::Invalid(e.to_string()))?,
                resources: method.resources.clone(),
            },
        );
    }
    Ok(Export::Service {
        implementation,
        methods: compiled,
    })
}

fn event_schemas(
    events: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, jsonschema::Validator>> {
    events
        .iter()
        .map(|(name, schema)| {
            jsonschema::validator_for(schema)
                .map(|schema| (name.clone(), schema))
                .map_err(|error| Error::Invalid(error.to_string()))
        })
        .collect()
}

pub fn surface_contract(target: PresentationTarget) -> Contract {
    let resource = match target {
        PresentationTarget::Desktop => "surface.desktop",
        PresentationTarget::Tui => "surface.tui",
    };
    Contract {
        name: format!("maka.ui.{resource}"),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Resource {
            resource: resource.into(),
            schema: json!({"type":"object","additionalProperties":false}),
        },
    }
}

fn capture_schemas(
    captures: &BTreeMap<String, maka_plugin_core::contract::Capture>,
) -> Result<BTreeMap<String, CaptureDefinition>> {
    captures
        .iter()
        .map(|(name, c)| {
            Ok((
                name.clone(),
                CaptureDefinition {
                    input: jsonschema::validator_for(&c.input)
                        .map_err(|e| Error::Invalid(e.to_string()))?,
                    resource: c.resource.clone(),
                },
            ))
        })
        .collect()
}
