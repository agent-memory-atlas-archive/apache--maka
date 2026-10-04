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
    AccessibleAction, AnyWindowHandle, Bounds, FocusHandle, InputEvent, ListAlignment, ListState,
    Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Role, ScrollHandle,
    StatefulInteractiveElement, TestAppContext, accesskit, canvas, list, point, px, size,
};
use std::cell::Cell;

struct Content {
    focus: FocusHandle,
    outer: ScrollHandle,
    inner: ScrollHandle,
    target_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    clicks: Rc<Cell<usize>>,
    spacer: f32,
    height: f32,
    nested: bool,
    wrapped: bool,
    sibling_list: bool,
    noisy_sibling: bool,
    sibling_first: bool,
    noise: Rc<Cell<bool>>,
    caret: bool,
    list: ListState,
}
impl Content {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            outer: ScrollHandle::new(),
            inner: ScrollHandle::new(),
            target_bounds: Rc::default(),
            clicks: Rc::default(),
            spacer: 250.,
            height: 32.,
            nested: false,
            wrapped: false,
            sibling_list: false,
            noisy_sibling: false,
            sibling_first: false,
            noise: Rc::default(),
            caret: false,
            list: ListState::new(4, ListAlignment::Top, px(0.)),
        }
    }
}
impl Render for Content {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let bounds = self.target_bounds.clone();
        let clicks = self.clicks.clone();
        let caret = self.caret;
        let focus = self.focus.clone();
        let target = div()
            .id("target")
            .track_focus(&self.focus)
            .tab_stop(true)
            .role(Role::Button)
            .aria_label("Reveal target")
            .flex_shrink_0()
            .h(px(self.height))
            .w_full()
            .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
            .child(
                canvas(
                    move |actual, window, _| {
                        bounds.set(Some(actual));
                        if caret && focus.is_focused(window) {
                            window.request_autoscroll(Bounds::new(
                                actual.origin + point(px(0.), px(300.)),
                                size(px(8.), px(16.)),
                            ));
                        }
                    },
                    |_, _, _, _| {},
                )
                .size_full(),
            );
        let content = if self.nested {
            div()
                .id("inner")
                .flex()
                .flex_col()
                .flex_shrink_0()
                .h(px(80.))
                .w_full()
                .overflow_y_scroll()
                .track_scroll(&self.inner)
                .child(div().flex_shrink_0().h(px(200.)))
                .child(target)
                .child(div().flex_shrink_0().h(px(100.)))
                .into_any_element()
        } else if self.wrapped {
            div()
                .id("outer-control-border")
                .track_focus(&self.focus)
                .flex_shrink_0()
                .border_1()
                .h(px(self.height + 2.))
                .w_full()
                .child(target)
                .into_any_element()
        } else {
            target.into_any_element()
        };
        let sibling_first = self.sibling_first;
        let mut outer = div()
            .with_dynamic_prepaint_order(move |_, _| {
                if sibling_first {
                    smallvec::smallvec![3, 0, 1, 2, 4]
                } else {
                    smallvec::smallvec![0, 1, 2, 3, 4]
                }
            })
            .id("outer")
            .flex()
            .flex_col()
            .h(px(120.))
            .w(px(200.))
            .overflow_y_scroll()
            .track_scroll(&self.outer)
            .child(div().flex_shrink_0().h(px(self.spacer)))
            .child(content);
        if self.sibling_list {
            outer = outer.child(
                list(self.list.clone(), |_, _, _| {
                    div().h(px(24.)).into_any_element()
                })
                .h(px(48.))
                .flex_shrink_0(),
            );
        }
        outer = outer.relative().child(div().flex_shrink_0().h(px(450.)));
        if self.noisy_sibling {
            let noise = self.noise.clone();
            let sibling = div()
                .id("noisy-sibling")
                .absolute()
                .top_0()
                .left_0()
                .size(px(16.))
                .overflow_y_scroll()
                .child(
                    canvas(
                        move |bounds, window, _| {
                            if noise.replace(false) {
                                window.request_autoscroll(bounds);
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .size_full(),
                )
                .into_any_element();
            outer = outer.child(sibling);
        }
        outer
    }
}

fn open(
    cx: &mut TestAppContext,
    configure: impl FnOnce(&mut Content),
) -> (AnyWindowHandle, Entity<Content>) {
    cx.update(crate::init);
    let mut content = None;
    let window = cx.open_window(size(px(240.), px(180.)), |window, cx| {
        let view = cx.new(|cx| {
            let mut view = Content::new(cx);
            configure(&mut view);
            view
        });
        content = Some(view.clone());
        Root::new(view, window, cx)
    });
    let window = window.into();
    settle(cx, window);
    (window, content.unwrap())
}
fn settle(cx: &mut TestAppContext, window: AnyWindowHandle) {
    for _ in 0..8 {
        if !cx.simulate_scheduled_frame(window) {
            return;
        }
    }
    panic!("native focus reveal did not settle within scheduled frames");
}
fn focus(cx: &mut TestAppContext, window: AnyWindowHandle, view: &Entity<Content>) {
    let focus = cx.read(|cx| view.read(cx).focus.clone());
    let before = target(cx, view);
    cx.update_window(window, |_, window, cx| window.focus(&focus, cx))
        .unwrap();
    assert_eq!(
        target(cx, view),
        before,
        "focus request does not force a draw"
    );
    assert!(
        cx.simulate_scheduled_frame(window),
        "native frame was actually scheduled"
    );
    assert_eq!(
        target(cx, view),
        before,
        "planning scroll does not rewrite this frame's geometry"
    );
    assert!(
        cx.simulate_scheduled_frame(window),
        "planned offset requests a normal next frame"
    );
    settle(cx, window);
}
fn target(cx: &TestAppContext, view: &Entity<Content>) -> Bounds<Pixels> {
    cx.read(|cx| view.read(cx).target_bounds.get().unwrap())
}

#[gpui::test]
fn focus_reveal_uses_scheduled_geometry_and_preserves_manual_scroll(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        let (window, view) = open(cx, |_| {});
        assert!(target(cx, &view).top() > px(120.));
        focus(cx, window, &view);
        let actual = target(cx, &view);
        assert_eq!(actual.bottom(), px(120.));
        let outer = cx.read(|cx| view.read(cx).outer.clone());
        assert_eq!(outer.offset().y, px(-162.));
        outer.set_offset(point(px(0.), px(0.)));
        cx.update_window(window, |_, window, _| window.refresh())
            .unwrap();
        settle(cx, window);
        assert_eq!(outer.offset().y, px(0.));
        assert_eq!(target(cx, &view).top(), px(250.));
    });
}

#[gpui::test]
fn focus_reveal_nested_offscreen_viewport_survives_sibling_list(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        let (window, view) = open(cx, |v| {
            v.nested = true;
            v.sibling_list = true;
        });
        focus(cx, window, &view);
        let actual = target(cx, &view);
        assert!(
            actual.top() >= px(0.) && actual.bottom() <= px(120.),
            "{actual:?}"
        );
        cx.read(|cx| {
            assert_eq!(view.read(cx).inner.offset().y, px(-152.));
            assert_eq!(view.read(cx).outer.offset().y, px(-210.));
        });
    });
}

#[gpui::test]
fn focus_reveal_oversized_control_aligns_leading_without_oscillation(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        let (window, view) = open(cx, |v| v.height = 400.);
        focus(cx, window, &view);
        assert_eq!(target(cx, &view).top(), px(0.));
        for _ in 0..3 {
            cx.update_window(window, |_, window, _| window.refresh())
                .unwrap();
            settle(cx, window);
            assert_eq!(target(cx, &view).top(), px(0.));
        }
    });
}

#[gpui::test]
fn focus_reveal_preserves_child_caret_request(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        let (window, view) = open(cx, |v| {
            v.height = 400.;
            v.caret = true;
        });
        focus(cx, window, &view);
        let actual = target(cx, &view);
        assert_eq!(actual.top() + px(316.), px(120.));
        assert_eq!(cx.read(|cx| view.read(cx).outer.offset().y), px(-446.));
    });
}

#[gpui::test]
fn focus_reveal_pointer_does_not_move_pressed_target_but_tab_reveals(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        let (window, view) = open(cx, |v| {
            v.spacer = 100.;
            v.height = 40.;
        });
        let position = point(px(10.), px(110.));
        cx.update_window(window, |_, window, cx| {
            window.dispatch_event(
                MouseDownEvent {
                    position,
                    modifiers: Modifiers::default(),
                    button: MouseButton::Left,
                    click_count: 1,
                    first_mouse: false,
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();
        settle(cx, window);
        assert_eq!(target(cx, &view).top(), px(100.));
        cx.update_window(window, |_, window, cx| {
            window.dispatch_event(
                MouseUpEvent {
                    position,
                    modifiers: Modifiers::default(),
                    button: MouseButton::Left,
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();
        settle(cx, window);
        assert_eq!(cx.read(|cx| view.read(cx).clicks.get()), 1);
        cx.update_window(window, |_, window, cx| window.blur(cx))
            .unwrap();
        cx.dispatch_keystroke(window, gpui::Keystroke::parse("tab").unwrap());
        settle(cx, window);
        assert_eq!(target(cx, &view).bottom(), px(120.));
    });
}

#[gpui::test]
fn focus_reveal_accessibility_focus_moves_actual_paint_and_accesskit_bounds(
    cx: &mut TestAppContext,
) {
    cx.with_paused_automatic_drawing(|cx| {
        let (window, view) = open(cx, |_| {});
        cx.activate_accessibility(window);
        settle(cx, window);
        let node = cx
            .accessibility_update(window)
            .unwrap()
            .nodes
            .into_iter()
            .find(|(_, node)| node.label() == Some("Reveal target"))
            .unwrap()
            .0;
        let before = target(cx, &view);
        cx.simulate_accessibility_action(
            window,
            accesskit::ActionRequest {
                action: AccessibleAction::Focus,
                target_tree: accesskit::TreeId::ROOT,
                target_node: node,
                data: None,
            },
        );
        assert_eq!(target(cx, &view), before);
        assert!(cx.simulate_scheduled_frame(window));
        assert_eq!(target(cx, &view), before);
        let first = cx
            .accessibility_update(window)
            .unwrap()
            .nodes
            .into_iter()
            .find(|(id, _)| *id == node)
            .unwrap()
            .1
            .bounds()
            .unwrap();
        let scale = cx
            .update_window(window, |_, window, _| window.scale_factor() as f64)
            .unwrap();
        assert_eq!(first.y0, f64::from(before.top()) * scale);
        assert!(cx.simulate_scheduled_frame(window));
        settle(cx, window);
        let actual = target(cx, &view);
        assert_eq!(actual.bottom(), px(120.));
        let scale = cx
            .update_window(window, |_, window, _| window.scale_factor() as f64)
            .unwrap();
        let update = cx.accessibility_update(window).unwrap();
        assert_eq!(update.focus, node);
        let bounds = update
            .nodes
            .into_iter()
            .find(|(id, _)| *id == node)
            .unwrap()
            .1
            .bounds()
            .unwrap();
        assert_eq!(bounds.y0, f64::from(actual.top()) * scale);
        assert_eq!(bounds.y1, f64::from(actual.bottom()) * scale);
    });
}

#[gpui::test]
fn focus_reveal_prioritizes_focus_over_unrelated_explicit_sibling_request(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        for first in [false, true] {
            let (window, view) = open(cx, |v| {
                v.noisy_sibling = true;
                v.sibling_first = first;
            });
            cx.read(|cx| view.read(cx).noise.set(true));
            focus(cx, window, &view);
            assert_eq!(target(cx, &view).bottom(), px(120.));
            assert!(
                !cx.read(|cx| view.read(cx).noise.get()),
                "sibling actually emitted its request"
            );
            cx.update_window(window, |_, window, _| window.remove_window())
                .unwrap();
            cx.run_until_parked();
        }
    });
}

struct CompetingRows {
    focus: FocusHandle,
    list: ListState,
    target: Rc<Cell<Option<Bounds<Pixels>>>>,
    noise: Rc<Cell<bool>>,
    noise_first: bool,
}
impl Render for CompetingRows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let focus = self.focus.clone();
        let target = self.target.clone();
        let noise = self.noise.clone();
        let noise_first = self.noise_first;
        list(self.list.clone(), move |row, _, _| {
            if (row == 0) == noise_first {
                let noise = noise.clone();
                div()
                    .h(px(400.))
                    .child(
                        canvas(
                            move |bounds, window, _| {
                                if noise.replace(false) {
                                    window.request_autoscroll(Bounds::new(
                                        bounds.origin + point(px(0.), px(300.)),
                                        size(px(8.), px(16.)),
                                    ));
                                }
                            },
                            |_, _, _, _| {},
                        )
                        .size_full(),
                    )
                    .into_any_element()
            } else {
                let target = target.clone();
                div()
                    .id("focused-row")
                    .track_focus(&focus)
                    .h(px(32.))
                    .child(
                        canvas(
                            move |bounds, _, _| target.set(Some(bounds)),
                            |_, _, _, _| {},
                        )
                        .size_full(),
                    )
                    .into_any_element()
            }
        })
        .h(px(120.))
        .w(px(200.))
    }
}
#[gpui::test]
fn focus_reveal_list_selects_focus_before_any_unrelated_row_retry(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        cx.update(crate::init);
        for noise_first in [false, true] {
            let mut view = None;
            let window = cx
                .open_window(size(px(240.), px(180.)), |window, cx| {
                    let content = cx.new(|cx| CompetingRows {
                        focus: cx.focus_handle(),
                        list: ListState::new(2, ListAlignment::Top, px(0.)).measure_all(),
                        target: Rc::default(),
                        noise: Rc::default(),
                        noise_first,
                    });
                    view = Some(content.clone());
                    Root::new(content, window, cx)
                })
                .into();
            let view = view.unwrap();
            settle(cx, window);
            let (focus, list, target, noise) = cx.read(|cx| {
                let v = view.read(cx);
                (
                    v.focus.clone(),
                    v.list.clone(),
                    v.target.clone(),
                    v.noise.clone(),
                )
            });
            if noise_first {
                list.scroll_to(gpui::ListOffset {
                    item_ix: 0,
                    offset_in_item: px(368.),
                });
                cx.update_window(window, |_, window, _| window.refresh())
                    .unwrap();
                settle(cx, window);
            }
            let before = target.get().unwrap();
            assert!(
                before.top() >= px(0.) && before.bottom() <= px(120.),
                "{before:?}"
            );
            let scroll_before = list.logical_scroll_top();
            noise.set(true);
            cx.update_window(window, |_, window, cx| window.focus(&focus, cx))
                .unwrap();
            assert_eq!(target.get().unwrap(), before);
            assert!(cx.simulate_scheduled_frame(window));
            settle(cx, window);
            assert!(!noise.get(), "competing row actually emitted");
            assert_eq!(target.get().unwrap(), before);
            let after = list.logical_scroll_top();
            assert_eq!(after.item_ix, scroll_before.item_ix);
            assert_eq!(after.offset_in_item, scroll_before.offset_in_item);
            cx.update_window(window, |_, window, _| window.remove_window())
                .unwrap();
            cx.run_until_parked();
        }
    });
}

struct FailedFocusAttempt {
    focus: FocusHandle,
    fail: Rc<Cell<bool>>,
    outer: ScrollHandle,
    target: Rc<Cell<Option<Bounds<Pixels>>>>,
}
impl Render for FailedFocusAttempt {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let focus = self.focus.clone();
        let fail = self.fail.clone();
        let target = self.target.clone();
        div()
            .id("attempt-scroll")
            .h(px(120.))
            .w(px(200.))
            .overflow_y_scroll()
            .track_scroll(&self.outer)
            .child(
                canvas(
                    move |_, window, cx| {
                        let target = target.clone();
                        let mut attempt = div()
                            .id("attempt-focus")
                            .track_focus(&focus)
                            .h(px(32.))
                            .w(px(200.))
                            .child(
                                canvas(
                                    move |bounds, _, _| target.set(Some(bounds)),
                                    |_, _, _, _| {},
                                )
                                .size_full(),
                            )
                            .into_any_element();
                        attempt.layout_as_root(size(px(200.).into(), px(32.).into()), window, cx);
                        if fail.get() {
                            let result: Result<(), ()> = window.transact(|window| {
                                attempt.prepaint_at(
                                    point(px(0.), px(250.)) + window.element_offset(),
                                    window,
                                    cx,
                                );
                                Err(())
                            });
                            assert!(result.is_err());
                        } else {
                            attempt.prepaint_at(
                                point(px(0.), px(250.)) + window.element_offset(),
                                window,
                                cx,
                            );
                        }
                    },
                    |_, _, _, _| {},
                )
                .h(px(700.))
                .w_full(),
            )
    }
}
#[gpui::test]
fn focus_reveal_failed_prepaint_cannot_ack_discarded_target(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        cx.update(crate::init);
        let mut view = None;
        let window = cx
            .open_window(size(px(240.), px(180.)), |window, cx| {
                let content = cx.new(|cx| FailedFocusAttempt {
                    focus: cx.focus_handle(),
                    fail: Rc::new(Cell::new(true)),
                    outer: ScrollHandle::new(),
                    target: Rc::default(),
                });
                view = Some(content.clone());
                Root::new(content, window, cx)
            })
            .into();
        let view = view.unwrap();
        settle(cx, window);
        let (focus, fail, outer, target) = cx.read(|cx| {
            let v = view.read(cx);
            (
                v.focus.clone(),
                v.fail.clone(),
                v.outer.clone(),
                v.target.clone(),
            )
        });
        cx.update_window(window, |_, window, cx| window.focus(&focus, cx))
            .unwrap();
        assert!(cx.simulate_scheduled_frame(window));
        settle(cx, window);
        assert_eq!(
            outer.offset().y,
            px(0.),
            "discarded request cannot scroll ancestor"
        );
        fail.set(false);
        cx.update_window(window, |_, window, _| window.refresh())
            .unwrap();
        settle(cx, window);
        assert_eq!(
            outer.offset().y,
            px(-162.),
            "same original focus still needs reveal after fallback"
        );
        assert_eq!(target.get().unwrap().bottom(), px(120.));
    });
}

#[gpui::test]
fn focus_reveal_includes_same_owner_outer_control_border(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        let (window, view) = open(cx, |v| v.wrapped = true);
        focus(cx, window, &view);
        assert_eq!(target(cx, &view).bottom(), px(119.));
        assert_eq!(cx.read(|cx| view.read(cx).outer.offset().y), px(-164.));
    });
}
