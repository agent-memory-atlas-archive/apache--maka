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
use gpui_kit::{AccessibleAction, Role, accesskit};
use serde_json::{Value, json};

pub(super) fn scoped_fixture() -> (
    maka_plugin_core::composition::Input,
    Vec<maka_packages::Package>,
) {
    use maka_plugin_core::{composition::Target, package::EntryTarget};
    let (mut input, mut packages) = support::editor_fixture();
    let brand = packages
        .iter()
        .find(|p| p.manifest().id == "dynamic.brand")
        .unwrap();
    let contract = brand.manifest().application.as_ref().unwrap().entries[&EntryTarget::Desktop]
        .exports["Appearance"]
        .contract
        .clone();
    let mut other = support::node(brand, 991);
    let mut appearance = support::brand_appearance();
    appearance["textarea"]["text"]["font"]["size"] = json!(24);
    appearance["textarea"]["text"]["foreground"] = json!(0xe5e7ebffu32);
    appearance["textarea"]["background"] = json!(0); // Explicit transparency.
    appearance["textarea"]["selection"] = json!(0);
    appearance["textarea"]["caret"] = json!(0x99c9baffu32);
    other.configuration.values = serde_json::from_value(appearance).unwrap();
    input.instances.insert("other_brand".into(), other);
    input.instances.get_mut("app").unwrap().bindings.insert(
        "otherBrand".into(),
        vec![Target::Instance {
            path: vec!["other_brand".into()],
            export: "Appearance".into(),
        }],
    );
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
            "otherBrand".into(),
            maka_plugin_core::contract::Import {
                contract,
                cardinality: maka_plugin_core::contract::Cardinality::One,
            },
        );
    let source = std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap()
        .replace("brand:values.brand[0]", "brand:values.brand[0],otherBrand:values.otherBrand[0]")
        .replace("appearance:state.brand.textarea", "appearance:(state.alternate?state.otherBrand:state.brand).textarea")
        .replace("menu:state.brand.menu", "menu:state.menuClosed?null:(state.alternate?state.otherBrand:state.brand).menu")
        .replace("softWrap:true", "softWrap:state.softWrap??true")
        .replace("const next={...state,revision:state.revision+1};", "const next={...state,revision:state.revision+1};if(event.event?.softWrap!==undefined)next.softWrap=event.event.softWrap;")
        .replace("readOnly:false,disabled:false", "readOnly:state.readOnly??false,disabled:state.disabled??false")
        .replace("const next={...state,revision:state.revision+1};", "const next={...state,revision:state.revision+1};if(event.event?.switch)next.alternate=!state.alternate;if(event.event?.menuClosed!==undefined)next.menuClosed=event.event.menuClosed;if(event.event?.readOnly!==undefined)next.readOnly=event.event.readOnly;if(event.event?.disabled!==undefined)next.disabled=event.event.disabled;")
        .replace("},{kind:'component',key:'nativeLabel'", "},{kind:'component',key:'second',binding:'editor',member:0,on:{},props:{initialText:'Second',placeholder:'Other draft',appearance:state.otherBrand.textarea,accessibilityLabel:'Other draft',readOnly:false,disabled:false,softWrap:true,tabIndex:0}},{kind:'component',key:'nativeLabel'");
    let root = support::package("dynamic.app", declaration, Some(source));
    input.instances.get_mut("app").unwrap().package = root.application_ref().into();
    packages.push(root);
    (input, packages)
}
pub(super) fn event(scene: &Scene, event: Value, cx: &mut TestAppContext) {
    let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
    scene
        .app
        .borrow_mut()
        .dispatch(&scene.root, revision, "parent", event)
        .unwrap();
    scene.draw(cx);
    scene.draw(cx);
}
fn textarea_node(cx: &TestAppContext, scene: &Scene) -> (accesskit::NodeId, accesskit::Node) {
    cx.accessibility_update(scene.window)
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| {
            node.role() == Role::MultilineTextInput && node.label() == Some("Write a draft")
        })
        .expect("actual platform textarea node")
}

#[gpui::test]
fn branded_textareas_share_window_but_not_appearance_and_switch_preserves_native_owner(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, scoped_fixture());
    scene.draw(cx);
    scene.draw(cx);
    let first = scene.owner();
    let second_mount = scene
        .app
        .borrow()
        .child_mount(&scene.root, "second")
        .unwrap()
        .clone();
    let second = scene.resources.borrow().owners[&second_mount.id().to_string()]
        .state
        .downcast_ref::<Entity<TextareaOwner>>()
        .unwrap()
        .clone();
    let state = cx.update(|cx| first.read(cx).state.clone());
    let second_state = cx.update(|cx| second.read(cx).state.clone());
    cx.update_window(scene.window, |_, window, cx| {
        assert_eq!(state.read(cx).line_height(), Some(px(24.)));
        assert_eq!(second_state.read(cx).line_height(), Some(px(36.)));
        assert!(state.read(cx).input_bounds().size.height >= px(24.));
        assert!(second_state.read(cx).input_bounds().size.height >= px(36.));
        first.update(cx, |owner, cx| {
            owner
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .focus(window, cx);
            owner.replace_text_in_range(Some(0..5), "Draft", window, cx)
        });
        state.update(cx, |state, cx| {
            state.set_selected_range(1..4, cx);
            state.focus(window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    scene.draw(cx);
    let revision = cx.update(|cx| first.read(cx).revision);
    event(&scene, json!({"switch":true}), cx);
    cx.update_window(scene.window, |_, window, cx| {
        assert_eq!(scene.owner().entity_id(), first.entity_id());
        assert_eq!(first.read(cx).state.entity_id(), state.entity_id());
        assert_eq!(first.read(cx).revision, revision);
        assert_eq!(state.read(cx).value().as_ref(), "Draft");
        assert_eq!(state.read(cx).line_height(), Some(px(36.)));
        assert_eq!(second_state.read(cx).value().as_ref(), "Second");
        assert!(
            state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
        assert_eq!(
            state
                .update(cx, |s, cx| s.selected_text_range(true, window, cx))
                .unwrap()
                .range,
            1..4
        );
    })
    .unwrap();
    cx.dispatch_action(scene.window, gpui_kit::base::input::Undo);
    scene.draw(cx);
    cx.update(|cx| assert_eq!(state.read(cx).value().as_ref(), "Start"));
    cx.update_window(scene.window, |_, window, cx| {
        first.update(cx, |owner, cx| {
            owner
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .focus(window, cx);
            owner.replace_and_mark_text_in_range(Some(0..5), "中文", Some(0..2), window, cx)
        })
    })
    .unwrap();
    cx.run_until_parked();
    let revision = cx.update(|cx| first.read(cx).revision);
    event(&scene, json!({"switch":true}), cx);
    cx.update_window(scene.window, |_, window, cx| {
        assert_eq!(first.read(cx).revision, revision);
        assert_eq!(state.read(cx).value().as_ref(), "中文");
        assert_eq!(
            first.update(cx, |s, cx| s.marked_text_range(window, cx)),
            Some(0..2)
        );
        assert_eq!(state.read(cx).line_height(), Some(px(24.)));
        assert_eq!(second_state.read(cx).line_height(), Some(px(36.)));
    })
    .unwrap();
    scene.close(cx);
}

#[gpui::test]
fn textarea_platform_accessibility_uses_real_focus_value_and_edit_fences(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, scoped_fixture());
    cx.activate_accessibility(scene.window);
    scene.draw(cx);
    scene.draw(cx);
    let (id, node) = textarea_node(cx, &scene);
    assert_eq!(node.value(), Some("Start"));
    assert_eq!(node.placeholder(), Some("Write here"));
    assert!(!node.is_disabled());
    assert!(!node.is_read_only());
    cx.simulate_accessibility_action(
        scene.window,
        accesskit::ActionRequest {
            action: AccessibleAction::Focus,
            target_tree: accesskit::TreeId::ROOT,
            target_node: id,
            data: None,
        },
    );
    scene.draw(cx);
    assert_eq!(cx.accessibility_update(scene.window).unwrap().focus, id);
    cx.simulate_accessibility_action(
        scene.window,
        accesskit::ActionRequest {
            action: AccessibleAction::SetValue,
            target_tree: accesskit::TreeId::ROOT,
            target_node: id,
            data: Some(accesskit::ActionData::Value("Accessible draft".into())),
        },
    );
    scene.draw(cx);
    assert_eq!(
        textarea_node(cx, &scene).1.value(),
        Some("Accessible draft")
    );
    // Both facades track the same native id. Forward/back navigation must reach
    // the other real textarea rather than duplicate the current stop.
    let second_mount = scene
        .app
        .borrow()
        .child_mount(&scene.root, "second")
        .unwrap()
        .clone();
    let second = scene.resources.borrow().owners[&second_mount.id().to_string()]
        .state
        .downcast_ref::<Entity<TextareaOwner>>()
        .unwrap()
        .clone();
    cx.update_window(scene.window, |_, window, cx| {
        window.focus_next(cx);
        assert!(
            second
                .read(cx)
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
    })
    .unwrap();
    scene.draw(cx);
    scene.draw(cx);
    cx.replay_focus_history(scene.window);
    cx.update_window(scene.window, |_, window, cx| {
        window.focus_prev(cx);
        assert!(
            scene
                .owner()
                .read(cx)
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
    })
    .unwrap();
    event(&scene, json!({"readOnly":true}), cx);
    let (id, node) = textarea_node(cx, &scene);
    assert!(node.is_read_only());
    assert!(!node.supports_action(AccessibleAction::SetValue));
    cx.simulate_accessibility_action(
        scene.window,
        accesskit::ActionRequest {
            action: AccessibleAction::SetValue,
            target_tree: accesskit::TreeId::ROOT,
            target_node: id,
            data: Some(accesskit::ActionData::Value("Forbidden".into())),
        },
    );
    scene.draw(cx);
    assert_eq!(
        textarea_node(cx, &scene).1.value(),
        Some("Accessible draft")
    );
    event(&scene, json!({"disabled":true}), cx);
    let (id, node) = textarea_node(cx, &scene);
    assert!(node.is_disabled());
    cx.simulate_accessibility_action(
        scene.window,
        accesskit::ActionRequest {
            action: AccessibleAction::Focus,
            target_tree: accesskit::TreeId::ROOT,
            target_node: id,
            data: None,
        },
    );
    scene.draw(cx);
    let owner = scene.owner();
    let position = cx
        .update_window(scene.window, |_, window, cx| {
            let state = owner.read(cx).state.read(cx);
            assert!(!state.presentation().focus_handle().is_focused(window));
            state.input_bounds().center()
        })
        .unwrap();
    let mut visual = gpui_kit::VisualTestContext::from_window(scene.window, cx);
    visual.simulate_click(position, gpui_kit::Modifiers::none());
    scene.draw(cx);
    cx.update_window(scene.window, |_, window, cx| {
        assert!(
            !owner
                .read(cx)
                .state
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
fn committed_disabled_textarea_rejects_mouse_input_before_next_paint(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, scoped_fixture());
    scene.draw(cx);
    scene.draw(cx);
    let owner = scene.owner();
    let state = cx.update(|cx| owner.read(cx).state.clone());
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |state, cx| {
            state.focus(window, cx);
            state.set_selected_range(1..3, cx);
        });
    })
    .unwrap();
    scene.draw(cx);
    scene.draw(cx);
    let position = cx
        .update_window(scene.window, |_, window, cx| {
            window.blur(cx);
            state.read(cx).text_bounds().unwrap().origin + point(px(4.), px(4.))
        })
        .unwrap();
    let mut visual = gpui_kit::VisualTestContext::from_window(scene.window, cx);
    visual.simulate_click(position, gpui_kit::Modifiers::none());
    cx.update_window(scene.window, |_, window, cx| {
        assert!(
            state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window),
            "enabled click must hit the real textarea; {:?}",
            state.read(cx).input_bounds()
        );
        state.update(cx, |state, cx| state.set_selected_range(1..3, cx));
    })
    .unwrap();
    scene.draw(cx);
    scene.draw(cx);
    cx.update_window(scene.window, |_, window, cx| {
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "parent", json!({"disabled":true}))
            .unwrap();
        scene.settle_in(window, cx).unwrap();
        assert!(state.read(cx).presentation().is_disabled());
        assert!(
            !state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
        // Stay within the update: App's flush can otherwise draw before input.
        // Delivery must use the previously painted enabled dispatch tree.
        window.dispatch_event(
            gpui_kit::PlatformInput::MouseDown(gpui_kit::MouseDownEvent {
                position,
                button: gpui_kit::MouseButton::Left,
                modifiers: gpui_kit::Modifiers::none(),
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.dispatch_event(
            gpui_kit::PlatformInput::MouseUp(gpui_kit::MouseUpEvent {
                position,
                button: gpui_kit::MouseButton::Left,
                modifiers: gpui_kit::Modifiers::none(),
                click_count: 1,
            }),
            cx,
        );
        assert!(
            !state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
        assert_eq!(
            state
                .update(cx, |state, cx| state.selected_text_range(true, window, cx))
                .unwrap()
                .range,
            1..3
        );
        assert_eq!(state.read(cx).value().as_ref(), "Start");
    })
    .unwrap();
    scene.close(cx);
}

#[gpui::test]
fn committed_disabled_textarea_is_not_a_tab_stop_before_next_paint(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, scoped_fixture());
    scene.draw(cx);
    scene.draw(cx);
    let first = scene.owner();
    let second_mount = scene
        .app
        .borrow()
        .child_mount(&scene.root, "second")
        .unwrap()
        .clone();
    let second = scene.resources.borrow().owners[&second_mount.id().to_string()]
        .state
        .downcast_ref::<Entity<TextareaOwner>>()
        .unwrap()
        .clone();
    cx.update_window(scene.window, |_, window, cx| {
        first
            .read(cx)
            .state
            .read(cx)
            .presentation()
            .focus_handle()
            .focus(window, cx);
        window.focus_next(cx);
        assert!(
            second
                .read(cx)
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
        window.focus_prev(cx);
        assert!(
            first
                .read(cx)
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
        window.focus_next(cx);
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "parent", json!({"disabled":true}))
            .unwrap();
        scene.settle_in(window, cx).unwrap();
        assert!(first.read(cx).state.read(cx).presentation().is_disabled());
        assert!(
            second
                .read(cx)
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
        window.focus_prev(cx);
        assert!(
            !first
                .read(cx)
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
    })
    .unwrap();
    scene.close(cx);
}

#[gpui::test]
fn textarea_history_availability_uses_real_history_and_current_editability(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::new(cx, &timer, false);
    scene.draw(cx);
    let owner = scene.owner();
    let state = cx.update(|cx| owner.read(cx).state.clone());
    cx.update_window(scene.window, |_, window, cx| {
        assert!(!state.read(cx).can_undo());
        assert!(!state.read(cx).can_redo());
        state.update(cx, |state, cx| {
            state.focus(window, cx);
            state.replace_all("Draft", window, cx);
        });
        assert!(state.read(cx).can_undo());
        assert!(!state.read(cx).can_redo());
    })
    .unwrap();
    scene.draw(cx);
    cx.update_window(scene.window, |_, window, cx| {
        let focus = state.read(cx).presentation().focus_handle().clone();
        focus.dispatch_action(&gpui_kit::base::input::Undo, window, cx);
        assert_eq!(state.read(cx).value().as_ref(), "Start");
        assert!(!state.read(cx).can_undo());
        assert!(state.read(cx).can_redo());
        state.update(cx, |state, cx| {
            state.set_readonly(true, cx);
            assert!(!state.can_undo());
            assert!(!state.can_redo());
            state.set_readonly(false, cx);
            assert!(state.can_redo());
            state.set_disabled(true, cx);
            assert!(!state.can_undo());
            assert!(!state.can_redo());
            state.set_disabled(false, cx);
            assert!(state.can_redo());
        });
        focus.dispatch_action(&gpui_kit::base::input::Redo, window, cx);
        assert_eq!(state.read(cx).value().as_ref(), "Draft");
        assert!(state.read(cx).can_undo());
        assert!(!state.read(cx).can_redo());
        owner.update(cx, |owner, cx| {
            owner
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .focus(window, cx);
            owner.replace_and_mark_text_in_range(Some(0..5), "中文", Some(0..2), window, cx);
        });
        assert!(!state.read(cx).can_undo());
        assert!(!state.read(cx).can_redo());
        state.update(cx, |state, cx| state.set_readonly(true, cx));
        owner.update(cx, |owner, cx| owner.unmark_text(window, cx));
        assert_eq!(
            owner.update(cx, |owner, cx| owner.marked_text_range(window, cx)),
            None
        );
        assert!(!state.read(cx).can_undo());
        state.update(cx, |state, cx| state.set_readonly(false, cx));
        assert!(state.read(cx).can_undo());
        focus.dispatch_action(&gpui_kit::base::input::Undo, window, cx);
        assert_eq!(state.read(cx).value().as_ref(), "Draft");
        assert!(state.read(cx).can_redo());
        assert_eq!(owner.read(cx).state.entity_id(), state.entity_id());
    })
    .unwrap();
    scene.close(cx);
}
