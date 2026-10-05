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
mod popup;

use maka_ui::{ComponentSlot, NativeView};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::{rc::Rc, time::Instant};

/// The nominal ABI supplied by this presenter's pure scene compiler.
pub fn language() -> maka_plugin_core::package::SceneLanguage {
    maka_plugin_core::package::SceneLanguage {
        name: "maka.desktop.scene".into(),
        version: 1,
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Row,
    Column,
}
/// A scroll policy belongs to native presentation, never a plugin pixel mirror.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Scroll {
    #[default]
    None,
    Manual,
    Anchor,
    FollowEnd,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "DesktopScene")]
pub enum Node {
    Popup {
        key: String,
        label: String,
        trigger: Box<Node>,
        content: Box<Node>,
        open: bool,
        #[serde(default)]
        anchor: popup::Anchor,
        #[serde(default)]
        offset: u16,
        /// Styles the floating content; the trigger keeps its own layout.
        #[serde(default)]
        style: Style,
    },
    Stack {
        key: String,
        direction: Direction,
        gap: u16,
        padding: u16,
        background: u32,
        children: Vec<Node>,
        #[serde(default = "visible")]
        visible: bool,
        #[serde(default)]
        style: Style,
        #[serde(default)]
        scroll: Scroll,
    },
    Text {
        key: String,
        text: String,
        color: u32,
        #[serde(default)]
        style: Style,
    },
    Button {
        key: String,
        label: String,
        #[serde(default)]
        style: Style,
        #[serde(default)]
        hover: Box<Style>,
        #[serde(default)]
        active: Box<Style>,
        #[serde(default)]
        focus: Box<Style>,
        #[serde(default)]
        disabled: bool,
        #[serde(default)]
        tab_index: i16,
    },
    Canvas {
        key: String,
        contents: canvas::Contents,
        #[serde(default)]
        style: Style,
    },
    Component(ComponentSlot),
}
fn visible() -> bool {
    true
}
impl Node {
    pub(crate) fn validate_root(&self, deadline: Instant) -> Result<(), String> {
        self.validate(0, &mut BTreeSet::new(), deadline)
    }
    fn validate(
        &self,
        depth: usize,
        keys: &mut BTreeSet<String>,
        deadline: Instant,
    ) -> Result<(), String> {
        if Instant::now() >= deadline {
            return Err("scene compilation deadline".into());
        }
        let key = match self {
            Self::Stack { key, .. }
            | Self::Text { key, .. }
            | Self::Button { key, .. }
            | Self::Canvas { key, .. }
            | Self::Popup { key, .. } => key,
            Self::Component(slot) => &slot.key,
        };
        if depth > 32
            || keys.len() >= 4096
            || key.is_empty()
            || key.len() > 128
            || !keys.insert(key.clone())
        {
            return Err("invalid or duplicate UI node identity".into());
        }
        match self {
            Self::Stack { style, .. }
            | Self::Text { style, .. }
            | Self::Canvas { style, .. }
            | Self::Popup { style, .. } => style.validate()?,
            Self::Button {
                style,
                hover,
                active,
                focus,
                tab_index,
                ..
            } => {
                for style in [style, hover.as_ref(), active.as_ref(), focus.as_ref()] {
                    style.validate()?;
                }
                if !(-1..=32767).contains(tab_index) {
                    return Err("invalid tab index".into());
                }
            }
            Self::Component(_) => {}
        }
        match self {
            Self::Popup {
                trigger,
                content,
                label,
                offset,
                ..
            } => {
                if label.is_empty() || label.len() > 4096 || *offset > 1024 {
                    return Err("invalid popup presentation".into());
                }
                trigger.validate(depth + 1, keys, deadline)?;
                content.validate(depth + 1, keys, deadline)?;
            }
            Self::Stack {
                gap,
                padding,
                background,
                children,
                ..
            } => {
                if *gap > 1024 || *padding > 1024 || *background > 0xffffff {
                    return Err("invalid UI style".into());
                }
                for child in children {
                    child.validate(depth + 1, keys, deadline)?;
                }
            }
            Self::Text { text, color, .. } => {
                if text.len() > 64 * 1024 || *color > 0xffffff {
                    return Err("invalid text".into());
                }
            }
            Self::Button { label, .. } => {
                if label.len() > 4096 {
                    return Err("invalid button label".into());
                }
            }
            Self::Canvas { contents, .. } => contents.validate(deadline)?,
            Self::Component(slot) => {
                if slot.binding.is_empty() || slot.binding.len() > 128 {
                    return Err("invalid slot binding".into());
                }
            }
        }
        Ok(())
    }
    fn render_depth(
        &self,
        enclosing: u16,
        visible: bool,
        depth: &mut u16,
        slots: &mut BTreeMap<String, u16>,
    ) {
        if !visible {
            return;
        }
        *depth = (*depth).max(enclosing);
        match self {
            Self::Popup {
                trigger,
                content,
                open,
                ..
            } => {
                trigger.render_depth(enclosing, true, depth, slots);
                content.render_depth(enclosing + 1, *open, depth, slots);
            }
            Self::Stack {
                children, visible, ..
            } => {
                for child in children {
                    child.render_depth(enclosing, *visible, depth, slots);
                }
            }
            Self::Component(slot) => {
                slots.insert(slot.key.clone(), enclosing);
            }
            _ => {}
        }
    }
    fn metadata(
        &self,
        slots: &mut Vec<ComponentSlot>,
        inputs: &mut BTreeMap<String, Value>,
        hidden_slots: &mut BTreeSet<String>,
        input_count: &mut usize,
        visible: bool,
        deadline: Instant,
    ) -> Result<(), String> {
        if Instant::now() >= deadline {
            return Err("scene compilation deadline".into());
        }
        match self {
            Self::Popup {
                key,
                trigger,
                content,
                open,
                ..
            } => {
                *input_count += 1;
                if *input_count > 64 {
                    return Err("scene input limit".into());
                }
                if visible && *open {
                    inputs.insert(
                        key.clone(),
                        serde_json::to_value(schemars::schema_for!(popup::Input))
                            .expect("popup input schema"),
                    );
                }
                trigger.metadata(slots, inputs, hidden_slots, input_count, visible, deadline)?;
                content.metadata(
                    slots,
                    inputs,
                    hidden_slots,
                    input_count,
                    visible && *open,
                    deadline,
                )?;
            }
            Self::Component(slot) => {
                if slots.len() >= 64 {
                    return Err("scene slot limit".into());
                }
                slots.push(slot.clone());
                if !visible {
                    hidden_slots.insert(slot.key.clone());
                }
            }
            Self::Button { key, .. } => {
                *input_count += 1;
                if *input_count > 64 {
                    return Err("scene input limit".into());
                }
                if visible {
                    inputs.insert(key.clone(), json!(true));
                }
            }
            Self::Canvas { key, .. } => {
                *input_count += 1;
                if *input_count > 64 {
                    return Err("scene input limit".into());
                }
                if visible {
                    inputs.insert(
                        key.clone(),
                        serde_json::to_value(schemars::schema_for!(canvas::CanvasInput))
                            .expect("canvas input schema"),
                    );
                }
            }
            Self::Stack {
                children,
                visible: local,
                ..
            } => {
                for child in children {
                    child.metadata(
                        slots,
                        inputs,
                        hidden_slots,
                        input_count,
                        visible && *local,
                        deadline,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

pub fn compile(
    source: &Value,
    deadline: Instant,
) -> maka_ui::Result<NativeView<dyn super::Widget>> {
    if Instant::now() >= deadline {
        return Err(maka_ui::Error::Invalid("desktop scene deadline".into()));
    }
    fn scan(
        source: &Value,
        depth: usize,
        count: &mut usize,
        deadline: Instant,
    ) -> maka_ui::Result<()> {
        *count += 1;
        if Instant::now() >= deadline || depth > 32 || *count > 4096 {
            return Err(maka_ui::Error::Invalid(
                "scene source depth/node/deadline limit".into(),
            ));
        }
        for field in ["style", "hover", "active", "focus"] {
            if let Some(value) = source.get(field) {
                Style::validate_source(value).map_err(maka_ui::Error::Invalid)?;
            }
            if Instant::now() >= deadline {
                return Err(maka_ui::Error::Invalid("scene compilation deadline".into()));
            }
        }
        if source.get("kind").and_then(Value::as_str) == Some("canvas")
            && let Some(contents) = source.get("contents")
        {
            canvas::Contents::preflight(contents, count, deadline)
                .map_err(maka_ui::Error::Invalid)?;
        }
        if let Some(children) = source.get("children").and_then(Value::as_array) {
            for child in children {
                scan(child, depth + 1, count, deadline)?;
            }
        }
        for field in ["trigger", "content"] {
            if let Some(child) = source.get(field) {
                scan(child, depth + 1, count, deadline)?;
            }
        }
        Ok(())
    }
    scan(source, 0, &mut 0, deadline)?;
    let node = Node::deserialize(source).map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
    node.validate_root(deadline)
        .map_err(maka_ui::Error::Invalid)?;
    let mut slots = vec![];
    let mut inputs = BTreeMap::new();
    let mut hidden_slots = BTreeSet::new();
    node.metadata(
        &mut slots,
        &mut inputs,
        &mut hidden_slots,
        &mut 0,
        true,
        deadline,
    )
    .map_err(maka_ui::Error::Invalid)?;
    let mut depth = 0;
    let mut slot_depth = BTreeMap::new();
    node.render_depth(0, true, &mut depth, &mut slot_depth);
    if depth > gpui_kit::MAX_DEFERRED_DEPTH {
        return Err(maka_ui::Error::Invalid(
            "native scene render depth limit".into(),
        ));
    }
    NativeView::new(Rc::new(node) as Rc<dyn super::Widget>, inputs, slots)?
        .with_hidden_slots(hidden_slots)?
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
struct SceneState {
    visible: bool,
    input: super::LiveInput,
    buttons: BTreeMap<String, ButtonState>,
    scrolls: BTreeMap<String, gpui_kit::ScrollHandle>,
    canvases: BTreeMap<String, canvas::State>,
    popups: BTreeMap<String, popup::State>,
}
impl SceneState {
    fn reconcile(&mut self, root: &Node, window: &mut gpui_kit::Window, cx: &mut gpui_kit::App) {
        let mut buttons = std::mem::take(&mut self.buttons);
        let mut scrolls = std::mem::take(&mut self.scrolls);
        let mut canvases = std::mem::take(&mut self.canvases);
        let mut popups = std::mem::take(&mut self.popups);
        // Original mechanical owners and native context remain separate.
        #[allow(clippy::too_many_arguments)]
        fn visit(
            node: &Node,
            current: &mut SceneState,
            buttons: &mut BTreeMap<String, ButtonState>,
            scrolls: &mut BTreeMap<String, gpui_kit::ScrollHandle>,
            canvases: &mut BTreeMap<String, canvas::State>,
            popups: &mut BTreeMap<String, popup::State>,
            window: &mut gpui_kit::Window,
            cx: &mut gpui_kit::App,
            visible: bool,
        ) {
            match node {
                Node::Popup {
                    key,
                    trigger,
                    content,
                    open,
                    ..
                } => {
                    visit(
                        trigger, current, buttons, scrolls, canvases, popups, window, cx, visible,
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
                    visible: local,
                    ..
                } => {
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
            window,
            cx,
            visible,
        );
        for canvas in canvases.values_mut() {
            canvas.deactivate(window, cx);
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
            input,
            buttons: BTreeMap::new(),
            scrolls: BTreeMap::new(),
            canvases: BTreeMap::new(),
            popups: BTreeMap::new(),
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
    ) -> maka_ui::Result<gpui_kit::AnyElement> {
        let state = state.downcast_ref::<SceneState>().expect("scene owner");
        if !state.visible {
            use gpui_kit::{IntoElement, Styled};
            return Ok(gpui_kit::div().hidden().into_any_element());
        }
        self.element(state, slots, window, cx)
    }
}
impl Node {
    fn element(
        &self,
        state: &SceneState,
        slots: &super::Slots,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) -> maka_ui::Result<gpui_kit::AnyElement> {
        use gpui_kit::prelude::FluentBuilder as _;
        use gpui_kit::{
            InteractiveElement, IntoElement, ParentElement, SharedString,
            StatefulInteractiveElement, Styled, base::Button, div, px, rgb,
        };
        Ok(match self {
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
                let popup = state
                    .popups
                    .get(key)
                    .ok_or_else(|| maka_ui::Error::Invalid("scene popup owner replaced".into()))?;
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
                style,
                hover,
                active,
                focus,
                disabled: _,
                tab_index: _,
            } => {
                let button = state
                    .buttons
                    .get(key)
                    .ok_or_else(|| maka_ui::Error::Invalid("scene button owner replaced".into()))?;
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
                let click = slots.input.bind(key.clone(), |_| json!({"kind":"click"}));
                let enabled = button.admitted.get() && alive.0.borrow().is_some();
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
                    .track_focus(&handle)
                    .tab_index(handle.tab_index)
                    .tab_stop(handle.tab_stop)
                    .capture_any_mouse_down(move |_, _, cx| {
                        if !down.get() || down_alive.0.borrow().is_none() {
                            cx.stop_propagation();
                        }
                    })
                    .capture_any_mouse_up(move |_, _, cx| {
                        if !up.get() || up_alive.0.borrow().is_none() {
                            cx.stop_propagation();
                        }
                    })
                    .capture_key_down(move |_, _, cx| {
                        if !key_down.get() || key_down_alive.0.borrow().is_none() {
                            cx.stop_propagation();
                        }
                    })
                    .capture_key_up(move |_, _, cx| {
                        if !key_up.get() || key_up_alive.0.borrow().is_none() {
                            cx.stop_propagation();
                        }
                    })
                    .when(enabled, |this| {
                        this.on_a11y_action(
                            gpui_kit::AccessibleAction::Focus,
                            move |_, window, cx| {
                                if a11y.get() && a11y_alive.0.borrow().is_some() {
                                    focus_action.focus(window, cx);
                                }
                            },
                        )
                    })
                    .hover(move |s| hover.apply(s))
                    .active(move |s| active.apply(s))
                    .focus_visible(move |s| focus.apply(s))
                    .child(SharedString::from(label.clone()))
                    .on_click(move |event, window, cx| {
                        if click_admission.get() && click_alive.0.borrow().is_some() {
                            click(event, window, cx);
                        }
                    })
                    .into_any_element()
            }
            Self::Canvas {
                key,
                contents,
                style,
            } => {
                let canvas = state
                    .canvases
                    .get(key)
                    .ok_or_else(|| maka_ui::Error::Invalid("scene canvas owner replaced".into()))?;
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
                if !visible {
                    return Ok(element.hidden().into_any_element());
                }
                if *scroll != Scroll::None {
                    let handle = state.scrolls.get(key).ok_or_else(|| {
                        maka_ui::Error::Invalid("scene scroll owner replaced".into())
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
                    element = element.child(node.element(state, slots, window, cx)?);
                }
                element.into_any_element()
            }
        })
    }
}

/// Desktop layout and appearance are a presenter ABI, never kernel policy.
/// Unspecified refinements inherit from the explicit surrounding scene scope.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Style {
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    #[schemars(range(min = 0, max = 16000))]
    pub min_width: Option<f32>,
    #[schemars(range(min = 0, max = 16000))]
    pub max_width: Option<f32>,
    #[schemars(range(min = 0, max = 16000))]
    pub min_height: Option<f32>,
    #[schemars(range(min = 0, max = 16000))]
    pub max_height: Option<f32>,
    #[schemars(range(min = 0, max = 100))]
    pub grow: Option<f32>,
    #[schemars(range(min = 0, max = 100))]
    pub shrink: Option<f32>,
    pub wrap: Option<bool>,
    /// Draw one measured line with an ellipsis; retain the complete text in AX.
    #[serde(default)]
    pub truncate: bool,
    pub align: Option<Align>,
    pub justify: Option<Justify>,
    pub text: Option<maka_ui::appearance::TextAppearance>,
    pub background: Option<maka_ui::appearance::Color>,
    pub border: Option<maka_ui::appearance::Color>,
    #[schemars(range(min = 0, max = 8))]
    pub border_width: Option<f32>,
    #[schemars(range(min = 0, max = 128))]
    pub radius: Option<f32>,
    pub padding: Option<maka_ui::appearance::Insets>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "unit",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Dimension {
    Pixels(#[schemars(range(min = 0, max = 16000))] f32),
    Fraction(#[schemars(range(min = 0, max = 1))] f32),
    Auto,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
    Baseline,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Justify {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
}
impl Dimension {
    fn length(self) -> gpui_kit::Length {
        match self {
            Self::Pixels(n) => gpui_kit::px(n).into(),
            Self::Fraction(n) => gpui_kit::relative(n).into(),
            Self::Auto => gpui_kit::Length::Auto,
        }
    }
}
impl Style {
    fn schema() -> &'static jsonschema::Validator {
        static SCHEMA: std::sync::LazyLock<jsonschema::Validator> =
            std::sync::LazyLock::new(|| {
                jsonschema::validator_for(
                    &serde_json::to_value(schemars::schema_for!(Style)).expect("style schema"),
                )
                .expect("valid style schema")
            });
        &SCHEMA
    }
    fn validate_source(source: &Value) -> Result<(), String> {
        Self::schema().validate(source).map_err(|e| e.to_string())
    }
    fn validate(&self) -> Result<(), String> {
        if [
            self.min_width,
            self.max_width,
            self.min_height,
            self.max_height,
            self.grow,
            self.shrink,
            self.border_width,
            self.radius,
        ]
        .into_iter()
        .flatten()
        .any(|v| !v.is_finite())
        {
            return Err("non-finite desktop style".into());
        }
        Self::validate_source(&serde_json::to_value(self).map_err(|e| e.to_string())?)
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
