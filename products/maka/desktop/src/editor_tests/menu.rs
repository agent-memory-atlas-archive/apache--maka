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
use gpui_kit::{
    AccessibleAction, ClipboardItem, KeyDownEvent, Keystroke, Modifiers, MouseButton, Pixels,
    PlatformInput, Point, Role, ScrollHandle, StatefulInteractiveElement, VisualTestContext,
    Window, accesskit,
};
use serde_json::json;

fn node(
    scene: &Scene,
    cx: &TestAppContext,
    role: Role,
    label: &str,
) -> (accesskit::NodeId, accesskit::Node) {
    cx.accessibility_update(scene.window)
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| node.role() == role && node.label() == Some(label))
        .unwrap_or_else(|| panic!("missing actual AX node {role:?} {label}"))
}
fn ax(scene: &Scene, cx: &mut TestAppContext, action: AccessibleAction, label: &str) {
    let target_node = node(scene, cx, Role::MenuItem, label).0;
    cx.simulate_accessibility_action(
        scene.window,
        accesskit::ActionRequest {
            action,
            target_tree: accesskit::TreeId::ROOT,
            target_node,
            data: None,
        },
    );
    scene.draw(cx);
}
fn focus(scene: &Scene, cx: &mut TestAppContext) {
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
    scene.draw(cx);
}
fn right_down(scene: &Scene, cx: &mut TestAppContext) -> Point<Pixels> {
    let owner = scene.owner();
    let position = cx.update(|cx| {
        owner.read(cx).state.read(cx).text_bounds().unwrap().origin + point(px(4.), px(4.))
    });
    let mut visual = VisualTestContext::from_window(scene.window, cx);
    visual.simulate_mouse_down(position, MouseButton::Right, Modifiers::none());
    position
}
fn right_up(scene: &Scene, cx: &mut TestAppContext, position: Point<Pixels>) {
    VisualTestContext::from_window(scene.window, cx).simulate_mouse_up(
        position,
        MouseButton::Right,
        Modifiers::none(),
    );
    scene.draw(cx);
}

#[gpui::test]
fn textarea_menu_keyboard_mouse_and_ax_use_original_native_history_and_clipboard(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, appearance::scoped_fixture());
    cx.activate_accessibility(scene.window);
    focus(&scene, cx);
    let owner = scene.owner();
    let state = cx.update(|cx| owner.read(cx).state.clone());
    cx.update_window(scene.window, |_, window, cx| {
        state.update(cx, |state, cx| state.replace_all("Draft", window, cx))
    })
    .unwrap();
    scene.draw(cx);
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    assert!(!node(&scene, cx, Role::MenuItem, "Undo").1.is_disabled());
    assert!(node(&scene, cx, Role::MenuItem, "Redo").1.is_disabled());
    cx.simulate_keystrokes(scene.window, "enter");
    scene.draw(cx);
    cx.update_window(scene.window, |_, window, cx| {
        assert_eq!(state.read(cx).value().as_ref(), "Start");
        assert!(
            state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
        assert_eq!(owner.read(cx).state.entity_id(), state.entity_id());
    })
    .unwrap();
    // Physical right-click opens the same local menu; platform AX activates
    // the real original Redo action rather than a copied text/history buffer.
    let position = right_down(&scene, cx);
    right_up(&scene, cx, position);
    assert!(!node(&scene, cx, Role::MenuItem, "Redo").1.is_disabled());
    ax(&scene, cx, AccessibleAction::Click, "Redo");
    cx.update(|cx| assert_eq!(state.read(cx).value().as_ref(), "Draft"));
    cx.simulate_keystrokes(scene.window, "menu");
    scene.draw(cx);
    ax(&scene, cx, AccessibleAction::Click, "Select all");
    appearance::event(&scene, json!({"readOnly":true}), cx);
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    assert!(node(&scene, cx, Role::MenuItem, "Cut").1.is_disabled());
    assert!(node(&scene, cx, Role::MenuItem, "Paste").1.is_disabled());
    assert!(!node(&scene, cx, Role::MenuItem, "Copy").1.is_disabled());
    ax(&scene, cx, AccessibleAction::Click, "Copy");
    cx.update(|cx| {
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("Draft")
        )
    });
    appearance::event(&scene, json!({"readOnly":false}), cx);
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    ax(&scene, cx, AccessibleAction::Click, "Cut");
    cx.update(|cx| {
        assert_eq!(state.read(cx).value().as_ref(), "");
        cx.write_to_clipboard(ClipboardItem::new_string("中文 pasted".into()));
    });
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    ax(&scene, cx, AccessibleAction::Click, "Paste");
    cx.update(|cx| assert_eq!(state.read(cx).value().as_ref(), "中文 pasted"));
    // An actual key binding for native Undo cannot escape the menu interceptor.
    cx.simulate_keystrokes(scene.window, "shift-f10 cmd-z");
    scene.draw(cx);
    cx.update(|cx| assert_eq!(state.read(cx).value().as_ref(), "中文 pasted"));
    cx.simulate_keystrokes(scene.window, "escape");
    scene.draw(cx);
    cx.simulate_input(scene.window, "!");
    scene.draw(cx);
    cx.update(|cx| assert_eq!(state.read(cx).value().as_ref(), "中文 pasted!"));
    scene.close(cx);
}

#[gpui::test]
fn textarea_menu_pending_right_click_cannot_relocate_new_composition_or_survive_revoke(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, appearance::scoped_fixture());
    cx.activate_accessibility(scene.window);
    focus(&scene, cx);
    let owner = scene.owner();
    let state = cx.update(|cx| owner.read(cx).state.clone());
    let position = right_down(&scene, cx);
    cx.update_window(scene.window, |_, window, cx| {
        owner.update(cx, |owner, cx| {
            owner.replace_and_mark_text_in_range(Some(0..5), "中文", Some(1..2), window, cx)
        });
    })
    .unwrap();
    right_up(&scene, cx, position);
    cx.update_window(scene.window, |_, window, cx| {
        assert_eq!(
            owner.update(cx, |owner, cx| owner.marked_text_range(window, cx)),
            Some(0..2)
        );
        assert_eq!(state.read(cx).selected_range(), 3..6); // Kit's actual UTF-8 selection.
        assert!(
            state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        );
    })
    .unwrap();
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    assert!(
        !cx.accessibility_update(scene.window)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == Role::Menu)
    );
    cx.update_window(scene.window, |_, window, cx| {
        owner.update(cx, |owner, cx| owner.unmark_text(window, cx))
    })
    .unwrap();
    // A later mouse-up cannot resurrect the discarded pending request.
    right_up(&scene, cx, position);
    assert!(
        !cx.accessibility_update(scene.window)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == Role::Menu)
    );
    let position = right_down(&scene, cx);
    appearance::event(&scene, json!({"menuClosed":true}), cx);
    cx.update(|cx| {
        state.update(cx, |state, cx| {
            state.set_selected_range(0..state.text().len(), cx)
        })
    });
    right_up(&scene, cx, position);
    cx.update(|cx| assert_eq!(state.read(cx).selected_range(), 0..6));
    assert!(
        !cx.accessibility_update(scene.window)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == Role::Menu)
    );
    scene.close(cx);
}

#[gpui::test]
fn textarea_menu_opening_blocks_old_native_handler_before_paint_and_escape_restores_input(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::new(cx, &timer, false);
    focus(&scene, cx);
    let owner = scene.owner();
    cx.update_window(scene.window, |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse("shift-f10").unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        owner.update(cx, |owner, cx| {
            assert!(!owner.accepts_text_input(window, cx));
            owner.replace_text_in_range(Some(0..5), "Forbidden", window, cx);
            owner.replace_and_mark_text_in_range(Some(0..5), "错误", Some(0..2), window, cx);
            assert_eq!(owner.state.read(cx).value().as_ref(), "Start");
            assert_eq!(owner.marked_text_range(window, cx), None);
        });
    })
    .unwrap();
    scene.draw(cx);
    cx.simulate_input(scene.window, "Forbidden");
    cx.update(|cx| assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "Start"));
    cx.simulate_keystrokes(scene.window, "escape");
    scene.draw(cx);
    cx.simulate_input(scene.window, "!");
    cx.update(|cx| assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "!Start"));
    scene.close(cx);
}

#[gpui::test]
fn textarea_menu_reveals_initial_and_ax_selection_inside_small_viewports_and_brand_changes(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let (mut input, packages) = appearance::scoped_fixture();
    for (name, size, background) in [
        ("brand", 16, 0xfaf9f6ffu32),
        ("other_brand", 24, 0x182420ffu32),
    ] {
        let mut brand = support::brand_appearance();
        brand["menu"]["appearance"]["windowMargin"] = json!(128);
        brand["menu"]["appearance"]["width"] = json!(1024);
        brand["menu"]["appearance"]["text"]["font"]["size"] = json!(size);
        brand["menu"]["appearance"]["background"] = json!(background);
        brand["menu"]["labels"]["copy"] =
            json!("复制当前选区 — a deliberately long translated action label");
        input.instances.get_mut(name).unwrap().configuration.values =
            serde_json::from_value(brand).unwrap();
    }
    let scene = Scene::from_packages(cx, &timer, false, (input, packages));
    cx.activate_accessibility(scene.window);
    focus(&scene, cx);
    let owner = scene.owner();
    cx.update_window(scene.window, |_, window, cx| {
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |state, cx| state.set_value("", window, cx))
    })
    .unwrap();
    cx.simulate_window_resize(scene.window, size(px(240.), px(180.)));
    cx.run_until_parked();
    scene.draw(cx);
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    let scale = cx
        .update_window(scene.window, |_, window, _| window.scale_factor())
        .unwrap() as f64;
    let assert_visible = |cx: &TestAppContext, label: &str, width: f64, height: f64| {
        let width = width * scale;
        let height = height * scale;
        let menu = node(&scene, cx, Role::Menu, "Edit draft")
            .1
            .bounds()
            .unwrap();
        let row = node(&scene, cx, Role::MenuItem, label).1.bounds().unwrap();
        assert!(
            menu.x0 >= 0. && menu.y0 >= 0. && menu.x1 <= width && menu.y1 <= height,
            "menu {menu:?}"
        );
        assert!(
            row.y0 >= menu.y0 && row.y1 <= menu.y1 && row.y1 > row.y0,
            "selected row {row:?} outside {menu:?}"
        );
        row.y1 - row.y0
    };
    // Empty native history/selection: the first enabled action is row five,
    // Paste. Opening must reveal it without an initial navigation keystroke.
    let first_height = assert_visible(cx, "Paste", 240., 180.);
    cx.simulate_window_resize(scene.window, size(px(240.), px(140.)));
    cx.run_until_parked();
    scene.draw(cx);
    assert_visible(cx, "Paste", 240., 140.);
    appearance::event(&scene, json!({"switch":true}), cx);
    let second_height = assert_visible(cx, "Paste", 240., 140.);
    assert!(
        second_height > first_height,
        "actual brand font projection must change row layout"
    );
    cx.simulate_keystrokes(scene.window, "escape");
    cx.update_window(scene.window, |_, window, cx| {
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |state, cx| state.set_value("selection", window, cx))
    })
    .unwrap();
    scene.draw(cx);
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    ax(&scene, cx, AccessibleAction::Focus, "Select all");
    assert_visible(cx, "Select all", 240., 140.);
    ax(&scene, cx, AccessibleAction::Click, "Select all");
    cx.update(|cx| assert_eq!(owner.read(cx).state.read(cx).selected_range(), 0..9));
    scene.close(cx);
}

#[gpui::test]
fn textarea_menu_tab_and_outside_click_preserve_external_native_focus(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, appearance::scoped_fixture());
    cx.activate_accessibility(scene.window);
    focus(&scene, cx);
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
    cx.simulate_keystrokes(scene.window, "shift-f10 tab");
    scene.draw(cx);
    cx.update_window(scene.window, |_, window, cx| {
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
    focus(&scene, cx);
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    let position = cx.update(|cx| {
        let bounds = second.read(cx).state.read(cx).input_bounds();
        point(bounds.right() - px(8.), bounds.top() + px(8.))
    });
    VisualTestContext::from_window(scene.window, cx).simulate_click(position, Modifiers::none());
    scene.draw(cx);
    cx.update_window(scene.window, |_, window, cx| {
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
    cx.simulate_input(scene.window, "!");
    scene.draw(cx);
    cx.update(|cx| {
        assert_eq!(second.read(cx).state.read(cx).value().as_ref(), "Second!");
        assert_eq!(
            scene.owner().read(cx).state.read(cx).value().as_ref(),
            "Start"
        );
    });
    scene.close(cx);
}

#[gpui::test]
fn textarea_menu_wheel_browsing_survives_unrelated_native_notifications(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, appearance::scoped_fixture());
    cx.activate_accessibility(scene.window);
    focus(&scene, cx);
    let owner = scene.owner();
    cx.update_window(scene.window, |_, window, cx| {
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |state, cx| state.set_value("", window, cx))
    })
    .unwrap();
    cx.simulate_window_resize(scene.window, size(px(240.), px(140.)));
    cx.run_until_parked();
    scene.draw(cx);
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    let before = node(&scene, cx, Role::MenuItem, "Undo").1.bounds().unwrap();
    let menu = node(&scene, cx, Role::Menu, "Edit draft")
        .1
        .bounds()
        .unwrap();
    let scale = cx
        .update_window(scene.window, |_, window, _| window.scale_factor())
        .unwrap() as f64;
    VisualTestContext::from_window(scene.window, cx).simulate_event(gpui_kit::ScrollWheelEvent {
        // Over a disabled row so hover cannot select a different action.
        position: point(
            px(((menu.x0 + 10.) / scale) as f32),
            px(((menu.y0 + 12.) / scale) as f32),
        ),
        delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), px(200.))),
        modifiers: Modifiers::none(),
        touch_phase: gpui_kit::TouchPhase::Moved,
    });
    scene.draw(cx);
    let browsed = node(&scene, cx, Role::MenuItem, "Undo").1.bounds().unwrap();
    assert!(
        browsed.y0 > before.y0,
        "actual wheel must move the original scroller: {before:?} -> {browsed:?}"
    );
    owner.update(cx, |_, cx| cx.notify());
    scene.draw(cx);
    let after = node(&scene, cx, Role::MenuItem, "Undo").1.bounds().unwrap();
    assert_eq!(
        after, browsed,
        "an unrelated Owner render must not snap back to Paste"
    );
    cx.simulate_keystrokes(scene.window, "home");
    scene.draw(cx);
    assert_eq!(
        node(&scene, cx, Role::MenuItem, "Undo").1.bounds().unwrap(),
        before,
        "explicit same-value Home must reveal the current Paste again"
    );
    VisualTestContext::from_window(scene.window, cx).simulate_event(gpui_kit::ScrollWheelEvent {
        position: point(
            px(((menu.x0 + 10.) / scale) as f32),
            px(((menu.y0 + 12.) / scale) as f32),
        ),
        delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), px(200.))),
        modifiers: Modifiers::none(),
        touch_phase: gpui_kit::TouchPhase::Moved,
    });
    scene.draw(cx);
    assert_eq!(
        node(&scene, cx, Role::MenuItem, "Undo").1.bounds().unwrap(),
        browsed
    );
    ax(&scene, cx, AccessibleAction::Focus, "Paste");
    assert_eq!(
        node(&scene, cx, Role::MenuItem, "Undo").1.bounds().unwrap(),
        before,
        "explicit same-value AX Focus must reveal the current Paste again"
    );
    scene.close(cx);
}

struct RevealLayoutTestView {
    scroll: ScrollHandle,
    height: Pixels,
}
impl Render for RevealLayoutTestView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("reveal")
            .w(px(100.))
            .h(self.height)
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .children((0..6).map(|_| div().h(px(20.)).flex_shrink_0()))
    }
}

#[gpui::test]
fn active_item_uses_first_prepaint_and_current_resized_viewport(cx: &mut TestAppContext) {
    let scroll = ScrollHandle::new();
    // Queue before any layout has published bounds or scroll overflow.
    scroll.scroll_to_item(4);
    let window = cx.add_window({
        let scroll = scroll.clone();
        move |_, _| RevealLayoutTestView {
            scroll,
            height: px(40.),
        }
    });
    window
        .update(cx, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(
                scroll.offset().y,
                px(-60.),
                "first prepaint must reveal row five"
            );
        })
        .unwrap();
    // Only this draw may resolve the newly requested item, using 20 px,
    // not the previous 40 px viewport, before children get prepainted.
    window
        .update(cx, |view, _, cx| {
            view.height = px(20.);
            scroll.scroll_to_item(5);
            cx.notify();
        })
        .unwrap();
    window
        .update(cx, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(scroll.bounds().size.height, px(20.));
            assert_eq!(
                scroll.offset().y,
                px(-100.),
                "same prepaint must use current height"
            );
        })
        .unwrap();
}

#[gpui::test]
fn retired_textarea_menu_rejects_old_painted_paste_callback_before_next_paint(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::from_packages(cx, &timer, false, appearance::scoped_fixture());
    cx.activate_accessibility(scene.window);
    focus(&scene, cx);
    let owner = scene.owner();
    let state = cx.update(|cx| owner.read(cx).state.clone());
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    let paste = node(&scene, cx, Role::MenuItem, "Paste")
        .1
        .bounds()
        .unwrap();
    cx.update_window(scene.window, |_, window, cx| {
        let scale = window.scale_factor() as f64;
        let position = point(
            px(((paste.x0 + paste.x1) / (2. * scale)) as f32),
            px(((paste.y0 + paste.y1) / (2. * scale)) as f32),
        );
        cx.write_to_clipboard(ClipboardItem::new_string("Forbidden retired paste".into()));
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "toggle", serde_json::Value::Null)
            .unwrap();
        scene.settle_in(window, cx).unwrap();
        assert!(
            scene
                .app
                .borrow()
                .child_mount(&scene.root, "editor")
                .is_err()
        );
        assert!(
            window.focused(cx).is_none(),
            "retirement must release the menu's original focus before any input or paint"
        );
        // Keep the original native owner alive, then deliver against its old
        // painted row before the effect cycle can flush a replacement frame.
        window.dispatch_event(
            PlatformInput::MouseDown(gpui_kit::MouseDownEvent {
                position,
                button: MouseButton::Left,
                modifiers: Modifiers::none(),
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(gpui_kit::MouseUpEvent {
                position,
                button: MouseButton::Left,
                modifiers: Modifiers::none(),
                click_count: 1,
            }),
            cx,
        );
        assert_eq!(state.read(cx).value().as_ref(), "Start");
        owner.update(cx, |owner, cx| {
            assert!(!owner.accepts_text_input(window, cx))
        });
    })
    .unwrap();
    scene.close(cx);
}

#[gpui::test]
fn retired_textarea_menu_does_not_blur_focus_that_has_moved_to_another_editor(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let (mut input, mut packages) = appearance::scoped_fixture();
    let index = packages
        .iter()
        .position(|package| package.manifest().id == "dynamic.app")
        .unwrap();
    let original = packages.remove(index);
    // Keep the second real editor independently of the first editor's toggle.
    let source = format!(
        r#"{}
        const originalFrame=frame;
        frame=(state)=>{{
            const output=originalFrame(state);
            if(!state.right)output.frame.view.children.unshift({{kind:'component',key:'second',binding:'editor',member:0,on:{{}},props:{{initialText:'Second',placeholder:'Other draft',appearance:state.otherBrand.textarea,accessibilityLabel:'Other draft',readOnly:false,disabled:false,softWrap:true,tabIndex:0}}}});
            return output;
        }};
        "#,
        std::str::from_utf8(original.file("index.mjs").unwrap()).unwrap()
    );
    let root = support::package(
        "dynamic.app",
        original.manifest().application.clone().unwrap(),
        Some(source),
    );
    input.instances.get_mut("app").unwrap().package = root.application_ref().into();
    packages.push(root);
    let scene = Scene::from_packages(cx, &timer, false, (input, packages));
    focus(&scene, cx);
    let owner = scene.owner();
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
    cx.simulate_keystrokes(scene.window, "shift-f10");
    scene.draw(cx);
    cx.update_window(scene.window, |_, window, cx| {
        let external = second
            .read(cx)
            .state
            .read(cx)
            .presentation()
            .focus_handle()
            .clone();
        external.focus(window, cx);
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "toggle", serde_json::Value::Null)
            .unwrap();
        scene.settle_in(window, cx).unwrap();
        assert!(external.is_focused(window));
        owner.update(cx, |owner, cx| {
            assert!(!owner.accepts_text_input(window, cx));
        });
    })
    .unwrap();
    scene.draw(cx);
    cx.simulate_input(scene.window, "!");
    scene.draw(cx);
    cx.update(|cx| {
        assert_eq!(second.read(cx).state.read(cx).value().as_ref(), "!Second");
        assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "Start");
    });
    scene.close(cx);
}
