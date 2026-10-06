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

use maka_ui::{ComponentSlot, NativeView};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::{rc::Rc, time::Instant};

/// The nominal ABI supplied by this presenter's pure scene compiler.
pub fn language() -> maka_plugin_core::package::SceneLanguage {
    maka_plugin_core::package::SceneLanguage {
        name: "maka.tui.scene".into(),
        version: 1,
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "TerminalDirection")]
pub enum Direction {
    Row,
    Column,
}
/// Native terminal cell constraints on a child in its parent's main axis.
/// `grow` distributes remaining cells after fixed/intrinsic siblings; `basis`
/// supplies an explicit base instead of intrinsic length. Row wrap preserves
/// source order and measures its rows before allocating the parent viewport.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[schemars(rename = "TerminalLayout")]
pub struct Layout {
    #[schemars(range(max = 1024))]
    pub grow: u16,
    pub basis: Option<u16>,
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
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "TerminalScene")]
pub enum Node {
    Stack {
        key: String,
        direction: Direction,
        gap: u16,
        padding: u16,
        background: u32,
        children: Vec<Node>,
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
    },
    Button {
        key: String,
        label: String,
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
    pub(super) fn key(&self) -> &str {
        match self {
            Self::Stack { key, .. } | Self::Text { key, .. } | Self::Button { key, .. } => key,
            Self::Component(slot) => &slot.key,
        }
    }
    pub(super) fn layout(&self) -> Layout {
        match self {
            Self::Stack { layout, .. } => *layout,
            _ => Layout::default(),
        }
    }
    pub(super) fn visible(&self) -> bool {
        !matches!(self, Self::Stack { visible: false, .. })
    }
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
            Self::Stack { key, .. } | Self::Text { key, .. } | Self::Button { key, .. } => key,
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
            Self::Component(slot) => {
                if slots.len() >= 64 {
                    return Err("scene slot limit".into());
                }
                slots.push(slot.clone());
                if !visible {
                    hidden_slots.insert(slot.key.clone());
                }
            }
            Self::Button { key, disabled, .. } => {
                *input_count += 1;
                if *input_count > 64 {
                    return Err("scene input limit".into());
                }
                if visible && !disabled && inputs.insert(key.clone(), json!(true)).is_some() {
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

pub(super) fn compile(
    source: &Value,
    deadline: Instant,
) -> maka_ui::Result<NativeView<dyn super::Widget>> {
    compile_with_inputs(source, deadline, BTreeMap::new())
}
pub(super) fn compile_with_inputs(
    source: &Value,
    deadline: Instant,
    mut inputs: BTreeMap<String, Value>,
) -> maka_ui::Result<NativeView<dyn super::Widget>> {
    if Instant::now() >= deadline {
        return Err(maka_ui::Error::Invalid("terminal scene deadline".into()));
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
        if let Some(children) = source.get("children").and_then(Value::as_array) {
            for child in children {
                scan(child, depth + 1, count, deadline)?;
            }
        }
        Ok(())
    }
    scan(source, 0, &mut 0, deadline)?;
    let node: Node = serde_json::from_value(source.clone())
        .map_err(|e| maka_ui::Error::Invalid(e.to_string()))?;
    node.validate_root(deadline)
        .map_err(maka_ui::Error::Invalid)?;
    let mut slots = vec![];
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
    let visible = node.visible();
    NativeView::new(Rc::new(node) as Rc<dyn super::Widget>, inputs, slots)?
        .with_hidden_slots(hidden_slots)
        .map(|view| view.with_root_visibility(visible))
}
impl super::Widget for Node {
    fn layout(&self) -> Layout {
        Node::layout(self)
    }
    fn create(&self) -> Box<dyn std::any::Any> {
        Box::<super::scroll::SceneState>::default()
    }
    fn commit(&self, state: &mut dyn std::any::Any) -> maka_ui::Result<Option<super::NativeInput>> {
        state
            .downcast_mut::<super::scroll::SceneState>()
            .unwrap()
            .commit(self);
        Ok(None)
    }
    fn input(
        &self,
        state: &mut dyn std::any::Any,
        key: &str,
        event: &crossterm::event::Event,
    ) -> maka_ui::Result<super::WidgetInput> {
        Ok(
            if state
                .downcast_mut::<super::scroll::SceneState>()
                .unwrap()
                .input(key, event)
            {
                super::WidgetInput::Consumed(None)
            } else {
                super::WidgetInput::Unhandled
            },
        )
    }
    fn present(
        &self,
        state: &mut dyn std::any::Any,
        geometry: Box<dyn std::any::Any>,
    ) -> Option<super::NativeInput> {
        state
            .downcast_mut::<super::scroll::SceneState>()
            .unwrap()
            .present(*geometry.downcast::<super::scroll::Geometry>().unwrap());
        None
    }

    fn measure(&self, slots: &super::Slots<'_>, width: u16) -> maka_ui::Result<(u16, u32)> {
        slots
            .presenter
            .measure(slots.parent, self, width, slots.measurements)
    }
    fn paint(
        &self,
        slots: &mut super::Slots<'_>,
        area: super::Placement,
        buffer: &mut ratatui::buffer::Buffer,
    ) -> maka_ui::Result<()> {
        let generation = slots
            .native_state::<super::scroll::SceneState>()?
            .generation;
        let mut viewports = Vec::new();
        slots.presenter.paint_node(
            slots.parent,
            self,
            area,
            buffer,
            slots
                .scene
                .as_deref_mut()
                .ok_or_else(|| maka_ui::Error::Invalid("cannot paint while measuring".into()))?,
            &mut super::paint::NodePaint {
                measurements: slots.measurements,
                viewports: &mut viewports,
            },
        )?;
        slots.native_geometry(Box::new(super::scroll::Geometry {
            generation,
            viewports,
        }))
    }
}
