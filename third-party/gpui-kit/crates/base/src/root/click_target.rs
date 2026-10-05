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
use gpui::{
    AccessibleAction, AnyWindowHandle, Bounds, ClickEvent, ClickHandle, Element, ElementId,
    FocusHandle, GlobalElementId, InputEvent, InspectorElementId, KeyDownEvent, KeyUpEvent,
    Keystroke, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Role,
    ScrollHandle, StatefulInteractiveElement, TestAppContext, accesskit, point, px, size,
};
use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

#[derive(Default)]
struct Observations {
    handle: RefCell<Option<ClickHandle>>,
    bounds: Cell<Option<Bounds<Pixels>>>,
    clicks: RefCell<Vec<ClickEvent>>,
    hover: Cell<bool>,
    swallowed_mouse_up: Cell<bool>,
    swallowed_key_up: Cell<bool>,
    cancel_captured_up: Cell<bool>,
    nested_input: Cell<bool>,
    pointer_events: Cell<usize>,
    custom_ax: Cell<usize>,
    fresh_ax_context: Cell<bool>,
    drags: Cell<usize>,
    paints: Cell<usize>,
}

// Delegates every native element phase and identity. The only instrumentation
// reads the original weak press locator and cancels it after native capture.
struct Target {
    inner: Stateful<Div>,
    observations: Rc<Observations>,
}
impl IntoElement for Target {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Target {
    type RequestLayoutState = <Stateful<Div> as Element>::RequestLayoutState;
    type PrepaintState = <Stateful<Div> as Element>::PrepaintState;
    fn id(&self) -> Option<ElementId> {
        Element::id(&self.inner)
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        self.inner.source_location()
    }
    fn a11y_role(&self) -> Option<Role> {
        self.inner.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.inner.write_a11y_info(node);
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout = self.inner.request_layout(id, inspector, window, cx);
        *self.observations.handle.borrow_mut() =
            self.inner.interactivity().click_handle(id, window);
        layout
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.observations.bounds.set(Some(bounds));
        self.inner
            .prepaint(id, inspector, bounds, state, window, cx)
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.observations
            .paints
            .set(self.observations.paints.get() + 1);
        self.inner
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
        let observations = self.observations.clone();
        window.on_mouse_event(move |_: &MouseUpEvent, phase, window, cx| {
            if phase.capture() {
                if observations.cancel_captured_up.replace(false) {
                    assert!(observations.handle.borrow().as_ref().unwrap().cancel());
                }
                if observations.nested_input.replace(false) {
                    window.dispatch_event(
                        KeyDownEvent {
                            keystroke: Keystroke::parse("a").unwrap(),
                            is_held: false,
                            prefer_character_input: false,
                        }
                        .to_platform_input(),
                        cx,
                    );
                }
            }
        });
        let observations = self.observations.clone();
        window.on_key_event(move |_: &KeyUpEvent, phase, _, _| {
            if phase.capture() && observations.cancel_captured_up.replace(false) {
                assert!(observations.handle.borrow().as_ref().unwrap().cancel());
            }
        });
    }
}

struct DragView;
impl Render for DragView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

struct Content {
    focus: FocusHandle,
    observations: Rc<Observations>,
    empty: bool,
    clipped: bool,
    custom_ax: bool,
    draggable: bool,
    scroll: ScrollHandle,
}
impl Render for Content {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let regions: Arc<[Bounds<Pixels>]> = if self.empty {
            Arc::from([])
        } else {
            Arc::from([
                Bounds::new(point(px(0.), px(0.)), size(px(20.), px(20.))),
                Bounds::new(point(px(80.), px(60.)), size(px(20.), px(20.))),
            ])
        };
        let clicks = self.observations.clone();
        let hover = self.observations.clone();
        let pointer = self.observations.clone();
        let mut inner = div()
            .id("logical-link")
            .track_focus(&self.focus)
            .tab_stop(true)
            .role(Role::Link)
            .aria_label("one wrapped link")
            .hitbox_regions(regions)
            .w(px(100.))
            .h(px(80.))
            .flex_shrink_0()
            .on_click(move |click, window, cx| {
                if matches!(click, ClickEvent::Accessibility(_)) {
                    clicks
                        .fresh_ax_context
                        .set(cx.is_propagating() && !window.default_prevented());
                }
                clicks.clicks.borrow_mut().push(click.clone());
            })
            .on_hover(move |value, _, _| hover.hover.set(*value))
            .capture_any_mouse_down(move |_, _, _| {
                pointer.pointer_events.set(pointer.pointer_events.get() + 1)
            });
        if self.draggable {
            let observations = self.observations.clone();
            inner = inner.on_drag((), move |_, _, _, cx| {
                // Original press borrow must be released before external construction.
                assert!(observations.handle.borrow().as_ref().unwrap().cancel());
                observations.drags.set(observations.drags.get() + 1);
                cx.new(|_| DragView)
            });
        }
        if self.custom_ax {
            let ax = self.observations.clone();
            inner = inner.on_a11y_action(AccessibleAction::Click, move |_, _, _| {
                ax.custom_ax.set(ax.custom_ax.get() + 1)
            });
        }
        let mouse = self.observations.clone();
        let key = self.observations.clone();
        div()
            .id("parent")
            .w(px(160.))
            .h(px(140.))
            .capture_any_mouse_up(move |_, _, cx| {
                if mouse.swallowed_mouse_up.replace(false) {
                    cx.stop_propagation();
                }
            })
            .capture_key_up(move |_, _, cx| {
                if key.swallowed_key_up.replace(false) {
                    cx.stop_propagation();
                }
            })
            .child(
                div()
                    .id("clip")
                    .w(px(if self.clipped { 60. } else { 120. }))
                    .h(px(100.))
                    .overflow_hidden()
                    .child(
                        div()
                            .id("scroll")
                            .w(px(120.))
                            .h(px(100.))
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(Target {
                                        inner,
                                        observations: self.observations.clone(),
                                    })
                                    .child(div().h(px(160.)).flex_shrink_0()),
                            ),
                    ),
            )
    }
}
fn open(
    cx: &mut TestAppContext,
    configure: impl FnOnce(&mut Content),
) -> (AnyWindowHandle, Entity<Content>, Rc<Observations>) {
    cx.update(crate::init);
    let observations = Rc::new(Observations::default());
    let mut content = None;
    let window = cx
        .open_window(size(px(240.), px(180.)), |window, cx| {
            let view = cx.new(|cx| {
                let mut view = Content {
                    focus: cx.focus_handle().tab_stop(true),
                    observations: observations.clone(),
                    empty: false,
                    clipped: false,
                    custom_ax: false,
                    draggable: false,
                    scroll: ScrollHandle::new(),
                };
                configure(&mut view);
                view
            });
            content = Some(view.clone());
            Root::new(view, window, cx)
        })
        .into();
    settle(cx, window);
    (window, content.unwrap(), observations)
}
fn settle(cx: &mut TestAppContext, window: AnyWindowHandle) {
    for _ in 0..8 {
        if !cx.simulate_scheduled_frame(window) {
            return;
        }
    }
    panic!("native click target did not settle");
}
fn dispatch<E: InputEvent>(cx: &mut TestAppContext, window: AnyWindowHandle, event: E) {
    cx.update_window(window, |_, window, cx| {
        window.dispatch_event(event.to_platform_input(), cx);
    })
    .unwrap();
    cx.run_until_parked();
}
fn position(observations: &Observations, x: f32, y: f32) -> gpui::Point<Pixels> {
    observations.bounds.get().unwrap().origin + point(px(x), px(y))
}
fn down(cx: &mut TestAppContext, window: AnyWindowHandle, position: gpui::Point<Pixels>) {
    dispatch(
        cx,
        window,
        MouseDownEvent {
            button: MouseButton::Left,
            position,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        },
    );
}
fn up(
    cx: &mut TestAppContext,
    window: AnyWindowHandle,
    position: gpui::Point<Pixels>,
    button: MouseButton,
) {
    dispatch(
        cx,
        window,
        MouseUpEvent {
            button,
            position,
            modifiers: Default::default(),
            click_count: 1,
        },
    );
}
fn focus(cx: &mut TestAppContext, window: AnyWindowHandle, view: &Entity<Content>) -> FocusHandle {
    let focus = cx.read(|cx| view.read(cx).focus.clone());
    cx.update_window(window, |_, window, cx| window.focus(&focus, cx))
        .unwrap();
    settle(cx, window);
    focus
}
fn key_down(cx: &mut TestAppContext, window: AnyWindowHandle, key: &str) {
    dispatch(
        cx,
        window,
        KeyDownEvent {
            keystroke: Keystroke::parse(key).unwrap(),
            is_held: false,
            prefer_character_input: false,
        },
    );
}
fn key_up(cx: &mut TestAppContext, window: AnyWindowHandle, key: &str) {
    dispatch(
        cx,
        window,
        KeyUpEvent {
            keystroke: Keystroke::parse(key).unwrap(),
        },
    );
}
fn ax_click(cx: &mut TestAppContext, window: AnyWindowHandle) {
    cx.activate_accessibility(window);
    settle(cx, window);
    let tree = cx.accessibility_update(window).unwrap();
    let links: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Link)
        .collect();
    assert_eq!(
        links.len(),
        1,
        "one logical native accessibility node for both fragments"
    );
    assert!(links[0].1.supports_action(AccessibleAction::Click));
    cx.simulate_accessibility_action(
        window,
        accesskit::ActionRequest {
            action: AccessibleAction::Click,
            target_node: links[0].0,
            target_tree: accesskit::TreeId::ROOT,
            data: None,
        },
    );
}

#[gpui::test]
fn native_fragments_share_one_real_press_and_leave_blank_union_uninteractive(
    cx: &mut TestAppContext,
) {
    let (window, _, observations) = open(cx, |_| {});
    let first = position(&observations, 10., 10.);
    let last = position(&observations, 90., 70.);
    let blank = position(&observations, 50., 40.);
    down(cx, window, first);
    dispatch(
        cx,
        window,
        MouseMoveEvent {
            position: last,
            pressed_button: Some(MouseButton::Left),
            modifiers: Default::default(),
        },
    );
    up(cx, window, last, MouseButton::Left);
    assert_eq!(observations.clicks.borrow().len(), 1);
    let ClickEvent::Mouse(event) = observations.clicks.borrow()[0].clone() else {
        panic!("actual original mouse pair required")
    };
    assert_eq!(event.down.position, first);
    assert_eq!(event.up.position, last);
    down(cx, window, blank);
    up(cx, window, blank, MouseButton::Left);
    down(cx, window, first);
    up(cx, window, last, MouseButton::Right);
    up(cx, window, last, MouseButton::Left);
    assert_eq!(observations.clicks.borrow().len(), 1);
}

#[gpui::test]
fn native_fragments_obey_original_mask_and_scroll_coordinates(cx: &mut TestAppContext) {
    let (window, view, observations) = open(cx, |v| v.clipped = true);
    let hidden = position(&observations, 90., 70.);
    down(cx, window, hidden);
    up(cx, window, hidden, MouseButton::Left);
    assert!(observations.clicks.borrow().is_empty());
    cx.update(|cx| {
        view.update(cx, |view, cx| {
            view.clipped = false;
            cx.notify();
        })
    });
    settle(cx, window);
    let scroll = cx.read(|cx| view.read(cx).scroll.clone());
    let before = observations.bounds.get().unwrap();
    scroll.set_offset(point(px(0.), px(-40.)));
    cx.update_window(window, |_, window, _| window.refresh())
        .unwrap();
    settle(cx, window);
    assert_eq!(
        observations.bounds.get().unwrap().origin.y,
        before.origin.y - px(40.)
    );
    let visible = position(&observations, 90., 70.);
    down(cx, window, visible);
    up(cx, window, visible, MouseButton::Left);
    assert_eq!(observations.clicks.borrow().len(), 1);
    let clipped_first = position(&observations, 10., 10.);
    down(cx, window, clipped_first);
    up(cx, window, clipped_first, MouseButton::Left);
    assert_eq!(observations.clicks.borrow().len(), 1);
}

#[gpui::test]
fn empty_native_fragments_never_hit(cx: &mut TestAppContext) {
    let (window, _, observations) = open(cx, |v| v.empty = true);
    let first = position(&observations, 10., 10.);
    down(cx, window, first);
    up(cx, window, first, MouseButton::Left);
    assert!(observations.clicks.borrow().is_empty());
    assert_eq!(observations.pointer_events.get(), 0);
}

#[gpui::test]
fn native_cancellation_rejects_captured_defaults_and_window_deactivation(cx: &mut TestAppContext) {
    let (window, view, observations) = open(cx, |_| {});
    let first = position(&observations, 10., 10.);
    down(cx, window, first);
    assert!(observations.handle.borrow().as_ref().unwrap().cancel());
    up(cx, window, first, MouseButton::Left);
    down(cx, window, first);
    observations.cancel_captured_up.set(true);
    up(cx, window, first, MouseButton::Left);
    focus(cx, window, &view);
    key_down(cx, window, "enter");
    observations.cancel_captured_up.set(true);
    key_up(cx, window, "enter");
    cx.simulate_window_activation(window, true);
    down(cx, window, first);
    cx.simulate_window_activation(window, false);
    cx.simulate_window_activation(window, true);
    up(cx, window, first, MouseButton::Left);
    key_down(cx, window, "space");
    cx.simulate_window_activation(window, false);
    cx.simulate_window_activation(window, true);
    key_up(cx, window, "space");
    assert!(observations.clicks.borrow().is_empty());
    key_down(cx, window, "space");
    key_up(cx, window, "space");
    assert_eq!(observations.clicks.borrow().len(), 1);
}

#[gpui::test]
fn native_parent_swallowed_release_and_nested_input_cannot_resurrect_press(
    cx: &mut TestAppContext,
) {
    let (window, view, observations) = open(cx, |_| {});
    let first = position(&observations, 10., 10.);
    down(cx, window, first);
    observations.swallowed_mouse_up.set(true);
    up(cx, window, first, MouseButton::Left);
    up(cx, window, first, MouseButton::Left);
    focus(cx, window, &view);
    key_down(cx, window, "enter");
    observations.swallowed_key_up.set(true);
    key_up(cx, window, "enter");
    key_up(cx, window, "enter");
    down(cx, window, first);
    observations.nested_input.set(true);
    up(cx, window, first, MouseButton::Left);
    assert!(observations.clicks.borrow().is_empty());
    down(cx, window, first);
    up(cx, window, first, MouseButton::Left);
    assert_eq!(observations.clicks.borrow().len(), 1);
}

#[gpui::test]
fn native_keyboard_requires_exact_key_and_same_original_focus_generation(cx: &mut TestAppContext) {
    let (window, view, observations) = open(cx, |_| {});
    let handle = focus(cx, window, &view);
    key_down(cx, window, "space");
    key_up(cx, window, "enter");
    key_up(cx, window, "space");
    key_down(cx, window, "enter");
    cx.update_window(window, |_, window, cx| {
        window.blur(cx);
        window.focus(&handle, cx);
    })
    .unwrap();
    settle(cx, window);
    key_up(cx, window, "enter");
    assert!(observations.clicks.borrow().is_empty());
    cx.update_window(window, |_, window, cx| {
        window.focus_next(cx);
        assert!(handle.is_focused(window));
    })
    .unwrap();
    key_down(cx, window, "enter");
    key_up(cx, window, "enter");
    assert_eq!(observations.clicks.borrow().len(), 1);
}

#[gpui::test]
fn native_accessibility_activates_logical_target_without_fabricated_pointer(
    cx: &mut TestAppContext,
) {
    let (window, _, observations) = open(cx, |_| {});
    ax_click(cx, window);
    assert_eq!(
        observations.pointer_events.get(),
        0,
        "union centre is blank and no physical event is fabricated"
    );
    let clicks = observations.clicks.borrow();
    assert_eq!(clicks.len(), 1);
    assert!(matches!(clicks[0], ClickEvent::Accessibility(_)));
    assert_eq!(clicks[0].mouse_position(), None);
}

#[gpui::test]
fn native_explicit_accessibility_callback_wins_once(cx: &mut TestAppContext) {
    let (window, _, observations) = open(cx, |v| v.custom_ax = true);
    ax_click(cx, window);
    assert_eq!(observations.custom_ax.get(), 1);
    assert!(observations.clicks.borrow().is_empty());
    assert_eq!(observations.pointer_events.get(), 0);
}

#[gpui::test]
fn native_swallowed_release_restores_hover_without_starting_drag(cx: &mut TestAppContext) {
    let (window, _, observations) = open(cx, |v| v.draggable = true);
    let first = position(&observations, 10., 10.);
    let last = position(&observations, 90., 70.);
    down(cx, window, first);
    let painted = observations.paints.get();
    cx.with_paused_automatic_drawing(|cx| {
        observations.swallowed_mouse_up.set(true);
        up(cx, window, first, MouseButton::Left);
        assert_eq!(
            observations.paints.get(),
            painted,
            "old press frame retained"
        );
        dispatch(
            cx,
            window,
            MouseMoveEvent {
                position: last,
                pressed_button: None,
                modifiers: Default::default(),
            },
        );
        assert_eq!(
            observations.paints.get(),
            painted,
            "move owns stale cleanup"
        );
        assert_eq!(observations.drags.get(), 0);
        assert!(!cx.read(App::has_active_drag));
        assert!(
            observations.hover.get(),
            "released press no longer suppresses hover"
        );
    });
    down(cx, window, first);
    dispatch(
        cx,
        window,
        MouseMoveEvent {
            position: last,
            pressed_button: Some(MouseButton::Left),
            modifiers: Default::default(),
        },
    );
    assert_eq!(
        observations.drags.get(),
        1,
        "a real current press still starts native drag"
    );
    assert!(cx.read(App::has_active_drag));
}

#[gpui::test]
fn native_keyboard_release_survives_dirty_draw_before_capture(cx: &mut TestAppContext) {
    let (window, view, observations) = open(cx, |_| {});
    focus(cx, window, &view);
    key_down(cx, window, "enter");
    let painted = observations.paints.get();
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(window, |_, window, cx| {
            window.refresh();
            assert_eq!(
                observations.paints.get(),
                painted,
                "dirty frame not yet drawn"
            );
            window.dispatch_event(
                KeyUpEvent {
                    keystroke: Keystroke::parse("enter").unwrap(),
                }
                .to_platform_input(),
                cx,
            );
            assert!(
                observations.paints.get() > painted,
                "keyup synchronously drew the dirty frame"
            );
            assert_eq!(observations.clicks.borrow().len(), 1);
        })
        .unwrap();
    });
    assert_eq!(observations.clicks.borrow().len(), 1);
    assert!(matches!(
        observations.clicks.borrow()[0],
        ClickEvent::Keyboard(_)
    ));
}

#[gpui::test]
fn native_accessibility_activation_starts_fresh_after_consumed_input(cx: &mut TestAppContext) {
    let (window, _, observations) = open(cx, |_| {});
    cx.update_window(window, |_, window, cx| {
        window.prevent_default();
        cx.stop_propagation();
    })
    .unwrap();
    ax_click(cx, window);
    assert!(
        observations.fresh_ax_context.get(),
        "AX listeners see fresh original input flags"
    );
    assert_eq!(
        observations.clicks.borrow().len(),
        1,
        "original target action ran exactly once"
    );
    assert_eq!(observations.pointer_events.get(), 0);
}
