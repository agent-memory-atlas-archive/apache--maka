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
use maka_event_log::{EventLog, root::RootOwner};
use serde_json::json;

fn fixture() -> (
    maka_plugin_core::composition::Input,
    Vec<maka_packages::Package>,
) {
    let (mut input, mut packages) = support::editor_events_fixture();
    let index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(index);
    let source = std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap()
        .replace("on:{changed:'editor_changed'}", "on:{changed:'editor_changed',checkpoint:'checkpoint'}")
        .replace("props:{initialText:'Start'", "props:{persistence:{key:'conversation',format:'test@1',dataSource:{host:'original'},metadata:{attachments:['original-artifact']},restore:state.checkpoint?.state.kind==='restore'?{owner:state.checkpoint.owner,version:state.checkpoint.state.version,clearText:false}:null},initialText:'Start'")
        .replace(support::EDITOR_RECEIVE, &format!("if(event.kind==='component'&&event.name==='checkpoint')return frame({{...state,revision:state.revision+1,checkpoint:event.event}});{}", support::EDITOR_RECEIVE));
    let root = support::package(
        "dynamic.app",
        old.manifest().application.clone().unwrap(),
        Some(source),
    );
    input.instances.get_mut("app").unwrap().package = root.application_ref().into();
    packages.push(root);
    (input, packages)
}
fn start(
    cx: &mut TestAppContext,
    timer: &tokio::runtime::Runtime,
    store: Arc<EventLog>,
    source: maka_plugin_core::deployment::DeploymentRef,
) -> Scene {
    let application = timer
        .block_on(Application::start_from_store(
            store,
            source,
            PresentationTarget::Desktop,
            timer.handle().clone(),
            maka_ui_js::execution(Default::default()),
            widgets::registry(),
            vec![],
        ))
        .unwrap();
    Scene::from_application(cx, application)
}
fn ready(scene: &Scene, cx: &mut TestAppContext) {
    // Deliberately do not paint/poll the scene: the worker must wake its window.
    await_action(cx, &scene.app, |app| {
        app.frame(&scene.root).unwrap().state["checkpoint"]["state"]["initialized"] == true
    });
}
fn close(scene: Scene, cx: &mut TestAppContext) {
    scene.close(cx);
    await_action(cx, &scene.app, |app| app.can_shutdown());
    cx.update(|cx| actions::quit_when_drained(&scene.app, cx));
    drop(scene);
    cx.run_until_parked();
}
#[gpui::test]
fn native_checkpoint_wakes_without_paint_and_restores_exact_selection_after_root_reopen(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    cx.executor().allow_parking();
    let (dir, namespaces, store) = stored_root(&timer);
    let (input, packages) = fixture();
    let source = timer
        .block_on(store.commit_application(uuid::Uuid::new_v4(), 0, input, vec![], packages))
        .unwrap();
    let scene = start(cx, &timer, store.clone(), source.clone());
    ready(&scene, cx);
    let original = cx.update(|cx| scene.editor_owner(cx));
    cx.update_window(scene.window, |_, window, cx| {
        scene.owner().update(cx, |owner, cx| {
            owner
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .focus(window, cx);
            owner.replace_text_in_range(Some(0..5), "甲👩‍💻z", window, cx);
            owner.state.update(cx, |state, cx| {
                assert!(state.set_selection_anchor_focus(14, 3, false, cx))
            });
        });
    })
    .unwrap();
    // The final selection notification need not paint before physical close.
    close(scene, cx);
    timer.block_on(store.shutdown()).unwrap();
    drop(store);
    let root = Arc::new(RootOwner::open(&dir.path().join("root"), &namespaces).unwrap());
    let store = Arc::new(timer.block_on(EventLog::for_root(root)).unwrap());
    let scene = start(cx, &timer, store.clone(), source);
    ready(&scene, cx);
    let checkpoint = scene.app.borrow().frame(&scene.root).unwrap().state["checkpoint"].clone();
    assert_eq!(checkpoint["state"]["kind"], "ready");
    cx.update_window(scene.window, |_, window, cx| {
        assert_ne!(scene.editor_owner(cx), original);
        scene.owner().update(cx, |owner, cx| {
            owner
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .focus(window, cx);
            owner.state.update(cx, |state, cx| {
                assert_eq!(state.value().as_ref(), "甲👩‍💻z");
                let selection = state.selected_text_range(true, window, cx).unwrap();
                assert_eq!(selection.range, 1..6);
                assert!(selection.reversed);
            });
            owner.replace_text_in_range(None, "R", window, cx);
            assert_eq!(owner.state.read(cx).value().as_ref(), "甲Rz");
        });
    })
    .unwrap();
    close(scene, cx);
    timer.block_on(store.shutdown()).unwrap();
    // Checkpoint metadata is available to the JS product without body exposure.
    assert!(!checkpoint.to_string().contains("甲"));
    assert_ne!(checkpoint["owner"], json!(original));
}
