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
fn button(key: &str, label: &str, tab: i16) -> Value {
    json!({"kind":"button","key":key,"label":label,"tab_index":tab,
        "style":{"width":{"unit":"pixels","value":120},"height":{"unit":"pixels","value":40}}})
}
fn modal(bg: Value, open: bool, nested: bool) -> Value {
    let content = button("foreground", "Foreground", 0);
    let content = if nested {
        json!({"kind":"dialog","key":"inner","label":"Inner","open":true,
            "background":content,"content":button("nested", "Nested", 0)})
    } else {
        content
    };
    json!({"kind":"dialog","key":"modal","label":"Choose","open":open,
        "background":bg,"content":content})
}
fn ax_action(
    cx: &mut TestAppContext,
    window: AnyWindowHandle,
    label: &str,
    action: AccessibleAction,
) {
    let target = cx
        .accessibility_update(window)
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some(label))
        .unwrap()
        .0;
    cx.simulate_accessibility_action(
        window,
        accesskit::ActionRequest {
            action,
            target_tree: accesskit::TreeId::ROOT,
            target_node: target,
            data: None,
        },
    );
}
#[gpui::test]
fn controlled_dialog_preserves_background_geometry_and_requests_closure(cx: &mut TestAppContext) {
    let (entity, window, _) = harness(cx);
    cx.update(gpui_kit::init);
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    cx.update_window(window, |_, _, cx| {
        entity.update(cx, |h, _| {
            h.slots.input = Input::testing(Rc::new(move |key, value, _, _| {
                seen.borrow_mut().push((key.to_owned(), value));
                Ok(())
            }));
        })
    })
    .unwrap();
    cx.activate_accessibility(window);
    let bg = button("background", "Background", 0);
    replace(cx, window, &entity, modal(bg.clone(), false, false));
    draw(cx, window);
    let bounds = ax_bounds(cx, window, "Background");
    ax_action(cx, window, "Background", AccessibleAction::Focus);
    replace(cx, window, &entity, modal(bg.clone(), true, false));
    draw(cx, window);
    assert_eq!(ax_bounds(cx, window, "Background"), bounds);
    assert!(
        cx.accessibility_update(window)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.role() == Role::Dialog && n.label() == Some("Choose")),
        "actual AX tree: {:?}",
        cx.accessibility_update(window).unwrap().nodes
    );
    ax_action(cx, window, "Background", AccessibleAction::Click);
    assert!(events.borrow().is_empty());
    cx.simulate_keystrokes(window, "enter");
    assert!(
        events.borrow().is_empty(),
        "Enter does not imply confirmation"
    );
    cx.simulate_keystrokes(window, "escape");
    assert_eq!(
        *events.borrow(),
        vec![("modal".into(), json!({"kind":"dismiss","reason":"escape"}))]
    );
    cx.update_window(window, |_, window, cx| {
        assert!(entity.read(cx).state().dialogs["modal"].admitted());
        assert!(window.focused(cx).is_some());
    })
    .unwrap();
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(5.), px(5.)), Modifiers::default());
    assert_eq!(
        events.borrow().last().unwrap(),
        &("modal".into(), json!({"kind":"dismiss","reason":"outside"}))
    );
    assert!(!events.borrow().iter().any(|(key, _)| key == "background"));
    replace(cx, window, &entity, modal(bg, false, false));
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        assert!(
            entity.read(cx).state().buttons["background"]
                .focus
                .is_focused(window)
        )
    })
    .unwrap();
}
#[gpui::test]
fn controlled_dialog_restores_original_non_tab_target(cx: &mut TestAppContext) {
    let (entity, window, _) = harness(cx);
    cx.update(gpui_kit::init);
    cx.activate_accessibility(window);
    let bg = button("background", "Background", -1);
    replace(cx, window, &entity, modal(bg.clone(), false, false));
    draw(cx, window);
    ax_action(cx, window, "Background", AccessibleAction::Focus);
    replace(cx, window, &entity, modal(bg.clone(), true, false));
    draw(cx, window);
    replace(cx, window, &entity, modal(bg, false, false));
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        assert!(
            entity.read(cx).state().buttons["background"]
                .focus
                .is_focused(window)
        )
    })
    .unwrap();
}
#[gpui::test]
fn controlled_dialog_nested_close_in_one_update_restores_background(cx: &mut TestAppContext) {
    let (entity, window, _) = harness(cx);
    cx.update(gpui_kit::init);
    cx.activate_accessibility(window);
    let bg = button("background", "Background", 0);
    replace(cx, window, &entity, modal(bg.clone(), false, false));
    draw(cx, window);
    ax_action(cx, window, "Background", AccessibleAction::Focus);
    replace(cx, window, &entity, modal(bg.clone(), true, false));
    draw(cx, window);
    replace(cx, window, &entity, modal(bg.clone(), true, true));
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        entity.update(cx, |h, cx| {
            h.replace(modal(bg.clone(), true, false), window, cx);
            h.replace(modal(bg, false, false), window, cx);
        })
    })
    .unwrap();
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        assert!(
            entity.read(cx).state().buttons["background"]
                .focus
                .is_focused(window)
        )
    })
    .unwrap();
}
#[gpui::test]
fn controlled_dialog_deferred_restore_respects_intervening_focus(cx: &mut TestAppContext) {
    let (entity, window, _) = harness(cx);
    cx.update(gpui_kit::init);
    let bg = json!({"kind":"stack","key":"background-root","direction":"row","gap":0,"padding":0,"background":0,
        "children":[button("background","Background",0),button("other","Other",0)]});
    replace(cx, window, &entity, modal(bg.clone(), false, false));
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        let focus = entity.read(cx).state().buttons["background"].focus.clone();
        focus.focus(window, cx)
    })
    .unwrap();
    replace(cx, window, &entity, modal(bg.clone(), true, false));
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        entity.update(cx, |h, cx| {
            h.replace(modal(bg, false, false), window, cx);
            h.state().buttons["other"].focus.focus(window, cx);
        })
    })
    .unwrap();
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        assert!(
            entity.read(cx).state().buttons["other"]
                .focus
                .is_focused(window)
        )
    })
    .unwrap();
}

#[gpui::test]
fn controlled_dialog_retained_original_handle_cannot_restore_removed_target(
    cx: &mut TestAppContext,
) {
    let (entity, window, _) = harness(cx);
    cx.update(gpui_kit::init);
    cx.activate_accessibility(window);
    let bg = button("background", "Background", -1);
    replace(cx, window, &entity, modal(bg.clone(), false, false));
    draw(cx, window);
    ax_action(cx, window, "Background", AccessibleAction::Focus);
    let original = cx.update(|cx| entity.read(cx).state().buttons["background"].focus.clone());
    replace(cx, window, &entity, modal(bg, true, false));
    draw(cx, window);
    cx.with_paused_automatic_drawing(|cx| {
        replace(
            cx,
            window,
            &entity,
            modal(
                json!({"kind":"text","key":"empty","text":"Empty","color":0}),
                false,
                false,
            ),
        );
        cx.update_window(window, |_, window, cx| {
            assert!(window.focused(cx).is_none())
        })
        .unwrap();
        draw(cx, window);
        cx.update_window(window, |_, window, cx| {
            assert!(!original.is_focused(window));
            assert!(window.focused(cx).is_none());
        })
        .unwrap();
    });
    replace(
        cx,
        window,
        &entity,
        modal(button("background", "Background", -1), false, false),
    );
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        assert!(window.focused(cx).is_none())
    })
    .unwrap();
}
#[test]
fn controlled_dialog_compiler_retains_hidden_children_and_counts_deferred_work() {
    let mut source = modal(button("background", "Background", 0), false, false);
    source["content"] =
        json!({"kind":"component","key":"query","binding":"input","member":0,"props":{},"on":{}});
    let compiled = compile(&source, Instant::now() + std::time::Duration::from_secs(1)).unwrap();
    assert_eq!(compiled.slots().len(), 1);
    assert_eq!(compiled.visible_slots().count(), 0);
    for level in 0..11 {
        source = json!({"kind":"dialog","key":format!("modal-{level}"),"label":"Nested","open":true,
            "background":{"kind":"text","key":format!("background-{level}"),"text":"Background","color":0},"content":source});
    }
    assert!(
        compile(&source, Instant::now() + std::time::Duration::from_secs(1))
            .err()
            .unwrap()
            .to_string()
            .contains("render depth")
    );
    source["open"] = json!(false);
    assert!(compile(&source, Instant::now() + std::time::Duration::from_secs(1)).is_ok());
    source["content"]["style"] = json!({"maxWidth":1e300});
    assert!(compile(&source, Instant::now() + std::time::Duration::from_secs(1)).is_err());
}
