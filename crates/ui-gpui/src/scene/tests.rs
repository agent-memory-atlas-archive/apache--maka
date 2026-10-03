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
use crate::{Input, LiveInput, Slots, Widget};
use gpui_kit::{
    AccessibleAction, AnyWindowHandle, AppContext, Context, Entity, IntoElement, Modifiers, Render,
    Role, TestAppContext, VisualTestContext, accesskit, point, px,
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
};
struct Harness {
    recipe: Rc<dyn Widget>,
    state: Box<dyn Any>,
    slots: Slots,
}
impl Render for Harness {
    fn render(
        &mut self,
        window: &mut gpui_kit::Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.recipe
            .render(self.state.as_ref(), &self.slots, window, cx)
            .unwrap()
    }
}
impl Harness {
    fn replace(&mut self, source: Value, window: &mut gpui_kit::Window, cx: &mut Context<Self>) {
        let recipe = compile(&source, Instant::now() + std::time::Duration::from_secs(1))
            .unwrap()
            .view;
        recipe.commit(self.state.as_mut(), window, cx);
        self.recipe = recipe;
        cx.notify();
    }
    fn state(&self) -> &SceneState {
        self.state.downcast_ref().unwrap()
    }
}
fn source(disabled: bool, moved: bool, removed: bool) -> Value {
    let children = vec![
        json!({"kind":"button","key":"action","label":"Run","disabled":disabled,"style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40},"background":0x416653ffu32}}),
        json!({"kind":"stack","key":"scroll","direction":"column","gap":0,"padding":0,"background":0xffffff,"scroll":true,"style":{"width":{"unit":"pixels","value":100},"height":{"unit":"pixels","value":40},"shrink":0},"children":[{"kind":"text","key":"long","text":"Long document","color":0,"style":{"height":{"unit":"pixels","value":400},"shrink":0}}]}),
    ];
    json!({"kind":"stack","key":"root","direction":"row","gap":0,"padding":0,"background":0xffffff,"style":{"width":{"unit":"pixels","value":200},"height":{"unit":"pixels","value":200}},"children":[
        {"kind":"stack","key":"a","direction":"column","gap":0,"padding":0,"background":0xffffff,"children":if !moved && !removed { children.clone() } else { vec![] }},
        {"kind":"stack","key":"b","direction":"column","gap":0,"padding":0,"background":0xffffff,"children":if moved && !removed { children } else { vec![] }}
    ]})
}
fn harness(cx: &mut TestAppContext) -> (Entity<Harness>, AnyWindowHandle, Rc<Cell<usize>>) {
    let clicks = Rc::new(Cell::new(0));
    let seen = clicks.clone();
    let input = Input(Rc::new(move |_, _, _, _| {
        seen.set(seen.get() + 1);
        Ok(Default::default())
    }));
    let (entity, _) = cx.add_window_view(move |window, cx| {
        let recipe = compile(
            &source(false, false, false),
            Instant::now() + std::time::Duration::from_secs(1),
        )
        .unwrap()
        .view;
        let live = LiveInput(Rc::new(RefCell::new(Some(input.clone()))));
        Harness {
            state: recipe.create(live, window, cx),
            recipe,
            slots: Slots::new(vec![], input, |_, _, _| {
                Err(maka_ui::Error::Invalid("unknown slot".into()))
            }),
        }
    });
    let window = cx.windows()[0];
    draw(cx, window);
    (entity, window, clicks)
}
fn draw(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.update_window(handle, |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
}
fn replace(
    cx: &mut TestAppContext,
    window: AnyWindowHandle,
    entity: &Entity<Harness>,
    value: Value,
) {
    cx.update_window(window, |_, window, cx| {
        entity.update(cx, |h, cx| h.replace(value, window, cx))
    })
    .unwrap();
}
#[gpui::test]
fn scene_focus_and_scroll_follow_local_key_across_layout_ancestors(cx: &mut TestAppContext) {
    let (entity, window, clicks) = harness(cx);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    assert_eq!(clicks.get(), 1, "native button positive control");
    let focus = cx.update(|cx| entity.read(cx).state().buttons["action"].focus.clone());
    cx.update_window(window, |_, w, cx| {
        assert!(focus.is_focused(w));
        entity.read(cx).state().scrolls["scroll"].set_offset(point(px(0.), px(-20.)));
    })
    .unwrap();
    replace(cx, window, &entity, source(false, true, false));
    draw(cx, window);
    cx.update_window(window, |_, w, cx| {
        let state = entity.read(cx).state();
        assert_eq!(state.buttons["action"].focus, focus);
        assert!(focus.is_focused(w));
        assert_eq!(state.scrolls["scroll"].offset(), point(px(0.), px(-20.)));
    })
    .unwrap();
}
#[gpui::test]
fn scene_disabled_and_removed_nodes_reject_old_pointer_keyboard_and_ax_focus(
    cx: &mut TestAppContext,
) {
    let (entity, window, clicks) = harness(cx);
    cx.activate_accessibility(window);
    draw(cx, window);
    let target = cx
        .accessibility_update(window)
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Button && n.label() == Some("Run"))
        .unwrap()
        .0;
    let action = |cx: &mut TestAppContext, kind| {
        cx.simulate_accessibility_action(
            window,
            accesskit::ActionRequest {
                action: kind,
                target_tree: accesskit::TreeId::ROOT,
                target_node: target,
                data: None,
            },
        )
    };
    action(cx, AccessibleAction::Click);
    assert_eq!(clicks.get(), 1);
    cx.with_paused_automatic_drawing(|cx| {
        let painted = cx.accessibility_update(window).unwrap();
        replace(cx, window, &entity, source(true, false, false));
        assert_eq!(
            cx.accessibility_update(window).unwrap().nodes,
            painted.nodes
        );
        VisualTestContext::from_window(window, cx)
            .simulate_click(point(px(10.), px(10.)), Modifiers::default());
        action(cx, AccessibleAction::Focus);
        action(cx, AccessibleAction::Click);
        cx.simulate_keystrokes(window, "enter space tab enter");
        assert_eq!(
            clicks.get(),
            1,
            "all old enabled listeners consult live disabled facts"
        );
        cx.update_window(window, |_, w, cx| assert!(w.focused(cx).is_none()))
            .unwrap();
    });
    draw(cx, window);
    let update = cx.accessibility_update(window).unwrap();
    let button = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Button && n.label() == Some("Run"))
        .unwrap();
    assert!(button.1.is_disabled());
    replace(cx, window, &entity, source(false, false, false));
    draw(cx, window);
    let update = cx.accessibility_update(window).unwrap();
    let target = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Button && n.label() == Some("Run"))
        .unwrap()
        .0;
    cx.simulate_accessibility_action(
        window,
        accesskit::ActionRequest {
            action: AccessibleAction::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: target,
            data: None,
        },
    );
    assert_eq!(
        clicks.get(),
        2,
        "re-enabled native action restores admission"
    );
    cx.with_paused_automatic_drawing(|cx| {
        replace(cx, window, &entity, source(false, false, true));
        VisualTestContext::from_window(window, cx)
            .simulate_click(point(px(10.), px(10.)), Modifiers::default());
        cx.simulate_accessibility_action(
            window,
            accesskit::ActionRequest {
                action: AccessibleAction::Focus,
                target_tree: accesskit::TreeId::ROOT,
                target_node: target,
                data: None,
            },
        );
        assert_eq!(clicks.get(), 2);
        cx.update_window(window, |_, w, cx| assert!(w.focused(cx).is_none()))
            .unwrap();
    });
}
#[gpui::test]
fn scene_slot_render_errors_are_returned_before_publication(cx: &mut TestAppContext) {
    let (_, window, _) = harness(cx);
    let recipe=compile(&json!({"kind":"component","key":"missing","binding":"child","member":0,"props":{},"on":{}}),Instant::now()+std::time::Duration::from_secs(1)).unwrap().view;
    cx.update_window(window, |_, w, cx| {
        let input = Input(Rc::new(|_, _, _, _| Ok(Default::default())));
        let state = recipe.create(LiveInput(Rc::new(RefCell::new(Some(input.clone())))), w, cx);
        let slots = Slots::new(vec!["missing".into()], input, |_, _, _| {
            Err(maka_ui::Error::Invalid("detached exact child".into()))
        });
        let error = recipe
            .render(state.as_ref(), &slots, w, cx)
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("detached exact child"));
    })
    .unwrap();
}
#[test]
fn scene_compilation_rejects_unsupported_shapes_expansion_deadline_and_style() {
    let deadline = || Instant::now() + std::time::Duration::from_secs(1);
    assert!(compile(&source(false, false, false), deadline()).is_ok());
    assert!(compile(&source(false, false, false), Instant::now()).is_err());
    let mut bad = source(false, false, false);
    bad["style"]["width"] = json!({"unit":"pixels","value":-1});
    assert!(compile(&bad, deadline()).is_err());
    let mut bad = source(false, false, false);
    bad["children"][0]["children"][0]["kind"] = json!("execute_service");
    assert!(compile(&bad, deadline()).is_err());
    let mut deep = json!({"kind":"text","key":"text","text":"x","color":0});
    for i in 0..33 {
        deep = json!({"kind":"stack","key":format!("s{i}"),"direction":"column","gap":0,"padding":0,"background":0,"children":[deep]});
    }
    assert!(compile(&deep, deadline()).is_err());
}

#[gpui::test]
fn retained_scene_recipe_cannot_restore_disabled_tab_admission(cx: &mut TestAppContext) {
    let (entity, window, clicks) = harness(cx);
    let old = cx.update(|cx| entity.read(cx).recipe.clone());
    replace(cx, window, &entity, source(true, false, false));
    // Exercise the actual older recipe against the original, current owner.
    cx.update(|cx| {
        entity.update(cx, |h, cx| {
            h.recipe = old;
            cx.notify();
        })
    });
    draw(cx, window);
    cx.simulate_keystrokes(window, "tab enter space");
    cx.update_window(window, |_, w, cx| {
        let state = entity.read(cx).state();
        assert!(!state.buttons["action"].focus.tab_stop);
        assert!(w.focused(cx).is_none());
    })
    .unwrap();
    assert_eq!(clicks.get(), 0);
}
#[gpui::test]
fn native_scene_deactivation_blurs_only_its_focus_with_old_listeners_retained(
    cx: &mut TestAppContext,
) {
    let (entity, window, clicks) = harness(cx);
    VisualTestContext::from_window(window, cx)
        .simulate_click(point(px(10.), px(10.)), Modifiers::default());
    assert_eq!(clicks.get(), 1);
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, w, cx| {
            entity.update(cx, |h, cx| {
                let state = h.state.downcast_mut::<SceneState>().unwrap();
                assert!(state.buttons["action"].focus.is_focused(w));
                state.input.0.borrow_mut().take();
                h.recipe.deactivate(h.state.as_mut(), w, cx);
                assert!(w.focused(cx).is_none());
            })
        })
        .unwrap();
        VisualTestContext::from_window(window, cx)
            .simulate_click(point(px(10.), px(10.)), Modifiers::default());
        cx.simulate_keystrokes(window, "tab enter space");
        assert_eq!(clicks.get(), 1);
        cx.update_window(window, |_, w, cx| assert!(w.focused(cx).is_none()))
            .unwrap();
    });
}
#[test]
fn desktop_styles_reject_source_overflow_and_native_nonfinite_values() {
    for value in [1e300, -1e300, 16000.0001] {
        let mut scene = source(false, false, false);
        scene["style"]["minWidth"] = json!(value);
        assert!(compile(&scene, Instant::now() + std::time::Duration::from_secs(1)).is_err());
    }
    for value in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        assert!(
            Style {
                min_width: Some(value),
                ..Style::default()
            }
            .validate()
            .is_err()
        );
    }
}
