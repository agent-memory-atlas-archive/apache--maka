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

//! Terminal cell scene declarations and protocol validation.

use crate::ComponentSlot;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "TerminalDirection")]
pub enum Direction {
    Row,
    Column,
}
/// Native terminal cell constraints on a child in its parent's main axis.
/// `grow` distributes remaining cells after fixed/intrinsic siblings; `basis`
/// supplies an explicit base instead of intrinsic length. `shrink` allows that
/// base to shrink proportionally when siblings exceed the available cells.
/// Row wrap preserves
/// source order and measures its rows before allocating the parent viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[schemars(rename = "TerminalLayout")]
pub struct Layout {
    #[schemars(range(max = 1024))]
    pub grow: u16,
    pub basis: Option<u16>,
    pub shrink: bool,
    pub wrap: bool,
}
/// Native scroll policy; offsets and accepted viewport geometry remain native.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "TerminalScrollPolicy")]
pub enum ScrollPolicy {
    #[default]
    None,
    Manual,
    Anchor,
    FollowEnd,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "TerminalScene")]
pub enum Node {
    Dialog {
        key: String,
        label: String,
        background: Box<Node>,
        content: Box<Node>,
        open: bool,
        /// Maximum foreground body dimensions, in terminal cells.
        width: u16,
        height: u16,
        background_color: u32,
        border_color: u32,
        #[serde(default)]
        padding: u16,
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
        #[serde(default)]
        #[schemars(length(max = 32))]
        focus_shortcuts: Vec<crate::shortcut::FocusShortcut>,
        #[serde(default)]
        layout: Layout,
        #[serde(default)]
        scroll: ScrollPolicy,
        #[serde(default = "visible")]
        visible: bool,
    },
    Text {
        key: String,
        text: String,
        color: u32,
        /// Render a single line, eliding overflow at a complete grapheme.
        #[serde(default)]
        truncate: bool,
    },
    Button {
        key: String,
        label: String,
        /// Enter a declared child's native focus target instead of emitting a click.
        #[serde(default)]
        focus_child: Option<String>,
        #[serde(default)]
        disabled: bool,
        /// Absent foreground uses the terminal's default text color.
        #[serde(default)]
        #[schemars(range(max = 0xffffff))]
        color: Option<u32>,
    },
    Component(ComponentSlot),
}
fn visible() -> bool {
    true
}
impl Node {
    pub fn key(&self) -> &str {
        match self {
            Self::Stack { key, .. }
            | Self::Text { key, .. }
            | Self::Button { key, .. }
            | Self::Dialog { key, .. } => key,
            Self::Component(slot) => &slot.key,
        }
    }
    pub fn layout(&self) -> Layout {
        match self {
            Self::Stack { layout, .. } => *layout,
            Self::Dialog { background, .. } => background.layout(),
            _ => Layout::default(),
        }
    }
    pub fn visible(&self) -> bool {
        !matches!(self, Self::Stack { visible: false, .. })
    }
    pub fn validate_root(&self, deadline: Instant) -> Result<(), String> {
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
        if let Self::Stack {
            shortcuts,
            focus_shortcuts,
            ..
        } = self
        {
            crate::shortcut::validate_focus(shortcuts, focus_shortcuts)?;
        }
        match self {
            Self::Dialog {
                label,
                background,
                content,
                width,
                height,
                background_color,
                border_color,
                padding,
                ..
            } => {
                if label.is_empty()
                    || label.len() > 512
                    || *width == 0
                    || *height == 0
                    || *background_color > 0xffffff
                    || *border_color > 0xffffff
                    || *padding > 1024
                {
                    return Err("invalid terminal dialog presentation".into());
                }
                background.validate(depth + 1, keys, deadline)?;
                content.validate(depth + 1, keys, deadline)?;
            }
            Self::Stack {
                gap,
                padding,
                background,
                children,
                layout,
                direction,
                scroll,
                ..
            } => {
                if layout.grow > 1024
                    || (layout.wrap && !matches!(direction, Direction::Row))
                    || (*scroll != ScrollPolicy::None && !matches!(direction, Direction::Column))
                {
                    return Err("invalid terminal layout".into());
                }
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
            Self::Button { label, color, .. } => {
                if label.len() > 4096 || color.is_some_and(|color| color > 0xffffff) {
                    return Err("invalid button".into());
                }
            }
            Self::Component(slot) => {
                if slot.binding.is_empty() || slot.binding.len() > 128 {
                    return Err("invalid slot binding".into());
                }
            }
        }
        Ok(())
    }
    pub fn validate_focus_targets(&self, slots: &[ComponentSlot]) -> Result<(), String> {
        match self {
            Self::Button {
                focus_child: Some(child),
                ..
            } if !slots.iter().any(|slot| slot.key == *child) => {
                return Err("focus button names an undeclared child".into());
            }
            Self::Stack {
                focus_shortcuts,
                children,
                ..
            } => {
                if focus_shortcuts
                    .iter()
                    .any(|binding| !slots.iter().any(|slot| slot.key == binding.child))
                {
                    return Err("focus shortcut names an undeclared child".into());
                }
                for child in children {
                    child.validate_focus_targets(slots)?;
                }
            }
            Self::Dialog {
                background,
                content,
                ..
            } => {
                background.validate_focus_targets(slots)?;
                content.validate_focus_targets(slots)?;
            }
            _ => {}
        }
        Ok(())
    }
    pub fn shortcut_inputs(
        &self,
        inputs: &mut BTreeMap<String, Value>,
        visible: bool,
    ) -> Result<(), String> {
        if let Self::Dialog {
            background,
            content,
            open,
            ..
        } = self
        {
            background.shortcut_inputs(inputs, visible)?;
            content.shortcut_inputs(inputs, visible && *open)?;
        }
        if let Self::Stack {
            children,
            shortcuts,
            visible: local,
            ..
        } = self
        {
            if visible && *local {
                crate::shortcut::inputs(shortcuts, inputs)?;
            }
            for child in children {
                child.shortcut_inputs(inputs, visible && *local)?;
            }
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
                if visible
                    && *open
                    && inputs
                        .insert(
                            key.clone(),
                            serde_json::to_value(schemars::schema_for!(crate::dismiss::Input))
                                .expect("dismiss schema"),
                        )
                        .is_some()
                {
                    return Err("reserved scene input identity".into());
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
            Self::Component(slot) => {
                if slots.len() >= 64 {
                    return Err("scene slot limit".into());
                }
                slots.push(slot.clone());
                if !visible {
                    hidden_slots.insert(slot.key.clone());
                }
            }
            Self::Button {
                key,
                disabled,
                focus_child,
                ..
            } => {
                *input_count += 1;
                if *input_count > 64 {
                    return Err("scene input limit".into());
                }
                if visible
                    && !disabled
                    && focus_child.is_none()
                    && inputs.insert(key.clone(), json!(true)).is_some()
                {
                    return Err("reserved scene input identity".into());
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

impl Node {
    pub fn dialog(&self, key: &str) -> Option<&Node> {
        match self {
            Self::Dialog {
                key: own,
                background,
                content,
                ..
            } => {
                if own == key {
                    Some(self)
                } else {
                    background.dialog(key).or_else(|| content.dialog(key))
                }
            }
            Self::Stack { children, .. } => children.iter().find_map(|node| node.dialog(key)),
            _ => None,
        }
    }
}
