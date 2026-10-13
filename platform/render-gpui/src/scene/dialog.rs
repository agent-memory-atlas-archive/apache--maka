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

use super::{Dimension, Style, StylePresentation};
use crate::{Input, LiveInput};
use gpui_kit::{
    base::{Dialog, DialogChangeReason, DialogPopup, Popup, TextSelectionLayer},
    *,
};
use maka_ui_runtime::dismiss::{Input as Dismiss, Reason};

// Only original native custody. Open belongs to the committed scene recipe.
pub(super) struct State {
    focus: FocusHandle,
    scope: Option<InputScope>,
    previous: Option<WeakFocusHandle>,
    restore: Option<Subscription>,
    scroll: ScrollHandle,
}
impl State {
    pub fn new(cx: &mut App) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(false),
            scope: None,
            previous: None,
            restore: None,
            scroll: ScrollHandle::default(),
        }
    }
    pub fn admitted(&self) -> bool {
        self.scope.is_some()
    }
    pub fn presentation(&mut self, open: bool, window: &mut Window, cx: &mut App) {
        if open == self.admitted() {
            return;
        }
        if !open {
            self.deactivate(window, cx);
            return;
        }
        self.previous = window.focused(cx).map(|focus| focus.downgrade());
        let scope = window.input_scope(&self.focus, cx);
        let opening = scope.downgrade();
        let group = self.focus.downgrade();
        self.scope = Some(scope);
        self.focus.focus(window, cx);
        // A retained inner opening may remain the published foreground. Offer
        // its original focus after paint; a background target is never admitted.
        if let Some(previous) = &self.previous {
            window.restore_focus_after_paint(previous.clone(), window.focus_generation());
        }
        self.restore = Some(window.on_focus_lost(cx, move |window, cx| {
            if !opening.is_active(window) || window.focused(cx).is_some() {
                return;
            }
            let Some(group) = group.upgrade() else {
                return;
            };
            let target = window
                .focus_lost_restore_target(cx)
                .filter(|target| group.contains(target, window))
                .unwrap_or(group);
            // Original close restoration has priority over this group fallback.
            window.restore_focus_after_paint(target.downgrade(), window.focus_generation());
        }));
    }
    pub fn deactivate(&mut self, window: &mut Window, cx: &mut App) {
        let active = self
            .scope
            .as_ref()
            .is_some_and(|scope| scope.downgrade().is_active(window));
        let owned = self.focus.is_focused(window) || self.focus.contains_focused(window, cx);
        self.scope.take();
        self.restore = None;
        let previous = self.previous.take();
        if !owned && !(active && window.focused(cx).is_none()) {
            return;
        }
        window.blur(cx);
        let receipt = window.focus_generation();
        // Native paint validates the original weak target against the new tree.
        // No owner lookup or key replacement can revive a retired presentation.
        if let Some(previous) = previous {
            window.restore_focus_after_paint(previous, receipt);
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        key: &str,
        label: &str,
        background: AnyElement,
        content: Option<AnyElement>,
        style: &Style,
        backdrop: maka_ui_runtime::appearance::Color,
        live: &LiveInput,
        input: Input,
        window: &Window,
        cx: &mut App,
    ) -> AnyElement {
        let foreground = self.scope.as_ref().zip(content).map(|(scope, content)| {
            let opening = scope.downgrade();
            let live = live.clone();
            let dismiss = input.bind(key.into(), |reason: &Reason| {
                serde_json::to_value(Dismiss::Dismiss { reason: *reason })
                    .expect("dialog dismissal")
            });
            let mut style = style.clone();
            style.width.get_or_insert(Dimension::Pixels(560.));
            let style = style.bounded(Popup::available_size(window));
            let body = style
                .apply(
                    div()
                        .id(SharedString::from(format!("dialog-body:{key}")))
                        .flex()
                        .flex_col(),
                )
                .overflow_x_hidden()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .child(content);
            let dialog = Dialog::new(cx)
                .open(true)
                .focus_handle(self.focus.clone())
                .accessibility_label(SharedString::from(label.to_owned()))
                .confirm_on_enter(false)
                .on_ok(|_, _, _| false)
                .on_open_change(move |open, reason, window, cx| {
                    if open || !opening.is_active(window) || live.current.borrow().is_none() {
                        return;
                    }
                    let reason = match reason {
                        DialogChangeReason::Cancel => Reason::Escape,
                        DialogChangeReason::BackdropPress => Reason::Outside,
                        _ => return,
                    };
                    dismiss(&reason, window, cx);
                })
                .backdrop(div().size_full().bg(crate::appearance::color(backdrop)))
                .popup(DialogPopup::new().child(TextSelectionLayer).child(body));
            scope
                .element(
                    div()
                        .id(SharedString::from(format!("dialog-foreground:{key}")))
                        .child(dialog),
                )
                .into_any_element()
        });
        Shell {
            id: SharedString::from(format!("dialog:{key}")).into(),
            background,
            foreground,
        }
        .into_any_element()
    }
}

// Transparent background layout. The controlled native host uses its own
// viewport root; adding an overlay cannot resize the original background.
struct Shell {
    id: ElementId,
    background: AnyElement,
    foreground: Option<AnyElement>,
}
impl IntoElement for Shell {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Shell {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.background.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.background.prepaint(window, cx);
        if let Some(foreground) = &mut self.foreground {
            let viewport = window.viewport_size();
            foreground.prepaint_as_root(
                point(px(0.), px(0.)),
                size(
                    AvailableSpace::Definite(viewport.width),
                    AvailableSpace::Definite(viewport.height),
                ),
                window,
                cx,
            );
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
        self.background.paint(window, cx);
        if let Some(foreground) = &mut self.foreground {
            foreground.paint(window, cx);
        }
    }
}
