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

//! Background JavaScript execution. The original UI owner alone admits input,
//! commits candidate batches and settles native resources and mutation receipts.
use maka_js_runtime::plugin::{Foreground, ForegroundCallbacks, ForegroundModule, Limits};
use maka_ui_runtime::actions::{CaptureCall, CapturePlan};
use maka_ui_runtime::{
    ComponentCall, ComponentOutput, Error, EventOrigin, ExecutionOptions, PrepareScene,
    PreparedScene, Result, abi, backend,
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    time::Instant,
};
use tokio::sync::oneshot;

const QUEUE_ITEMS: usize = 64;
const QUEUE_BYTES: usize = 8 * 1024 * 1024;

pub fn execution(limits: Limits) -> ExecutionOptions {
    ExecutionOptions {
        synchronous_slice: limits.synchronous_slice,
        script: Some(Rc::new(Factory {
            limits,
            worker: RefCell::new(None),
        })),
    }
}
struct Factory {
    limits: Limits,
    worker: RefCell<Option<Worker>>,
}
impl backend::Factory for Factory {
    fn create(&self, scheduler: tokio::runtime::Handle) -> Result<Rc<dyn backend::Realm>> {
        if self.limits.heap_bytes < 16 * 1024 * 1024 || self.limits.synchronous_slice.is_zero() {
            return Err(script("invalid UI guest limits"));
        }
        let mut worker = self.worker.borrow_mut();
        if worker.is_none() {
            *worker = Some(Worker::new()?);
        }
        let worker = worker.as_ref().expect("created UI worker");
        let id = worker.identity();
        let health = Arc::new(Mutex::new(None));
        let retained = Arc::downgrade(&health);
        let limits = self.limits.clone();
        worker.submit(0, move |state| {
            match Foreground::new(limits, scheduler) {
                Ok(vm) => {
                    state.realms.insert(
                        id,
                        Vm {
                            vm,
                            modules: BTreeMap::new(),
                            health: retained,
                        },
                    );
                }
                Err(error) => {
                    if let Some(health) = retained.upgrade() {
                        *health.lock().unwrap() = Some(error.to_string());
                    }
                }
            }
            Ok(())
        })?;
        Ok(Rc::new(Realm {
            id,
            worker: worker.clone(),
            health,
        }))
    }
}

/// One sleeping worker multiplexes realms without sharing their V8 failure
/// domains. Queue admission is nonblocking and accounts pending plus active bytes.
#[derive(Clone)]
struct Worker {
    queue: mpsc::SyncSender<Command>,
    bytes: Arc<AtomicUsize>,
    ids: Arc<AtomicU64>,
}
struct Command {
    run: Box<dyn FnOnce(&mut State) + Send>,
    _reservation: Reservation,
}
struct Reservation {
    usage: Arc<AtomicUsize>,
    bytes: usize,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        self.usage.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}
impl Worker {
    fn new() -> Result<Self> {
        let (queue, receive) = mpsc::sync_channel::<Command>(QUEUE_ITEMS);
        std::thread::Builder::new()
            .name("maka-ui-js".into())
            .spawn(move || {
                let mut state = State {
                    realms: BTreeMap::new(),
                };
                while let Ok(command) = receive.recv() {
                    (command.run)(&mut state);
                    // Realm disposal may encounter a full bounded queue. The
                    // original frontend health owner is then gone; an already
                    // queued turn performs its destruction on this VM thread.
                    state.realms.retain(|_, vm| vm.health.strong_count() != 0);
                }
                // All VM and captured callback destruction happens on this thread.
            })
            .map_err(script)?;
        Ok(Self {
            queue,
            bytes: Arc::new(AtomicUsize::new(0)),
            ids: Arc::new(AtomicU64::new(1)),
        })
    }
    fn identity(&self) -> u64 {
        self.ids.fetch_add(1, Ordering::Relaxed)
    }
    fn submit<T: Send + 'static>(
        &self,
        bytes: usize,
        action: impl FnOnce(&mut State) -> Result<T> + Send + 'static,
    ) -> Result<backend::Job<T>> {
        self.bytes
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes).filter(|next| *next <= QUEUE_BYTES)
            })
            .map_err(|_| Error::Invalid("UI execution queue byte limit".into()))?;
        let reservation = Reservation {
            usage: self.bytes.clone(),
            bytes,
        };
        let (send, receive) = oneshot::channel();
        self.queue
            .try_send(Command {
                run: Box::new(move |state| {
                    let _ = send.send(action(state));
                }),
                _reservation: reservation,
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => {
                    Error::Invalid("UI execution queue item limit".into())
                }
                mpsc::TrySendError::Disconnected(_) => {
                    script("UI worker stopped without cleanup acknowledgement")
                }
            })?;
        Ok(backend::Job::new(async move {
            receive
                .await
                .map_err(|_| script("UI worker stopped without cleanup acknowledgement"))?
        }))
    }
}
struct State {
    realms: BTreeMap<u64, Vm>,
}
struct Vm {
    vm: Foreground,
    modules: BTreeMap<u64, OwnedModule>,
    health: std::sync::Weak<Mutex<Option<String>>>,
}
struct OwnedModule {
    module: ForegroundModule,
    callbacks: BTreeMap<Vec<String>, ForegroundCallbacks>,
}
impl State {
    fn with_vm<T>(&mut self, id: u64, action: impl FnOnce(&mut Vm) -> Result<T>) -> Result<T> {
        let vm = self
            .realms
            .get_mut(&id)
            .ok_or_else(|| script("UI realm is unavailable"))?;
        let result = action(vm);
        if let Some(error) = vm.vm.failure()
            && let Some(health) = vm.health.upgrade()
        {
            *health.lock().unwrap() = Some(error.to_string());
        }
        result
    }
}
struct Realm {
    id: u64,
    worker: Worker,
    health: Arc<Mutex<Option<String>>>,
}
impl backend::Realm for Realm {
    fn load(&self, name: &str, source: &str) -> Result<Rc<dyn backend::Module>> {
        let id = self.worker.identity();
        let realm = self.id;
        let name = name.to_owned();
        let source = source.to_owned();
        self.worker.submit(source.len(), move |state| {
            state.with_vm(realm, |vm| {
                let module = vm.vm.load(&name, &source).map_err(script)?;
                vm.modules.insert(
                    id,
                    OwnedModule {
                        module,
                        callbacks: BTreeMap::new(),
                    },
                );
                Ok(())
            })
        })?;
        Ok(Rc::new(Module {
            realm,
            id,
            worker: self.worker.clone(),
        }))
    }
    fn failure(&self) -> Option<String> {
        self.health.lock().unwrap().clone()
    }
}
impl Drop for Realm {
    fn drop(&mut self) {
        let id = self.id;
        // Module close receipts precede releasing the final realm reference.
        // Failure to admit disposal remains an execution failure, never a close ACK.
        let _ = self.worker.submit(0, move |state| {
            state.realms.remove(&id);
            Ok(())
        });
    }
}
struct Module {
    realm: u64,
    id: u64,
    worker: Worker,
}
impl backend::Module for Module {
    fn activate(
        &self,
        configuration: &Value,
        values: &abi::ValueBindings,
    ) -> Result<backend::Job<BTreeMap<String, abi::UiExport>>> {
        let args = vec![
            configuration.clone(),
            serde_json::to_value(values).map_err(invalid)?,
        ];
        let bytes = delivery(&args)?;
        let (realm, id) = (self.realm, self.id);
        self.worker.submit(bytes, move |state| {
            state.with_vm(realm, |vm| {
                let owned = vm
                    .modules
                    .get_mut(&id)
                    .ok_or_else(|| script("UI module is unavailable"))?;
                let value = vm
                    .vm
                    .call(&owned.module, &["activate".into()], &args)
                    .map_err(script)?;
                let exports: BTreeMap<String, abi::UiExport> =
                    serde_json::from_value(value).map_err(invalid)?;
                for export in exports.values() {
                    if let abi::UiExport::Component {
                        contract,
                        component,
                    } = export
                    {
                        let maka_plugin_core::contract::Interface::Component { captures, .. } =
                            &contract.interface
                        else {
                            return Err(invalid("UI export must declare a component"));
                        };
                        let methods = if captures.is_empty() {
                            &["mount", "event", "update"][..]
                        } else {
                            &["mount", "event", "update", "capture"][..]
                        };
                        let callbacks = vm
                            .vm
                            .capture(&owned.module, component, methods)
                            .map_err(script)?;
                        owned.callbacks.insert(component.clone(), callbacks);
                    }
                }
                Ok(exports)
            })
        })
    }
    fn component(&self, namespace: &[String], _: bool) -> Result<Rc<dyn backend::Component>> {
        Ok(Rc::new(Component {
            realm: self.realm,
            module: self.id,
            namespace: namespace.to_vec(),
            worker: self.worker.clone(),
        }))
    }
    fn close(&self) -> Result<backend::Job<()>> {
        let (realm, id) = (self.realm, self.id);
        self.worker.submit(0, move |state| {
            state.with_vm(realm, |vm| {
                let owned = vm
                    .modules
                    .remove(&id)
                    .ok_or_else(|| script("UI module is unavailable"))?;
                vm.vm.close(&owned.module).map_err(script)
            })
        })
    }
}
struct Component {
    realm: u64,
    module: u64,
    namespace: Vec<String>,
    worker: Worker,
}
impl Component {
    fn execute<T: Send + 'static>(
        &self,
        method: &'static str,
        args: Vec<Value>,
        deadline: Instant,
        decode: impl FnOnce(Value) -> Result<T> + Send + 'static,
    ) -> Result<backend::Job<T>> {
        let bytes = delivery(&args)?;
        let (realm, id, namespace) = (self.realm, self.module, self.namespace.clone());
        self.worker.submit(bytes, move |state| {
            state.with_vm(realm, |vm| {
                let callbacks = vm
                    .modules
                    .get(&id)
                    .and_then(|module| module.callbacks.get(&namespace))
                    .ok_or_else(|| script("UI callbacks are retired or unavailable"))?;
                let value = vm
                    .vm
                    .call_bound_until(callbacks, method, &args, deadline)
                    .map_err(script)?;
                decode(value)
            })
        })
    }
}
impl backend::Component for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        values: &abi::ValueBindings,
        deadline: Instant,
        prepare: Arc<PrepareScene>,
    ) -> Result<backend::Job<ComponentOutput<PreparedScene>>> {
        let (method, mut args) = match call {
            ComponentCall::Mount { props } => ("mount", vec![props.clone()]),
            ComponentCall::Update { state, props } => {
                ("update", vec![state.clone(), props.clone()])
            }
            ComponentCall::Event {
                state,
                props,
                key,
                input,
                origin,
            } => {
                let event = match origin {
                    EventOrigin::Raw => abi::ComponentInput::Raw {
                        key: key.into(),
                        event: input.clone(),
                    },
                    EventOrigin::Action => abi::ComponentInput::Action {
                        key: key.into(),
                        event: input.clone(),
                    },
                    EventOrigin::Component { child, name } => abi::ComponentInput::Component {
                        key: key.into(),
                        child: child.into(),
                        name: name.into(),
                        event: input.clone(),
                    },
                };
                (
                    "event",
                    vec![
                        state.clone(),
                        serde_json::to_value(event).map_err(invalid)?,
                        props.clone(),
                    ],
                )
            }
        };
        args.push(serde_json::to_value(values).map_err(invalid)?);
        self.execute(method, args, deadline, move |value| {
            // Reject the complete guest result before compiler code observes it.
            maka_plugin_core::json::exact_numbers(&value).map_err(Error::Invalid)?;
            maka_plugin_core::json::encoded_size(&value, 1024 * 1024).map_err(invalid)?;
            let output: ComponentOutput = serde_json::from_value(value).map_err(script)?;
            let view = prepare(&output.frame.view, deadline)?
                .with_declaration(output.frame.view.clone())?;
            Ok(output.map_view(|_| view))
        })
    }
    fn capture(
        &self,
        call: CaptureCall<'_>,
        deadline: Instant,
    ) -> Result<backend::Job<CapturePlan>> {
        self.execute(
            "capture",
            vec![
                call.state.clone(),
                call.props.clone(),
                json!(call.method),
                call.input.clone(),
                serde_json::to_value(call.values).map_err(invalid)?,
            ],
            deadline,
            |value| serde_json::from_value(value).map_err(invalid),
        )
    }
}
fn delivery(args: &[Value]) -> Result<usize> {
    for value in args {
        maka_plugin_core::json::exact_numbers(value).map_err(Error::Invalid)?;
    }
    maka_plugin_core::json::encoded_size(args, 1024 * 1024).map_err(invalid)
}
fn script(error: impl ToString) -> Error {
    Error::Script(error.to_string())
}
fn invalid(error: impl ToString) -> Error {
    Error::Invalid(error.to_string())
}
