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

//! Desktop wire declarations. Native layout, focus and gestures belong to the presenter.

pub mod canvas;
use crate::ComponentSlot;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "DesktopDirection")]
pub enum Direction {
    Row,
    Column,
}
/// A scroll policy belongs to native presentation, never a plugin pixel mirror.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "DesktopScroll")]
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
    Dialog {
        key: String,
        label: String,
        background: Box<Node>,
        content: Box<Node>,
        open: bool,
        /// Styles the bounded foreground body; background layout remains original.
        #[serde(default)]
        style: Style,
        #[serde(default = "backdrop")]
        backdrop: crate::appearance::Color,
    },
    Popup {
        key: String,
        label: String,
        trigger: Box<Node>,
        content: Box<Node>,
        open: bool,
        #[serde(default)]
        anchor: Anchor,
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
        #[serde(default)]
        #[schemars(length(max = 32))]
        shortcuts: Vec<crate::shortcut::Shortcut>,
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
    pub fn validate_root(&self, deadline: Instant) -> Result<(), String> {
        self.validate(0, &mut BTreeSet::new(), deadline)
    }
    pub fn key(&self) -> &str {
        match self {
            Self::Stack { key, .. }
            | Self::Text { key, .. }
            | Self::Button { key, .. }
            | Self::Canvas { key, .. }
            | Self::Popup { key, .. }
            | Self::Dialog { key, .. } => key,
            Self::Component(slot) => &slot.key,
        }
    }
    pub fn reading_key(&self) -> Option<&str> {
        match self {
            Self::Stack {
                key,
                direction: Direction::Column,
                scroll,
                ..
            } if *scroll != Scroll::None => Some(key),
            _ => None,
        }
    }
    pub fn reading_target(&self, key: &str) -> bool {
        match self {
            Self::Stack { children, .. } => children.iter().any(|child| {
                child.key() == key && !matches!(child, Self::Stack { visible: false, .. })
            }),
            _ => false,
        }
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
            | Self::Popup { key, .. }
            | Self::Dialog { key, .. } => key,
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
        if let Self::Stack { shortcuts, .. } = self {
            crate::shortcut::validate(shortcuts, cfg!(target_os = "macos"))?;
        }
        match self {
            Self::Stack { style, .. }
            | Self::Text { style, .. }
            | Self::Canvas { style, .. }
            | Self::Popup { style, .. }
            | Self::Dialog { style, .. } => style.validate()?,
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
            Self::Dialog {
                background,
                content,
                label,
                ..
            } => {
                if label.is_empty() || label.len() > 4096 {
                    return Err("invalid dialog presentation".into());
                }
                background.validate(depth + 1, keys, deadline)?;
                content.validate(depth + 1, keys, deadline)?;
            }
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
    pub fn render_depth(
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
            Self::Dialog {
                background,
                content,
                open,
                ..
            } => {
                background.render_depth(enclosing, true, depth, slots);
                content.render_depth(enclosing + 1, *open, depth, slots);
            }
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
    pub fn shortcut_inputs(
        &self,
        inputs: &mut BTreeMap<String, Value>,
        visible: bool,
    ) -> Result<(), String> {
        match self {
            Self::Stack {
                children,
                shortcuts,
                visible: local,
                ..
            } => {
                if visible && *local {
                    crate::shortcut::inputs(shortcuts, inputs)?;
                }
                for child in children {
                    child.shortcut_inputs(inputs, visible && *local)?;
                }
            }
            Self::Dialog {
                background,
                content,
                open,
                ..
            } => {
                background.shortcut_inputs(inputs, visible)?;
                content.shortcut_inputs(inputs, visible && *open)?;
            }
            Self::Popup {
                trigger,
                content,
                open,
                ..
            } => {
                trigger.shortcut_inputs(inputs, visible)?;
                content.shortcut_inputs(inputs, visible && *open)?;
            }
            _ => {}
        }
        Ok(())
    }
    pub fn metadata(
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
            Self::Dialog {
                key,
                background,
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
                        serde_json::to_value(schemars::schema_for!(crate::dismiss::Input))
                            .expect("dialog input schema"),
                    );
                }
                background.metadata(slots, inputs, hidden_slots, input_count, visible, deadline)?;
                content.metadata(
                    slots,
                    inputs,
                    hidden_slots,
                    input_count,
                    visible && *open,
                    deadline,
                )?;
            }
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
                        serde_json::to_value(schemars::schema_for!(crate::dismiss::Input))
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

/// Desktop layout and appearance are a presenter ABI, never kernel policy.
/// Unspecified refinements inherit from the explicit surrounding scene scope.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "DesktopStyle")]
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
    pub text: Option<crate::appearance::TextAppearance>,
    pub background: Option<crate::appearance::Color>,
    pub border: Option<crate::appearance::Color>,
    #[schemars(range(min = 0, max = 8))]
    pub border_width: Option<f32>,
    #[schemars(range(min = 0, max = 128))]
    pub radius: Option<f32>,
    pub padding: Option<crate::appearance::Insets>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "unit",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
#[schemars(rename = "DesktopDimension")]
pub enum Dimension {
    Pixels(#[schemars(range(min = 0, max = 16000))] f32),
    Fraction(#[schemars(range(min = 0, max = 1))] f32),
    Auto,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "DesktopAlign")]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
    Baseline,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "DesktopJustify")]
pub enum Justify {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
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
    pub fn validate_source(source: &Value) -> Result<(), String> {
        Self::schema().validate(source).map_err(|e| e.to_string())
    }
    pub fn validate(&self) -> Result<(), String> {
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
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "DesktopAnchor")]
pub enum Anchor {
    TopLeft,
    TopCenter,
    #[default]
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    LeftCenter,
    RightCenter,
}
fn backdrop() -> crate::appearance::Color {
    crate::appearance::Color(0x00000066)
}
