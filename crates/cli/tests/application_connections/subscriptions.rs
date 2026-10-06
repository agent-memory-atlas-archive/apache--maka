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

use super::*;
use maka_plugin_core::{
    contract::{Contract, Interface, MethodMode},
    package::EntryTarget,
};
use maka_plugins::package::{MANIFEST_FILE, Package};
use maka_ui::actions::{ActionTask, TaskCompletion};
use std::time::Duration;

fn subscription_contract() -> Contract {
    let mut contract = support::business();
    let Interface::Service { methods } = &mut contract.interface else {
        unreachable!()
    };
    methods.get_mut("read").unwrap().mode = MethodMode::Subscription;
    contract
}
fn fixture(target: PresentationTarget) -> Fixture {
    let contract = subscription_contract();
    let (mut host_input, host_packages) = support::host();
    let mut packages = Vec::new();
    for old in host_packages {
        if old.manifest().id == "local.receipts" {
            packages.push(old);
            continue;
        }
        let mut declaration = old.manifest().application.clone().unwrap();
        let entry = declaration.entries.get_mut(&EntryTarget::Host).unwrap();
        for import in entry.imports.values_mut() {
            if import.contract.name == contract.name {
                import.contract = contract.clone();
            }
        }
        for export in entry.exports.values_mut() {
            if export.contract.name == contract.name {
                export.contract = contract.clone();
            }
        }
        let source=(old.manifest().id=="local.business").then(||format!(r#"
            let starts=0;
            export function activate(){{return {{Counter:{{contract:{},service:['service']}}}};}}
            export const service={{async read(input,observation,context){{
                const start=++starts;
                for(let i=0;i<35;i++)await context.invoke({{operation:'emit',value:{{start,input,i}}}});
                return {{done:true,start}};
            }},write(){{return {{}};}}}};
        "#,serde_json::to_string(&contract).unwrap()));
        let name = old.manifest().id.clone();
        let package = support::counter::package(&name, declaration, source);
        for node in host_input.instances.values_mut() {
            if node.package == old.application_ref().into() {
                node.package = package.application_ref().into();
            }
        }
        packages.push(package);
    }
    let (mut input, mut ui_packages, mut observations) = support::ui(target);
    for observation in &mut observations {
        if observation.declaration.contract.name == contract.name {
            observation.declaration.contract = contract.clone();
        }
    }
    let old = ui_packages.pop().unwrap();
    let mut declaration = old.manifest().application.clone().unwrap();
    for entry in declaration.entries.values_mut() {
        entry.imports.get_mut("business").unwrap().contract = contract.clone();
    }
    let mut files = BTreeMap::from([(
        MANIFEST_FILE.into(),
        serde_json::to_vec(&json!({"schemaVersion":2,"id":"stream.ui","application":declaration}))
            .unwrap(),
    )]);
    for (target, module) in [
        (EntryTarget::Desktop, "index.mjs"),
        (EntryTarget::Tui, "terminal.mjs"),
    ] {
        let surface = if target == EntryTarget::Desktop {
            "{kind:'window',binding:'surface',title:'Stream',width:640,height:400}"
        } else {
            "{kind:'terminal',binding:'surface'}"
        };
        files.insert(module.into(),format!(r#"
            export function activate(){{return {{App:{{contract:{},kind:'component',component:['app']}}}};}}
            function frame(state){{return {{events:[],actions:[],subscriptions:state.active?[{{key:'feed',binding:'business',member:0,method:'read',input:state.selector,resources:{{}}}}]:[],frame:{{state,revision:state.revision,surface:{surface},view:{{kind:'stack',key:'root',direction:'column',padding:1,gap:1,background:1513239,children:[{{kind:'text',key:'progress',text:JSON.stringify(state.seen),color:16777215}},{{kind:'button',key:'read',label:'Start / stop'}},{{kind:'button',key:'replace',label:'Replace'}},{{kind:'button',key:'error',label:'Throw'}},{{kind:'button',key:'invalid',label:'Invalid update'}}]}}}}}};}}
            export const app={{mount(){{return frame({{active:false,selector:0,seen:[],revision:0,outcome:null}});}},
                event(state,input){{
                    if(input.kind==='action'){{return frame({{...state,revision:state.revision+1,seen:input.event.status==='updated'?[...state.seen,input.event.value]:state.seen,outcome:input.event}});}}
                    if(input.key==='error')throw Error('raw failed realm');
                    const next={{...state,revision:state.revision+1}};
                    if(input.key==='read')next.active=!state.active;
                    if(input.key==='replace'){{next.selector++;next.seen=[];}}
                    const result=frame(next);
                    if(input.key==='invalid'){{result.subscriptions=[];result.frame.view={{kind:'invalid'}};}}
                    return result;
                }},update(state){{return frame({{...state,revision:state.revision+1}});}}}};
        "#,serde_json::to_string(&declaration.entries[&target].exports["App"].contract).unwrap()).into_bytes());
    }
    let package = Package::new(files).unwrap();
    input.instances.get_mut("app").unwrap().package = package.application_ref().into();
    ui_packages.push(package);
    Fixture::from_sources((host_input, packages), (input, ui_packages, observations))
}
fn dispatch(app: &mut Application, root: &maka_plugin_core::runtime::Mount, key: &str) {
    app.dispatch(root, app.frame(root).unwrap().revision, key, Value::Null)
        .unwrap();
}
fn start(app: &mut Application, root: &maka_plugin_core::runtime::Mount) -> ActionTask {
    app.capture_actions(root, |_| panic!("no resources"))
        .unwrap();
    app.start_task(root).unwrap().expect("actual original task")
}
fn run(f: &Fixture, task: ActionTask) -> (Uuid, TaskCompletion) {
    (
        task.id,
        f.timer.block_on(async {
            tokio::time::timeout(Duration::from_secs(15), task.future)
                .await
                .unwrap()
        }),
    )
}
fn finish(
    f: &Fixture,
    app: &mut Application,
    root: &maka_plugin_core::runtime::Mount,
    mut task: ActionTask,
) {
    loop {
        let (id, completion) = run(f, task);
        let next = app.finish_task(id, completion).unwrap();
        app.deliver_action_results(root, |_, _, _| Ok(false))
            .unwrap();
        match next {
            Some(value) => task = value,
            None => break,
        }
    }
}

#[test]
fn actual_javascript_host_streams_into_both_presentations_without_restarting_on_feedback() {
    for target in [PresentationTarget::Desktop, PresentationTarget::Tui] {
        let f = fixture(target);
        let mut connections = f.connect(f.configuration(true, false));
        let mut app = f.ui_target(&mut connections, target);
        let root = app.mount_launch(json!({})).unwrap();
        dispatch(&mut app, &root, "read");
        let task = start(&mut app, &root);
        finish(&f, &mut app, &root, task);
        let state = &app.frame(&root).unwrap().state;
        assert_eq!(state["seen"].as_array().unwrap().len(), 35);
        for i in 0..35 {
            assert_eq!(state["seen"][i], json!({"start":1,"input":0,"i":i}));
        }
        assert_eq!(
            state["outcome"],
            json!({"status":"succeeded","value":{"done":true,"start":1}})
        );
        assert!(app.start_task(&root).unwrap().is_none());
        // A natural end does not replay just because a frame is updated.
        dispatch(&mut app, &root, "replace");
        let task = start(&mut app, &root);
        finish(&f, &mut app, &root, task);
        assert_eq!(app.frame(&root).unwrap().state["seen"][0]["start"], 2);
        stop(&f, app, &root, &mut connections);
    }
}

#[test]
fn feedback_backpressure_and_changed_request_do_not_redirect_old_items_or_terminal_result() {
    let f = fixture(PresentationTarget::Desktop);
    let mut connections = f.connect(f.configuration(true, false));
    let mut app = f.ui(&mut connections);
    let root = app.mount_launch(json!({})).unwrap();
    dispatch(&mut app, &root, "read");
    let (id, item) = run(&f, start(&mut app, &root));
    let successor = app.finish_task(id, item).unwrap().unwrap();
    let mut future = successor.future;
    f.timer.block_on(async {
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                future.as_mut().poll(cx).is_pending()
            ))
            .await
        );
    });
    // Replace after item receipt but before its feedback callback runs.
    dispatch(&mut app, &root, "replace");
    assert_eq!(
        app.deliver_action_results(&root, |_, _, _| Ok(false))
            .unwrap(),
        0
    );
    let old = run(&f, ActionTask { id, future });
    assert!(app.finish_task(old.0, old.1).unwrap().is_none());
    assert_eq!(
        app.deliver_action_results(&root, |_, _, _| Ok(false))
            .unwrap(),
        0
    );
    assert_eq!(app.frame(&root).unwrap().state["seen"], json!([]));
    let next = start(&mut app, &root);
    assert_ne!(next.id, id);
    finish(&f, &mut app, &root, next);
    assert_eq!(app.frame(&root).unwrap().state["seen"][0]["start"], 2);
    stop(&f, app, &root, &mut connections);
}

#[test]
fn healthy_rollback_keeps_original_demand_and_failed_realm_cancels_waiting_ack() {
    let f = fixture(PresentationTarget::Desktop);
    let mut connections = f.connect(f.configuration(true, false));
    let mut app = f.ui(&mut connections);
    let root = app.mount_launch(json!({})).unwrap();
    dispatch(&mut app, &root, "read");
    let (id, item) = run(&f, start(&mut app, &root));
    let successor = app.finish_task(id, item).unwrap().unwrap();
    assert!(
        app.dispatch(
            &root,
            app.frame(&root).unwrap().revision,
            "invalid",
            Value::Null
        )
        .is_err()
    );
    assert!(!app.is_retiring(&root));
    assert_eq!(
        app.deliver_action_results(&root, |_, _, _| Ok(false))
            .unwrap(),
        1
    );
    let (id, item) = run(&f, successor);
    let successor = app.finish_task(id, item).unwrap().unwrap();
    assert_eq!(
        app.frame(&root).unwrap().state["seen"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        app.dispatch(
            &root,
            app.frame(&root).unwrap().revision,
            "error",
            Value::Null
        )
        .is_err()
    );
    assert!(app.is_retiring(&root));
    let (id, completion) = run(&f, successor);
    assert!(app.finish_task(id, completion).unwrap().is_none());
    assert_eq!(app.accepted_action_count(), 0);
    app.deliver_action_results(&root, |_, _, _| Ok(false))
        .unwrap();
    stop(&f, app, &root, &mut connections);
}

#[test]
fn retired_surface_cancels_stream_but_work_requires_original_host_completion() {
    let f = fixture(PresentationTarget::Desktop);
    let mut connections = f.connect(f.configuration(true, false));
    let mut app = f.ui(&mut connections);
    let root = app.mount_launch(json!({})).unwrap();
    dispatch(&mut app, &root, "read");
    let (id, item) = run(&f, start(&mut app, &root));
    let successor = app.finish_task(id, item).unwrap().unwrap();
    app.retire(&root).unwrap();
    app.close(&root, Ok(())).unwrap();
    assert_eq!(app.accepted_action_count(), 1);
    assert!(!app.can_shutdown());
    let (id, completion) = run(&f, successor);
    assert!(app.finish_task(id, completion).unwrap().is_none());
    assert_eq!(
        app.deliver_action_results(&root, |_, _, _| Ok(false))
            .unwrap(),
        0
    );
    assert!(app.can_shutdown());
    app.shutdown().unwrap();
    drop(app);
    f.timer.block_on(connections.close()).unwrap();
    f.reopened("host");
}
