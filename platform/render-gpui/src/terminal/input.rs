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
use gpui_kit::{
    ClipboardItem, EntityInputHandler, KeyDownEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ScrollWheelEvent, UTF16Selection,
};
use maka_ui_runtime::terminal::input::{
    InputAction, Key, Modifier, Modifiers, MouseAction, MouseButton, MouseEvent, ScrollDirection,
};
use std::ops::Range;
fn modifiers(value: gpui_kit::Modifiers) -> Modifiers {
    Modifiers::try_from(
        [
            value.control.then_some(Modifier::Ctrl),
            value.alt.then_some(Modifier::Alt),
            value.shift.then_some(Modifier::Shift),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>(),
    )
    .expect("unique native modifiers")
}
fn button(value: gpui_kit::MouseButton) -> Option<MouseButton> {
    Some(match value {
        gpui_kit::MouseButton::Left => MouseButton::Left,
        gpui_kit::MouseButton::Right => MouseButton::Right,
        gpui_kit::MouseButton::Middle => MouseButton::Middle,
        _ => return None,
    })
}
impl Owner {
    pub(super) fn key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.focused(window) {
            return;
        }
        let key = &event.keystroke;
        let m = key.modifiers;
        if m.control && matches!(key.key.as_str(), "]" | "5") {
            self.core.capture(false);
            self.marked.clear();
            self.selected = 0..0;
            window.focus_prev(cx);
            // The tab order wraps when this is the only stop. Releasing capture
            // must still leave the terminal before the next geometry update.
            if self.focus.is_focused(window) {
                window.blur(cx);
            }
        } else if (m.platform || m.control && m.shift) && key.key.eq_ignore_ascii_case("c") {
            if let Some(display) = &self.core.display {
                cx.write_to_clipboard(ClipboardItem::new_string(display.screen.clone()));
            }
        } else if !self.core.ready() {
            return;
        } else if (m.platform || m.control && m.shift) && key.key.eq_ignore_ascii_case("v") {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                self.core.paste(&text);
            }
        } else {
            if m.platform {
                return;
            }
            // Printable text is committed exactly once by the platform input handler,
            // including composed text. Special/control keys use original typed encoding.
            if !m.control
                && !m.alt
                && key
                    .key_char
                    .as_ref()
                    .is_some_and(|s| !s.is_empty() && s.chars().all(|c| !c.is_control()))
            {
                return;
            }
            let name = match key.key.as_str() {
                "up" => "arrow_up",
                "down" => "arrow_down",
                "left" => "arrow_left",
                "right" => "arrow_right",
                "pageup" => "page_up",
                "pagedown" => "page_down",
                "esc" => "escape",
                name => name,
            };
            if let Ok(key) = Key::parse(name) {
                self.core.write(InputAction::Key {
                    key,
                    modifiers: modifiers(m),
                });
            }
        }
        window.prevent_default();
        cx.stop_propagation();
        self.schedule(window, cx);
        cx.notify();
    }
    fn pointer(
        &mut self,
        position: gpui_kit::Point<Pixels>,
        event: MouseEvent,
        m: gpui_kit::Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.admits(window) || !self.core.captured() {
            return;
        }
        let Some(g) = &self.geometry else {
            return;
        };
        if !g.bounds.contains(&position) {
            return;
        }
        let relative = position - g.bounds.origin;
        self.core.write(InputAction::Mouse(MouseAction {
            x: (f32::from(relative.x) / f32::from(g.cell)).floor() as u64,
            y: (f32::from(relative.y) / f32::from(g.line)).floor() as u64,
            event,
            modifiers: modifiers(m),
        }));
        window.prevent_default();
        cx.stop_propagation();
        self.schedule(window, cx);
        cx.notify();
    }
    pub(super) fn mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.enabled()
            || self
                .geometry
                .as_ref()
                .is_none_or(|g| g.epoch != self.epoch || !g.bounds.contains(&event.position))
        {
            return;
        }
        self.focus.focus(window, cx);
        self.core.capture(true);
        if let Some(button) = button(event.button) {
            self.pointer(
                event.position,
                MouseEvent::Press(button),
                event.modifiers,
                window,
                cx,
            );
        }
    }
    pub(super) fn mouse_up(
        &mut self,
        event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(button) = button(event.button) {
            self.pointer(
                event.position,
                MouseEvent::Release(button),
                event.modifiers,
                window,
                cx,
            );
        }
    }
    pub(super) fn mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pointer(
            event.position,
            MouseEvent::Move(event.pressed_button.and_then(button)),
            event.modifiers,
            window,
            cx,
        );
    }
    pub(super) fn wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.enabled() {
            return;
        }
        let Some(g) = &self.geometry else {
            return;
        };
        if g.epoch != self.epoch || !g.bounds.contains(&event.position) {
            return;
        }
        let delta = match event.delta {
            gpui_kit::ScrollDelta::Lines(p) => p.y,
            gpui_kit::ScrollDelta::Pixels(p) => f32::from(p.y) / f32::from(g.line),
        };
        if delta == 0. {
            return;
        }
        if event.modifiers.shift || !self.core.captured() {
            self.core.scroll_by(if delta > 0. { 3 } else { -3 });
            window.prevent_default();
            cx.stop_propagation();
            cx.notify();
        } else {
            self.pointer(
                event.position,
                MouseEvent::Scroll(if delta > 0. {
                    ScrollDirection::Up
                } else {
                    ScrollDirection::Down
                }),
                event.modifiers,
                window,
                cx,
            );
        }
    }
}
fn byte_at(text: &str, at: usize) -> Option<usize> {
    let mut n = 0;
    for (byte, ch) in text.char_indices() {
        if n == at {
            return Some(byte);
        }
        n += ch.len_utf16();
    }
    (n == at).then_some(text.len())
}
impl EntityInputHandler for Owner {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let start = byte_at(&self.marked, range.start)?;
        let end = byte_at(&self.marked, range.end)?;
        *adjusted = Some(range);
        self.marked.get(start..end).map(str::to_owned)
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        self.admits(window).then(|| UTF16Selection {
            range: self.selected.clone(),
            reversed: false,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        (!self.marked.is_empty()).then(|| 0..self.marked.encode_utf16().count())
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked.clear();
        self.selected = 0..0;
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.admits(window) {
            return;
        }
        let Some(text) = replace(&self.marked, range, text) else {
            return;
        };
        self.marked.clear();
        self.selected = 0..0;
        if !text.is_empty() {
            self.core.write(InputAction::Text(text));
        }
        self.schedule(window, cx);
        cx.notify();
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.admits(window) {
            return;
        }
        let Some(text) = replace(&self.marked, range, text) else {
            return;
        };
        let n = text.encode_utf16().count();
        let selected = selected.unwrap_or(n..n);
        if byte_at(&text, selected.start).is_none()
            || byte_at(&text, selected.end).is_none()
            || selected.start > selected.end
        {
            return;
        }
        self.marked = text;
        self.selected = selected;
        cx.notify();
    }
    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        _: Bounds<Pixels>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        if !self.admits(window) {
            return None;
        }
        let g = self.geometry.as_ref()?;
        let at = &self.core.display.as_ref()?.cursor;
        Some(Bounds::new(
            g.bounds.origin + point(g.cell * f32::from(at.x), g.line * f32::from(at.y)),
            size(g.cell, g.line),
        ))
    }
    fn character_index_for_point(
        &mut self,
        _: gpui_kit::Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
    fn accepts_text_input(&self, window: &mut Window, _: &mut Context<Self>) -> bool {
        self.admits(window)
    }
    fn paste(&mut self, item: ClipboardItem, window: &mut Window, cx: &mut Context<Self>) {
        if self.admits(window)
            && let Some(text) = item.text()
        {
            self.core.paste(&text);
            self.schedule(window, cx);
            cx.notify();
        }
    }
}
pub(super) fn replace(old: &str, range: Option<Range<usize>>, text: &str) -> Option<String> {
    if text.len() > 32 * 1024 || text.chars().any(char::is_control) {
        return None;
    }
    let range = range.unwrap_or(0..old.encode_utf16().count());
    let start = byte_at(old, range.start)?;
    let end = byte_at(old, range.end)?;
    if start > end || old.len() - (end - start) + text.len() > 32 * 1024 {
        return None;
    }
    Some(format!("{}{}{}", &old[..start], text, &old[end..]))
}
