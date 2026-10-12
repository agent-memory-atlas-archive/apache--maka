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

use maka_ui_runtime::{ComponentSlot, NativeView};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::{rc::Rc, time::Instant};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Row,
    Column,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "UiNode")]
pub enum Node {
    Stack {
        key: String,
        direction: Direction,
        gap: u16,
        padding: u16,
        background: u32,
        children: Vec<Node>,
    },
    Text {
        key: String,
        text: String,
        color: u32,
    },
    Button {
        key: String,
        label: String,
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

pub(crate) fn compile<V: Default + 'static>(
    source: &Value,
    deadline: Instant,
) -> maka_ui_runtime::Result<NativeView<V>> {
    if Instant::now() >= deadline {
        return Err(maka_ui_runtime::Error::Invalid(
            "fixture scene deadline".into(),
        ));
    }
    fn scan(
        source: &Value,
        depth: usize,
        count: &mut usize,
        deadline: Instant,
    ) -> maka_ui_runtime::Result<()> {
        *count += 1;
        if Instant::now() >= deadline || depth > 32 || *count > 4096 {
            return Err(maka_ui_runtime::Error::Invalid(
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
        .map_err(|e| maka_ui_runtime::Error::Invalid(e.to_string()))?;
    node.validate_root(deadline)
        .map_err(maka_ui_runtime::Error::Invalid)?;
    let mut slots = vec![];
    let mut inputs = BTreeMap::new();
    node.metadata(&mut slots, &mut inputs, deadline)
        .map_err(maka_ui_runtime::Error::Invalid)?;
    NativeView::new(Rc::new(V::default()), inputs, slots)?.with_declaration(source.clone())
}
pub(crate) fn view<V: Default + 'static>(node: Node) -> maka_ui_runtime::Result<NativeView<V>> {
    compile(
        &serde_json::to_value(node).unwrap(),
        Instant::now() + std::time::Duration::from_secs(1),
    )
}
pub(crate) fn registry<V: Default + 'static>(
    target: maka_plugin_core::composition::PresentationTarget,
) -> maka_ui_runtime::NativeRegistry<V> {
    maka_ui_runtime::NativeRegistry::with_surface(target)
        .with_scene_compiler(language(target), compile::<V>)
        .expect("valid fixture scene language")
}

// This fixture compiles the common subset used by the two platform languages.
// It is a test adapter, not a kernel language or production fallback.
pub(crate) fn language(
    target: maka_plugin_core::composition::PresentationTarget,
) -> maka_plugin_core::package::SceneLanguage {
    maka_plugin_core::package::SceneLanguage {
        name: match target {
            maka_plugin_core::composition::PresentationTarget::Desktop => "maka.desktop.scene",
            maka_plugin_core::composition::PresentationTarget::Tui => "maka.tui.scene",
        }
        .into(),
        version: 1,
    }
}
