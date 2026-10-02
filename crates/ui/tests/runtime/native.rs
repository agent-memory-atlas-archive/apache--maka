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

use super::{support, timer};
use maka_plugin_core::{
    composition::PresentationTarget,
    contract::Contract,
    package::{EntryTarget, Implementation},
};
use maka_ui::{
    Application, ComponentCall, Frame, NativeCapability, NativeComponent, NativeExport,
    NativeRegistry, NativeView, Node, Presentation, Window,
};
use serde_json::{Value, json};
use std::{cell::Cell, collections::BTreeMap, rc::Rc};

struct Counter {
    calls: Rc<Cell<usize>>,
}
impl NativeComponent<String> for Counter {
    fn call(&self, call: ComponentCall<'_>) -> maka_ui::Result<Frame<Presentation<String>>> {
        self.calls.set(self.calls.get() + 1);
        let state = match call {
            ComponentCall::Mount { props } => {
                json!({"count":props["initial"],"label":props["label"],"revision":0})
            }
            ComponentCall::Update { state, props } => {
                if props["label"] == "Right 1" {
                    return Err(maka_ui::Error::Invalid("reject later sibling".into()));
                }
                json!({"count":state["count"],"label":props["label"],"revision":state["revision"].as_u64().unwrap()+1})
            }
            ComponentCall::Event { state, input, .. } => {
                json!({"count":state["count"].as_i64().unwrap()+input["delta"].as_i64().unwrap(),"label":state["label"],"revision":state["revision"].as_u64().unwrap()+1})
            }
        };
        Ok(Frame {
            revision: state["revision"].as_u64().unwrap(),
            window: None,
            view: Presentation::Native(NativeView::new(
                Rc::new(format!(
                    "{}={}",
                    state["label"].as_str().unwrap(),
                    state["count"]
                )),
                BTreeMap::from([(
                    "inc".into(),
                    json!({"type":"object","properties":{"delta":{"type":"integer","minimum":-10,"maximum":10},"large":{"type":"string"}},"required":["delta"],"additionalProperties":false}),
                )]),
            )?),
            state,
        })
    }
}
fn registered(contract: Contract, calls: Rc<Cell<usize>>) -> NativeRegistry<String> {
    let mut registry = NativeRegistry::with_surface(PresentationTarget::Desktop);
    registry
        .register("test.counter", move |_| {
            Ok(BTreeMap::from([(
                "Counter".into(),
                NativeExport {
                    contract: contract.clone(),
                    value: NativeCapability::Component(Rc::new(Counter {
                        calls: calls.clone(),
                    })),
                },
            )]))
        })
        .unwrap();
    registry
}
fn fixture() -> (
    maka_plugin_core::composition::Input,
    Vec<maka_plugins::package::Package>,
    Contract,
) {
    let (mut input, mut packages) =
        support::composed_fixture(support::CHILD_UPDATE, support::PARENT_EVENT);
    let old = packages.pop().unwrap();
    let mut declaration = old.manifest().application.clone().unwrap();
    let entry = declaration.entries.get_mut(&EntryTarget::Desktop).unwrap();
    let contract = entry.exports["Counter"].contract.clone();
    entry.implementation = Implementation::Native {
        implementation: "test.counter".into(),
    };
    let counter = support::package("native.counter", declaration, None);
    input.instances.get_mut("counter").unwrap().package = counter.application_ref().into();
    packages.push(counter);
    (input, packages, contract)
}
fn close_all<V: ?Sized + 'static>(
    app: &mut Application<V>,
    root: &maka_plugin_core::runtime::Mount,
) {
    app.retire(root).unwrap();
    for mount in app.retiring_mounts() {
        app.close(&mount, Ok(())).unwrap();
    }
    app.shutdown().unwrap();
}

#[test]
fn opaque_native_children_share_transaction_identity_and_input_admission_with_js() {
    let timer = timer();
    let (input, packages, contract) = fixture();
    let calls = Rc::new(Cell::new(0));
    let mut app = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        registered(contract, calls.clone()),
    )
    .unwrap();
    let root = app.mount_launch(json!({})).unwrap();
    let left = app.child_mount(&root, "left").unwrap().clone();
    let right = app.child_mount(&root, "right").unwrap().clone();
    let count = calls.get();
    assert!(
        app.dispatch(&left, 0, "missing", json!({"delta":1}))
            .is_err()
    );
    assert!(app.dispatch(&left, 0, "inc", json!({"delta":11})).is_err());
    assert!(
        app.dispatch(
            &left,
            0,
            "inc",
            json!({"delta":1,"large":"x".repeat(1024*1024)})
        )
        .is_err()
    );
    assert_eq!(
        calls.get(),
        count,
        "invalid raw inputs cannot enter the native callback"
    );
    app.dispatch(&left, 0, "inc", json!({"delta":2})).unwrap();
    let Presentation::Native(view) = &app.frame(&left).unwrap().view else {
        panic!("opaque native view")
    };
    assert_eq!(view.view.as_str(), "Left 0=12");
    assert_eq!(app.frame(&right).unwrap().state["count"], 20);
    assert!(app.dispatch(&left, 0, "inc", json!({"delta":1})).is_err());
    // The first sibling prepares successfully, but a later native rejection
    // must leave the root and both previously published scenes untouched.
    assert!(app.dispatch(&root, 0, "parent", Value::Null).is_err());
    assert_eq!(app.frame(&root).unwrap().revision, 0);
    assert_eq!(app.frame(&left).unwrap().revision, 1);
    assert_eq!(app.frame(&left).unwrap().state["label"], "Left 0");
    assert_eq!(app.frame(&right).unwrap().revision, 0);
    assert!(!app.is_retiring(&left));
    app.dispatch(&root, 0, "toggle", Value::Null).unwrap();
    app.close(&right, Ok(())).unwrap();
    // With the rejecting sibling gone, props update retains the first native
    // child's identity and independently accumulated local state.
    app.dispatch(&root, 1, "parent", Value::Null).unwrap();
    assert_eq!(app.child_mount(&root, "left").unwrap(), &left);
    assert_eq!(app.frame(&left).unwrap().state["count"], 12);
    assert_eq!(app.frame(&left).unwrap().state["label"], "Left 1");
    app.dispatch(&root, 2, "toggle", Value::Null).unwrap();
    let replacement = app.child_mount(&root, "right").unwrap();
    assert_ne!(replacement, &right);
    assert_eq!(app.frame(replacement).unwrap().state["count"], 20);
    assert!(app.dispatch(&right, 0, "inc", json!({"delta":1})).is_err());
    close_all(&mut app, &root);
}

#[test]
fn native_availability_is_not_activation_and_actual_contract_or_value_mismatches_fail() {
    let timer = timer();
    for failure in 0..4 {
        let (input, packages, mut contract) = fixture();
        let mut registry = NativeRegistry::<String>::with_surface(PresentationTarget::Desktop);
        if failure != 0 {
            if failure == 1 {
                contract.version += 1;
            }
            registry
                .register("test.counter", move |_| {
                    Ok(BTreeMap::from([(
                        "Counter".into(),
                        NativeExport {
                            contract: contract.clone(),
                            value: if failure == 2 {
                                NativeCapability::Surface(PresentationTarget::Desktop)
                            } else {
                                NativeCapability::Component(Rc::new(Counter {
                                    calls: Rc::new(Cell::new(0)),
                                }))
                            },
                        },
                    )]))
                })
                .unwrap();
            if failure == 3 {
                // Extra implementation availability must remain inert.
                registry
                    .register("unused", |_| panic!("unselected native factory activated"))
                    .unwrap();
            }
        }
        let result = Application::new(
            &input,
            packages,
            timer.handle().clone(),
            Default::default(),
            registry,
        );
        if failure == 3 {
            let mut app = result.unwrap();
            let root = app.mount_launch(json!({})).unwrap();
            close_all(&mut app, &root);
        } else {
            assert!(result.is_err());
        }
    }
    // The example imports the label explicitly; the surface factory alone
    // cannot stand in for a declared native component implementation.
    let (input, packages) = support::native_fixture();
    assert!(
        Application::new(
            &input,
            packages,
            timer.handle().clone(),
            Default::default(),
            NativeRegistry::<String>::with_surface(PresentationTarget::Desktop)
        )
        .is_err()
    );
}

struct Root;
impl NativeComponent<String> for Root {
    fn call(&self, call: ComponentCall<'_>) -> maka_ui::Result<Frame<Presentation<String>>> {
        assert!(matches!(call, ComponentCall::Mount { .. }));
        Ok(Frame {
            revision: 0,
            state: json!({}),
            window: Some(Window {
                surface: "surface".into(),
                title: "Native root".into(),
                width: 640,
                height: 400,
            }),
            view: Presentation::Tree(Node::Component {
                key: "child".into(),
                binding: "counter".into(),
                member: 0,
                props: json!({"label":"Native","initial":4}),
            }),
        })
    }
}
#[test]
fn native_root_uses_explicit_surface_and_can_compose_an_imported_component() {
    let timer = timer();
    let (mut input, mut packages, contract) = fixture();
    let index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(index);
    let mut declaration = old.manifest().application.clone().unwrap();
    let entry = declaration.entries.get_mut(&EntryTarget::Desktop).unwrap();
    entry.implementation = Implementation::Native {
        implementation: "test.root".into(),
    };
    let root_contract = entry.exports["App"].contract.clone();
    let root = support::package("native.root", declaration, None);
    input.instances.get_mut("app").unwrap().package = root.application_ref().into();
    packages.push(root);
    let mut registry = registered(contract, Rc::new(Cell::new(0)));
    registry
        .register("test.root", move |_| {
            Ok(BTreeMap::from([(
                "App".into(),
                NativeExport {
                    contract: root_contract.clone(),
                    value: NativeCapability::Component(Rc::new(Root)),
                },
            )]))
        })
        .unwrap();
    let mut app = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        registry,
    )
    .unwrap();
    let root = app.mount_launch(json!({})).unwrap();
    let child = app.child_mount(&root, "child").unwrap().clone();
    assert_eq!(
        app.frame(&root).unwrap().window.as_ref().unwrap().title,
        "Native root"
    );
    app.dispatch(&child, 0, "inc", json!({"delta":3})).unwrap();
    assert_eq!(app.frame(&child).unwrap().state["count"], 7);
    close_all(&mut app, &root);
}
