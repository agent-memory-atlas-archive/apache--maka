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
use crate::support::scene::Node;
use maka_packages::Package;
use maka_plugin_core::{
    composition::{Input, PresentationTarget, Target},
    contract::{Cardinality, Contract, Import, Interface},
    package::{EntryTarget, Implementation},
};
use maka_ui_runtime::{
    Application, ComponentCall, ComponentOutput, Emission, Frame, NativeCapability,
    NativeComponent, NativeExport, NativeView,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, rc::Rc};

const CHILD_EVENT: &str = "return frame({...state,count:state.count+1,revision:state.revision+1},[{name:'changed',value:state.count+1}]);";
const RECEIVE: &str = "return frame({...state,revision:state.revision+1,seen:[...(state.seen??[]),[event.child,event.name,event.key,event.event]],total:(state.total??0)+event.event});";

pub(super) fn fixture(
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
        maka_ui_js::execution(Default::default()),
        crate::support::scene::registry(PresentationTarget::Desktop),
        Default::default(),
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
    fn call(
        &self,
        call: ComponentCall<'_>,
        _values: &maka_ui_runtime::abi::ValueBindings,
    ) -> maka_ui_runtime::Result<ComponentOutput<NativeView<()>>> {
        let raw = matches!(
            call,
            ComponentCall::Event {
                origin: maka_ui_runtime::EventOrigin::Raw,
                ..
            }
        );
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
        let mut output = Frame {
            revision: state["revision"].as_u64().unwrap(),
            state,
            surface: None,
            view: crate::support::scene::view(Node::Button {
                key: "inc".into(),
                label: "Native".into(),
            })?,
        }
        .output(events);
        output.decision = raw.then_some(maka_ui_runtime::InputDecision {
            prevent_default: true,
            stop_propagation: false,
        });
        Ok(output)
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
    let decision = app.dispatch(&right, 0, "inc", Value::Null).unwrap();
    assert!(decision.prevent_default && !decision.stop_propagation);
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
    entry.scene = None;
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
    let mut registry = crate::support::scene::registry(PresentationTarget::Desktop);
    registry
        .register("test.events", move |_, _values| {
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
        maka_ui_js::execution(Default::default()),
        registry,
        Default::default(),
    )
    .unwrap()
}
struct PaddedEmitter;
impl NativeComponent<()> for PaddedEmitter {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _values: &maka_ui_runtime::abi::ValueBindings,
    ) -> maka_ui_runtime::Result<ComponentOutput<NativeView<()>>> {
        let (state, events) = match call {
            ComponentCall::Mount { props } => (
                json!({"revision":0,"count":props["initial"],"label":props["label"],"padding":"x".repeat(930_000)}),
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
                            value: json!("e".repeat(39_000)),
                        })
                        .collect(),
                )
            }
        };
        Ok(Frame {
            revision: state["revision"].as_u64().unwrap(),
            state,
            surface: None,
            view: crate::support::scene::view(Node::Button {
                key: "inc".into(),
                label: "Native".into(),
            })?,
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
    // 36 native states put the actual retained scene just below 32 MiB.
    // A complete callback (930 kB state + 117 kB events) still fits 1 MiB,
    // but counting its transient events as retained would exceed 32 MiB.
    // Three parent transitions must also replace, not accumulate, state.
    let mut roots = Vec::new();
    for _ in 0..18 {
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

#[test]
fn raw_input_decision_waits_for_the_complete_causal_commit_and_is_not_retained() {
    let timer = timer();
    let (input, packages, _) = fixture(
        MOUNT,
        "const out=frame({...state,count:state.count+1,revision:state.revision+1},[{name:'changed',value:3}]);out.decision={preventDefault:true,stopPropagation:true};return out;",
        support::CHILD_UPDATE,
        RECEIVE,
        "{changed:'notice'}",
    );
    let mut app = application(&input, packages, &timer);
    let root = app.mount_launch(json!({})).unwrap();
    let child = app.child_mount(&root, "left").unwrap().clone();
    let decision = app.dispatch(&child, 0, "inc", Value::Null).unwrap();
    assert!(decision.prevent_default && decision.stop_propagation);
    assert_eq!(app.frame(&root).unwrap().state["total"], 3);
    assert_eq!(app.frame(&child).unwrap().state["count"], 11);
    assert!(app.dispatch(&child, 0, "inc", Value::Null).is_err());
    close_all(&mut app, &root);
}

#[test]
fn input_decisions_cannot_escape_a_failed_causal_transaction() {
    let timer = timer();
    for receive in [
        "throw new Error('parent rejects batch');",
        "const out=frame({...state,revision:state.revision+2});return out;",
    ] {
        let (input, packages, _) = fixture(
            MOUNT,
            "const out=frame({...state,count:99,revision:state.revision+1},[{name:'changed',value:3}]);out.decision={preventDefault:true};return out;",
            support::CHILD_UPDATE,
            receive,
            "{changed:'notice'}",
        );
        let mut app = application(&input, packages, &timer);
        let root = app.mount_launch(json!({})).unwrap();
        let child = app.child_mount(&root, "left").unwrap().clone();
        assert!(app.dispatch(&child, 0, "inc", Value::Null).is_err());
        assert_eq!(app.frame(&root).unwrap().revision, 0);
        assert_eq!(app.frame(&child).unwrap().revision, 0);
        assert_eq!(app.frame(&child).unwrap().state["count"], 10);
        close_all(&mut app, &root);
    }
}

#[test]
fn synchronous_parent_decisions_join_the_original_input_without_changing_origin() {
    let timer = timer();
    for source_decision in ["{}", "{preventDefault:true}"] {
        let (input, packages, _) = fixture(
            MOUNT,
            &format!(
                "const out=frame({{...state,revision:state.revision+1}},[{{name:'changed',value:3}}]);out.decision={source_decision};return out;"
            ),
            support::CHILD_UPDATE,
            "if(event.kind!=='component')throw new Error('origin was rewritten');const out=frame({...state,revision:state.revision+1});out.decision={preventDefault:false,stopPropagation:true};return out;",
            "{changed:'notice'}",
        );
        let mut app = application(&input, packages, &timer);
        let root = app.mount_launch(json!({})).unwrap();
        let child = app.child_mount(&root, "left").unwrap().clone();
        let decision = app.dispatch(&child, 0, "inc", Value::Null).unwrap();
        assert_eq!(decision.prevent_default, source_decision != "{}");
        assert!(
            decision.stop_propagation,
            "the checked parent joins the native decision"
        );
        close_all(&mut app, &root);
    }
}

#[test]
fn props_update_notifications_do_not_inherit_a_raw_transactions_input_admission() {
    let timer = timer();
    let (input, packages, _) = fixture(
        MOUNT,
        CHILD_EVENT,
        "return frame({...state,label:props.label,revision:state.revision+1},[{name:'changed',value:42}]);",
        "if(event.event===42){const out=frame({...state,revision:state.revision+1});out.decision={preventDefault:true};return out;}return frame({...state,version:state.version+1,revision:state.revision+1});",
        "{changed:'notice'}",
    );
    let mut app = application(&input, packages, &timer);
    let root = app.mount_launch(json!({})).unwrap();
    let child = app.child_mount(&root, "left").unwrap().clone();
    let error = app.dispatch(&child, 0, "inc", Value::Null).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("original synchronous input route")
    );
    assert_eq!(app.frame(&root).unwrap().revision, 0);
    assert_eq!(app.frame(&child).unwrap().revision, 0);
    assert_eq!(app.frame(&child).unwrap().state["count"], 10);
    close_all(&mut app, &root);
}

#[test]
fn lifecycle_outputs_cannot_decide_input_and_payload_cannot_forge_its_origin() {
    let timer = timer();
    let (input, packages, _) = fixture(
        "const out=frame({count:props.initial,label:props.label,revision:0});out.decision={};return out;",
        CHILD_EVENT,
        support::CHILD_UPDATE,
        RECEIVE,
        "{}",
    );
    let mut app = application(&input, packages, &timer);
    assert!(app.mount_launch(json!({})).is_err());
    app.shutdown().unwrap();
    let mut app = super::app(
        "if(event.kind!=='raw')throw new Error('wrong origin');const out=frame({count:state.count+1,revision:state.revision+1});out.decision={stopPropagation:true};return out;",
        &timer,
    );
    let root = app.mount_launch(json!({})).unwrap();
    let decision = app
        .dispatch(&root, 0, "inc", json!({"kind":"component","decision":{}}))
        .unwrap();
    assert_eq!(
        decision,
        maka_ui_runtime::InputDecision {
            prevent_default: false,
            stop_propagation: true
        }
    );
    close_all(&mut app, &root);
}

#[test]
fn presentation_change_updates_a_wide_retained_tree_atomically() {
    let timer = timer();
    let (mut input, mut packages) =
        support::composed_fixture(support::CHILD_UPDATE, support::PARENT_EVENT);
    let original = packages.pop().unwrap();
    let source = std::str::from_utf8(original.file("index.mjs").unwrap()).unwrap();
    let mut declaration = original.manifest().application.clone().unwrap();
    let leaf = support::package("dynamic.leaf", declaration.clone(), Some(source.into()));
    let contract = declaration.entries[&EntryTarget::Desktop].exports["Counter"]
        .contract
        .clone();
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
    let counter = support::package(
        "dynamic.counter",
        declaration,
        Some(format!(
            r#"{source}
        const originalFrame=frame;
        frame=state=>{{
            const out=originalFrame(state);
            out.frame.view.children.push(...Array.from({{length:40}},(_,i)=>({{
                kind:'component',key:'leaf-'+i,binding:'leaf',member:0,on:{{}},props:{{label:state.label,initial:i}}
            }})));
            return out;
        }};
    "#
        )),
    );
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
    let groups: Vec<_> = ["left", "right"]
        .into_iter()
        .map(|key| app.child_mount(&root, key).unwrap().clone())
        .collect();
    let children: Vec<_> = groups
        .iter()
        .flat_map(|group| {
            (0..40).map(|i| {
                app.child_mount(group, &format!("leaf-{i}"))
                    .unwrap()
                    .clone()
            })
        })
        .collect();
    for version in 1..=3 {
        app.dispatch(&root, version - 1, "parent", Value::Null)
            .unwrap();
        for (i, child) in children.iter().enumerate() {
            assert_eq!(
                app.child_mount(&groups[i / 40], &format!("leaf-{}", i % 40))
                    .unwrap(),
                child
            );
            let frame = app.frame(child).unwrap();
            assert_eq!(frame.revision, version);
            assert_eq!(
                frame.state["label"],
                format!("{} {version}", if i < 40 { "Left" } else { "Right" })
            );
            assert_eq!(frame.state["count"], i % 40);
        }
    }
    close_all(&mut app, &root);
}
