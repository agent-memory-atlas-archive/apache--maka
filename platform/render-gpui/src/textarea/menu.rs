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
use crate::appearance::{color, font};
use gpui_kit::base::{Positioner, input};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Edit {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}
impl Edit {
    const ALL: [Self; 6] = [
        Self::Undo,
        Self::Redo,
        Self::Cut,
        Self::Copy,
        Self::Paste,
        Self::SelectAll,
    ];
    fn label(self, labels: &TextareaMenuLabels) -> &str {
        match self {
            Self::Undo => &labels.undo,
            Self::Redo => &labels.redo,
            Self::Cut => &labels.cut,
            Self::Copy => &labels.copy,
            Self::Paste => &labels.paste,
            Self::SelectAll => &labels.select_all,
        }
    }
    fn enabled(self, state: &TextareaState) -> bool {
        if state.presentation().is_disabled() {
            return false;
        }
        match self {
            Self::Undo => state.can_undo(),
            Self::Redo => state.can_redo(),
            Self::Cut => state.is_editable() && state.is_copyable(),
            Self::Copy => state.is_copyable(),
            Self::Paste => state.is_editable(),
            Self::SelectAll => state.text().len() > 0,
        }
    }
    fn action(self) -> Box<dyn Action> {
        match self {
            Self::Undo => Box::new(input::Undo),
            Self::Redo => Box::new(input::Redo),
            Self::Cut => Box::new(input::Cut),
            Self::Copy => Box::new(input::Copy),
            Self::Paste => Box::new(input::Paste),
            Self::SelectAll => Box::new(input::SelectAll),
        }
    }
}

/// Only transient navigation. The original State still owns every edit,
/// selection, clipboard operation, undo transaction and composition.
pub(super) struct OpenMenu {
    position: Point<Pixels>,
    focus: FocusHandle,
    selected: Option<Edit>,
    scroll: ScrollHandle,
    revealed: Option<(Option<Edit>, MenuGeometry)>,
    _focus_out: Subscription,
}

/// Derived layout cache used only to decide when navigation needs revealing.
/// Wheel scrolling never changes selection or acquires a second authority.
#[derive(PartialEq)]
struct MenuGeometry {
    font: maka_ui_runtime::appearance::FontAppearance,
    padding: maka_ui_runtime::appearance::Insets,
    row_padding: maka_ui_runtime::appearance::Insets,
    border: f32,
    width: Pixels,
    height: Pixels,
}

impl TextareaOwner {
    pub(super) fn install_menu_input(owner: &Entity<Self>, cx: &mut App) -> Subscription {
        let weak = owner.downgrade();
        owner.read(cx).state.clone().update(cx, |state, _| {
            let weak = weak.clone();
            state.on_context_menu(Rc::new(move |_, _, position, window, cx| {
                let _ = weak.update(cx, |owner, cx| owner.open_menu(position, window, cx));
            }));
        });
        // A real pre-dispatch interceptor is needed: raw key capture runs
        // after a matching native action binding has already executed.
        cx.intercept_keystrokes(move |event, window, cx| {
            let request = weak
                .update(cx, |owner, cx| owner.menu_key(&event.keystroke, window, cx))
                .ok()
                .flatten();
            if let Some((session, edit)) = request {
                activate(&weak, &session, edit, window, cx);
            }
        })
    }

    fn menu_allowed(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.input.0.borrow().is_some()
            && window.input_focus_is_admitted(self.state.read(cx).presentation().focus_handle())
            && self.menu.is_some()
            && !self.state.read(cx).presentation().is_disabled()
            && self
                .state
                .update(cx, |s, cx| s.marked_text_range(window, cx))
                .is_none()
    }

    fn menu_session(&self, session: &FocusHandle, window: &Window) -> bool {
        self.open_menu
            .as_ref()
            .is_some_and(|open| open.focus == *session && open.focus.is_focused(window))
    }

    fn open_menu(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if !self.menu_allowed(window, cx)
            || !self
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .is_focused(window)
        {
            return;
        }
        let focus = cx.focus_handle().tab_stop(false);
        let session = focus.clone();
        let focus_out = cx.on_focus_out(&focus, window, move |owner, _, window, cx| {
            if owner
                .open_menu
                .as_ref()
                .is_some_and(|open| open.focus == session)
            {
                owner.close_menu(false, window, cx);
            }
        });
        let selected = Edit::ALL
            .into_iter()
            .find(|edit| edit.enabled(self.state.read(cx)));
        focus.focus(window, cx);
        self.open_menu = Some(OpenMenu {
            position,
            focus,
            selected,
            scroll: ScrollHandle::new(),
            revealed: None,
            _focus_out: focus_out,
        });
        cx.notify();
    }

    pub(super) fn close_menu(
        &mut self,
        restore: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(open) = self.open_menu.take() else {
            return;
        };
        if restore && open.focus.is_focused(window) {
            if self.input.0.borrow().is_some() && !self.state.read(cx).presentation().is_disabled()
            {
                self.state
                    .read(cx)
                    .presentation()
                    .focus_handle()
                    .focus(window, cx);
            } else {
                window.blur(cx);
            }
        }
        cx.notify();
    }

    fn menu_key(
        &mut self,
        key: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(FocusHandle, Edit)> {
        if self.input.0.borrow().is_none()
            || self.menu.is_none()
            || self.state.read(cx).presentation().is_disabled()
        {
            self.close_menu(true, window, cx);
            return None;
        }
        let native_focused = self
            .state
            .read(cx)
            .presentation()
            .focus_handle()
            .is_focused(window);
        if self.open_menu.is_none() {
            if native_focused
                && !key.modifiers.control
                && !key.modifiers.alt
                && !key.modifiers.platform
                && ((key.key == "f10" && key.modifiers.shift)
                    || (key.key == "menu" && !key.modifiers.shift))
            {
                // Consume even a rejected opening during composition so an
                // ancestor binding cannot relocate the original selection.
                cx.stop_propagation();
                let position = self.state.read(cx).input_bounds().origin;
                self.open_menu(position, window, cx);
            }
            return None;
        }
        let session = self.open_menu.as_ref()?.focus.clone();
        if !session.is_focused(window) {
            self.close_menu(false, window, cx);
            return None;
        }
        // Menus never accept text or let editor/ancestor shortcuts mutate the
        // underlying native buffer. All keys stay with this exact navigation.
        cx.stop_propagation();
        if !self.menu_allowed(window, cx) {
            self.close_menu(true, window, cx);
            return None;
        }
        match key.key.as_str() {
            "escape" => self.close_menu(true, window, cx),
            "tab" => {
                self.close_menu(true, window, cx);
                if key.modifiers.shift {
                    window.focus_prev(cx);
                } else {
                    window.focus_next(cx);
                }
            }
            "enter" | "space" => return self.open_menu.as_ref()?.selected.map(|e| (session, e)),
            "up" | "down" | "home" | "end" => {
                let enabled: Vec<_> = Edit::ALL
                    .into_iter()
                    .filter(|edit| edit.enabled(self.state.read(cx)))
                    .collect();
                if enabled.is_empty() {
                    return None;
                }
                let open = self.open_menu.as_mut()?;
                let current = enabled.iter().position(|edit| Some(*edit) == open.selected);
                let index = match key.key.as_str() {
                    "home" => 0,
                    "end" => enabled.len() - 1,
                    "up" => current.map_or(enabled.len() - 1, |i| {
                        (i + enabled.len() - 1) % enabled.len()
                    }),
                    _ => current.map_or(0, |i| (i + 1) % enabled.len()),
                };
                open.selected = Some(enabled[index]);
                open.revealed = None;
                cx.notify();
            }
            _ => {}
        }
        None
    }

    pub(super) fn render_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let session = self.open_menu.as_ref()?.focus.clone();
        if !self.menu_allowed(window, cx) || !session.is_focused(window) {
            self.close_menu(false, window, cx);
            return None;
        }
        let state = self.state.read(cx);
        let open = self.open_menu.as_mut()?;
        if open.selected.is_none_or(|edit| !edit.enabled(state)) {
            open.selected = Edit::ALL.into_iter().find(|edit| edit.enabled(state));
        }
        let menu = self.menu.as_ref()?;
        let style = &menu.appearance;
        let content = Positioner::available_size(window, px(0.));
        // The preferred margin yields to usable viewport space, including
        // platform decoration insets, as the window becomes smaller.
        let margin = px(style.window_margin)
            .min(content.width / 4.)
            .min(content.height / 4.);
        let available = Positioner::available_size(window, margin);
        let width = px(style.width).min(available.width);
        let height = available.height;
        let geometry = MenuGeometry {
            font: style.text.font.clone(),
            padding: style.padding,
            row_padding: style.row_padding,
            border: style.border_width,
            width,
            height,
        };
        if open
            .revealed
            .as_ref()
            .is_none_or(|(selected, last)| *selected != open.selected || *last != geometry)
        {
            // Reveal only for navigation or real layout changes. Ordinary
            // notifications, including wheel browsing, preserve scroll.
            if let Some(selected) = open.selected {
                open.scroll
                    .scroll_to_item(Edit::ALL.iter().position(|e| *e == selected).unwrap());
            }
            open.revealed = Some((open.selected, geometry));
        }
        // A surface cannot be smaller than its own border and padding. If
        // that leaves no readable content, dismiss instead of spilling into
        // window decorations or showing an unreachable navigation surface.
        if width <= px(style.padding.left + style.padding.right + style.border_width * 2.)
            || height <= px(style.padding.top + style.padding.bottom + style.border_width * 2.)
        {
            self.close_menu(true, window, cx);
            return None;
        }
        let outside = cx.weak_entity();
        let outside_session = session.clone();
        let mut rows = div()
            .id("edit-menu-rows")
            .flex()
            .flex_col()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&open.scroll);
        for (index, edit) in Edit::ALL.into_iter().enumerate() {
            let enabled = edit.enabled(self.state.read(cx));
            let selected = enabled && open.selected == Some(edit);
            let label = edit.label(&menu.labels).to_owned();
            let weak = cx.weak_entity();
            let click_session = session.clone();
            let hover = weak.clone();
            let hover_session = session.clone();
            let ax = weak.clone();
            let ax_session = session.clone();
            let ax_focus = weak.clone();
            let ax_focus_session = session.clone();
            let p = style.row_padding;
            rows = rows.child(
                div()
                    .id(("edit", index))
                    .flex_shrink_0()
                    .w_full()
                    .overflow_hidden()
                    .pt(px(p.top))
                    .pr(px(p.right))
                    .pb(px(p.bottom))
                    .pl(px(p.left))
                    .bg(color(if selected {
                        style.active_background
                    } else {
                        maka_ui_runtime::appearance::Color(0)
                    }))
                    .text_color(color(if !enabled {
                        style.disabled_foreground
                    } else if selected {
                        style.active_foreground
                    } else {
                        style.text.foreground
                    }))
                    .role(Role::MenuItem)
                    .aria_label(label.clone())
                    .aria_disabled(!enabled)
                    .when(selected, |row| row.aria_active_descendant())
                    .on_hover(move |hovered, window, cx| {
                        if *hovered {
                            select(&hover, &hover_session, edit, false, window, cx);
                        }
                    })
                    .on_click(move |_, window, cx| {
                        activate(&weak, &click_session, edit, window, cx)
                    })
                    .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                        activate(&ax, &ax_session, edit, window, cx)
                    })
                    .on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
                        select(&ax_focus, &ax_focus_session, edit, true, window, cx)
                    })
                    .child(div().w_full().truncate().child(label)),
            );
        }
        let p = style.padding;
        let body = div()
            .id(("edit-menu", self.state.entity_id()))
            .track_focus(&session)
            .role(Role::Menu)
            .aria_label(menu.labels.title.clone())
            .flex()
            .flex_col()
            .min_w_0()
            .min_h_0()
            .w(width)
            .max_h(height)
            .overflow_hidden()
            .font(font(&style.text.font))
            .text_size(px(style.text.font.size))
            .line_height(relative(style.text.font.line_height))
            .text_color(color(style.text.foreground))
            .bg(color(style.background))
            .border(px(style.border_width))
            .border_color(color(style.border))
            .rounded(px(style.radius))
            .pt(px(p.top))
            .pr(px(p.right))
            .pb(px(p.bottom))
            .pl(px(p.left))
            .on_mouse_down_out(move |_, window, cx| {
                let _ = outside.update(cx, |owner, cx| {
                    if owner.menu_session(&outside_session, window) {
                        owner.close_menu(false, window, cx);
                    }
                });
            })
            .child(rows);
        Some(
            deferred(
                Positioner::corner(Anchor::TopLeft, open.position)
                    .margin(margin)
                    .occlude()
                    .child(body),
            )
            .into_any_element(),
        )
    }
}

fn select(
    owner: &WeakEntity<TextareaOwner>,
    session: &FocusHandle,
    edit: Edit,
    reveal: bool,
    window: &mut Window,
    cx: &mut App,
) {
    let _ = owner.update(cx, |owner, cx| {
        if owner.menu_session(session, window)
            && owner.menu_allowed(window, cx)
            && edit.enabled(owner.state.read(cx))
            && let Some(open) = owner.open_menu.as_mut()
            && (open.selected != Some(edit) || reveal)
        {
            open.selected = Some(edit);
            if reveal {
                open.revealed = None;
            }
            cx.notify();
        }
    });
}

fn activate(
    owner: &WeakEntity<TextareaOwner>,
    session: &FocusHandle,
    edit: Edit,
    window: &mut Window,
    cx: &mut App,
) {
    let target = owner.update(cx, |owner, cx| {
        if !owner.menu_session(session, window)
            || !owner.menu_allowed(window, cx)
            || !edit.enabled(owner.state.read(cx))
        {
            return None;
        }
        let target = owner.state.read(cx).presentation().focus_handle().clone();
        owner.close_menu(true, window, cx);
        Some(target)
    });
    if let Ok(Some(target)) = target {
        // Synchronous exact-node dispatch after the Owner and State borrows
        // end. Window::dispatch_action would defer and may fall back to root.
        target.dispatch_action(edit.action().as_ref(), window, cx);
    }
}
