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
pub mod budget;
pub mod checkpoint;
mod deployment;
pub mod diff;
pub mod dismiss;
pub mod editor;
pub mod elapsed;
pub mod file;
pub mod focus;
mod frame;
pub use deployment::{CommittedUpdate, DeploymentStatus, PreparedUpdate};
pub mod image;
pub mod input;
pub mod markdown;
pub mod morph_mark;
mod native;
pub mod operations;
pub mod reading;
pub mod rows;
pub mod scene;
pub mod selection_menu;
pub mod shortcut;
pub mod source;
pub mod terminal;
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
// Native input causality is never guest JSON or a retained frame property.
struct Invocation<V: ?Sized> {
    output: ComponentOutput<NativeView<V>>,
    input_admission: bool,
}
struct CaptureDefinition {
    input: jsonschema::Validator,
    output: maka_plugin_core::contract::CaptureOutput,
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
struct OriginalSource {
    reference: maka_plugin_core::deployment::DeploymentRef,
    checkpoints: std::sync::Arc<
        tokio::sync::Mutex<std::sync::Weak<maka_event_log::applications::checkpoints::Checkpoints>>,
    >,
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
    render_depth_limit: Option<u16>,
    update_owner: Uuid,
    source: Option<maka_plugin_core::deployment::DeploymentRef>,
    store: Option<std::sync::Arc<maka_event_log::EventLog>>,
    origins: BTreeMap<Uuid, OriginalSource>,
    effective: Option<maka_plugin_core::deployment::DeploymentRef>,
    deployment_error: Option<String>,
    pending: Vec<maka_plugin_core::plan::Instance>,
    sources: BTreeMap<PackageRef, Package>,
    native: Option<NativeRegistry<V>>,
    limits: Limits,
    shared: std::rc::Weak<RefCell<Foreground>>,
    remote_epochs: BTreeMap<maka_plugin_core::plan::RemoteCapability, Uuid>,
    pending_remotes: Vec<actions::RemoteService>,
    closing: bool,
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

    /// The committed source adopted by this executor; status reports convergence.
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
        let deployment::Candidate {
            plan,
            sources,
            instances: pending,
            target,
        } = deployment::prepare(input, bytes, &native, &remotes.declarations)?;
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
            render_depth_limit: native.render_depth_limit,
            scheduler: timer.clone(),
            update_owner: Uuid::new_v4(),
            effective: None,
            source,
            store,
            origins: BTreeMap::new(),
            deployment_error: None,
            pending,
            sources,
            native: Some(native),
            limits,
            shared: std::rc::Weak::new(),
            remote_epochs: BTreeMap::new(),
            pending_remotes: Vec::new(),
            closing: false,
        };
        for remote in remotes.registrations {
            app.remote_epochs
                .insert(remote.target.clone(), remote.registration_epoch);
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
        while !app.pending.is_empty() {
            let before = app.pending.len();
            for instance in std::mem::take(&mut app.pending) {
                if !app.dependencies_ready(&instance) {
                    app.pending.push(instance);
                    continue;
                }
                let identity = instance.identity.clone();
                if let Err(error) = app.activate_instance(instance) {
                    if app.runtime.desired().recovery().is_none() {
                        return Err(error);
                    }
                    if let Some(activation) = app.activations.get(&identity) {
                        app.runtime.retire(activation)?;
                    }
                    app.deployment_error.get_or_insert(error.to_string());
                }
            }
            if app.pending.len() == before {
                if app.runtime.desired().recovery().is_some() && app.deployment_error.is_some() {
                    break;
                }
                return Err(Error::Invalid("UI dependencies are not ready".into()));
            }
        }
        if app.deployment_error.is_none() && !app.deployment_status().converging {
            app.effective = app.source.clone();
        }
        Ok(app)
    }

    fn activate_instance(&mut self, instance: maka_plugin_core::plan::Instance) -> Result<()> {
        let activation = self.runtime.begin(&instance.identity)?;
        self.activations
            .insert(instance.identity.clone(), activation.clone());
        if let Some(source) = &self.source {
            self.origins.insert(
                activation.epoch(),
                OriginalSource {
                    reference: source.clone(),
                    checkpoints: Default::default(),
                },
            );
        }
        if let Some(entry) = &instance.entry {
            self.capture_values(&activation, &entry.imports)?;
        }
        let mut batch = BTreeMap::new();
        if let Some(entry) = &instance.entry {
            match &entry.implementation {
                Implementation::Native { implementation } => {
                    let values = self.values_snapshot(&activation, &instance.configuration)?;
                    for (name, actual) in self.native.as_ref().expect("open UI registry").activate(
                        implementation,
                        &instance.configuration,
                        &values,
                    )? {
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
                                    props: jsonschema::validator_for(props)
                                        .map_err(|error| Error::Invalid(error.to_string()))?,
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
                        Realm::Shared => self
                            .shared
                            .upgrade()
                            .filter(|realm| realm.borrow().failure().is_none()),
                        Realm::Dedicated => None,
                    };
                    let realm = match realm {
                        Some(realm) => realm,
                        None => Rc::new(RefCell::new(Foreground::new(
                            self.limits.clone(),
                            self.scheduler.clone(),
                        )?)),
                    };
                    if matches!(
                        entry.implementation,
                        Implementation::JavaScript {
                            realm: Realm::Shared,
                            ..
                        }
                    ) {
                        self.shared = Rc::downgrade(&realm);
                    }
                    let package = &self.sources[&instance.package];
                    let source = std::str::from_utf8(
                        package
                            .file(module)
                            .ok_or_else(|| Error::Invalid("entry bytes missing".into()))?,
                    )
                    .map_err(|error| Error::Invalid(error.to_string()))?;
                    let loaded = realm.borrow_mut().load(module, source)?;
                    let resource = self
                        .runtime
                        .own_resource(&activation, "foreground module")?;
                    self.modules.insert(
                        instance.identity.clone(),
                        Loaded {
                            realm: realm.clone(),
                            module: loaded,
                            resource,
                        },
                    );
                    let loaded = &self.modules[&instance.identity].module;
                    let values = self.values_snapshot(&activation, &instance.configuration)?;
                    let args = vec![
                        instance.configuration.clone(),
                        serde_json::to_value(values).map_err(|e| Error::Invalid(e.to_string()))?,
                    ];
                    maka_js_runtime::plugin::check_json_delivery(
                        &Value::Array(args.clone()),
                        1024 * 1024,
                    )?;
                    let value = realm
                        .borrow_mut()
                        .call(loaded, &["activate".into()], &args)?;
                    tree::check_call_bytes(&value)?;
                    let actual: BTreeMap<String, abi::UiExport> = serde_json::from_value(value)
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
                                            loaded,
                                            &component,
                                            &callback_methods,
                                        )?,
                                    },
                                },
                            },
                        );
                    }
                }
            }
        }
        self.charge_values(&activation, &batch)?;
        self.runtime
            .publish(&activation, batch)
            .map_err(|rejected| Error::Core(rejected.error))?;
        Ok(())
    }

    fn invoke(
        &mut self,
        mount: &Mount,
        method: &str,
        args: &[Value],
        deadline: std::time::Instant,
        inherited_input: bool,
    ) -> Result<Invocation<V>> {
        let input = if method == "event" {
            Some(
                serde_json::from_value::<abi::ComponentInput>(args[1].clone())
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            )
        } else {
            None
        };
        let input_admission = matches!(input, Some(abi::ComponentInput::Raw { .. }))
            || (inherited_input && matches!(input, Some(abi::ComponentInput::Component { .. })));
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
            if output.decision.is_some() && !input_admission {
                return Err(Error::Invalid(
                    "input decision requires an original synchronous input route".into(),
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
                    if self.runtime.status(&self.activations[id])?.phase != Phase::Retiring {
                        self.deployment_error.get_or_insert_with(|| {
                            result
                                .as_ref()
                                .err()
                                .map(ToString::to_string)
                                .unwrap_or_else(|| "foreground domain failed".into())
                        });
                    }
                    self.runtime.retire(&self.activations[id])?;
                }
            }
        }
        self.sync_subscription_demand();
        result.map(|output| Invocation {
            output,
            input_admission,
        })
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
        self.closing = true;
        self.pending.clear();
        self.pending_remotes.clear();
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
                            self.origins.remove(&activation.epoch());
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
                self.native.take();
                self.sources.clear();
                self.remote_epochs.clear();
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
        if method.resources.len() > 16 {
            return Err(Error::Invalid("maximum 16 resource slots".into()));
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
                    output: c.output.clone(),
                },
            ))
        })
        .collect()
}
