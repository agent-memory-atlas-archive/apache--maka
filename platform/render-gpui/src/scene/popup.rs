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

use super::{Style, StylePresentation};
use crate::LiveInput;
use gpui_kit::{base::Popup, *};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

use maka_ui_runtime::scene::desktop::Anchor;
fn native(anchor: Anchor) -> gpui_kit::Anchor {
    match anchor {
        Anchor::TopLeft => gpui_kit::Anchor::TopLeft,
        Anchor::TopCenter => gpui_kit::Anchor::TopCenter,
        Anchor::TopRight => gpui_kit::Anchor::TopRight,
        Anchor::BottomLeft => gpui_kit::Anchor::BottomLeft,
        Anchor::BottomCenter => gpui_kit::Anchor::BottomCenter,
        Anchor::BottomRight => gpui_kit::Anchor::BottomRight,
        Anchor::LeftCenter => gpui_kit::Anchor::LeftCenter,
        Anchor::RightCenter => gpui_kit::Anchor::RightCenter,
    }
}
use maka_ui_runtime::dismiss::{Input, Reason};

type Gate = Rc<Cell<bool>>;
// This is mechanical focus/scroll and listener admission derived from the
// committed scene. It neither decides nor publishes product open state.
pub(super) struct State {
    gate: Gate,
    focus: FocusHandle,
    previous: Option<WeakFocusHandle>,
    restore: Option<Subscription>,
    scroll: ScrollHandle,
    trigger: Rc<Cell<Bounds<Pixels>>>,
    window: WindowId,
}
struct Admission {
    gate: Weak<Cell<bool>>,
    window: WindowId,
}
impl Admission {
    fn allows(&self, window: &Window) -> bool {
        self.window == window.window_handle().window_id()
            && self.gate.upgrade().is_some_and(|gate| gate.get())
    }
}
impl State {
    pub fn new(window: &Window, cx: &mut App) -> Self {
        Self {
            gate: Rc::new(Cell::new(false)),
            focus: cx.focus_handle(),
            previous: None,
            restore: None,
            scroll: ScrollHandle::default(),
            trigger: Rc::new(Cell::new(Bounds::default())),
            window: window.window_handle().window_id(),
        }
    }
    pub fn admitted(&self) -> bool {
        self.gate.get()
    }
    pub fn presentation(&mut self, open: bool, window: &mut Window, cx: &mut App) {
        if self.admitted() == open {
            return;
        }
        // This allocation is the exact presentation lease. Revoke the old
        // opening before replacing it; weak callbacks cannot follow a reopen.
        self.gate.set(false);
        self.gate = Rc::new(Cell::new(open));
        self.focus = self
            .focus
            .clone()
            .tab_stop(open)
            .tab_index(if open { 0 } else { -1 });
        if open {
            self.previous = window.focused(cx).map(|focus| focus.downgrade());
            self.focus.focus(window, cx);
            let admission = self.admission();
            let scope = self.focus.downgrade();
            self.restore = Some(window.on_focus_lost(cx, move |window, cx| {
                if !admission.allows(window) || window.focused(cx).is_some() {
                    return;
                }
                let Some(scope) = scope.upgrade().filter(|focus| focus.tab_stop) else {
                    return;
                };
                let Some(target) = window
                    .focus_lost_restore_target(cx)
                    .filter(|target| scope.contains(target, window))
                else {
                    return;
                };
                // A page can hide the focused control without closing its
                // popup. The newly rendered original ancestry chooses the
                // nearest surviving scope, including a nested child popup.
                target.focus(window, cx);
            }));
        } else {
            self.restore = None;
            let owned_focus = self.focus.contains_focused(window, cx);
            let previous = self.previous.take();
            if owned_focus {
                window.blur(cx);
                let gate = Rc::downgrade(&self.gate);
                let original_window = self.window;
                // Parent synchronization precedes child presentation. Restore
                // only after the whole pass has disabled/blurred hidden inputs.
                window.defer(cx, move |window, cx| {
                    if window.window_handle().window_id() != original_window
                        || window.focused(cx).is_some()
                    {
                        return;
                    }
                    let Some(gate) = gate.upgrade() else {
                        return;
                    };
                    if gate.get() {
                        return;
                    }
                    if let Some(previous) = previous
                        .and_then(|previous| previous.upgrade())
                        .filter(|focus| focus.tab_stop)
                    {
                        previous.focus(window, cx);
                    }
                });
            }
        }
    }
    pub fn deactivate(&mut self, window: &mut Window, cx: &mut App) {
        self.gate.set(false);
        self.gate = Rc::new(Cell::new(false));
        self.previous = None;
        self.restore = None;
        self.focus = self.focus.clone().tab_stop(false).tab_index(-1);
        if self.focus.contains_focused(window, cx) {
            window.blur(cx);
        }
    }
    fn admission(&self) -> Admission {
        Admission {
            gate: Rc::downgrade(&self.gate),
            window: self.window,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        key: &str,
        label: &str,
        trigger: AnyElement,
        content: Option<AnyElement>,
        anchor: Anchor,
        offset: u16,
        style: &Style,
        live: &LiveInput,
        input: crate::Input,
        window: &Window,
    ) -> AnyElement {
        let measured_trigger = self.trigger.clone();
        let popup = Popup::new(SharedString::from(format!("popup:{key}")), trigger)
            .anchor(native(anchor))
            .offset(px(offset.into()))
            .on_position(move |_, bounds| measured_trigger.set(bounds));
        let Some(content) = content else {
            return popup.into_any_element();
        };
        let available = Popup::available_size(window);
        let style = style.bounded(available);
        let escape_admission = self.admission();
        let outside_admission = self.admission();
        let trigger_bounds = self.trigger.clone();
        let pointer_scope = self.focus.clone();
        let escape_live = live.clone();
        let outside_live = live.clone();
        let escape = input.bind(key.into(), |_: &KeyDownEvent| {
            serde_json::to_value(Input::Dismiss {
                reason: Reason::Escape,
            })
            .expect("popup input")
        });
        let outside = input.bind(key.into(), |_: &MouseDownEvent| {
            serde_json::to_value(Input::Dismiss {
                reason: Reason::Outside,
            })
            .expect("popup input")
        });
        let body = style
            .apply(
                div()
                    .id(SharedString::from(format!("popup-body:{key}")))
                    .flex()
                    .flex_col(),
            )
            .role(Role::Dialog)
            .aria_label(SharedString::from(label.to_owned()))
            .track_focus(&self.focus)
            .tab_stop(self.focus.tab_stop)
            .tab_index(self.focus.tab_index)
            .tab_group()
            .overflow_x_hidden()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .on_key_down(move |event, window: &mut Window, cx| {
                if event.keystroke.key == "escape"
                    && escape_admission.allows(window)
                    && escape_live.current.borrow().is_some()
                {
                    window.prevent_default();
                    cx.stop_propagation();
                    escape(event, window, cx);
                }
            })
            .on_mouse_down_out(move |event, window: &mut Window, cx| {
                if outside_admission.allows(window)
                    && outside_live.current.borrow().is_some()
                    && !trigger_bounds.get().contains(&event.position)
                    && !pointer_scope.contains_hovered(window)
                {
                    outside(event, window, cx);
                }
            })
            .child(content);
        popup.content(body).into_any_element()
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.focus = self.focus.clone().tab_stop(false).tab_index(-1);
    }
}
