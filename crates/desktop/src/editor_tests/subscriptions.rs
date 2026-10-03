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

//! Original GPUI owner keeps custody after the last actual window closes.
use super::*;
use maka_plugin_core::contract::{Contract, Interface, Method, MethodMode, Transport};
use maka_plugin_core::package::{EntryTarget, Implementation};
use maka_ui::{NativeCapability, NativeExport, actions::*};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

struct State {
    waiting: Semaphore,
    closing: Semaphore,
    release: Semaphore,
    payload: Mutex<Option<std::sync::Weak<()>>>,
    snapshot: Mutex<Option<std::sync::Weak<dyn maka_plugins::call::ReadResource>>>,
}
struct Service(Arc<State>, bool);
impl NativeService for Service {
    fn call(&self, _: ServiceRequest) -> ServiceFuture {
        Box::pin(async { panic!("not a query") })
    }
    fn subscribe(&self, request: ServiceRequest, stop: CancellationToken) -> StreamFuture {
        if let Some(text) = request.resources.get("text") {
            assert_eq!(text.read(0..5).unwrap(), b"Start");
            *self.0.snapshot.lock().unwrap() = Some(Arc::downgrade(text));
        }
        let payload = Arc::new(());
        *self.0.payload.lock().unwrap() = Some(Arc::downgrade(&payload));
        pull(0, self.0.clone(), payload, stop, self.1)
    }
}
fn pull(
    index: usize,
    state: Arc<State>,
    payload: Arc<()>,
    stop: CancellationToken,
    inexact: bool,
) -> StreamFuture {
    Box::pin(async move {
        if index < 2 && !stop.is_cancelled() {
            return StreamCompletion::Item {
                value: if inexact {
                    json!(9_007_199_254_740_993_u64)
                } else {
                    json!(index)
                },
                next: pull(index + 1, state, payload, stop, inexact),
            };
        }
        state.waiting.add_permits(1);
        stop.cancelled().await;
        state.closing.add_permits(1);
        state.release.acquire().await.unwrap().forget();
        drop(payload);
        StreamCompletion::Finished(ServiceCompletion {
            result: Ok(json!(2)),
            cleanup: Ok(()),
        })
    })
}

#[gpui::test]
fn original_foreground_owner_delivers_items_and_drains_real_stream_after_last_window_closes(
    cx: &mut TestAppContext,
) {
    original_stream(cx, false, false);
}
#[gpui::test]
fn inexact_native_item_fails_javascript_delivery_and_still_closes_original_executor(
    cx: &mut TestAppContext,
) {
    original_stream(cx, true, false);
}
#[gpui::test]
fn declarative_stream_captures_original_text_once_and_keeps_snapshot_until_real_cleanup(
    cx: &mut TestAppContext,
) {
    original_stream(cx, false, true);
}
fn original_stream(cx: &mut TestAppContext, inexact: bool, capture: bool) {
    cx.executor().allow_parking();
    let timer = timer();
    let mut contract = Contract {
        name: "test.Stream".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Service {
            methods: BTreeMap::from([(
                "read".into(),
                Method {
                    mode: MethodMode::Subscription,
                    input: json!({}),
                    output: json!({"type":"integer"}),
                    resources: BTreeMap::new(),
                },
            )]),
        },
    };
    if capture {
        let Interface::Service { methods } = &mut contract.interface else {
            unreachable!()
        };
        let Interface::Service { methods: reader } = maka_ui::text::reader_contract().interface
        else {
            unreachable!()
        };
        methods.get_mut("read").unwrap().resources = reader["read"].resources.clone();
    }
    let (mut input, mut packages) = support::resource_fixture();
    let original = packages.remove(
        packages
            .iter()
            .position(|p| p.manifest().id == "native.reader")
            .unwrap(),
    );
    let mut decl = original.manifest().application.clone().unwrap();
    let entry = decl.entries.get_mut(&EntryTarget::Desktop).unwrap();
    entry.implementation = Implementation::Native {
        implementation: "test.stream".into(),
    };
    entry.exports.get_mut("Reader").unwrap().contract = contract.clone();
    let reader = support::package("native.reader", decl, None);
    input.instances.get_mut("reader").unwrap().package = reader.application_ref().into();
    packages.push(reader);
    let original = packages.remove(
        packages
            .iter()
            .position(|p| p.manifest().id == "dynamic.app")
            .unwrap(),
    );
    let mut decl = original.manifest().application.clone().unwrap();
    decl.entries
        .get_mut(&EntryTarget::Desktop)
        .unwrap()
        .imports
        .get_mut("reader")
        .unwrap()
        .contract = contract.clone();
    let mut source = std::str::from_utf8(original.file("index.mjs").unwrap())
        .unwrap()
        .to_string();
    source = source.replace("function frame(state){return {actions:[],events:[],frame:",
        &format!("function frame(state){{return {{subscriptions:state.streaming?[{}]:[],actions:[],events:[],frame:",
            json!({"key":"feed","binding":"reader","member":0,"method":"read","input":null,"resources":if capture {json!({"text":{"child":"editor","method":"snapshot","input":{"kind":"current"}}})} else {json!({})}})));

    let start = source.find("else if(event.key==='read'){").unwrap();
    let end = start + source[start..].find("return out;}").unwrap() + "return out;}".len();
    source.replace_range(
        start..end,
        "else if(event.key==='read')next.streaming=true;",
    );
    let root = support::package("dynamic.app", decl, Some(source));
    input.instances.get_mut("app").unwrap().package = root.application_ref().into();
    packages.push(root);
    let state = Arc::new(State {
        waiting: Semaphore::new(0),
        closing: Semaphore::new(0),
        release: Semaphore::new(0),
        payload: Mutex::new(None),
        snapshot: Mutex::new(None),
    });
    let service = Arc::new(Service(state.clone(), inexact));
    let mut native = widgets::registry();
    native
        .register("test.stream", move |_, _| {
            Ok(BTreeMap::from([(
                "Reader".into(),
                NativeExport {
                    contract: contract.clone(),
                    value: NativeCapability::Service(service.clone()),
                },
            )]))
        })
        .unwrap();
    let scene = Scene::with_registry(
        cx,
        &timer,
        false,
        (input, packages),
        native,
        Default::default(),
    );
    scene.draw(cx);
    cx.update_window(scene.window, |_, window, cx| {
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "read", Value::Null)
            .unwrap();
        scene
            .surface
            .update(cx, |surface, cx| surface.sync_native(window, cx).unwrap());
    })
    .unwrap();
    await_action(cx, &scene.app, |_| state.waiting.available_permits() == 1);
    let feedback = scene.app.borrow().frame(&scene.root).unwrap().state["outcome"].clone();
    if inexact {
        assert_eq!(feedback["status"], "failed");
        assert!(feedback["reason"].as_str().unwrap().contains("integer"));
        assert_eq!(state.closing.available_permits(), 1);
    } else {
        assert_eq!(feedback, json!({"status":"updated","value":1}));
    }
    assert_eq!(scene.app.borrow().accepted_action_count(), 1);
    let snapshot = state.snapshot.lock().unwrap().clone();
    if capture {
        let original = snapshot.as_ref().unwrap().upgrade().unwrap();
        assert_eq!(original.read(0..5).unwrap(), b"Start");
        let owner = scene.owner();
        cx.update_window(scene.window, |_, window, cx| {
            owner.update(cx, |owner, cx| {
                owner
                    .state
                    .read(cx)
                    .presentation()
                    .focus_handle()
                    .focus(window, cx);
                owner.replace_text_in_range(Some(0..5), "Changed", window, cx)
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(
            owner.read_with(cx, |owner, cx| owner.state.read(cx).value().to_string()),
            "Changed"
        );
        assert_eq!(original.read(0..5).unwrap(), b"Start");
        assert_eq!(
            scene.app.borrow().accepted_action_count(),
            1,
            "feedback must not recapture/restart the declaration"
        );
        drop(original);
        drop(owner);
    }
    let payload = state.payload.lock().unwrap().as_ref().unwrap().clone();
    scene.close(cx);
    await_action(cx, &scene.app, |_| state.closing.available_permits() == 1);
    assert!(cx.update(|cx| cx.windows().is_empty()));
    assert!(!scene.app.borrow().can_shutdown());
    assert!(payload.upgrade().is_some());
    if let Some(snapshot) = &snapshot {
        assert_eq!(snapshot.upgrade().unwrap().read(0..5).unwrap(), b"Start");
    }
    state.release.add_permits(1);
    await_action(cx, &scene.app, |app| app.accepted_action_count() == 0);
    assert!(payload.upgrade().is_none());
    if let Some(snapshot) = &snapshot {
        assert!(snapshot.upgrade().is_none());
    }
    assert!(scene.app.borrow().can_shutdown());
}
