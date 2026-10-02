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

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// One callback's result. Events are finite transaction intents, never retained
/// in a committed Frame or replayed during rendering.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComponentOutput<V = Node> {
    pub frame: Frame<V>,
    pub events: Vec<Emission>,
    pub actions: Vec<super::actions::Action>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Emission {
    pub name: String,
    pub value: Value,
}
impl<V> ComponentOutput<V> {
    pub fn map_view<T>(self, map: impl FnOnce(V) -> T) -> ComponentOutput<T> {
        ComponentOutput {
            frame: self.frame.map_view(map),
            events: self.events,
            actions: self.actions,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Frame<V = Node> {
    pub revision: u64,
    pub state: Value,
    pub window: Option<Window>,
    pub view: V,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub surface: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Row,
    Column,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
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
    Component {
        key: String,
        binding: String,
        member: u32,
        props: Value,
        on: BTreeMap<String, String>,
    },
}
impl<V> Frame<V> {
    pub fn output(self, events: Vec<Emission>) -> ComponentOutput<V> {
        ComponentOutput {
            frame: self,
            events,
            actions: Vec::new(),
        }
    }
    pub fn validate_metadata(&self) -> Result<(), String> {
        if self.revision > 9_007_199_254_740_991
            || self.window.as_ref().is_some_and(|window| {
                window.title.len() > 512
                    || window.surface.is_empty()
                    || window.surface.len() > 128
                    || !(64..=16000).contains(&window.width)
                    || !(64..=16000).contains(&window.height)
            })
        {
            return Err("invalid window/revision".into());
        }
        Ok(())
    }
    pub fn map_view<T>(self, map: impl FnOnce(V) -> T) -> Frame<T> {
        Frame {
            revision: self.revision,
            state: self.state,
            window: self.window,
            view: map(self.view),
        }
    }
}
impl Frame {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_metadata()?;
        self.view.validate_root()
    }
}
impl Node {
    pub(crate) fn validate_root(&self) -> Result<(), String> {
        self.validate(0, &mut BTreeSet::new())
    }
    fn validate(&self, depth: usize, keys: &mut BTreeSet<String>) -> Result<(), String> {
        let key = match self {
            Self::Stack { key, .. }
            | Self::Text { key, .. }
            | Self::Button { key, .. }
            | Self::Component { key, .. } => key,
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
                    child.validate(depth + 1, keys)?;
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
            Self::Component { binding, on, .. } => {
                if binding.is_empty()
                    || binding.len() > 128
                    || on.len() > 64
                    || on.iter().any(|(event, handler)| {
                        event.is_empty()
                            || event.len() > 128
                            || handler.is_empty()
                            || handler.len() > 128
                    })
                {
                    return Err("invalid component binding".into());
                }
            }
        }
        Ok(())
    }
    pub fn has_action(&self, key: &str) -> bool {
        match self {
            Self::Button { key: actual, .. } => actual == key,
            Self::Stack { children, .. } => children.iter().any(|child| child.has_action(key)),
            _ => false,
        }
    }

    pub(crate) fn components<'a>(&'a self, output: &mut Vec<ComponentRef<'a>>) {
        match self {
            Self::Component {
                key,
                binding,
                member,
                props,
                on,
            } => output.push(ComponentRef {
                key,
                binding,
                member: *member,
                props,
                on,
            }),
            Self::Stack { children, .. } => {
                for child in children {
                    child.components(output);
                }
            }
            _ => {}
        }
    }
}

pub(crate) struct ComponentRef<'a> {
    pub key: &'a str,
    pub binding: &'a str,
    pub member: u32,
    pub props: &'a Value,
    pub on: &'a BTreeMap<String, String>,
}
