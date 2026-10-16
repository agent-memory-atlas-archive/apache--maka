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

pub mod canvas;
mod dialog;
mod popup;

use gpui_kit::AppContext;
use maka_ui_runtime::{NativeView, PreparedScene};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::rc::Rc;

/// The nominal ABI supplied by this presenter's pure scene compiler.
pub fn language() -> maka_plugin_core::package::SceneLanguage {
    maka_plugin_core::package::SceneLanguage {
        name: "maka.desktop.scene".into(),
        version: 1,
    }
}

pub use maka_ui_runtime::scene::desktop::{
    Align, Dimension, Direction, Justify, Node, Scroll, Style,
};

pub fn compile(source: &Value) -> maka_ui_runtime::Result<NativeView<dyn super::Widget>> {
    compile_with_inputs(source, BTreeMap::new())
}

pub(crate) fn compile_with_inputs(
    source: &Value,
    additional_inputs: BTreeMap<String, Value>,
) -> maka_ui_runtime::Result<NativeView<dyn super::Widget>> {
    finish(prepare_with_inputs(source, additional_inputs)?)
}

pub fn prepare(source: &Value) -> maka_ui_runtime::Result<PreparedScene> {
    prepare_with_inputs(source, BTreeMap::new())
}
pub fn finish(prepared: PreparedScene) -> maka_ui_runtime::Result<NativeView<dyn super::Widget>> {
    prepared.finish::<Node, dyn super::Widget>(|node| Rc::new(node))
}
fn prepare_with_inputs(
    source: &Value,
    additional_inputs: BTreeMap<String, Value>,
) -> maka_ui_runtime::Result<PreparedScene> {
    fn scan(source: &Value, depth: usize, count: &mut usize) -> maka_ui_runtime::Result<()> {
        *count += 1;
        if depth > 32 {
            return Err(maka_ui_runtime::Error::Invalid(
                "scene source depth limit".into(),
            ));
        }
        for field in ["style", "hover", "active", "focus"] {
            if let Some(value) = source.get(field) {
                Style::validate_source(value).map_err(maka_ui_runtime::Error::Invalid)?;
            }
        }
        if source.get("kind").and_then(Value::as_str) == Some("canvas")
            && let Some(contents) = source.get("contents")
        {
            canvas::Contents::preflight(contents, count)
                .map_err(maka_ui_runtime::Error::Invalid)?;
        }
        if let Some(children) = source.get("children").and_then(Value::as_array) {
            for child in children {
                scan(child, depth + 1, count)?;
            }
        }
        if source.get("kind").and_then(Value::as_str) == Some("dialog")
            && let Some(background) = source.get("background")
        {
            scan(background, depth + 1, count)?;
        }
        for field in ["trigger", "content"] {
            if let Some(child) = source.get(field) {
                scan(child, depth + 1, count)?;
            }
        }
        Ok(())
    }
    scan(source, 0, &mut 0)?;
    let node =
        Node::deserialize(source).map_err(|e| maka_ui_runtime::Error::Invalid(e.to_string()))?;
    node.validate_root()
        .map_err(maka_ui_runtime::Error::Invalid)?;
    let maka_ui_runtime::scene::desktop::Metadata {
        slots,
        mut inputs,
        hidden_slots,
        render_depth: depth,
        slot_depth,
    } = node
        .metadata(BTreeMap::new())
        .map_err(maka_ui_runtime::Error::Invalid)?;
    for (key, schema) in additional_inputs {
        if inputs.insert(key, schema).is_some() {
            return Err(maka_ui_runtime::Error::Invalid(
                "native reading input collides with scene input".into(),
            ));
        }
    }
    if depth > gpui_kit::MAX_DEFERRED_DEPTH {
        return Err(maka_ui_runtime::Error::Invalid(
            "native scene render depth limit".into(),
        ));
    }
    let visible = !matches!(&node, Node::Stack { visible: false, .. });
    PreparedScene::new(node, inputs, slots)?
        .with_hidden_slots(hidden_slots)?
        .with_root_visibility(visible)
        .with_render_depth(depth, slot_depth)
}
// The mount owns mechanical state. Layout ancestry is not focus/scroll identity.
// Cached listeners retain only this node's admission, never the whole scene owner.
struct ButtonState {
    focus: gpui_kit::FocusHandle,
    admitted: Rc<std::cell::Cell<bool>>,
}
impl Drop for ButtonState {
    fn drop(&mut self) {
        self.admitted.set(false);
        let _ = self.focus.clone().tab_stop(false).tab_index(-1);
    }
}
struct ShortcutState {
    admitted: bool,
    shortcuts: Vec<maka_ui_runtime::shortcut::Shortcut>,
}
pub(crate) struct SceneState {
    visible: bool,
    keyboard_focus: Option<(String, gpui_kit::FocusHandle)>,
    input: super::LiveInput,
    buttons: BTreeMap<String, ButtonState>,
    shortcuts: BTreeMap<String, gpui_kit::Entity<ShortcutState>>,
    scrolls: BTreeMap<String, gpui_kit::ScrollHandle>,
    canvases: BTreeMap<String, canvas::State>,
    popups: BTreeMap<String, popup::State>,
    dialogs: BTreeMap<String, dialog::State>,
}
impl SceneState {
    fn reconcile(&mut self, root: &Node, window: &mut gpui_kit::Window, cx: &mut gpui_kit::App) {
        fn keyboard_root(node: &Node) -> Option<&String> {
            match node {
                Node::Stack { key, shortcuts, .. } if !shortcuts.is_empty() => Some(key),
                Node::Dialog { background, .. } => keyboard_root(background),
                _ => None,
            }
        }
        let root_key = keyboard_root(root);
        if self.keyboard_focus.as_ref().map(|(key, _)| key) != root_key {
            if let Some((_, focus)) = self.keyboard_focus.take()
                && focus.is_focused(window)
            {
                window.blur(cx);
            }
            self.keyboard_focus =
                root_key.map(|key| (key.clone(), cx.focus_handle().tab_stop(false)));
        }
        if !self.visible
            && let Some((_, focus)) = &self.keyboard_focus
            && focus.is_focused(window)
        {
            window.blur(cx);
        }
        let mut buttons = std::mem::take(&mut self.buttons);
        let mut scrolls = std::mem::take(&mut self.scrolls);
        let mut canvases = std::mem::take(&mut self.canvases);
        let mut popups = std::mem::take(&mut self.popups);
        let mut dialogs = std::mem::take(&mut self.dialogs);
        let mut shortcuts = std::mem::take(&mut self.shortcuts);
        // Original mechanical owners and native context remain separate.
        #[allow(clippy::too_many_arguments)]
        fn visit(
            node: &Node,
            current: &mut SceneState,
            buttons: &mut BTreeMap<String, ButtonState>,
            scrolls: &mut BTreeMap<String, gpui_kit::ScrollHandle>,
            canvases: &mut BTreeMap<String, canvas::State>,
            popups: &mut BTreeMap<String, popup::State>,
            dialogs: &mut BTreeMap<String, dialog::State>,
            shortcuts: &mut BTreeMap<String, gpui_kit::Entity<ShortcutState>>,
            window: &mut gpui_kit::Window,
            cx: &mut gpui_kit::App,
            visible: bool,
        ) {
            match node {
                Node::Dialog {
                    key,
                    background,
                    content,
                    open,
                    ..
                } => {
                    visit(
                        background, current, buttons, scrolls, canvases, popups, dialogs,
                        shortcuts, window, cx, visible,
                    );
                    let mut owner = dialogs
                        .remove(key)
                        .unwrap_or_else(|| dialog::State::new(cx));
                    owner.presentation(visible && *open, window, cx);
                    visit(
                        content,
                        current,
                        buttons,
                        scrolls,
                        canvases,
                        popups,
                        dialogs,
                        shortcuts,
                        window,
                        cx,
                        visible && *open,
                    );
                    current.dialogs.insert(key.clone(), owner);
                }
                Node::Popup {
                    key,
                    trigger,
                    content,
                    open,
                    ..
                } => {
                    visit(
                        trigger, current, buttons, scrolls, canvases, popups, dialogs, shortcuts,
                        window, cx, visible,
                    );
                    let mut popup = popups
                        .remove(key)
                        .unwrap_or_else(|| popup::State::new(window, cx));
                    popup.presentation(visible && *open, window, cx);
                    visit(
                        content,
                        current,
                        buttons,
                        scrolls,
                        canvases,
                        popups,
                        dialogs,
                        shortcuts,
                        window,
                        cx,
                        visible && *open,
                    );
                    current.popups.insert(key.clone(), popup);
                }
                Node::Button {
                    key,
                    disabled,
                    tab_index,
                    ..
                } => {
                    let mut button = buttons.remove(key).unwrap_or_else(|| ButtonState {
                        focus: cx.focus_handle(),
                        admitted: Rc::new(std::cell::Cell::new(false)),
                    });
                    let enabled = visible && !disabled;
                    button.admitted.set(enabled);
                    button.focus = button
                        .focus
                        .clone()
                        .tab_stop(enabled && *tab_index >= 0)
                        .tab_index(if enabled { *tab_index as isize } else { -1 });
                    if !enabled && button.focus.is_focused(window) {
                        window.blur(cx);
                    }
                    current.buttons.insert(key.clone(), button);
                }
                Node::Canvas { key, contents, .. } => {
                    let mut canvas = canvases.remove(key).unwrap_or_else(|| {
                        canvas::State::new(key.clone(), contents, current.input.clone(), window, cx)
                    });
                    canvas.reconcile(contents, window, cx);
                    canvas.presentation(visible, window, cx);
                    current.canvases.insert(key.clone(), canvas);
                }
                Node::Stack {
                    key,
                    scroll,
                    children,
                    shortcuts: declarations,
                    visible: local,
                    ..
                } => {
                    if !declarations.is_empty() {
                        let owner = shortcuts.remove(key).unwrap_or_else(|| {
                            cx.new(|_| ShortcutState {
                                admitted: false,
                                shortcuts: vec![],
                            })
                        });
                        owner.update(cx, |owner, _| {
                            owner.admitted = visible && *local;
                            owner.shortcuts = declarations.clone();
                        });
                        current.shortcuts.insert(key.clone(), owner);
                    }
                    if *scroll != Scroll::None {
                        current
                            .scrolls
                            .insert(key.clone(), scrolls.remove(key).unwrap_or_default());
                    }
                    for child in children {
                        visit(
                            child,
                            current,
                            buttons,
                            scrolls,
                            canvases,
                            popups,
                            dialogs,
                            shortcuts,
                            window,
                            cx,
                            visible && *local,
                        );
                    }
                }
                _ => {}
            }
        }
        let visible = self.visible;
        visit(
            root,
            self,
            &mut buttons,
            &mut scrolls,
            &mut canvases,
            &mut popups,
            &mut dialogs,
            &mut shortcuts,
            window,
            cx,
            visible,
        );
        for owner in shortcuts.values() {
            owner.update(cx, |owner, _| owner.admitted = false);
        }
        for canvas in canvases.values_mut() {
            canvas.deactivate(window, cx);
        }
        for dialog in dialogs.values_mut() {
            dialog.deactivate(window, cx);
        }
        for popup in popups.values_mut() {
            popup.deactivate(window, cx);
        }
        for button in buttons.values() {
            if button.focus.is_focused(window) {
                window.blur(cx);
            }
        }
    }
}
impl super::Widget for Node {
    fn create(
        &self,
        input: super::LiveInput,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> Box<dyn std::any::Any> {
        let mut state = SceneState {
            // Custody is allocated before the presenter knows actual visibility.
            // In particular, an initially hidden popup must never steal focus.
            visible: false,
            keyboard_focus: None,
            input,
            buttons: BTreeMap::new(),
            shortcuts: BTreeMap::new(),
            scrolls: BTreeMap::new(),
            canvases: BTreeMap::new(),
            popups: BTreeMap::new(),
            dialogs: BTreeMap::new(),
        };
        state.reconcile(self, window, cx);
        Box::new(state)
    }
    fn presentation(
        &self,
        state: &mut dyn std::any::Any,
        visible: bool,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) {
        let state = state.downcast_mut::<SceneState>().expect("scene owner");
        if state.visible != visible {
            state.visible = visible;
            state.reconcile(self, window, cx);
        }
    }
    fn commit(
        &self,
        state: &mut dyn std::any::Any,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) {
        state
            .downcast_mut::<SceneState>()
            .expect("scene owner")
            .reconcile(self, window, cx);
    }
    fn deactivate(
        &self,
        state: &mut dyn std::any::Any,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) {
        let state = state.downcast_mut::<SceneState>().expect("scene owner");
        if let Some((_, focus)) = &state.keyboard_focus
            && focus.is_focused(window)
        {
            window.blur(cx);
        }
        for owner in state.shortcuts.values() {
            owner.update(cx, |owner, _| owner.admitted = false);
        }
        for dialog in state.dialogs.values_mut() {
            dialog.deactivate(window, cx);
        }
        for popup in state.popups.values_mut() {
            popup.deactivate(window, cx);
        }
        for canvas in state.canvases.values_mut() {
            canvas.deactivate(window, cx);
        }
        for button in state.buttons.values_mut() {
            button.admitted.set(false);
            button.focus = button.focus.clone().tab_stop(false).tab_index(-1);
            if button.focus.is_focused(window) {
                window.blur(cx);
            }
        }
    }
    fn render(
        &self,
        state: &dyn std::any::Any,
        slots: &super::Slots,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui_runtime::Result<gpui_kit::AnyElement> {
        let state = state.downcast_ref::<SceneState>().expect("scene owner");
        if !state.visible {
            use gpui_kit::{IntoElement, Styled};
            return Ok(gpui_kit::div().hidden().into_any_element());
        }
        // Give an otherwise unfocused surface a non-tab command scope. Native
        // child focus remains authoritative and is never replaced by this fallback.
        if window.focused(cx).is_none()
            && !window.has_pending_focus_restoration()
            && let Some((_, focus)) = &state.keyboard_focus
        {
            window.focus(focus, cx);
        }
        self.element(state, slots, window, cx)
    }
}
pub(crate) trait NodePresentation {
    fn reading_visible(&self, state: &dyn std::any::Any) -> bool;
    fn reading_handle(&self, state: &dyn std::any::Any) -> gpui_kit::ScrollHandle;
    fn render_reading(
        &self,
        state: &dyn std::any::Any,
        slots: &super::Slots,
        scope: gpui_kit::base::TextSelectionScopeId,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui_runtime::Result<gpui_kit::AnyElement>;
    fn element(
        &self,
        state: &SceneState,
        slots: &super::Slots,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui_runtime::Result<gpui_kit::AnyElement>;
    fn element_root(
        &self,
        state: &SceneState,
        slots: &super::Slots,
        reading: Option<gpui_kit::base::TextSelectionScopeId>,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui_runtime::Result<gpui_kit::AnyElement>;
}
impl NodePresentation for Node {
    fn reading_visible(&self, state: &dyn std::any::Any) -> bool {
        state
            .downcast_ref::<SceneState>()
            .expect("scene owner")
            .visible
            && matches!(self, Self::Stack { visible: true, .. })
    }
    fn reading_handle(&self, state: &dyn std::any::Any) -> gpui_kit::ScrollHandle {
        state
            .downcast_ref::<SceneState>()
            .expect("scene owner")
            .scrolls[self.reading_key().expect("reading root")]
        .clone()
    }
    fn render_reading(
        &self,
        state: &dyn std::any::Any,
        slots: &super::Slots,
        scope: gpui_kit::base::TextSelectionScopeId,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui_runtime::Result<gpui_kit::AnyElement> {
        let state = state.downcast_ref::<SceneState>().expect("scene owner");
        if !state.visible {
            use gpui_kit::{IntoElement, Styled};
            return Ok(gpui_kit::div().hidden().into_any_element());
        }
        self.element_root(state, slots, Some(scope), window, cx)
    }
    fn element(
        &self,
        state: &SceneState,
        slots: &super::Slots,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui_runtime::Result<gpui_kit::AnyElement> {
        self.element_root(state, slots, None, window, cx)
    }
    fn element_root(
        &self,
        state: &SceneState,
        slots: &super::Slots,
        reading: Option<gpui_kit::base::TextSelectionScopeId>,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui_runtime::Result<gpui_kit::AnyElement> {
        use gpui_kit::prelude::FluentBuilder as _;
        use gpui_kit::{
            InteractiveElement, IntoElement, ParentElement, SharedString,
            StatefulInteractiveElement, Styled, base::Button, div, px, rgb,
        };
        Ok(match self {
            Self::Dialog {
                key,
                label,
                background,
                content,
                open,
                style,
                backdrop,
            } => {
                let dialog = state.dialogs.get(key).ok_or_else(|| {
                    maka_ui_runtime::Error::Invalid("scene dialog owner replaced".into())
                })?;
                let background = background.element_root(state, slots, reading, window, cx)?;
                let content = if *open && dialog.admitted() {
                    Some(content.element(state, slots, window, cx)?)
                } else {
                    None
                };
                dialog.render(
                    key,
                    label,
                    background,
                    content,
                    style,
                    *backdrop,
                    &state.input,
                    slots.input.clone(),
                    window,
                    cx,
                )
            }
            Self::Popup {
                key,
                label,
                trigger,
                content,
                anchor,
                offset,
                style,
                open,
            } => {
                let popup = state.popups.get(key).ok_or_else(|| {
                    maka_ui_runtime::Error::Invalid("scene popup owner replaced".into())
                })?;
                let trigger = trigger.element(state, slots, window, cx)?;
                let content = if *open && popup.admitted() {
                    Some(content.element(state, slots, window, cx)?)
                } else {
                    None
                };
                popup.render(
                    key,
                    label,
                    trigger,
                    content,
                    *anchor,
                    *offset,
                    style,
                    &state.input,
                    slots.input.clone(),
                    window,
                )
            }
            Self::Text {
                key,
                text,
                color,
                style,
            } => style
                .apply(
                    div()
                        .id(SharedString::from(format!("text:{key}")))
                        .text_color(rgb(*color)),
                )
                .child(gpui_kit::Text::new(
                    SharedString::from(format!("label:{key}")).into(),
                    SharedString::from(text.clone()),
                ))
                .into_any_element(),
            Self::Button {
                key,
                label,
                copy_text,
                bubble_activation,
                style,
                hover,
                active,
                focus,
                disabled: _,
                tab_index: _,
            } => {
                let button = state.buttons.get(key).ok_or_else(|| {
                    maka_ui_runtime::Error::Invalid("scene button owner replaced".into())
                })?;
                let alive = state.input.clone();
                let admission = button.admitted.clone();
                let down_alive = alive.clone();
                let up_alive = alive.clone();
                let key_down_alive = alive.clone();
                let key_up_alive = alive.clone();
                let a11y_alive = alive.clone();
                let down = admission.clone();
                let up = admission.clone();
                let key_down = admission.clone();
                let key_up = admission.clone();
                let a11y = admission.clone();
                let click_admission = admission.clone();
                let click_alive = alive.clone();
                let click_input = slots.input.clone();
                let click_key = key.clone();
                let copy_text = copy_text.clone();
                let bubble_activation = *bubble_activation;
                let enabled = button.admitted.get() && alive.current.borrow().is_some();
                let handle = button.focus.clone();
                let focus_action = handle.clone();
                let hover = hover.clone();
                let active = active.clone();
                let focus = focus.clone();
                style
                    .apply(Button::new(SharedString::from(format!("button:{key}"))))
                    .accessibility_label(SharedString::from(label.clone()))
                    .disabled(!enabled)
                    .aria_disabled(!enabled)
                    .when(enabled, |this| this.track_focus(&handle))
                    .tab_index(handle.tab_index)
                    .tab_stop(handle.tab_stop)
                    .capture_any_mouse_down(move |_, _, cx| {
                        if !down.get() || down_alive.current.borrow().is_none() {
                            cx.stop_propagation();
                        }
                    })
                    .capture_any_mouse_up(move |_, _, cx| {
                        if !up.get() || up_alive.current.borrow().is_none() {
                            cx.stop_propagation();
                        }
                    })
                    .capture_key_down(move |_, _, cx| {
                        if !key_down.get() || key_down_alive.current.borrow().is_none() {
                            cx.stop_propagation();
                        }
                    })
                    .capture_key_up(move |_, _, cx| {
                        if !key_up.get() || key_up_alive.current.borrow().is_none() {
                            cx.stop_propagation();
                        }
                    })
                    .when(enabled, |this| {
                        this.on_a11y_action(
                            gpui_kit::AccessibleAction::Focus,
                            move |_, window, cx| {
                                if a11y.get() && a11y_alive.current.borrow().is_some() {
                                    focus_action.focus(window, cx);
                                }
                            },
                        )
                    })
                    .hover(move |s| hover.apply(s))
                    .active(move |s| active.apply(s))
                    .focus_visible(move |s| focus.apply(s))
                    .child(SharedString::from(label.clone()))
                    .on_click(move |_, window, cx| {
                        if click_admission.get() && click_alive.current.borrow().is_some() {
                            let accepted = if let Some(text) = &copy_text {
                                click_alive
                                    .capture_input(&click_key, json!({"kind":"click"}), window, cx)
                                    .map(|receipt| {
                                        let alive = click_alive.clone();
                                        let text = text.clone();
                                        window
                                            .spawn(cx, async move |cx| {
                                                if let Err(error) = receipt.await {
                                                    eprintln!("component input rejected: {error}");
                                                    return;
                                                }
                                                let _ = cx.update(move |_, cx| {
                                                    // Hiding a menu does not retire its original
                                                    // owner; actual close revokes this default.
                                                    if alive.current.borrow().is_some() {
                                                        cx.write_to_clipboard(
                                                            gpui_kit::ClipboardItem::new_string(
                                                                text,
                                                            ),
                                                        );
                                                    }
                                                });
                                            })
                                            .detach();
                                    })
                            } else {
                                (click_input.dispatch)(
                                    &click_key,
                                    json!({"kind":"click"}),
                                    window,
                                    cx,
                                )
                            };
                            if let Err(error) = accepted {
                                eprintln!("component input rejected: {error}");
                                window.prevent_default();
                                cx.stop_propagation();
                                return;
                            }
                            if !bubble_activation {
                                cx.stop_propagation();
                            }
                        }
                    })
                    .into_any_element()
            }
            Self::Canvas {
                key,
                contents,
                style,
            } => {
                let canvas = state.canvases.get(key).ok_or_else(|| {
                    maka_ui_runtime::Error::Invalid("scene canvas owner replaced".into())
                })?;
                canvas.render(contents, style, slots.input.clone())
            }
            Self::Component(slot) => slots.render(&slot.key, window, cx)?,
            Self::Stack {
                key,
                direction,
                gap,
                padding,
                background,
                children,
                style,
                scroll,
                visible,
                shortcuts,
            } => {
                let element = div()
                    .id(SharedString::from(format!(
                        "stack:{key}:{}",
                        if *scroll != Scroll::None {
                            "scroll"
                        } else {
                            "layout"
                        }
                    )))
                    .flex()
                    .gap(px(*gap as f32))
                    .p(px(*padding as f32))
                    .bg(rgb(*background));
                let element = match direction {
                    Direction::Row => element.flex_row(),
                    Direction::Column => element.flex_col(),
                };
                let mut element = style.apply(element);
                if let Some((root, focus)) = &state.keyboard_focus
                    && root == key
                {
                    element = element.track_focus(focus);
                }
                if !visible {
                    return Ok(element.hidden().into_any_element());
                }
                if !shortcuts.is_empty() {
                    let owner = state
                        .shortcuts
                        .get(key)
                        .ok_or_else(|| {
                            maka_ui_runtime::Error::Invalid("scene shortcut owner replaced".into())
                        })?
                        .downgrade();
                    let declarations = shortcuts.clone();
                    let input = slots.input.clone();
                    let window_id = window.window_handle().window_id();
                    element = element.on_key_down(move |event, window, cx| {
                        if event.is_held
                            || event.prefer_character_input
                            || event.keystroke.modifiers.function
                            || window.window_handle().window_id() != window_id
                            || window.has_marked_text(cx)
                        {
                            return;
                        }
                        let modifiers = event.keystroke.modifiers;
                        let physical = maka_ui_runtime::shortcut::Modifiers {
                            control: modifiers.control,
                            alt: modifiers.alt,
                            shift: modifiers.shift,
                            super_key: modifiers.platform,
                        };
                        let Some(shortcut) = declarations.iter().find(|shortcut| {
                            shortcut.matches(
                                &event.keystroke.key,
                                physical,
                                cfg!(target_os = "macos"),
                            )
                        }) else {
                            return;
                        };
                        // A nearest old declaration remains a barrier. Admission and
                        // original input epoch decide whether it can still invoke.
                        window.prevent_default();
                        cx.stop_propagation();
                        let admitted = owner.upgrade().is_some_and(|owner| {
                            let current = owner.read(cx);
                            current.admitted && current.shortcuts == declarations
                        });
                        if admitted && shortcut.enabled {
                            input.bind(
                                shortcut.input.clone(),
                                |_: &gpui_kit::KeyDownEvent| json!({"kind":"click"}),
                            )(event, window, cx);
                        }
                    });
                }
                if *scroll != Scroll::None {
                    let handle = state.scrolls.get(key).ok_or_else(|| {
                        maka_ui_runtime::Error::Invalid("scene scroll owner replaced".into())
                    })?;
                    element = element
                        .overflow_y_scroll()
                        .track_scroll(handle)
                        .retain_scroll(match scroll {
                            Scroll::None | Scroll::Manual => gpui_kit::ScrollRetention::None,
                            Scroll::Anchor => gpui_kit::ScrollRetention::Anchor,
                            Scroll::FollowEnd => gpui_kit::ScrollRetention::FollowEnd,
                        });
                }
                for node in children {
                    let child = node.element(state, slots, window, cx)?;
                    element = element.child(if reading.is_some() {
                        super::reading::mark(node.key(), child)
                    } else {
                        child
                    });
                }
                element.into_any_element()
            }
        })
    }
}

trait DimensionPresentation {
    fn length(self) -> gpui_kit::Length;
}
impl DimensionPresentation for Dimension {
    fn length(self) -> gpui_kit::Length {
        match self {
            Self::Pixels(n) => gpui_kit::px(n).into(),
            Self::Fraction(n) => gpui_kit::relative(n).into(),
            Self::Auto => gpui_kit::Length::Auto,
        }
    }
}
trait StylePresentation {
    fn bounded(&self, available: gpui_kit::Size<gpui_kit::Pixels>) -> Self;
    fn apply<T: gpui_kit::Styled>(&self, element: T) -> T;
}
impl StylePresentation for Style {
    fn bounded(&self, available: gpui_kit::Size<gpui_kit::Pixels>) -> Self {
        let mut style = self.clone();
        let width = style
            .max_width
            .unwrap_or(available.width.into())
            .min(available.width.into());
        let height = style
            .max_height
            .unwrap_or(available.height.into())
            .min(available.height.into());
        style.max_width = Some(width);
        style.max_height = Some(height);
        style.min_width = style.min_width.map(|value| value.min(width));
        style.min_height = style.min_height.map(|value| value.min(height));
        if let Some(Dimension::Pixels(value)) = &mut style.width {
            *value = value.min(width);
        }
        if let Some(Dimension::Pixels(value)) = &mut style.height {
            *value = value.min(height);
        }
        // Taffy cannot shrink a border box below its padding and border. Keep
        // those insets within its viewport budget, leaving at least half of
        // each inner axis for actual content rather than only decorations.
        let effective = |dimension: Option<Dimension>, available: f32, maximum: f32| match dimension
        {
            Some(Dimension::Pixels(value)) => value.min(maximum),
            Some(Dimension::Fraction(value)) => (available * value).min(maximum),
            _ => maximum,
        };
        let inset_width = effective(style.width, available.width.into(), width);
        let inset_height = effective(style.height, available.height.into(), height);
        let border = style
            .border_width
            .unwrap_or(0.)
            .min(inset_width.min(inset_height) / 4.);
        style.border_width = style.border_width.map(|_| border);
        if let Some(padding) = &mut style.padding {
            let horizontal = padding.left + padding.right;
            let vertical = padding.top + padding.bottom;
            let horizontal_scale =
                ((inset_width - 2. * border) / (2. * horizontal).max(1.)).min(1.);
            let vertical_scale = ((inset_height - 2. * border) / (2. * vertical).max(1.)).min(1.);
            padding.left *= horizontal_scale;
            padding.right *= horizontal_scale;
            padding.top *= vertical_scale;
            padding.bottom *= vertical_scale;
        }
        style
    }

    fn apply<T: gpui_kit::Styled>(&self, mut element: T) -> T {
        use gpui_kit::px;
        if let Some(v) = self.width {
            element = element.w(v.length());
        }
        if let Some(v) = self.height {
            element = element.h(v.length());
        }
        if let Some(v) = self.min_width {
            element = element.min_w(px(v));
        }
        if let Some(v) = self.max_width {
            element = element.max_w(px(v));
        }
        if let Some(v) = self.min_height {
            element = element.min_h(px(v));
        }
        if let Some(v) = self.max_height {
            element = element.max_h(px(v));
        }
        if let Some(v) = self.grow {
            element = element.flex_grow(v);
        }
        if let Some(v) = self.shrink {
            element = element.flex_shrink(v);
        }
        if let Some(v) = self.wrap {
            element = if v {
                element.flex_wrap()
            } else {
                element.flex_nowrap()
            };
        }
        if let Some(v) = self.align {
            element = match v {
                Align::Start => element.items_start(),
                Align::Center => element.items_center(),
                Align::End => element.items_end(),
                Align::Stretch => element.items_stretch(),
                Align::Baseline => element.items_baseline(),
            };
        }
        if let Some(v) = self.justify {
            element = match v {
                Justify::Start => element.justify_start(),
                Justify::Center => element.justify_center(),
                Justify::End => element.justify_end(),
                Justify::Between => element.justify_between(),
                Justify::Around => element.justify_around(),
                Justify::Evenly => element.justify_evenly(),
            };
        }
        if let Some(v) = &self.text {
            element = element
                .font(crate::appearance::font(&v.font))
                .text_size(px(v.font.size))
                .line_height(gpui_kit::relative(v.font.line_height))
                .text_color(crate::appearance::color(v.foreground));
        }
        if self.truncate {
            element = element.truncate();
        }
        if let Some(v) = self.background {
            element = element.bg(crate::appearance::color(v));
        }
        if let Some(v) = self.border {
            element = element.border_color(crate::appearance::color(v));
        }
        if let Some(v) = self.border_width {
            element = element.border(px(v));
        }
        if let Some(v) = self.radius {
            element = element.rounded(px(v));
        }
        if let Some(v) = self.padding {
            element = element
                .pt(px(v.top))
                .pr(px(v.right))
                .pb(px(v.bottom))
                .pl(px(v.left));
        }
        element
    }
}

#[cfg(test)]
mod tests;
