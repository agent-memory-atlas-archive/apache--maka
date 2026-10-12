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

//! Foreground JavaScript execution for the UI runtime. Native input, mutation
//! receipts and resource ownership remain exclusively in the original UI owner.
use maka_js_runtime::plugin::{Foreground, ForegroundCallbacks, ForegroundModule, Limits};
use maka_ui_runtime::actions::{CaptureCall, CapturePlan};
use maka_ui_runtime::{
    ComponentCall, ComponentOutput, Error, EventOrigin, ExecutionOptions, Result, abi, backend,
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Instant};

pub fn execution(limits: Limits) -> ExecutionOptions {
    ExecutionOptions {
        synchronous_slice: limits.synchronous_slice,
        script: Some(Rc::new(Factory(limits))),
    }
}
struct Factory(Limits);
impl backend::Factory for Factory {
    fn create(&self, scheduler: tokio::runtime::Handle) -> Result<Rc<dyn backend::Realm>> {
        let vm = Foreground::new(self.0.clone(), scheduler).map_err(script)?;
        Ok(Rc::new(Realm(Rc::new(RefCell::new(vm)))))
    }
}
struct Realm(Rc<RefCell<Foreground>>);
impl backend::Realm for Realm {
    fn load(&self, name: &str, source: &str) -> Result<Rc<dyn backend::Module>> {
        let module = self.0.borrow_mut().load(name, source).map_err(script)?;
        Ok(Rc::new(Module {
            realm: self.0.clone(),
            module,
        }))
    }
    fn failure(&self) -> Option<String> {
        self.0.borrow().failure().map(|error| error.to_string())
    }
}
struct Module {
    realm: Rc<RefCell<Foreground>>,
    module: ForegroundModule,
}
impl backend::Module for Module {
    fn activate(
        &self,
        configuration: &Value,
        values: &abi::ValueBindings,
    ) -> Result<BTreeMap<String, abi::UiExport>> {
        let args = vec![
            configuration.clone(),
            serde_json::to_value(values).map_err(invalid)?,
        ];
        delivery(&args)?;
        let value = self
            .realm
            .borrow_mut()
            .call(&self.module, &["activate".into()], &args)
            .map_err(script)?;
        serde_json::from_value(value).map_err(invalid)
    }
    fn component(
        &self,
        namespace: &[String],
        captures: bool,
    ) -> Result<Rc<dyn backend::Component>> {
        let methods = if captures {
            &["mount", "event", "update", "capture"][..]
        } else {
            &["mount", "event", "update"][..]
        };
        let callbacks = self
            .realm
            .borrow_mut()
            .capture(&self.module, namespace, methods)
            .map_err(script)?;
        Ok(Rc::new(Component {
            realm: self.realm.clone(),
            callbacks,
        }))
    }
    fn close(&self) -> Result<()> {
        self.realm.borrow_mut().close(&self.module).map_err(script)
    }
}
struct Component {
    realm: Rc<RefCell<Foreground>>,
    callbacks: ForegroundCallbacks,
}
impl backend::Component for Component {
    fn call(
        &self,
        call: ComponentCall<'_>,
        values: &abi::ValueBindings,
        deadline: Instant,
    ) -> Result<ComponentOutput> {
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
        delivery(&args)?;
        let value = self
            .realm
            .borrow_mut()
            .call_bound_until(&self.callbacks, method, &args, deadline)
            .map_err(script)?;
        serde_json::from_value(value).map_err(script)
    }
    fn capture(&self, call: CaptureCall<'_>, deadline: Instant) -> Result<CapturePlan> {
        let args = vec![
            call.state.clone(),
            call.props.clone(),
            json!(call.method),
            call.input.clone(),
            serde_json::to_value(call.values).map_err(invalid)?,
        ];
        delivery(&args)?;
        let value = self
            .realm
            .borrow_mut()
            .call_bound_until(&self.callbacks, "capture", &args, deadline)
            .map_err(script)?;
        serde_json::from_value(value).map_err(invalid)
    }
}
fn delivery(args: &[Value]) -> Result<()> {
    maka_plugin_core::json::check_delivery(&Value::Array(args.to_vec()), 1024 * 1024)
        .map(|_| ())
        .map_err(Error::Invalid)
}
fn script(error: impl ToString) -> Error {
    Error::Script(error.to_string())
}
fn invalid(error: impl ToString) -> Error {
    Error::Invalid(error.to_string())
}
