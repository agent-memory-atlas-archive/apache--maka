use std::time::Duration;
use std::{cell::Cell, rc::Rc};

use gpui::{AsyncApp, Bounds, Context, Pixels, Point, Task, WeakEntity, WeakInputGesture, px};

/// Manages timer-based auto-scrolling during drag interactions.
///
/// Delta convention: positive moves toward the bottom and negative moves
/// toward the top.
pub struct AutoScroll {
    /// Shared by the original owner and its local UI task.
    /// Writing `None` is the stop signal; the task exits on its next tick.
    shared: Rc<Cell<Option<Pixels>>>,
    gesture: Option<WeakInputGesture>,
    task: Option<Task<()>>,
    /// Last drag position, available to the interaction that owns selection.
    pub last_drag_position: Option<Point<Pixels>>,
}

impl Default for AutoScroll {
    fn default() -> Self {
        Self {
            shared: Rc::new(Cell::new(None)),
            gesture: None,
            task: None,
            last_drag_position: None,
        }
    }
}

impl AutoScroll {
    /// Returns the current scroll delta.
    pub fn delta(&self) -> Option<Pixels> {
        self.gesture
            .as_ref()
            .filter(|gesture| gesture.is_current())
            .and_then(|_| self.shared.get())
    }

    /// Computes the scroll delta for a pointer Y position within the viewport.
    pub fn compute_delta(y: Pixels, bounds: Bounds<Pixels>) -> Option<Pixels> {
        const MIN_SPEED: f32 = 12.0;
        const MAX_SPEED: f32 = 64.0;
        // Trigger starts this far inside the bounds so scrolling works even in
        // full-screen where the mouse can't travel far outside the element.
        const INNER_ZONE: f32 = 16.0;
        // Distance from the bounds edge to reach MAX_SPEED.
        // Total ramp = INNER_ZONE + OUTER_RAMP, giving a single smooth curve
        // with no flat sections or discontinuities.
        const OUTER_RAMP: f32 = 80.0;

        let bottom_trigger = bounds.bottom() - px(INNER_ZONE);
        let top_trigger = bounds.top() + px(INNER_ZONE);

        if y > bottom_trigger {
            let t = ((y - bottom_trigger) / px(INNER_ZONE + OUTER_RAMP)).min(1.0);
            Some(px(MIN_SPEED + t * (MAX_SPEED - MIN_SPEED)))
        } else if y < top_trigger {
            let t = ((top_trigger - y) / px(INNER_ZONE + OUTER_RAMP)).min(1.0);
            Some(px(-(MIN_SPEED + t * (MAX_SPEED - MIN_SPEED))))
        } else {
            None
        }
    }

    /// Updates the scroll delta and starts the background task when needed.
    ///
    /// `tick` is called each frame (~60 fps) with the current delta.
    /// It should perform the actual scroll action for this entity.
    pub fn set<T, F>(
        &mut self,
        delta: Option<Pixels>,
        gesture: WeakInputGesture,
        cx: &mut Context<T>,
        tick: F,
    ) where
        T: 'static,
        F: Fn(Pixels, &mut T, &mut Context<T>) + 'static,
    {
        let same = self
            .gesture
            .as_ref()
            .is_some_and(|current| current.same_gesture(&gesture));
        if delta.is_none() {
            if same {
                self.stop();
            }
            return;
        }
        if !gesture.is_current() {
            return;
        }
        if same && self.task.is_some() && self.shared.get().is_some() {
            self.shared.set(delta);
            return;
        }
        self.task = None;
        self.shared = Rc::new(Cell::new(delta));
        self.gesture = Some(gesture.clone());
        self.task = Some(cx.spawn(Self::task_loop(self.shared.clone(), gesture, tick)));
    }

    fn task_loop<T, F>(
        shared: Rc<Cell<Option<Pixels>>>,
        gesture: WeakInputGesture,
        tick: F,
    ) -> impl AsyncFnOnce(WeakEntity<T>, &mut AsyncApp) + 'static
    where
        T: 'static,
        F: Fn(Pixels, &mut T, &mut Context<T>) + 'static,
    {
        async move |this: WeakEntity<T>, cx: &mut AsyncApp| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                if !gesture.is_current() {
                    shared.set(None);
                    break;
                }
                let Some(delta) = shared.get() else {
                    break;
                };
                let alive = this
                    .update(cx, |state, cx| {
                        if !gesture.is_current() {
                            return false;
                        }
                        tick(delta, state, cx);
                        gesture.is_current()
                    })
                    .unwrap_or(false);
                if !alive {
                    shared.set(None);
                    break;
                }
            }
        }
    }

    /// Returns whether automatic scrolling is active.
    pub fn is_active(&self) -> bool {
        self.delta().is_some()
    }

    /// Stops automatic scrolling and clears the last drag position.
    pub fn stop(&mut self) {
        self.shared.set(None);
        self.gesture = None;
        self.task = None;
        self.last_drag_position = None;
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, size};

    use super::*;

    #[test]
    fn delta_uses_dead_zone_and_symmetric_edge_ramps() {
        let bounds = Bounds::new(point(px(0.), px(100.)), size(px(200.), px(100.)));

        assert_eq!(AutoScroll::compute_delta(px(150.), bounds), None);

        let top = AutoScroll::compute_delta(px(80.), bounds).unwrap();
        let bottom = AutoScroll::compute_delta(px(220.), bounds).unwrap();
        assert_eq!(top, -bottom);
        assert!(top < px(-12.));
    }

    #[test]
    fn stop_clears_delta_and_drag_position() {
        let mut scroll = AutoScroll::default();
        scroll.shared.set(Some(px(20.)));
        scroll.last_drag_position = Some(point(px(1.), px(2.)));

        scroll.stop();

        assert!(!scroll.is_active());
        assert_eq!(scroll.last_drag_position, None);
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;
    use gpui::{IntoElement, Render, TestAppContext, Window};

    struct ScrollProbe {
        scroll: AutoScroll,
        offset: Pixels,
    }
    impl Render for ScrollProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            gpui::Empty
        }
    }

    #[gpui::test]
    fn original_gesture_ends_interrupts_and_restarts_auto_scroll_without_old_event_aba(
        cx: &mut TestAppContext,
    ) {
        let (probe, cx) = cx.add_window_view(|_, _| ScrollProbe {
            scroll: AutoScroll::default(),
            offset: px(0.),
        });
        let first = cx.update(|window, _| window.input_gesture());
        let old = first.downgrade();
        cx.update(|_, cx| {
            probe.update(cx, |state, cx| {
                state
                    .scroll
                    .set(Some(px(3.)), old.clone(), cx, |delta, state, _| {
                        state.offset += delta
                    })
            })
        });
        cx.executor().advance_clock(Duration::from_millis(16));
        cx.run_until_parked();
        assert_eq!(probe.read_with(cx, |state, _| state.offset), px(3.));
        drop(first);
        cx.executor().advance_clock(Duration::from_millis(16));
        cx.run_until_parked();
        assert_eq!(probe.read_with(cx, |state, _| state.offset), px(3.));
        let second = cx.update(|window, _| window.input_gesture());
        cx.update(|_, cx| {
            probe.update(cx, |state, cx| {
                state
                    .scroll
                    .set(Some(px(5.)), second.downgrade(), cx, |delta, state, _| {
                        state.offset += delta
                    });
                state
                    .scroll
                    .set(Some(px(100.)), old.clone(), cx, |delta, state, _| {
                        state.offset += delta
                    });
                state.scroll.set(None, old.clone(), cx, |delta, state, _| {
                    state.offset += delta
                });
                assert_eq!(state.scroll.delta(), Some(px(5.)));
            })
        });
        cx.executor().advance_clock(Duration::from_millis(16));
        cx.run_until_parked();
        assert_eq!(probe.read_with(cx, |state, _| state.offset), px(8.));
        cx.update(|window, cx| {
            let focus = cx.focus_handle();
            drop(window.input_scope(&focus, cx));
        });
        cx.executor().advance_clock(Duration::from_millis(16));
        cx.run_until_parked();
        assert_eq!(probe.read_with(cx, |state, _| state.offset), px(8.));
        assert!(!probe.read_with(cx, |state, _| state.scroll.is_active()));
        let third = cx.update(|window, _| window.input_gesture());
        cx.update(|_, cx| {
            probe.update(cx, |state, cx| {
                state
                    .scroll
                    .set(Some(px(7.)), third.downgrade(), cx, |delta, state, _| {
                        state.offset += delta
                    })
            })
        });
        cx.executor().advance_clock(Duration::from_millis(16));
        cx.run_until_parked();
        assert_eq!(probe.read_with(cx, |state, _| state.offset), px(15.));
    }
}
