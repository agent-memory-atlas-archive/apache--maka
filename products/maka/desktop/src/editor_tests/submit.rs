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
use gpui_kit::{KeyDownEvent, Keystroke, PlatformInput};
use maka_plugin_core::package::Implementation;

fn fixture() -> (
    maka_plugin_core::composition::Input,
    Vec<maka_packages::Package>,
) {
    let (mut input, packages) = support::resource_fixture();
    let packages = packages.into_iter().map(|original| {
        let declaration = original.manifest().application.as_ref().unwrap().declaration().clone();
        let entry = declaration.entries.values().next().unwrap();
        let Implementation::JavaScript {module,..} = &entry.implementation else { return original; };
        let source = std::str::from_utf8(original.file(module).unwrap()).unwrap()
            .replace("on:{changed:'changed'}", "on:{changed:'changed',submitted:'submitted'}")
            .replace("softWrap:true,", "softWrap:true,submit:'primary_enter',")
            .replace("name:'changed',value:event.event", "name:event.name,value:event.event")
            .replace("else if(event.kind==='component')next.observation=event.event;", "else if(event.kind==='component'){next.observation=event.event;if(event.name==='submitted')next.submissions=(state.submissions??0)+1;}");
        let package = support::package(&original.manifest().id, declaration, Some(source));
        for node in input.instances.values_mut().filter(|node| matches!(&node.package,maka_plugin_core::package::PackageSelection::Pinned {package} if package.id == original.manifest().id)) {
            node.package = package.application_ref().into();
        }
        package
    }).collect();
    (input, packages)
}
fn press(scene: &Scene, cx: &mut TestAppContext, key: &str) {
    cx.update_window(scene.window, |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse(key).unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
    })
    .unwrap();
    scene.settle(cx);
}
fn submissions(scene: &Scene) -> u64 {
    scene.app.borrow().frame(&scene.root).unwrap().state["submissions"]
        .as_u64()
        .unwrap_or(0)
}

#[gpui::test]
fn original_desktop_paste_and_submit_stay_immediate_during_a_slow_js_transaction(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let (mut input, packages) = fixture();
    let packages = packages.into_iter().map(|original| {
        if original.manifest().id != "dynamic.app" { return original; }
        let declaration = original.manifest().application.as_ref().unwrap().declaration().clone();
        let source = std::str::from_utf8(original.file("index.mjs").unwrap()).unwrap().replace(
            "const next={...state,revision:state.revision+1};",
            "if(event.kind==='raw'&&event.key==='slow'){const until=Date.now()+100;while(Date.now()<until){}}const next={...state,revision:state.revision+1};",
        ).replace("{kind:'button',key:'read',label:'Capture draft'}", "{kind:'button',key:'slow',label:'Slow business update'},{kind:'button',key:'read',label:'Capture draft'}");
        let package = support::package("dynamic.app", declaration, Some(source));
        input.instances.get_mut("app").unwrap().package = package.application_ref().into();
        package
    }).collect();
    let scene = Scene::from_packages(cx, &timer, false, (input, packages));
    scene.draw(cx);
    let owner = scene.owner();
    cx.update_window(scene.window, |_, window, cx| {
        owner
            .read(cx)
            .state
            .read(cx)
            .presentation()
            .focus_handle()
            .focus(window, cx);
    })
    .unwrap();
    let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
    let at = std::time::Instant::now();
    scene
        .app
        .borrow_mut()
        .dispatch(&scene.root, revision, "slow", serde_json::Value::Null)
        .unwrap();
    assert!(at.elapsed() < std::time::Duration::from_millis(30));
    assert!(scene.app.borrow().ui_pending());
    let at = std::time::Instant::now();
    cx.update_window(scene.window, |_, window, cx| {
        // Production IME/text input and key delivery, without a fixture await.
        owner.update(cx, |owner, cx| {
            owner.replace_text_in_range(Some(0..5), "fast🙂", window, cx)
        });
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse(if cfg!(target_os = "macos") {
                    "cmd-enter"
                } else {
                    "ctrl-enter"
                })
                .unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "fast🙂");
    })
    .unwrap();
    assert!(
        at.elapsed() < std::time::Duration::from_millis(30),
        "native input waited for JS: {:?}",
        at.elapsed()
    );
    assert_eq!(
        scene.app.borrow().frame(&scene.root).unwrap().revision,
        revision
    );
    assert_eq!(submissions(&scene), 0);
    scene.settle(cx);
    assert_eq!(submissions(&scene), 1);
    assert_eq!(
        scene.app.borrow().frame(&scene.root).unwrap().state["observation"]["snapshot"]["text"]["bytes"],
        "fast🙂".len()
    );
    drop(owner);
    scene.close(cx);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while cx.update(|cx| cx.has_global::<actions::Owner>()) {
        cx.run_until_parked();
        assert!(
            std::time::Instant::now() < until,
            "original desktop cleanup did not finish"
        );
        std::thread::park_timeout(std::time::Duration::from_millis(1));
    }
    assert!(scene.app.borrow().can_shutdown());
    assert_eq!(scene.resources.borrow().budget.usage(), (0, 0, 0));
}
#[gpui::test]
fn textarea_submit_observes_latest_native_draft_without_leaving_focus_or_inserting_newline(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, fixture());
    scene.draw(cx);
    let owner = scene.owner();
    // Do not pump the queued changed event before submission: submit itself
    // must read the original native state at this synchronous boundary.
    cx.update_window(scene.window, |_, window, cx| {
        let state = owner.read(cx).state.clone();
        state.update(cx, |state, cx| {
            state.focus(window, cx);
            state.set_value("中文🙂", window, cx);
        });
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse(if cfg!(target_os = "macos") {
                    "cmd-enter"
                } else {
                    "ctrl-enter"
                })
                .unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        assert!(
            owner
                .read(cx)
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
    })
    .unwrap();
    scene.settle(cx);
    assert_eq!(submissions(&scene), 1);
    let observation: EditorObservation = serde_json::from_value(
        scene.app.borrow().frame(&scene.root).unwrap().state["observation"].clone(),
    )
    .unwrap();
    cx.update_window(scene.window, |_, window, cx| {
        assert_eq!(
            owner
                .update(cx, |s, cx| s.read_text(
                    &observation.snapshot.text,
                    0..observation.snapshot.text.bytes,
                    window,
                    cx
                ))
                .unwrap(),
            "中文🙂"
        );
    })
    .unwrap();
    press(&scene, cx, "enter");
    assert_eq!(submissions(&scene), 1);
    assert!(cx.update(|cx| owner.read(cx).state.read(cx).value().contains('\n')));
    scene.close(cx);
}
#[gpui::test]
fn textarea_submit_refuses_marked_readonly_unfocused_and_retired_native_input(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, fixture());
    scene.draw(cx);
    let owner = scene.owner();
    cx.update_window(scene.window, |_, window, cx| {
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |s, cx| s.focus(window, cx));
        owner.update(cx, |s, cx| {
            s.replace_and_mark_text_in_range(Some(0..5), "中文", Some(0..2), window, cx)
        });
    })
    .unwrap();
    let key = if cfg!(target_os = "macos") {
        "cmd-enter"
    } else {
        "ctrl-enter"
    };
    press(&scene, cx, key);
    assert_eq!(submissions(&scene), 0);
    cx.update_window(scene.window, |_, window, cx| {
        owner.update(cx, |s, cx| s.unmark_text(window, cx));
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |s, cx| s.set_readonly(true, cx));
    })
    .unwrap();
    press(&scene, cx, key);
    assert_eq!(submissions(&scene), 0);
    cx.update_window(scene.window, |_, window, cx| {
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |s, cx| s.set_readonly(false, cx));
        window.blur(cx);
    })
    .unwrap();
    press(&scene, cx, key);
    assert_eq!(submissions(&scene), 0);
    cx.update_window(scene.window, |_, window, cx| {
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "remove", serde_json::json!({}))
            .unwrap();
        scene
            .surface
            .update(cx, |s, cx| s.sync_native(window, cx))
            .unwrap();
    })
    .unwrap();
    press(&scene, cx, key);
    assert_eq!(submissions(&scene), 0);
    scene.close(cx);
}
