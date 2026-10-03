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
use crate::support::scene::Node;
use maka_plugin_core::{
    composition::PresentationTarget,
    contract::Contract,
    package::{EntryTarget, Implementation},
};
use maka_ui::{
    Application, ComponentCall, ComponentOutput, Frame, NativeCapability, NativeComponent,
    NativeExport, NativeRegistry, NativeView, Surface,
};
use serde_json::{Value, json};
use std::{cell::Cell, collections::BTreeMap, rc::Rc};

struct Counter {
    calls: Rc<Cell<usize>>,
}
impl NativeComponent<String> for Counter {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _values: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<String>>> {
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
            surface: None,
            view: NativeView::new(
                Rc::new(format!(
                    "{}={}",
                    state["label"].as_str().unwrap(),
                    state["count"]
                )),
                BTreeMap::from([(
                    "inc".into(),
                    json!({"type":"object","properties":{"delta":{"type":"integer","minimum":-10,"maximum":10},"large":{"type":"string"}},"required":["delta"],"additionalProperties":false}),
                )]), vec![])?,
            state,
        }.output(Vec::new()))
    }
}
fn registered(contract: Contract, calls: Rc<Cell<usize>>) -> NativeRegistry<String> {
    let mut registry = crate::support::scene::registry(PresentationTarget::Desktop);
    registry
        .register("test.counter", move |_, _values| {
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
        Default::default(),
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
    let view = &app.frame(&left).unwrap().view;
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
        let mut registry = crate::support::scene::registry::<String>(PresentationTarget::Desktop);
        if failure != 0 {
            if failure == 1 {
                contract.version += 1;
            }
            registry
                .register("test.counter", move |_, _values| {
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
                    .register("unused", |_, _values| {
                        panic!("unselected native factory activated")
                    })
                    .unwrap();
            }
        }
        let result = Application::new(
            &input,
            packages,
            timer.handle().clone(),
            Default::default(),
            registry,
            Default::default(),
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
    let (input, packages) = support::editor_fixture();
    assert!(
        Application::new(
            &input,
            packages,
            timer.handle().clone(),
            Default::default(),
            crate::support::scene::registry::<String>(PresentationTarget::Desktop),
            Default::default()
        )
        .is_err()
    );
}

struct Root;
impl NativeComponent<String> for Root {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _values: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<String>>> {
        assert!(matches!(call, ComponentCall::Mount { .. }));
        Ok(Frame {
            revision: 0,
            state: json!({}),
            surface: Some(Surface::Window {
                binding: "surface".into(),
                title: "Native root".into(),
                width: 640,
                height: 400,
            }),
            view: crate::support::scene::view(Node::Component(maka_ui::ComponentSlot {
                key: "child".into(),
                binding: "counter".into(),
                member: 0,
                props: json!({"label":"Native","initial":4}),
                on: BTreeMap::new(),
            }))?,
        }
        .output(Vec::new()))
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
        .register("test.root", move |_, _values| {
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
        Default::default(),
    )
    .unwrap();
    let root = app.mount_launch(json!({})).unwrap();
    let child = app.child_mount(&root, "child").unwrap().clone();
    assert_eq!(
        app.frame(&root).unwrap().surface,
        Some(Surface::Window {
            binding: "surface".into(),
            title: "Native root".into(),
            width: 640,
            height: 400
        })
    );
    app.dispatch(&child, 0, "inc", json!({"delta":3})).unwrap();
    assert_eq!(app.frame(&child).unwrap().state["count"], 7);
    close_all(&mut app, &root);
}

struct SlotParent;
impl NativeComponent<String> for SlotParent {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _values: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<String>>> {
        let mut next = match call {
            ComponentCall::Mount { .. } => {
                json!({"revision":0,"version":0,"right":true,"native":true,"seen":[]})
            }
            ComponentCall::Update { state, .. } | ComponentCall::Event { state, .. } => {
                state.clone()
            }
        };
        if let ComponentCall::Event {
            key, input, origin, ..
        } = call
        {
            next["revision"] = json!(next["revision"].as_u64().unwrap() + 1);
            match key {
                "notice" => {
                    let maka_ui::EventOrigin::Component { child, name } = origin else {
                        panic!("kernel child origin")
                    };
                    next["seen"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!([child, name, input]));
                }
                "parent" => next["version"] = json!(next["version"].as_u64().unwrap() + 1),
                "toggle" => next["right"] = json!(!next["right"].as_bool().unwrap()),
                "tree" => next["native"] = json!(!next["native"].as_bool().unwrap()),
                "oversized" => next["blob"] = json!("x".repeat(1024 * 1024)),
                "aggregate" => next["blob"] = json!("x".repeat(600_000)),
                _ => {}
            }
            next["last"] = json!(key);
        }
        let mut slots = vec![maka_ui::ComponentSlot {
            key: "left".into(),
            binding: "counter".into(),
            member: 0,
            props: json!({"label":format!("Left {}",next["version"]),"initial":10}),
            on: BTreeMap::from([("changed".into(), "notice".into())]),
        }];
        if next["right"] == true {
            slots.push(maka_ui::ComponentSlot {
                key: "right".into(),
                binding: "counter".into(),
                member: 0,
                props: json!({"label":if next["last"]=="reject" { "Right Reject".into() } else { format!("Right {}",next["version"]) },"initial":20}),
                on: BTreeMap::new(),
            });
        }
        match next["last"].as_str() {
            Some("inexact") => slots[0].props["initial"] = json!(9_007_199_254_740_993u64),
            Some("aggregate") => slots[0].props["label"] = json!("x".repeat(600_000)),
            Some("invalid") => slots[0].binding = "undeclared".into(),
            Some("bad_event") => {
                slots[0].on.insert("undeclared".into(), "notice".into());
            }
            Some("duplicate") => slots.push(slots[0].clone()),
            _ => {}
        }
        let view = if next["native"] == true {
            NativeView::new(
                Rc::new("Native parent".into()),
                [
                    "parent",
                    "toggle",
                    "tree",
                    "invalid",
                    "bad_event",
                    "duplicate",
                    "oversized",
                    "aggregate",
                    "inexact",
                    "reject",
                ]
                .into_iter()
                .map(|k| (k.into(), json!(true)))
                .collect(),
                slots,
            )?
        } else {
            crate::support::scene::view(Node::Stack {
                key: "root".into(),
                direction: crate::support::scene::Direction::Column,
                gap: 0,
                padding: 0,
                background: 0,
                children: slots
                    .into_iter()
                    .map(Node::Component)
                    .chain([
                        Node::Button {
                            key: "tree".into(),
                            label: "Native".into(),
                        },
                        Node::Button {
                            key: "oversized".into(),
                            label: "Oversized".into(),
                        },
                        Node::Button {
                            key: "aggregate".into(),
                            label: "Aggregate".into(),
                        },
                    ])
                    .collect(),
            })?
        };
        Ok(Frame {
            revision: next["revision"].as_u64().unwrap(),
            state: next,
            surface: Some(Surface::Window {
                binding: "surface".into(),
                title: "Slots".into(),
                width: 640,
                height: 400,
            }),
            view,
        }
        .output(vec![]))
    }
}
#[test]
fn native_parent_slots_share_js_child_events_rollback_identity_and_retirement() {
    let timer = timer();
    let (mut input, mut packages, _) = super::events::fixture(
        "return frame({count:props.initial,label:props.label,revision:0,calls:0});",
        "return frame({...state,count:state.count+1,revision:state.revision+1},[{name:'changed',value:state.count+1}]);",
        "if(props.label==='Right Reject')throw new Error('later sibling');globalThis.updates=(globalThis.updates??0)+1;return frame({...state,label:props.label,revision:state.revision+1,calls:globalThis.updates});",
        support::PARENT_EVENT,
        "{}",
    );
    let i = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(i);
    let mut declaration = old.manifest().application.clone().unwrap();
    let entry = declaration.entries.get_mut(&EntryTarget::Desktop).unwrap();
    entry.implementation = Implementation::Native {
        implementation: "test.slot-parent".into(),
    };
    let contract = entry.exports["App"].contract.clone();
    let parent = support::package("native.slot-parent", declaration, None);
    input.instances.get_mut("app").unwrap().package = parent.application_ref().into();
    packages.push(parent);
    let mut native = crate::support::scene::registry(PresentationTarget::Desktop);
    native
        .register("test.slot-parent", move |_, _values| {
            Ok(BTreeMap::from([(
                "App".into(),
                NativeExport {
                    contract: contract.clone(),
                    value: NativeCapability::Component(Rc::new(SlotParent)),
                },
            )]))
        })
        .unwrap();
    let mut app = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        native,
        Default::default(),
    )
    .unwrap();
    let root = app.mount_launch(json!({})).unwrap();
    let left = app.child_mount(&root, "left").unwrap().clone();
    let right = app.child_mount(&root, "right").unwrap().clone();
    assert_eq!(
        app.frame(&root).unwrap().view.view.as_str(),
        "Native parent"
    );
    app.dispatch(&left, 0, "inc", Value::Null).unwrap();
    assert_eq!(
        app.frame(&root).unwrap().state["seen"],
        json!([["left", "changed", 11]])
    );
    for key in [
        "invalid",
        "bad_event",
        "duplicate",
        "oversized",
        "aggregate",
        "inexact",
    ] {
        assert!(app.dispatch(&root, 1, key, Value::Null).is_err(), "{key}");
        assert_eq!(app.frame(&root).unwrap().revision, 1);
        assert_eq!(app.child_mount(&root, "left").unwrap(), &left);
        assert_eq!(app.frame(&left).unwrap().state["label"], "Left 0");
        assert_eq!(app.frame(&left).unwrap().state["count"], 11);
        assert!(!app.is_retiring(&left));
        assert!(!app.is_retiring(&right));
    }
    app.dispatch(&root, 1, "toggle", Value::Null).unwrap();
    assert!(app.is_retiring(&right));
    app.close(&right, Ok(())).unwrap();
    app.dispatch(&root, 2, "parent", Value::Null).unwrap();
    assert_eq!(app.child_mount(&root, "left").unwrap(), &left);
    assert_eq!(app.frame(&left).unwrap().state["label"], "Left 1");
    assert_eq!(
        app.frame(&left).unwrap().state["calls"],
        1,
        "inexact props never executed JS callback or retired shared realm"
    );
    app.dispatch(&root, 3, "tree", Value::Null).unwrap();
    assert_eq!(app.frame(&root).unwrap().view.view.as_str(), "");
    assert_eq!(app.child_mount(&root, "left").unwrap(), &left);
    assert!(
        app.dispatch(&root, 4, "oversized", Value::Null).is_err(),
        "Declarative output of native callback has same complete budget"
    );
    assert_eq!(app.frame(&root).unwrap().revision, 4);
    assert!(app.dispatch(&root, 4, "aggregate", Value::Null).is_err());
    assert_eq!(app.frame(&root).unwrap().revision, 4);
    app.dispatch(&root, 4, "tree", Value::Null).unwrap();
    assert_eq!(
        app.frame(&root).unwrap().view.view.as_str(),
        "Native parent"
    );
    assert_eq!(app.child_mount(&root, "left").unwrap(), &left);
    // Restore the removed slot at the non-rejecting version 2; it is a new mount.
    app.dispatch(&root, 5, "parent", Value::Null).unwrap();
    app.dispatch(&root, 6, "toggle", Value::Null).unwrap();
    assert_ne!(app.child_mount(&root, "right").unwrap(), &right);
    assert_eq!(app.frame(&left).unwrap().state["count"], 11);
    assert!(app.dispatch(&right, 0, "inc", Value::Null).is_err());
    let before = app.frame(&left).unwrap().revision;
    assert!(app.dispatch(&root, 7, "reject", Value::Null).is_err());
    assert_eq!(app.frame(&root).unwrap().revision, 7);
    assert_eq!(app.frame(&left).unwrap().revision, before);
    assert_eq!(app.frame(&left).unwrap().state["label"], "Left 2");
    // A real JS callback failure retires its realm, while the last committed
    // scenes and exact mount identities remain intact pending actual cleanup.
    assert!(app.is_retiring(&left));
    assert!(
        app.is_retiring(&root),
        "fault retirement reaches dependent parent"
    );
    close_all(&mut app, &root);
}

#[test]
fn native_slot_metadata_has_independent_count_key_and_byte_bounds() {
    let slot = |key: String| maka_ui::ComponentSlot {
        key,
        binding: "child".into(),
        member: 0,
        props: json!({}),
        on: BTreeMap::new(),
    };
    assert!(
        NativeView::new(
            Rc::new(()),
            BTreeMap::new(),
            (0..65).map(|i| slot(i.to_string())).collect()
        )
        .is_err()
    );
    assert!(
        NativeView::new(
            Rc::new(()),
            BTreeMap::new(),
            vec![slot("same".into()), slot("same".into())]
        )
        .is_err()
    );
    assert!(NativeView::new(Rc::new(()), BTreeMap::new(), vec![slot(String::new())]).is_err());
    let mut huge = slot("large".into());
    huge.props = json!({"text":"x".repeat(1024*1024)});
    assert!(NativeView::new(Rc::new(()), BTreeMap::new(), vec![huge]).is_err());
}
