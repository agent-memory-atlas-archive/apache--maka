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
use gpui_kit::{FocusHandle, base::input::InputState};
use maka_plugin_core::{
    composition::Target,
    contract::{Cardinality, Import},
    package::{EntryTarget, Implementation},
};
use serde_json::json;
use widgets::input::InputOwner;

fn fixture() -> (
    maka_plugin_core::composition::Input,
    Vec<maka_packages::Package>,
) {
    let (mut input, mut packages) = appearance::scoped_fixture();
    let editor = packages
        .iter()
        .find(|p| p.manifest().id == "native.editor")
        .unwrap();
    let mut declaration = editor.manifest().application.clone().unwrap();
    let entry = declaration.entries.get_mut(&EntryTarget::Desktop).unwrap();
    entry.implementation = Implementation::Native {
        implementation: "maka.ui.input".into(),
    };
    let mut export = entry.exports.remove("Textarea").unwrap();
    export.contract = maka_ui_runtime::input::text_input_contract();
    entry.exports.insert("TextInput".into(), export.clone());
    export.contract = maka_ui_runtime::input::secret_input_contract();
    entry.exports.insert("SecretInput".into(), export);
    let single = support::package("native.single", declaration, None);
    input
        .instances
        .insert("single".into(), support::node(&single, 993));
    let index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(index);
    let mut declaration = old.manifest().application.clone().unwrap();
    declaration
        .entries
        .get_mut(&EntryTarget::Desktop)
        .unwrap()
        .imports
        .insert(
            "single".into(),
            Import {
                contract: maka_ui_runtime::input::text_input_contract(),
                cardinality: Cardinality::One,
            },
        );
    let mut appearance = support::brand_appearance()["textarea"].clone();
    appearance.as_object_mut().unwrap().remove("scrollbar");
    let props = json!({"appearance":appearance,"accessibilityLabel":"Single field","readOnly":false,"disabled":false,"tabIndex":0});
    let source = std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap().replace("},{kind:'component',key:'nativeLabel'", &format!("}},{{kind:'component',key:'single',binding:'single',member:0,on:{{}},props:{props}}},{{kind:'component',key:'nativeLabel'"));
    let root = support::package("dynamic.app", declaration, Some(source));
    let app = input.instances.get_mut("app").unwrap();
    app.package = root.application_ref().into();
    app.bindings.insert(
        "single".into(),
        vec![Target::Instance {
            path: vec!["single".into()],
            export: "TextInput".into(),
        }],
    );
    packages.extend([single, root]);
    (input, packages)
}
fn second(scene: &Scene) -> Entity<TextareaOwner> {
    let mount = scene
        .app
        .borrow()
        .child_mount(&scene.root, "second")
        .unwrap()
        .clone();
    scene.resources.borrow().owners[&mount.id().to_string()]
        .state
        .downcast_ref::<Entity<TextareaOwner>>()
        .unwrap()
        .clone()
}
fn single(scene: &Scene, cx: &TestAppContext) -> Entity<InputState> {
    let mount = scene
        .app
        .borrow()
        .child_mount(&scene.root, "single")
        .unwrap()
        .clone();
    let owner = scene.resources.borrow().owners[&mount.id().to_string()]
        .state
        .downcast_ref::<Entity<InputOwner>>()
        .unwrap()
        .clone();
    cx.read(|cx| owner.read(cx).state.clone())
}
fn focused(scene: &Scene, expected: &FocusHandle, cx: &mut TestAppContext) {
    cx.update_window(scene.window, |_, window, _| {
        assert!(expected.is_focused(window))
    })
    .unwrap();
}

#[gpui::test]
fn plain_form_tab_navigation_preserves_original_buffer_selection_and_history(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, fixture());
    scene.draw(cx);
    let first = cx.read(|cx| scene.owner().read(cx).state.clone());
    let second = cx.read(|cx| second(&scene).read(cx).state.clone());
    let single = single(&scene, cx);
    let handles = cx.read(|cx| {
        [
            first.read(cx).presentation().focus_handle().clone(),
            second.read(cx).presentation().focus_handle().clone(),
            single.read(cx).presentation().focus_handle().clone(),
        ]
    });
    cx.update_window(scene.window, |_, window, cx| {
        first.update(cx, |state, cx| {
            state.focus(window, cx);
            state.replace_all("A🙂é\n保留", window, cx);
            state.set_selected_text_range(1..3, window, cx);
        })
    })
    .unwrap();
    scene.draw(cx);
    let before = cx
        .update_window(scene.window, |_, window, cx| {
            first.update(cx, |s, cx| {
                (
                    s.value().to_string(),
                    s.selected_text_range(true, window, cx)
                        .map(|r| (r.range, r.reversed)),
                    s.can_undo(),
                    s.can_redo(),
                )
            })
        })
        .unwrap();
    cx.simulate_keystrokes(scene.window, "tab");
    focused(&scene, &handles[1], cx);
    scene.draw(cx);
    cx.simulate_keystrokes(scene.window, "tab");
    focused(&scene, &handles[2], cx);
    scene.draw(cx);
    cx.simulate_keystrokes(scene.window, "shift-tab");
    focused(&scene, &handles[1], cx);
    scene.draw(cx);
    cx.simulate_keystrokes(scene.window, "shift-tab");
    focused(&scene, &handles[0], cx);
    let after = cx
        .update_window(scene.window, |_, window, cx| {
            first.update(cx, |s, cx| {
                (
                    s.value().to_string(),
                    s.selected_text_range(true, window, cx)
                        .map(|r| (r.range, r.reversed)),
                    s.can_undo(),
                    s.can_redo(),
                )
            })
        })
        .unwrap();
    assert_eq!(before, after);
    cx.simulate_keystrokes(scene.window, "cmd-z");
    cx.read(|cx| assert_eq!(first.read(cx).value().as_ref(), "Start"));
    scene.close(cx);
}

#[gpui::test]
fn composing_tab_is_owned_by_single_line_and_readonly_multiline_inputs(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, fixture());
    scene.draw(cx);
    let single = single(&scene, cx);
    let first = cx.read(|cx| scene.owner().read(cx).state.clone());
    cx.update_window(scene.window, |_, window, cx| {
        single.update(cx, |s, cx| {
            s.focus(window, cx);
            s.replace_and_mark_text_in_range(None, "中文", Some(0..2), window, cx);
        })
    })
    .unwrap();
    scene.draw(cx);
    for key in ["tab", "shift-tab"] {
        cx.simulate_keystrokes(scene.window, key);
        cx.update_window(scene.window, |_, window, cx| {
            single.update(cx, |s, cx| {
                assert!(s.presentation().focus_handle().is_focused(window));
                assert_eq!(s.value().as_ref(), "中文");
                assert_eq!(s.marked_text_range(window, cx), Some(0..2));
            })
        })
        .unwrap();
    }
    cx.update_window(scene.window, |_, window, cx| {
        first.update(cx, |s, cx| {
            s.focus(window, cx);
            s.replace_and_mark_text_in_range(Some(0..5), "保留", Some(0..2), window, cx);
        })
    })
    .unwrap();
    appearance::event(&scene, json!({"readOnly":true}), cx);
    for key in ["tab", "shift-tab"] {
        cx.simulate_keystrokes(scene.window, key);
        cx.update_window(scene.window, |_, window, cx| {
            first.update(cx, |s, cx| {
                assert!(s.presentation().focus_handle().is_focused(window));
                assert_eq!(s.value().as_ref(), "保留");
                assert_eq!(s.marked_text_range(window, cx), Some(0..2));
            })
        })
        .unwrap();
    }
    cx.update_window(scene.window, |_, window, cx| {
        first.update(cx, |s, cx| s.unmark_text(window, cx))
    })
    .unwrap();
    scene.draw(cx);
    cx.simulate_keystrokes(scene.window, "tab");
    cx.update_window(scene.window, |_, window, cx| {
        assert!(
            !first
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        )
    })
    .unwrap();
    scene.close(cx);
}

#[gpui::test]
fn retired_inputs_are_removed_from_both_directions_of_the_old_native_tab_order(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, fixture());
    scene.draw(cx);
    let first = cx.read(|cx| scene.owner().read(cx).state.clone());
    let second = cx.read(|cx| second(&scene).read(cx).state.clone());
    let single = single(&scene, cx);
    let handles = cx.read(|cx| {
        [
            first.read(cx).presentation().focus_handle().clone(),
            second.read(cx).presentation().focus_handle().clone(),
            single.read(cx).presentation().focus_handle().clone(),
        ]
    });
    cx.update_window(scene.window, |_, window, cx| {
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "toggle", json!(null))
            .unwrap();
        scene
            .surface
            .update(cx, |s, cx| s.sync_native(window, cx).unwrap());
        // Exercise precisely the old rendered-frame order, before a new draw.
        window.blur(cx);
        for backwards in [false, true] {
            for _ in 0..10 {
                if backwards {
                    window.focus_prev(cx);
                } else {
                    window.focus_next(cx);
                }
                assert!(
                    window.focused(cx).is_some(),
                    "healthy controls remain navigable"
                );
                assert!(handles.iter().all(|h| !h.is_focused(window)));
            }
        }
    })
    .unwrap();
    scene.draw(cx);
    scene.close(cx);
}
