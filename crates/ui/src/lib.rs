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
mod frame;
mod native;
mod tree;
pub use frame::{Direction, Frame, Node, Window};
pub use native::{
    ComponentCall, LabelProps, NativeCapability, NativeComponent, NativeExport, NativeRegistry,
    NativeView, Presentation, label_contract,
};

use maka_js_runtime::plugin::{Foreground, ForegroundCallbacks, ForegroundModule, Limits};
use maka_plugin_core::{
    composition::{ExecutionLocation, Input, InstanceId, PresentationTarget},
    contract::{Contract, Interface, Transport},
    package::{Implementation, PackageRef, Packages, Realm},
    plan::{Plan, Provider, RemoteExports},
    runtime::{Activation, Mount, Phase, Published, Resource, Runtime},
};
use maka_plugins::package::Package;
use serde::Deserialize;
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
enum Export<V: ?Sized> {
    Component {
        implementation: Component<V>,
        props: jsonschema::Validator,
    },
    Surface(PresentationTarget),
}
struct Loaded {
    realm: ExecutionRealm,
    module: ForegroundModule,
    resource: Resource,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Actual {
    contract: Contract,
    component: Vec<String>,
}

pub struct Application<V: ?Sized = ()> {
    runtime: Runtime<Export<V>>,
    activations: BTreeMap<InstanceId, Activation>,
    modules: BTreeMap<InstanceId, Loaded>,
    views: BTreeMap<Uuid, tree::Mounted<V>>,
    slice: std::time::Duration,
    target: PresentationTarget,
}

impl<V: ?Sized + 'static> Application<V> {
    pub fn new(
        input: &Input,
        bytes: Vec<Package>,
        timer: tokio::runtime::Handle,
        limits: Limits,
        native: NativeRegistry<V>,
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
        let plan = Plan::prepare(input, &packages, &RemoteExports::new())?;
        let mut pending: Vec<_> = plan
            .instances()
            .values()
            .filter(|instance| !instance.disabled)
            .cloned()
            .collect();
        let mut app = Self {
            runtime: Runtime::new(plan),
            activations: BTreeMap::new(),
            modules: BTreeMap::new(),
            views: BTreeMap::new(),
            slice: limits.synchronous_slice,
            target,
        };
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
                                Provider::Remote { .. } => true,
                            }
                        }))
                {
                    pending.push(instance);
                    continue;
                }
                let activation = app.runtime.begin(&instance.identity)?;
                let mut batch = BTreeMap::new();
                if let Some(entry) = &instance.entry {
                    match &entry.implementation {
                        Implementation::Native { implementation } => {
                            for (name, actual) in
                                native.activate(implementation, &instance.configuration)?
                            {
                                let value = match actual.value {
                                    NativeCapability::Surface(target) => Export::Surface(target),
                                    NativeCapability::Component(component) => {
                                        let Interface::Component { props, .. } =
                                            &actual.contract.interface
                                        else {
                                            unreachable!()
                                        };
                                        Export::Component {
                                            implementation: Component::Native(component),
                                            props: jsonschema::validator_for(props).map_err(
                                                |error| Error::Invalid(error.to_string()),
                                            )?,
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
                            let value = realm.borrow_mut().call(
                                &loaded,
                                &["activate".into()],
                                std::slice::from_ref(&instance.configuration),
                            )?;
                            let actual: BTreeMap<String, Actual> = serde_json::from_value(value)
                                .map_err(|error| Error::Invalid(error.to_string()))?;
                            for (name, actual) in actual {
                                if !matches!(actual.contract.interface, Interface::Component { .. })
                                    || actual.component.is_empty()
                                {
                                    return Err(Error::Invalid(
                                        "local JS export must declare a component".into(),
                                    ));
                                }
                                let Interface::Component { props, .. } = &actual.contract.interface
                                else {
                                    unreachable!()
                                };
                                let props = jsonschema::validator_for(props)
                                    .map_err(|error| Error::Invalid(error.to_string()))?;
                                batch.insert(
                                    name,
                                    Published {
                                        contract: actual.contract,
                                        value: Export::Component {
                                            props,
                                            implementation: Component::JavaScript {
                                                realm: realm.clone(),
                                                callbacks: realm.borrow_mut().capture(
                                                    &loaded,
                                                    &actual.component,
                                                    &["mount", "event", "update"],
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
    ) -> Result<Frame<Presentation<V>>> {
        tree::check_call_bytes(&args)?;
        let execution = self.runtime.admit_mount(mount)?;
        let Export::Component { implementation, .. } = execution.value() else {
            unreachable!()
        };
        let mut failed_realm = None;
        let result = match implementation {
            Component::JavaScript { realm, callbacks } => {
                let result = realm
                    .borrow_mut()
                    .call_bound_until(callbacks, method, args, deadline)
                    .and_then(|value| {
                        serde_json::from_value::<Frame>(value)
                            .map_err(|error| maka_js_runtime::plugin::Error(error.to_string()))
                    });
                if result.is_err() {
                    failed_realm = Some(realm.clone());
                }
                result
                    .map(|frame| frame.map_view(Presentation::Tree))
                    .map_err(Error::from)
            }
            Component::Native(component) => {
                let call = match method {
                    "mount" => ComponentCall::Mount { props: &args[0] },
                    "update" => ComponentCall::Update {
                        state: &args[0],
                        props: &args[1],
                    },
                    "event" => ComponentCall::Event {
                        state: &args[0],
                        key: args[1]["key"].as_str().expect("typed UI input key"),
                        input: &args[1]["event"],
                    },
                    _ => unreachable!(),
                };
                component.call(call)
            }
        };
        self.runtime.settle(&execution, Ok(()))?; // Only declarative scene preparation.
        if let Some(realm) = failed_realm {
            for (id, loaded) in &self.modules {
                if Rc::ptr_eq(&loaded.realm, &realm) {
                    self.runtime.retire(&self.activations[id])?;
                }
            }
        }
        let frame = result?;
        frame.validate_metadata().map_err(Error::Invalid)?;
        frame.view.validate().map_err(Error::Invalid)?;
        Ok(frame)
    }

    fn validate_surface(&mut self, mount: &Mount, frame: &Frame<Presentation<V>>) -> Result<()> {
        let provider = self
            .runtime
            .mount_statuses()
            .into_iter()
            .find(|status| status.mount == *mount)
            .ok_or(maka_plugin_core::Error::Retired)?
            .provider;
        let window = frame
            .window
            .as_ref()
            .ok_or_else(|| Error::Invalid("launch component must request a surface".into()))?;
        let handles = self.runtime.bind(&provider, &window.surface)?;
        if handles.len() != 1 {
            return Err(Error::Invalid(
                "one explicit surface binding required".into(),
            ));
        }
        let execution = self.runtime.admit(&handles[0])?;
        let valid = matches!(execution.value(), Export::Surface(target) if *target == self.target);
        self.runtime.settle(&execution, Ok(()))?;
        if !valid {
            return Err(Error::Invalid("surface target mismatch".into()));
        }
        Ok(())
    }

    pub fn frame(&self, mount: &Mount) -> Result<&Frame<Presentation<V>>> {
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
        self.runtime.complete_mount(mount, native_cleanup)?;
        self.views.remove(&mount.id());
        Ok(())
    }
    /// Stops callbacks without asserting that the renderer/platform has closed
    /// any native resources. `close` requires a separate real acknowledgement.
    pub fn retire(&mut self, mount: &Mount) -> Result<()> {
        self.runtime.retire_mount(mount)?;
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
        for activation in self.activations.values() {
            self.runtime.retire(activation)?;
        }
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
            let before = self.activations.len();
            for id in self.activations.keys().cloned().collect::<Vec<_>>() {
                match self.runtime.finish_cleanup(&self.activations[&id]) {
                    Ok(()) => {
                        self.activations.remove(&id);
                    }
                    Err(maka_plugin_core::Error::CleanupPending) => {}
                    Err(error) => return Err(error.into()),
                }
            }
            if self.activations.is_empty() {
                return Ok(());
            }
            if self.activations.len() == before {
                return Err(maka_plugin_core::Error::CleanupPending.into());
            }
        }
    }
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
