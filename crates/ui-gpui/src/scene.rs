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
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "DesktopScene")]
pub enum Node {
    Stack {
        key: String,
        direction: Direction,
        gap: u16,
        padding: u16,
        background: u32,
        children: Vec<Node>,
        #[serde(default)]
        style: Style,
        #[serde(default)]
        scroll: bool,
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
            | Self::Canvas { key, .. } => key,
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
            Self::Stack { style, .. } | Self::Text { style, .. } | Self::Canvas { style, .. } => {
                style.validate()?
            }
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
    fn metadata(
        &self,
        slots: &mut Vec<ComponentSlot>,
        inputs: &mut BTreeMap<String, Value>,
        deadline: Instant,
    ) -> Result<(), String> {
        if Instant::now() >= deadline {
            return Err("scene compilation deadline".into());
        }
        match self {
            Self::Component(slot) => {
                if slots.len() >= 64 {
                    return Err("scene slot limit".into());
                }
                slots.push(slot.clone());
            }
            Self::Button { key, .. } => {
                if inputs.len() >= 64 {
                    return Err("scene input limit".into());
                }
                inputs.insert(key.clone(), json!(true));
            }
            Self::Canvas { key, .. } => {
                if inputs.len() >= 64 {
                    return Err("scene input limit".into());
                }
                inputs.insert(
                    key.clone(),
                    serde_json::to_value(schemars::schema_for!(canvas::CanvasInput))
                        .expect("canvas input schema"),
                );
            }
            Self::Stack { children, .. } => {
                for child in children {
                    child.metadata(slots, inputs, deadline)?;
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
        Ok(())
    }
    scan(source, 0, &mut 0, deadline)?;
    let node = Node::deserialize(source).map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
    node.validate_root(deadline)
        .map_err(maka_ui::Error::Invalid)?;
    let mut slots = vec![];
    let mut inputs = BTreeMap::new();
    node.metadata(&mut slots, &mut inputs, deadline)
        .map_err(maka_ui::Error::Invalid)?;
    NativeView::new(Rc::new(node) as Rc<dyn super::Widget>, inputs, slots)
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
    input: super::LiveInput,
    buttons: BTreeMap<String, ButtonState>,
    scrolls: BTreeMap<String, gpui_kit::ScrollHandle>,
    canvases: BTreeMap<String, canvas::State>,
}
impl SceneState {
    fn reconcile(&mut self, root: &Node, window: &mut gpui_kit::Window, cx: &mut gpui_kit::App) {
        let mut buttons = std::mem::take(&mut self.buttons);
        let mut scrolls = std::mem::take(&mut self.scrolls);
        let mut canvases = std::mem::take(&mut self.canvases);
        fn visit(
            node: &Node,
            current: &mut SceneState,
            buttons: &mut BTreeMap<String, ButtonState>,
            scrolls: &mut BTreeMap<String, gpui_kit::ScrollHandle>,
            canvases: &mut BTreeMap<String, canvas::State>,
            window: &mut gpui_kit::Window,
            cx: &mut gpui_kit::App,
        ) {
            match node {
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
                    button.admitted.set(!disabled);
                    button.focus = button
                        .focus
                        .clone()
                        .tab_stop(!disabled && *tab_index >= 0)
                        .tab_index(if *disabled { -1 } else { *tab_index as isize });
                    if *disabled && button.focus.is_focused(window) {
                        window.blur(cx);
                    }
                    current.buttons.insert(key.clone(), button);
                }
                Node::Canvas { key, contents, .. } => {
                    let mut canvas = canvases.remove(key).unwrap_or_else(|| {
                        canvas::State::new(key.clone(), contents, current.input.clone(), window, cx)
                    });
                    canvas.reconcile(contents, window, cx);
                    current.canvases.insert(key.clone(), canvas);
                }
                Node::Stack {
                    key,
                    scroll,
                    children,
                    ..
                } => {
                    if *scroll {
                        current
                            .scrolls
                            .insert(key.clone(), scrolls.remove(key).unwrap_or_default());
                    }
                    for child in children {
                        visit(child, current, buttons, scrolls, canvases, window, cx);
                    }
                }
                _ => {}
            }
        }
        visit(
            root,
            self,
            &mut buttons,
            &mut scrolls,
            &mut canvases,
            window,
            cx,
        );
        for canvas in canvases.values_mut() {
            canvas.deactivate(window, cx);
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
            input,
            buttons: BTreeMap::new(),
            scrolls: BTreeMap::new(),
            canvases: BTreeMap::new(),
        };
        state.reconcile(self, window, cx);
        Box::new(state)
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
        self.element(
            state.downcast_ref::<SceneState>().expect("scene owner"),
            slots,
            window,
            cx,
        )
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
                .child(SharedString::from(text.clone()))
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
                    .on_click(slots.input.bind(key.clone(), |_| json!({"kind":"click"})))
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
            } => {
                let element = div()
                    .id(SharedString::from(format!(
                        "stack:{key}:{}",
                        if *scroll { "scroll" } else { "layout" }
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
                if *scroll {
                    let handle = state.scrolls.get(key).ok_or_else(|| {
                        maka_ui::Error::Invalid("scene scroll owner replaced".into())
                    })?;
                    element = element.overflow_y_scroll().track_scroll(handle);
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
