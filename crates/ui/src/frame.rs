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
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub revision: u64,
    pub state: Value,
    pub window: Window,
    pub view: Node,
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
}
impl Frame {
    pub fn validate(&self) -> Result<(), String> {
        if self.window.title.len() > 512
            || self.window.surface.is_empty()
            || self.window.surface.len() > 128
            || !(64..=16000).contains(&self.window.width)
            || !(64..=16000).contains(&self.window.height)
            || self.revision > 9_007_199_254_740_991
        {
            return Err("invalid window/revision".into());
        }
        let mut keys = BTreeSet::new();
        self.view.validate(0, &mut keys)
    }
}
impl Node {
    fn validate(&self, depth: usize, keys: &mut BTreeSet<String>) -> Result<(), String> {
        let key = match self {
            Self::Stack { key, .. } | Self::Text { key, .. } | Self::Button { key, .. } => key,
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
}
