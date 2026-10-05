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

use gpui::{
    AnyElement, App, AppContext as _, AvailableSpace, Bounds, Context, Element, ElementId, Entity,
    FocusHandle, GlobalElementId, InspectorElementId, InteractiveElement as _, IntoElement,
    LayoutId, Modifiers, MouseMoveEvent, ParentElement as _, Pixels, Render, ScrollDelta,
    ScrollHandle, ScrollRetention, ScrollWheelEvent, SharedString, StatefulInteractiveElement as _,
    Styled as _, TestAppContext, TouchPhase, VisualTestContext, Window, div, point, px, size,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Copy)]
enum Conceal {
    Display,
    Visibility,
}

struct Candidate {
    rows: Vec<(u64, f32)>,
    policy: ScrollRetention,
    hidden: bool,
}
struct Reader {
    rows: Vec<(u64, f32)>,
    handle: ScrollHandle,
    height: f32,
    retry: bool,
    hover_hidden: Option<Conceal>,
    hidden: bool,
    deferred: bool,
    outer: Option<ScrollHandle>,
    abandoned: Option<Candidate>,
    explicit_after_discard: Option<Pixels>,
    invalidate_after_discard: bool,
    first_attempts: Rc<RefCell<Vec<Pixels>>>,
    focus: FocusHandle,
}
impl Render for Reader {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let render = |rows: &[(u64, f32)], policy, hidden| {
            let mut content = div()
                .id("reader")
                .w(px(200.))
                .h(px(self.height))
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .track_scroll(&self.handle)
                .retain_scroll(policy);
            if let Some(conceal) = self.hover_hidden.filter(|_| !self.deferred) {
                content = content.hover(move |mut style| match conceal {
                    Conceal::Display => style.hidden(),
                    Conceal::Visibility => {
                        style.visibility = Some(gpui::Visibility::Hidden);
                        style
                    }
                });
            }
            if hidden || (self.hidden && !self.deferred) {
                content = content.invisible();
            }
            for (id, height) in rows {
                let row = div()
                    .id(SharedString::from(format!("row-{id}")))
                    .debug_selector({
                        let id = *id;
                        move || format!("row-{id}")
                    })
                    .w_full()
                    .h(px(*height))
                    .flex_shrink_0();
                content = content.child(if *id == 0 {
                    row.track_focus(&self.focus)
                } else {
                    row
                });
            }
            content.into_any_element()
        };
        let child = match &self.abandoned {
            Some(candidate) => render(&candidate.rows, candidate.policy, candidate.hidden),
            None => render(&self.rows, ScrollRetention::FollowEnd, false),
        };
        let reader = Retry {
            child,
            retry_child: self
                .retry
                .then(|| render(&self.rows, ScrollRetention::FollowEnd, false)),
            explicit_after_discard: self.explicit_after_discard,
            invalidate_after_discard: self.invalidate_after_discard,
            handle: self.handle.clone(),
            attempts: self.first_attempts.clone(),
        };
        let reader = if self.deferred {
            let branch = div()
                .id("deferred-reader-branch")
                .w(px(200.))
                .h(px(self.height));
            let branch = if let Some(conceal) = self.hover_hidden {
                branch.hover(move |mut style| match conceal {
                    Conceal::Display => style.hidden(),
                    Conceal::Visibility => {
                        style.visibility = Some(gpui::Visibility::Hidden);
                        style
                    }
                })
            } else {
                branch
            };
            let branch = if self.hidden {
                branch.invisible()
            } else {
                branch
            };
            branch.child(gpui::deferred(reader)).into_any_element()
        } else {
            reader.into_any_element()
        };
        let reader = if let Some(outer) = &self.outer {
            div()
                .id("outer-reader")
                .w(px(200.))
                .h(px(100.))
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .track_scroll(outer)
                .child(div().h(px(200.)).flex_shrink_0())
                .child(div().h(px(self.height)).flex_shrink_0().child(reader))
                .child(div().h(px(200.)).flex_shrink_0())
                .into_any_element()
        } else {
            reader.into_any_element()
        };
        div().size_full().child(reader)
    }
}
struct Retry {
    child: AnyElement,
    retry_child: Option<AnyElement>,
    explicit_after_discard: Option<Pixels>,
    invalidate_after_discard: bool,
    handle: ScrollHandle,
    attempts: Rc<RefCell<Vec<Pixels>>>,
}
impl IntoElement for Retry {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Retry {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(child) = self.retry_child.take() {
            let result: Result<(), ()> = window.transact(|window| {
                self.child.prepaint(window, cx);
                self.attempts.borrow_mut().push(self.handle.offset().y);
                Err(())
            });
            assert!(result.is_err());
            if self.invalidate_after_discard {
                self.handle.invalidate();
            }
            if let Some(y) = self.explicit_after_discard {
                self.handle.set_offset(point(px(0.), y));
            }
            // Native retry creates fresh elements, as List::prepaint_items does.
            self.child = child;
            self.child.prepaint_as_root(
                bounds.origin,
                size(
                    AvailableSpace::Definite(bounds.size.width),
                    AvailableSpace::Definite(bounds.size.height),
                ),
                window,
                cx,
            );
        } else {
            self.child.prepaint(window, cx);
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}
fn reader(cx: &mut TestAppContext) -> (Entity<Reader>, &mut VisualTestContext, ScrollHandle) {
    let handle = ScrollHandle::new();
    let owner = handle.clone();
    let (view, cx) = cx.add_window_view(|_, cx| Reader {
        rows: (0..4).map(|id| (id, 100.)).collect(),
        handle: owner,
        height: 100.,
        retry: false,
        hover_hidden: None,
        hidden: false,
        deferred: false,
        outer: None,
        abandoned: None,
        explicit_after_discard: None,
        invalidate_after_discard: false,
        first_attempts: Rc::default(),
        focus: cx.focus_handle(),
    });
    draw(cx);
    (view, cx, handle)
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}
fn top(cx: &mut VisualTestContext, id: u64) -> Pixels {
    cx.debug_bounds(match id {
        0 => "row-0",
        1 => "row-1",
        2 => "row-2",
        _ => panic!("unknown test row"),
    })
    .unwrap()
    .top()
}

#[gpui::test]
fn native_wheel_detaches_growth_prepend_and_resize_preserve_reading(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        let (view, cx, handle) = reader(cx);
        assert_eq!(handle.offset().y, px(-300.));
        view.update(cx, |view, cx| {
            view.rows.push((4, 100.));
            cx.notify();
        });
        draw(cx);
        assert_eq!(handle.offset().y, px(-400.));
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(50.), px(50.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(250.))),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        draw(cx);
        assert_eq!(handle.offset().y, px(-150.));
        let before = top(cx, 1);
        view.update(cx, |view, cx| {
            view.rows.insert(0, (9, 70.));
            view.rows[1].1 += 30.;
            view.rows.push((5, 180.));
            view.height = 120.;
            cx.notify();
        });
        draw(cx);
        assert_eq!(top(cx, 1), before);
        assert_eq!(handle.offset().y, px(-250.));
        handle.scroll_to_bottom();
        cx.update(|window, _| window.refresh());
        draw(cx);
        assert_eq!(handle.offset().y, -handle.max_offset().y);
        view.update(cx, |view, cx| {
            view.rows.push((6, 60.));
            cx.notify();
        });
        draw(cx);
        assert_eq!(handle.offset().y, -handle.max_offset().y);
    });
}

#[gpui::test]
fn discarded_prepaint_retries_absolute_anchor_and_explicit_focus_wins(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        let (view, cx, handle) = reader(cx);
        handle.set_offset(point(px(0.), px(-150.)));
        cx.update(|window, _| window.refresh());
        draw(cx);
        let before = top(cx, 1);
        view.update(cx, |view, cx| {
            view.rows.insert(0, (9, 100.));
            view.retry = true;
            cx.notify();
        });
        draw(cx);
        assert_eq!(top(cx, 1), before);
        assert_eq!(handle.offset().y, px(-250.));
        let attempts = view.read_with(cx, |view, _| view.first_attempts.borrow().clone());
        assert_eq!(attempts, vec![px(-250.)]);
        // Begin at the end; growth proposes a new end, then real focus reveals
        // row 0 above it. Retrying must not follow or choose another old anchor.
        handle.scroll_to_bottom();
        cx.update(|window, _| window.refresh());
        draw(cx);
        view.update(cx, |view, cx| {
            view.rows.push((5, 100.));
            cx.notify();
        });
        let focus = view.read_with(cx, |view, _| view.focus.clone());
        cx.update(|window, cx| window.focus(&focus, cx));
        draw(cx);
        assert_eq!(top(cx, 0), px(0.));
        assert_eq!(handle.offset().y, px(-100.));
    });
}

#[gpui::test]
fn discarded_hidden_manual_and_removed_anchor_use_painted_reading_position(
    cx: &mut TestAppContext,
) {
    cx.with_paused_automatic_drawing(|cx| {
        let (view, cx, handle) = reader(cx);
        handle.set_offset(point(px(0.), px(-150.)));
        cx.update(|window, _| window.refresh());
        draw(cx);
        for (policy, hidden, rows) in [
            (ScrollRetention::None, false, vec![(0, 100.), (1, 100.)]),
            (
                ScrollRetention::None,
                false,
                (0..4).map(|id| (id, 100.)).collect(),
            ),
            (ScrollRetention::FollowEnd, true, vec![]),
            (
                ScrollRetention::Anchor,
                false,
                vec![(9, 100.), (0, 100.), (1, 100.), (2, 100.), (3, 100.)],
            ),
        ] {
            let expected_first = if hidden {
                px(-150.)
            } else if policy == ScrollRetention::Anchor {
                px(-250.)
            } else if rows.len() == 2 {
                px(-100.)
            } else {
                px(-150.)
            };
            view.update(cx, |view, cx| {
                view.retry = true;
                view.abandoned = Some(Candidate {
                    rows,
                    policy,
                    hidden,
                });
                cx.notify();
            });
            draw(cx);
            assert_eq!(
                view.read_with(cx, |view, _| *view.first_attempts.borrow().last().unwrap()),
                expected_first
            );
            assert_eq!(handle.offset().y, px(-150.));
            assert_eq!(top(cx, 1), px(-50.));
        }
        view.update(cx, |view, cx| {
            view.abandoned = Some(Candidate {
                rows: vec![(99, 40.)],
                policy: ScrollRetention::FollowEnd,
                hidden: false,
            });
            view.rows.insert(0, (9, 100.));
            cx.notify();
        });
        draw(cx);
        assert_eq!(top(cx, 1), px(-50.));
        assert_eq!(handle.offset().y, px(-250.));
        // Restore the original materialized window and its painted origin.
        view.update(cx, |view, cx| {
            view.retry = false;
            view.abandoned = None;
            view.rows.remove(0);
            cx.notify();
        });
        draw(cx);
        assert_eq!(handle.offset().y, px(-150.));
        view.update(cx, |view, cx| {
            view.retry = true;
            cx.notify();
        });
        // First candidate anchored row 1 after a prepend; the real retry
        // removed row 1. Row 2 was also visible in the original painted frame.
        view.update(cx, |view, cx| {
            view.abandoned = Some(Candidate {
                rows: vec![(9, 100.), (0, 100.), (1, 100.), (2, 100.), (3, 100.)],
                policy: ScrollRetention::FollowEnd,
                hidden: false,
            });
            view.rows.retain(|(id, _)| *id != 1);
            cx.notify();
        });
        draw(cx);
        assert_eq!(top(cx, 2), px(50.));
        assert_eq!(handle.offset().y, px(-50.));
    });
}

#[gpui::test]
fn end_request_survives_discarded_prepaint_until_real_paint(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        let (view, cx, handle) = reader(cx);
        handle.set_offset(point(px(0.), px(-150.)));
        cx.update(|window, _| window.refresh());
        draw(cx);
        handle.scroll_to_bottom();
        view.update(cx, |view, cx| {
            view.retry = true;
            view.abandoned = Some(Candidate {
                rows: vec![(0, 100.), (1, 100.), (2, 100.), (3, 100.)],
                policy: ScrollRetention::FollowEnd,
                hidden: false,
            });
            view.rows.push((4, 100.));
            cx.notify();
        });
        draw(cx);
        assert_eq!(handle.offset().y, px(-400.));
        handle.set_offset(point(px(0.), px(-150.)));
        handle.scroll_to_bottom();
        view.update(cx, |view, cx| {
            view.abandoned = Some(Candidate {
                rows: vec![],
                policy: ScrollRetention::FollowEnd,
                hidden: true,
            });
            cx.notify();
        });
        draw(cx);
        assert_eq!(
            view.read_with(cx, |view, _| *view.first_attempts.borrow().last().unwrap()),
            px(-150.)
        );
        assert_eq!(handle.offset().y, px(-400.));
        handle.set_offset(point(px(0.), px(-150.)));
        view.update(cx, |view, cx| {
            view.retry = false;
            view.abandoned = None;
            cx.notify();
        });
        draw(cx);
        assert_eq!(handle.offset().y, px(-150.));
        view.update(cx, |view, cx| {
            view.retry = true;
            view.explicit_after_discard = Some(px(-75.));
            view.abandoned = Some(Candidate {
                rows: vec![(0, 100.), (1, 100.), (2, 100.), (3, 100.)],
                policy: ScrollRetention::FollowEnd,
                hidden: false,
            });
            view.rows.push((5, 60.));
            cx.notify();
        });
        draw(cx);
        assert_eq!(handle.offset().y, px(-75.));
    });
}

#[gpui::test]
fn layout_invalidation_is_not_input_and_item_request_resolves_final_geometry(
    cx: &mut TestAppContext,
) {
    cx.with_paused_automatic_drawing(|cx| {
        let (view, cx, handle) = reader(cx);
        handle.set_offset(point(px(0.), px(-150.)));
        cx.update(|window, _| window.refresh());
        draw(cx);
        view.update(cx, |view, cx| {
            view.retry = true;
            view.invalidate_after_discard = true;
            view.abandoned = Some(Candidate {
                rows: vec![(9, 100.), (0, 100.), (1, 100.), (2, 100.), (3, 100.)],
                policy: ScrollRetention::FollowEnd,
                hidden: false,
            });
            view.rows.insert(0, (9, 200.));
            cx.notify();
        });
        draw(cx);
        assert_eq!(top(cx, 1), px(-50.));
        assert_eq!(handle.offset().y, px(-350.));
        handle.scroll_to_bottom();
        // A later item request supersedes the pending end request.
        handle.scroll_to_top_of_item(2);
        cx.update(|window, _| window.refresh());
        draw(cx);
        assert_eq!(handle.offset().y, px(-300.));
        assert_eq!(top(cx, 1), px(0.));
        assert_eq!(
            view.read_with(cx, |view, _| *view.first_attempts.borrow().last().unwrap()),
            px(-200.)
        );
        handle.set_offset(point(px(0.), px(-75.)));
        view.update(cx, |view, cx| {
            view.retry = false;
            view.abandoned = None;
            cx.notify();
        });
        draw(cx);
        assert_eq!(handle.offset().y, px(-75.));
    });
}

#[gpui::test]
fn newly_hover_hidden_paint_cannot_acknowledge_prepaint_request(cx: &mut TestAppContext) {
    for (deferred, conceal) in [false, true].into_iter().flat_map(|deferred| {
        [Conceal::Display, Conceal::Visibility].map(|conceal| (deferred, conceal))
    }) {
        cx.with_paused_automatic_drawing(|cx| {
            let (view, cx, handle) = reader(cx);
            let move_to = |cx: &mut VisualTestContext, x, y| {
                cx.simulate_event(MouseMoveEvent {
                    position: point(px(x), px(y)),
                    pressed_button: None,
                    modifiers: Modifiers::default(),
                });
            };
            move_to(cx, 400., 400.);
            draw(cx);
            view.update(cx, |view, cx| {
                view.hover_hidden = Some(conceal);
                view.deferred = deferred;
                view.rows.push((4, 100.));
                cx.notify();
            });
            handle.scroll_to_top_of_item(2);
            move_to(cx, 50., 50.);
            cx.update(|window, _| window.refresh());
            draw(cx);
            // Prepaint resolved row 2 at -200, but hover suppressed actual paint.
            assert_eq!(handle.offset().y, px(-200.));
            move_to(cx, 400., 400.);
            view.update(cx, |view, cx| {
                view.rows.insert(0, (9, 200.));
                cx.notify();
            });
            draw(cx);
            // The original pending item still resolves against the first visible
            // paint's final children; it was not acknowledged by the hidden frame.
            assert_eq!(handle.offset().y, px(-300.));
            assert_eq!(top(cx, 1), px(0.));
        });
    }
}

#[gpui::test]
fn hidden_focus_cannot_move_owners_cancel_request_or_ack_reveal(cx: &mut TestAppContext) {
    for deferred in [false, true] {
        for keep_focus in [false, true] {
            cx.with_paused_automatic_drawing(|cx| {
                let (view, cx, handle) = reader(cx);
                let outer = ScrollHandle::new();
                view.update(cx, |view, cx| {
                    view.outer = Some(outer.clone());
                    view.deferred = deferred;
                    cx.notify();
                });
                draw(cx);
                handle.set_offset(point(px(0.), px(-150.)));
                cx.update(|window, _| window.refresh());
                draw(cx);
                handle.scroll_to_top_of_item(2);
                let focus = view.read_with(cx, |view, _| view.focus.clone());
                view.update(cx, |view, cx| {
                    view.hidden = true;
                    cx.notify();
                });
                cx.update(|window, cx| window.focus(&focus, cx));
                draw(cx);
                assert!(cx.update(|window, _| focus.is_focused(window)));
                assert_eq!(handle.offset().y, px(-150.));
                assert_eq!(outer.offset().y, px(0.));
                if !keep_focus {
                    cx.update(|window, cx| window.blur(cx));
                }
                view.update(cx, |view, cx| {
                    view.hidden = false;
                    if !keep_focus {
                        view.rows.insert(0, (9, 200.));
                    }
                    cx.notify();
                });
                draw(cx);
                if keep_focus {
                    // Hidden geometry did not ACK this focus generation. Its first
                    // visible layout reveals row 0 through both original owners.
                    draw(cx);
                    assert_eq!(handle.offset().y, px(0.));
                    assert_eq!(outer.offset().y, px(if deferred { 0. } else { -200. }));
                } else {
                    // Hidden focus did not cancel the original logical item. It
                    // resolves against the newly presented children after blur.
                    assert_eq!(handle.offset().y, px(-300.));
                    assert_eq!(outer.offset().y, px(0.));
                }
            });
        }
    }
}

#[gpui::test]
fn changed_child_box_rebuilds_scroll_parent_but_internal_change_can_splice(
    cx: &mut TestAppContext,
) {
    struct Row {
        height: f32,
        color: u32,
    }
    impl Render for Row {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("short-child")
                .w(px(80.))
                .h(px(self.height))
                .flex_shrink_0()
                .bg(gpui::rgb(self.color))
        }
    }
    struct Parent {
        child: Entity<Row>,
        handle: ScrollHandle,
        renders: Rc<Cell<usize>>,
    }
    impl Render for Parent {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.renders.set(self.renders.get() + 1);
            div().size_full().child(
                div()
                    .id("managed")
                    .w(px(200.))
                    .h(px(300.))
                    .flex()
                    .flex_row()
                    .items_center()
                    .overflow_y_scroll()
                    .track_scroll(&self.handle)
                    .retain_scroll(ScrollRetention::Anchor)
                    .child(self.child.clone())
                    .child(div().w(px(80.)).h(px(300.)).flex_shrink_0()),
            )
        }
    }
    cx.with_paused_automatic_drawing(|cx| {
        let child = cx.new(|_| Row {
            height: 80.,
            color: 0,
        });
        let handle = ScrollHandle::new();
        let renders = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view(|_, _| Parent {
            child: child.clone(),
            handle: handle.clone(),
            renders: renders.clone(),
        });
        cx.update(|window, _| window.set_view_retention(true));
        draw(cx);
        let before = renders.get();
        assert_eq!(handle.bounds_for_item(0).unwrap().size.height, px(80.));
        child.update(cx, |child, cx| {
            child.height = 120.;
            cx.notify();
        });
        draw(cx);
        assert_eq!(renders.get(), before + 1);
        assert_eq!(handle.bounds_for_item(0).unwrap().size.height, px(120.));
        assert_eq!(handle.bounds_for_item(0).unwrap().top(), px(90.));
        let before = renders.get();
        child.update(cx, |child, cx| {
            child.color = 0xffffff;
            cx.notify();
        });
        draw(cx);
        assert_eq!(renders.get(), before);
        assert_eq!(handle.bounds_for_item(0).unwrap().size.height, px(120.));
    });
}
