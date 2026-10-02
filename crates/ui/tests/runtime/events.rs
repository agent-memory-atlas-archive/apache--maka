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

use super::{close_all, support, timer};
use maka_plugin_core::{
    composition::{Input, PresentationTarget, Target},
    contract::{Cardinality, Contract, Import, Interface},
    package::{EntryTarget, Implementation},
};
use maka_plugins::package::Package;
use maka_ui::{
    Application, ComponentCall, ComponentOutput, Emission, Frame, NativeCapability,
    NativeComponent, NativeExport, NativeRegistry, Node, Presentation,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, rc::Rc};

const CHILD_EVENT: &str = "return frame({...state,count:state.count+1,revision:state.revision+1},[{name:'changed',value:state.count+1}]);";
const RECEIVE: &str = "return frame({...state,revision:state.revision+1,seen:[...(state.seen??[]),[event.child,event.name,event.key,event.event]],total:(state.total??0)+event.event});";

fn fixture(
    mount: &str,
    event: &str,
    update: &str,
    receive: &str,
    on: &str,
) -> (Input, Vec<Package>, Contract) {
    let (mut input, mut packages) = support::composed_fixture(update, receive);
    let old_counter = packages.pop().unwrap();
    let old_root = packages.pop().unwrap();
    let mut declaration = old_counter.manifest().application.clone().unwrap();
    let contract = &mut declaration
        .entries
        .get_mut(&EntryTarget::Desktop)
        .unwrap()
        .exports
        .get_mut("Counter")
        .unwrap()
        .contract;
    let old_contract = serde_json::to_string(contract).unwrap();
    let Interface::Component { events, .. } = &mut contract.interface else {
        unreachable!()
    };
    *events = ["changed", "a", "b", "c"]
        .into_iter()
        .map(|name| (name.into(), json!({"type":"integer"})))
        .collect();
    let contract = contract.clone();
    let source = std::str::from_utf8(old_counter.file("index.mjs").unwrap()).unwrap()
        .replace(&old_contract, &serde_json::to_string(&contract).unwrap())
        .replace("function frame(state)", "function frame(state,events=[])")
        .replace("actions:[],events:[],frame:", "actions:[],events,frame:")
        .replace("mount(props){return frame({count:props.initial,label:props.label,revision:0});}", &format!("mount(props){{{mount}}}"))
        .replace("event(state,event){return frame({...state,count:state.count+1,revision:state.revision+1});}", &format!("event(state,event,props){{{event}}}"));
    let counter = support::package("dynamic.counter", declaration, Some(source));
    let mut declaration = old_root.manifest().application.clone().unwrap();
    declaration
        .entries
        .get_mut(&EntryTarget::Desktop)
        .unwrap()
        .imports
        .get_mut("counter")
        .unwrap()
        .contract = contract.clone();
    let source = std::str::from_utf8(old_root.file("index.mjs").unwrap())
        .unwrap()
        .replace("on:{},props:", &format!("on:{on},props:"));
    let root = support::package("dynamic.app", declaration, Some(source));
    input.instances.get_mut("app").unwrap().package = root.application_ref().into();
    input.instances.get_mut("counter").unwrap().package = counter.application_ref().into();
    packages.extend([root, counter]);
    (input, packages, contract)
}
const MOUNT: &str = "return frame({count:props.initial,label:props.label,revision:0});";
fn application(
    input: &Input,
    packages: Vec<Package>,
    timer: &tokio::runtime::Runtime,
) -> Application {
    Application::new(
        input,
        packages,
        timer.handle().clone(),
        Default::default(),
        NativeRegistry::with_surface(PresentationTarget::Desktop),
    )
    .unwrap()
}

#[test]
fn child_events_use_public_schema_exact_slots_and_staged_parent_state() {
    let timer = timer();
    let (input, packages, _) = fixture(
        MOUNT,
        "return frame({...state,count:state.count+1,revision:state.revision+1},[{name:'changed',value:2},{name:'changed',value:3}]);",
        support::CHILD_UPDATE,
        RECEIVE,
        "{changed:'notice'}",
    );
    let mut app = application(&input, packages, &timer);
    let root = app.mount_launch(json!({})).unwrap();
    let left = app.child_mount(&root, "left").unwrap().clone();
    let right = app.child_mount(&root, "right").unwrap().clone();
    // A second root owns independent subscriptions even with identical providers.
    let other = app.mount_launch(json!({})).unwrap();
    app.dispatch(&left, 0, "inc", Value::Null).unwrap();
    assert_eq!(app.frame(&root).unwrap().state["total"], 5);
    assert_eq!(
        app.frame(&root).unwrap().state["seen"],
        json!([
            ["left", "changed", "notice", 2],
            ["left", "changed", "notice", 3]
        ])
    );
    assert_eq!(app.frame(&root).unwrap().revision, 2);
    assert_eq!(app.frame(&left).unwrap().state["count"], 11);
    assert_eq!(app.frame(&right).unwrap().revision, 0);
    assert_eq!(app.frame(&other).unwrap().revision, 0);
    // A raw payload cannot impersonate the adapter's component origin or key.
    assert!(
        app.dispatch(
            &root,
            2,
            "notice",
            json!({"kind":"component","child":"left","name":"changed","event":999})
        )
        .is_err()
    );
    app.retire(&root).unwrap();
    for mount in app.retiring_mounts() {
        app.close(&mount, Ok(())).unwrap();
    }
    assert_eq!(app.frame(&other).unwrap().revision, 0);
    close_all(&mut app, &other);
}

#[test]
fn revoked_subscription_cannot_resurrect_queued_emission_after_same_batch_aba() {
    let timer = timer();
    let receive = "const next={...state,revision:state.revision+1}; if(event.key==='a')next.on={a:'a',c:'c'};else if(event.key==='c')next.on={a:'a',b:'b',c:'c'};else next.deliveries=(state.deliveries??0)+1;return frame(next);";
    let event = "return frame({...state,revision:state.revision+1},state.revision===0?[{name:'a',value:1},{name:'c',value:1},{name:'b',value:1}]:[{name:'b',value:1}]);";
    let (input, packages, _) = fixture(
        MOUNT,
        event,
        support::CHILD_UPDATE,
        receive,
        "state.on??{a:'a',b:'b',c:'c'}",
    );
    let mut app = application(&input, packages, &timer);
    let root = app.mount_launch(json!({})).unwrap();
    let left = app.child_mount(&root, "left").unwrap().clone();
    app.dispatch(&left, 0, "inc", Value::Null).unwrap();
    assert_eq!(app.frame(&root).unwrap().revision, 2);
    assert_eq!(app.frame(&root).unwrap().state["deliveries"], Value::Null);
    app.dispatch(&left, 1, "inc", Value::Null).unwrap();
    assert_eq!(app.frame(&root).unwrap().state["deliveries"], 1);
    assert_eq!(app.frame(&root).unwrap().revision, 3);
    close_all(&mut app, &root);
}

#[test]
fn invalid_emissions_and_invalid_subscriptions_never_publish_partial_scene() {
    let timer = timer();
    for (on, event, initial_error) in [
        ("{unknown:'notice'}", CHILD_EVENT, true),
        (
            "{}",
            "return frame({...state,revision:state.revision+1},[{name:'unknown',value:1}]);",
            false,
        ),
        (
            "{}",
            "return frame({...state,revision:state.revision+1},[{name:'changed',value:'bad'}]);",
            false,
        ),
        (
            "{changed:'notice'}",
            "return frame({...state,revision:state.revision+1},[{name:'changed',value:1},{name:'changed',value:'bad'}]);",
            false,
        ),
    ] {
        let (input, packages, _) = fixture(MOUNT, event, support::CHILD_UPDATE, RECEIVE, on);
        let mut app = application(&input, packages, &timer);
        if initial_error {
            assert!(app.mount_launch(json!({})).is_err());
            assert!(app.retiring_mounts().is_empty());
            app.shutdown().unwrap();
        } else {
            let root = app.mount_launch(json!({})).unwrap();
            let left = app.child_mount(&root, "left").unwrap().clone();
            assert!(app.dispatch(&left, 0, "inc", Value::Null).is_err());
            assert_eq!(app.frame(&left).unwrap().revision, 0);
            assert_eq!(app.frame(&root).unwrap().revision, 0);
            close_all(&mut app, &root);
        }
    }
}

#[test]
fn parent_event_sibling_failure_rolls_back_emitter_and_prior_parent_transitions() {
    let timer = timer();
    let (input, packages, _) = fixture(
        MOUNT,
        CHILD_EVENT,
        "if(props.label==='Right 1')throw new Error('reject sibling');return frame({...state,label:props.label,revision:state.revision+1});",
        "return frame({...state,version:state.version+1,revision:state.revision+1});",
        "{changed:'notice'}",
    );
    let mut app = application(&input, packages, &timer);
    let root = app.mount_launch(json!({})).unwrap();
    let left = app.child_mount(&root, "left").unwrap().clone();
    let right = app.child_mount(&root, "right").unwrap().clone();
    assert!(app.dispatch(&left, 0, "inc", Value::Null).is_err());
    for mount in [&root, &left, &right] {
        assert_eq!(app.frame(mount).unwrap().revision, 0);
    }
    assert_eq!(app.frame(&left).unwrap().state["count"], 10);
    close_all(&mut app, &root);
}

#[test]
fn child_parent_feedback_shares_step_and_byte_budget_and_rolls_back() {
    let timer = timer();
    let (input, packages, _) = fixture(
        MOUNT,
        CHILD_EVENT,
        "return frame({...state,label:props.label,revision:state.revision+1},[{name:'changed',value:1}]);",
        "return frame({...state,version:state.version+1,revision:state.revision+1});",
        "{changed:'notice'}",
    );
    let mut app = application(&input, packages, &timer);
    let root = app.mount_launch(json!({})).unwrap();
    let left = app.child_mount(&root, "left").unwrap().clone();
    let error = app
        .dispatch(&left, 0, "inc", Value::Null)
        .unwrap_err()
        .to_string();
    assert!(error.contains("limit"), "{error}");
    assert_eq!(app.frame(&root).unwrap().revision, 0);
    assert_eq!(app.frame(&left).unwrap().revision, 0);
    assert!(app.retiring_mounts().is_empty());
    close_all(&mut app, &root);
}

#[test]
fn mount_emission_can_remove_a_whole_private_subtree_without_renderer_ack() {
    let timer = timer();
    let (mut input, mut packages, contract) = fixture(
        "return frame({count:props.initial,label:props.label,revision:0},props.initial===20?[{name:'changed',value:1}]:[]);",
        CHILD_EVENT,
        support::CHILD_UPDATE,
        "return frame({...state,right:false,revision:state.revision+1});",
        "{changed:'remove'}",
    );
    let old = packages.pop().unwrap();
    let leaf = support::package(
        "dynamic.leaf",
        old.manifest().application.clone().unwrap(),
        Some(
            std::str::from_utf8(old.file("index.mjs").unwrap())
                .unwrap()
                .to_owned(),
        ),
    );
    let mut declaration = old.manifest().application.clone().unwrap();
    declaration
        .entries
        .get_mut(&EntryTarget::Desktop)
        .unwrap()
        .imports
        .insert(
            "leaf".into(),
            Import {
                contract,
                cardinality: Cardinality::One,
            },
        );
    let source=std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap().replace("children:[","children:[{kind:'component',key:'nested',binding:'leaf',member:0,on:{},props:{label:'Leaf',initial:1}},");
    let counter = support::package("dynamic.counter", declaration, Some(source));
    input.instances.get_mut("counter").unwrap().package = counter.application_ref().into();
    input.instances.get_mut("counter").unwrap().bindings.insert(
        "leaf".into(),
        vec![Target::Instance {
            path: vec!["leaf".into()],
            export: "Counter".into(),
        }],
    );
    input
        .instances
        .insert("leaf".into(), support::node(&leaf, 4));
    packages.extend([counter, leaf]);
    let mut app = application(&input, packages, &timer);
    let root = app.mount_launch(json!({})).unwrap();
    assert_eq!(app.frame(&root).unwrap().revision, 1);
    assert!(app.child_mount(&root, "right").is_err());
    let visible = app.mounted_subtree(&root).unwrap().len();
    assert_eq!(visible, 3);
    assert!(app.retiring_mounts().is_empty());
    app.retire(&root).unwrap();
    assert_eq!(
        app.retiring_mounts().len(),
        visible,
        "private orphan cannot become active or wait for a renderer"
    );
    for mount in app.retiring_mounts() {
        app.close(&mount, Ok(())).unwrap();
    }
    app.shutdown().unwrap();
}

struct NativeEmitter;
impl NativeComponent<()> for NativeEmitter {
    fn call(&self, call: ComponentCall<'_>) -> maka_ui::Result<ComponentOutput<Presentation<()>>> {
        let (state, events) = match call {
            ComponentCall::Mount { props } => (
                json!({"revision":0,"count":props["initial"],"label":props["label"]}),
                Vec::new(),
            ),
            ComponentCall::Update { state, props } => (
                json!({"revision":state["revision"].as_u64().unwrap()+1,"count":state["count"],"label":props["label"]}),
                Vec::new(),
            ),
            ComponentCall::Event { state, .. } => (
                json!({"revision":state["revision"].as_u64().unwrap()+1,"count":state["count"].as_u64().unwrap()+1,"label":state["label"]}),
                vec![Emission {
                    name: "changed".into(),
                    value: json!(7),
                }],
            ),
        };
        Ok(Frame {
            revision: state["revision"].as_u64().unwrap(),
            state,
            surface: None,
            view: Presentation::Tree(Node::Button {
                key: "inc".into(),
                label: "Native".into(),
            }),
        }
        .output(events))
    }
}
#[test]
fn native_emission_and_js_emission_share_the_same_contract_and_subscription() {
    let timer = timer();
    let (input, packages, contract) = fixture(
        MOUNT,
        CHILD_EVENT,
        support::CHILD_UPDATE,
        RECEIVE,
        "{changed:'notice'}",
    );
    let mut app = native_application(input, packages, contract, Rc::new(NativeEmitter), &timer);
    let root = app.mount_launch(json!({})).unwrap();
    let right = app.child_mount(&root, "right").unwrap().clone();
    app.dispatch(&right, 0, "inc", Value::Null).unwrap();
    assert_eq!(
        app.frame(&root).unwrap().state["seen"],
        json!([["right", "changed", "notice", 7]])
    );
    assert_eq!(app.frame(&right).unwrap().state["count"], 21);
    close_all(&mut app, &root);
}

fn native_application(
    mut input: Input,
    mut packages: Vec<Package>,
    contract: Contract,
    component: Rc<dyn NativeComponent<()>>,
    timer: &tokio::runtime::Runtime,
) -> Application {
    let old = packages.pop().unwrap();
    let mut declaration = old.manifest().application.clone().unwrap();
    let entry = declaration.entries.get_mut(&EntryTarget::Desktop).unwrap();
    entry.implementation = Implementation::Native {
        implementation: "test.events".into(),
    };
    entry.exports.get_mut("Counter").unwrap().contract = contract.clone();
    let native = support::package("native.counter", declaration, None);
    input.instances.get_mut("counter").unwrap().package = native.application_ref().into();
    let old = packages.pop().unwrap();
    let mut declaration = old.manifest().application.clone().unwrap();
    declaration
        .entries
        .get_mut(&EntryTarget::Desktop)
        .unwrap()
        .imports
        .get_mut("counter")
        .unwrap()
        .contract = contract.clone();
    let root = support::package(
        "dynamic.app",
        declaration,
        Some(
            std::str::from_utf8(old.file("index.mjs").unwrap())
                .unwrap()
                .to_owned(),
        ),
    );
    input.instances.get_mut("app").unwrap().package = root.application_ref().into();
    packages.extend([root, native]);
    let mut registry = NativeRegistry::with_surface(PresentationTarget::Desktop);
    registry
        .register("test.events", move |_| {
            Ok(BTreeMap::from([(
                "Counter".into(),
                NativeExport {
                    contract: contract.clone(),
                    value: NativeCapability::Component(component.clone()),
                },
            )]))
        })
        .unwrap();
    Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        registry,
    )
    .unwrap()
}
struct PaddedEmitter;
impl NativeComponent<()> for PaddedEmitter {
    fn call(&self, call: ComponentCall<'_>) -> maka_ui::Result<ComponentOutput<Presentation<()>>> {
        let (state, events) = match call {
            ComponentCall::Mount { props } => (
                json!({"revision":0,"count":props["initial"],"label":props["label"],"padding":"x".repeat(1_039_000)}),
                Vec::new(),
            ),
            ComponentCall::Update { state, props } => {
                let mut next = state.clone();
                next["revision"] = json!(state["revision"].as_u64().unwrap() + 1);
                next["label"] = props["label"].clone();
                (next, Vec::new())
            }
            ComponentCall::Event { state, .. } => {
                let mut next = state.clone();
                next["revision"] = json!(state["revision"].as_u64().unwrap() + 1);
                (
                    next,
                    (0..3)
                        .map(|_| Emission {
                            name: "changed".into(),
                            value: json!("e".repeat(262_144)),
                        })
                        .collect(),
                )
            }
        };
        Ok(Frame {
            revision: state["revision"].as_u64().unwrap(),
            state,
            surface: None,
            view: Presentation::Tree(Node::Button {
                key: "inc".into(),
                label: "Native".into(),
            }),
        }
        .output(events))
    }
}
#[test]
fn event_payloads_and_intermediate_frames_do_not_occupy_final_retained_scene_quota() {
    let timer = timer();
    let (input, packages, mut contract) = fixture(
        MOUNT,
        CHILD_EVENT,
        support::CHILD_UPDATE,
        "return frame({...state,revision:state.revision+1});",
        "{changed:'notice'}",
    );
    let Interface::Component { events, .. } = &mut contract.interface else {
        unreachable!()
    };
    events.insert(
        "changed".into(),
        json!({"type":"string","maxLength":262_144}),
    );
    let mut app = native_application(input, packages, contract, Rc::new(PaddedEmitter), &timer);
    // 32 native states put the actual retained scene just below 32 MiB. A
    // bounded event batch includes 768 KiB of transient payload and three
    // parent transitions but must not count them as newly retained state.
    let mut roots = Vec::new();
    for _ in 0..16 {
        roots.push(app.mount_launch(json!({})).unwrap());
    }
    let left = app.child_mount(&roots[0], "left").unwrap().clone();
    app.dispatch(&left, 0, "inc", Value::Null).unwrap();
    assert_eq!(app.frame(&roots[0]).unwrap().revision, 3);
    assert_eq!(app.frame(&left).unwrap().revision, 1);
    for root in roots {
        app.retire(&root).unwrap();
        for mount in app.retiring_mounts() {
            app.close(&mount, Ok(())).unwrap();
        }
    }
    app.shutdown().unwrap();
}
