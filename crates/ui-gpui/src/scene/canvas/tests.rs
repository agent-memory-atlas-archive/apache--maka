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
use crate::{
    Slots, Widget,
    scene::{SceneState, compile},
};
use gpui_kit::{
    AnyWindowHandle, AppContext, Context, Entity, Modifiers as NativeModifiers, Render,
    TestAppContext, VisualTestContext,
};
use serde_json::json;
use std::any::Any;

struct Harness {
    recipe: Rc<dyn Widget>,
    state: Box<dyn Any>,
    slots: Slots,
    live: LiveInput,
    actions: Rc<Cell<usize>>,
}
gpui_kit::actions!(canvas_test, [Shortcut]);
impl Render for Harness {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let actions = self.actions.clone();
        div()
            .key_context("CanvasTest")
            .on_action(move |_: &Shortcut, _, _| actions.set(actions.get() + 1))
            .child(
                self.recipe
                    .render(self.state.as_ref(), &self.slots, window, cx)
                    .unwrap(),
            )
    }
}
impl Harness {
    fn replace(&mut self, source: Value, window: &mut Window, cx: &mut Context<Self>) {
        let recipe = compile(&source, Instant::now() + std::time::Duration::from_secs(1))
            .unwrap()
            .view;
        recipe.commit(self.state.as_mut(), window, cx);
        self.recipe = recipe;
        cx.notify();
    }
    fn shared(&self) -> Rc<Shared> {
        self.state.downcast_ref::<SceneState>().unwrap().canvases["board"]
            .shared
            .clone()
    }
}
fn source(disabled: bool, removed: bool, move_rule: bool) -> Value {
    let listen = if move_rule {
        json!(["pointer_down", "drag_move", "drop", "key_down", "activate"])
    } else {
        json!(["pointer_down", "drop", "key_down", "activate"])
    };
    json!({"kind":"canvas","key":"board","style":{"width":{"unit":"pixels","value":240},"height":{"unit":"pixels","value":160}},
        "contents":{"commands":[{"kind":"rect","rect":{"origin":{"x":10,"y":10},"width":90,"height":60},"color":0x416653ffu32,"radius":8},
        {"kind":"line","from":{"x":100,"y":40},"to":{"x":140,"y":40},"color":0x777777ffu32,"width":2},
        {"kind":"text","position":{"x":18,"y":18},"text":"Card","color":0xffffffffu32,"size":14}],
        "regions":if removed { json!([]) } else { json!([
            {"key":"target","label":"Target","rect":{"origin":{"x":140,"y":10},"width":60,"height":60},"tabIndex":-1},
            {"key":"card","label":"Card","radius":8,"rect":{"origin":{"x":10,"y":10},"width":90,"height":60},"draggable":true,"disabled":disabled,"listen":listen}
        ]) },"focusColor":0x668877ffu32,"dragColor":0x416653ffu32}})
}
type Seen = Rc<RefCell<Vec<(u8, CanvasInput)>>>;
fn input(source: u8, seen: Seen, mode: Rc<Cell<u8>>) -> Input {
    Input(Rc::new(move |key, event, _, _| {
        assert_eq!(key, "board");
        seen.borrow_mut()
            .push((source, serde_json::from_value(event).unwrap()));
        if mode.get() == 4 {
            return Err(maka_ui::Error::Invalid("uncommitted rule".into()));
        }
        Ok(maka_ui::InputDecision {
            prevent_default: mode.get() & 1 != 0,
            stop_propagation: mode.get() & 2 != 0,
        })
    }))
}
type TestHarness = (
    Entity<Harness>,
    AnyWindowHandle,
    Seen,
    Rc<Cell<u8>>,
    Rc<Cell<usize>>,
);
fn harness(cx: &mut TestAppContext, move_rule: bool) -> TestHarness {
    let seen = Rc::new(RefCell::new(vec![]));
    let mode = Rc::new(Cell::new(0));
    let actions = Rc::new(Cell::new(0));
    let dispatcher = input(0, seen.clone(), mode.clone());
    let observed = actions.clone();
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.bind_keys([gpui_kit::KeyBinding::new(
            "enter",
            Shortcut,
            Some("CanvasTest"),
        )]);
    });
    let (entity, _) = cx.add_window_view(move |window, cx| {
        let recipe = compile(
            &source(false, false, move_rule),
            Instant::now() + std::time::Duration::from_secs(1),
        )
        .unwrap()
        .view;
        let live = LiveInput::new(
            Rc::new(RefCell::new(Some(dispatcher.clone()))),
            crate::NativeBudget::default(),
            std::sync::Arc::new(maka_plugins::call::Resources::default()),
        );
        let mut state = recipe.create(live.clone(), window, cx);
        recipe.presentation(state.as_mut(), true, window, cx);
        Harness {
            state,
            recipe,
            slots: Slots::new(vec![], dispatcher, |_, _, _| {
                Err(maka_ui::Error::Invalid("no slots".into()))
            }),
            live,
            actions: observed,
        }
    });
    let window = cx.windows()[0];
    draw(cx, window);
    (entity, window, seen, mode, actions)
}
fn draw(cx: &mut TestAppContext, window: AnyWindowHandle) {
    cx.update_window(window, |_, w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
}
fn down(cx: &mut TestAppContext, window: AnyWindowHandle, x: f32, y: f32) {
    cx.update_window(window, |_, w, cx| {
        w.dispatch_event(
            gpui_kit::PlatformInput::MouseDown(MouseDownEvent {
                button: MouseButton::Left,
                position: point(px(x), px(y)),
                modifiers: NativeModifiers::default(),
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
    })
    .unwrap();
}
fn movement(cx: &mut TestAppContext, window: AnyWindowHandle, x: f32, y: f32) {
    cx.update_window(window, |_, w, cx| {
        w.dispatch_event(
            gpui_kit::PlatformInput::MouseMove(MouseMoveEvent {
                position: point(px(x), px(y)),
                pressed_button: Some(MouseButton::Left),
                modifiers: NativeModifiers::default(),
            }),
            cx,
        );
    })
    .unwrap();
}
fn up(cx: &mut TestAppContext, window: AnyWindowHandle, x: f32, y: f32) {
    cx.update_window(window, |_, w, cx| {
        w.dispatch_event(
            gpui_kit::PlatformInput::MouseUp(MouseUpEvent {
                button: MouseButton::Left,
                position: point(px(x), px(y)),
                modifiers: NativeModifiers::default(),
                click_count: 1,
            }),
            cx,
        );
    })
    .unwrap();
}
#[gpui::test]
fn native_gesture_tracks_without_guest_moves_uses_latest_drop_and_original_window_geometry(
    cx: &mut TestAppContext,
) {
    let (entity, window, seen, mode, _) = harness(cx, false);
    down(cx, window, 20., 20.);
    let shared = cx.update(|cx| entity.read(cx).shared());
    let original = shared.gesture.borrow().clone().unwrap();
    assert_eq!(seen.borrow().len(), 1);
    cx.update_window(window, |_, w, _| {
        assert!(original.region.owner.focus.is_focused(w))
    })
    .unwrap();
    // A committed layout change moves the Canvas while the same local owner
    // and gesture remain. The physical grab point cannot jump with the parent.
    cx.update_window(window,|_,w,cx| {
        let mut value=source(false,false,false);
        value=json!({"kind":"stack","key":"layout","direction":"column","padding":0,"gap":0,"background":0xffffff,"children":[{"kind":"text","key":"spacer","text":"","color":0,"style":{"height":{"unit":"pixels","value":40},"shrink":0}},value]});
        entity.update(cx,|h,cx| { h.replace(value,w,cx); *h.live.0.borrow_mut()=Some(input(1,seen.clone(),mode.clone())); });
    }).unwrap();
    draw(cx, window);
    assert!(Rc::ptr_eq(
        &original.region.owner,
        &shared.regions.borrow()["card"]
    ));
    movement(cx, window, 40., 70.);
    assert_eq!(
        seen.borrow().len(),
        1,
        "unsubscribed high frequency move is wholly native"
    );
    let gesture = shared.gesture.borrow().clone().unwrap();
    assert_eq!(gesture.delta.x, 20.);
    assert_eq!(gesture.delta.y, 50.);
    assert_eq!(
        gesture.bounds.origin,
        point(px(10.), px(10.)),
        "outline remains based on original painted window rect"
    );
    assert_eq!(
        gesture.position.y, 30.,
        "input position uses latest painted canvas origin"
    );
    draw(cx, window);
    cx.update_window(window, |_, w, _| {
        assert!(
            w.painted_quads()
                .iter()
                .any(|q| q.bounds.origin.x.0 == 30. * w.scale_factor()
                    && q.bounds.origin.y.0 == 60. * w.scale_factor()
                    && q.border_color == gpui_kit::Hsla::from(rgba(0x416653ff))
                    && q.corner_radii.top_left.0 == 8. * w.scale_factor()),
            "actual drag outline uses original window rect after layout move"
        );
    })
    .unwrap();
    up(cx, window, 160., 60.);
    assert!(shared.gesture.borrow().is_none());
    let events = seen.borrow();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].0, 1);
    assert!(
        matches!(&events[1].1,CanvasInput::Drop { target:Some(target), delta, .. } if target=="target" && delta.x==140. && delta.y==40.)
    );
}
#[gpui::test]
fn foreground_rules_gate_focus_and_moves_and_keyboard_precedes_native_action(
    cx: &mut TestAppContext,
) {
    let (entity, window, seen, mode, actions) = harness(cx, true);
    let shared = cx.update(|cx| entity.read(cx).shared());
    for reject in [1, 4] {
        mode.set(reject);
        down(cx, window, 20., 20.);
        assert!(shared.gesture.borrow().is_none());
        cx.update_window(window, |_, w, cx| assert!(w.focused(cx).is_none()))
            .unwrap();
    }
    mode.set(0);
    down(cx, window, 20., 20.);
    let accepted = shared.gesture.borrow().clone().unwrap();
    mode.set(1);
    movement(cx, window, 50., 50.);
    assert_eq!(
        shared.gesture.borrow().as_ref().unwrap().position.x,
        accepted.position.x,
        "prevented move has no native default"
    );
    mode.set(0);
    movement(cx, window, 50., 50.);
    assert_eq!(shared.gesture.borrow().as_ref().unwrap().position.x, 50.);
    mode.set(2);
    cx.simulate_keystrokes(window, "enter");
    assert_eq!(
        actions.get(),
        0,
        "early stop propagation prevents actual bound action"
    );
    assert!(
        matches!(&seen.borrow().last().unwrap().1,CanvasInput::KeyDown { key,.. } if key=="enter")
    );
    mode.set(0);
    cx.simulate_keystrokes(window, "enter");
    assert_eq!(actions.get(), 1, "native action positive control");
    mode.set(1);
    cx.simulate_keystrokes(window, "enter");
    assert_eq!(
        actions.get(),
        2,
        "GPUI action consumption requires stopPropagation; preventDefault alone does not bypass action resolution"
    );
    mode.set(4);
    movement(cx, window, 60., 60.);
    assert!(
        shared.gesture.borrow().is_none(),
        "failure cancels native session"
    );
    let before = seen.borrow().len();
    up(cx, window, 160., 20.);
    assert_eq!(seen.borrow().len(), before);
}
#[gpui::test]
fn original_region_tokens_reject_before_paint_recreation_route_changes_and_ax(
    cx: &mut TestAppContext,
) {
    let (entity, window, seen, _, _) = harness(cx, false);
    cx.activate_accessibility(window);
    draw(cx, window);
    let target = cx
        .accessibility_update(window)
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.role() == gpui_kit::Role::Button && n.label() == Some("Card"))
        .unwrap()
        .0;
    down(cx, window, 20., 20.);
    let shared = cx.update(|cx| entity.read(cx).shared());
    let original = shared.regions.borrow()["card"].clone();
    let before = seen.borrow().len();
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, w, cx| {
            entity.update(cx, |h, cx| h.replace(source(false, true, false), w, cx));
            entity.update(cx, |h, cx| h.replace(source(false, false, false), w, cx));
        })
        .unwrap();
        assert!(!original.admitted.get());
        assert!(shared.gesture.borrow().is_none());
        assert!(!Rc::ptr_eq(&original, &shared.regions.borrow()["card"]));
        down(cx, window, 20., 20.);
        movement(cx, window, 40., 40.);
        up(cx, window, 160., 20.);
        for action in [
            gpui_kit::AccessibleAction::Focus,
            gpui_kit::AccessibleAction::Click,
        ] {
            cx.simulate_accessibility_action(
                window,
                gpui_kit::accesskit::ActionRequest {
                    action,
                    target_tree: gpui_kit::accesskit::TreeId::ROOT,
                    target_node: target,
                    data: None,
                },
            );
        }
        assert_eq!(seen.borrow().len(), before);
        cx.update_window(window, |_, w, cx| assert!(w.focused(cx).is_none()))
            .unwrap();
    });
    draw(cx, window);
    down(cx, window, 20., 20.);
    assert_eq!(seen.borrow().len(), before + 1);
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, w, cx| {
            entity.update(cx, |h, cx| h.replace(source(false, false, true), w, cx))
        })
        .unwrap();
        assert!(
            shared.gesture.borrow().is_none(),
            "changing original interaction route cancels gesture"
        );
        down(cx, window, 20., 20.);
        assert_eq!(
            seen.borrow().len(),
            before + 1,
            "old paint route stays rejected"
        );
    });
    cx.update_window(window, |_, w, cx| {
        entity.update(cx, |h, cx| h.recipe.deactivate(h.state.as_mut(), w, cx))
    })
    .unwrap();
    cx.simulate_keystrokes(window, "enter");
    assert_eq!(seen.borrow().len(), before + 1);
    assert!(shared.painted.borrow().is_none());
}

#[gpui::test]
fn retained_presentation_cancels_canvas_gesture_and_rejects_old_routes_before_paint(
    cx: &mut TestAppContext,
) {
    let (entity, window, seen, _, _) = harness(cx, true);
    cx.activate_accessibility(window);
    draw(cx, window);
    down(cx, window, 20., 20.);
    let shared = cx.update(|cx| entity.read(cx).shared());
    let region = shared.regions.borrow()["card"].clone();
    let target = cx
        .accessibility_update(window)
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Card") && node.role() == gpui_kit::Role::Button)
        .unwrap()
        .0;
    let before = seen.borrow().len();
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, window, cx| {
            entity.update(cx, |h, cx| {
                h.recipe.presentation(h.state.as_mut(), false, window, cx)
            })
        })
        .unwrap();
        assert!(shared.gesture.borrow().is_none());
        assert!(!region.admitted.get());
        assert!(shared.live.0.borrow().is_some());
        down(cx, window, 20., 20.);
        movement(cx, window, 40., 40.);
        up(cx, window, 160., 20.);
        for action in [
            gpui_kit::AccessibleAction::Focus,
            gpui_kit::AccessibleAction::Click,
        ] {
            cx.simulate_accessibility_action(
                window,
                gpui_kit::accesskit::ActionRequest {
                    action,
                    target_tree: gpui_kit::accesskit::TreeId::ROOT,
                    target_node: target,
                    data: None,
                },
            );
        }
        cx.simulate_keystrokes(window, "tab enter");
        assert_eq!(seen.borrow().len(), before);
        cx.update_window(window, |_, window, cx| {
            assert!(window.focused(cx).is_none());
            entity.update(cx, |h, cx| {
                h.recipe.presentation(h.state.as_mut(), true, window, cx)
            });
        })
        .unwrap();
        assert!(Rc::ptr_eq(&region, &shared.regions.borrow()["card"]));
        down(cx, window, 20., 20.);
        assert_eq!(
            seen.borrow().len(),
            before,
            "pre-hide paint route cannot revive after show"
        );
    });
    draw(cx, window);
    down(cx, window, 20., 20.);
    assert_eq!(seen.borrow().len(), before + 1);
}
#[test]
fn bounded_canvas_rejects_source_precision_and_work_before_native_conversion() {
    let deadline = || Instant::now() + std::time::Duration::from_secs(1);
    let mut value = source(false, false, false);
    value["contents"]["commands"][0]["rect"]["width"] = json!(16000.00000001_f64);
    assert!(compile(&value, deadline()).is_err());
    value = source(false, false, false);
    value["contents"]["commands"][2]["text"] = json!("a\nb");
    assert!(compile(&value, deadline()).is_err());
    value = source(false, false, false);
    let command = value["contents"]["commands"][0].clone();
    value["contents"]["commands"] = json!(vec![command; 1025]);
    assert!(compile(&value, deadline()).is_err());
    value = source(false, false, false);
    let region = value["contents"]["regions"][1].clone();
    value["contents"]["regions"]
        .as_array_mut()
        .unwrap()
        .push(region);
    assert!(compile(&value, deadline()).is_err());
    value = source(false, false, false);
    value["contents"]["regions"][1]["listen"] =
        json!(["drop", "drop", "drop", "drop", "drop", "drop"]);
    assert!(compile(&value, deadline()).is_err());
    value = source(false, false, false);
    value["contents"]["regions"][1]["radius"] = json!(8000.00000001_f64);
    assert!(compile(&value, deadline()).is_err());
    assert!(compile(&source(false, false, false), deadline()).is_ok());
}

#[gpui::test]
fn original_window_deactivation_cancels_native_gesture_without_plugin_callback(
    cx: &mut TestAppContext,
) {
    let (entity, window, seen, _, _) = harness(cx, false);
    cx.update_window(window, |_, w, _| w.activate_window())
        .unwrap();
    cx.run_until_parked();
    down(cx, window, 20., 20.);
    let shared = cx.update(|cx| entity.read(cx).shared());
    assert!(shared.gesture.borrow().is_some());
    let before = seen.borrow().len();
    VisualTestContext::from_window(window, cx).deactivate_window();
    assert!(shared.gesture.borrow().is_none());
    assert_eq!(seen.borrow().len(), before);
}

struct OtherWindow {
    focus: FocusHandle,
    actions: Rc<Cell<usize>>,
}
impl Render for OtherWindow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let actions = self.actions.clone();
        div()
            .size(px(200.))
            .track_focus(&self.focus)
            .key_context("CanvasTest")
            .on_action(move |_: &Shortcut, _, _| actions.set(actions.get() + 1))
    }
}
#[gpui::test]
fn global_key_interceptor_rejects_another_window_even_with_same_focus_handle(
    cx: &mut TestAppContext,
) {
    let (entity, window, seen, mode, actions) = harness(cx, false);
    mode.set(2);
    down(cx, window, 20., 20.);
    let focus = cx.update(|cx| {
        entity.read(cx).shared().regions.borrow()["card"]
            .focus
            .clone()
    });
    let other_focus = focus.clone();
    let other_actions = actions.clone();
    let (_, _) = cx.add_window_view(move |_, _| OtherWindow {
        focus: other_focus,
        actions: other_actions,
    });
    let other = cx.windows().into_iter().find(|w| *w != window).unwrap();
    draw(cx, other);
    cx.update_window(other, |_, w, cx| focus.focus(w, cx))
        .unwrap();
    let before = seen.borrow().len();
    cx.simulate_keystrokes(other, "enter");
    assert_eq!(seen.borrow().len(), before);
    assert_eq!(
        actions.get(),
        1,
        "other window's native action remains available"
    );
}
