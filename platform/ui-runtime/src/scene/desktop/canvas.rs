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

//! Desktop canvas wire drawing and input declarations.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "DesktopCanvasPoint")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    fn validate(self) -> Result<(), String> {
        if [self.x, self.y]
            .into_iter()
            .all(|v| v.is_finite() && (-16000.0..=16000.0).contains(&v))
        {
            Ok(())
        } else {
            Err("invalid canvas position".into())
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "DesktopCanvasRect")]
pub struct Rect {
    pub origin: Point,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    fn validate(self) -> Result<(), String> {
        self.origin.validate()?;
        if [self.width, self.height]
            .into_iter()
            .all(|v| v.is_finite() && (0.0..=16000.0).contains(&v))
        {
            Ok(())
        } else {
            Err("invalid canvas extent".into())
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "DesktopCanvasCommand")]
pub enum Command {
    Rect {
        rect: Rect,
        color: u32,
        #[serde(default)]
        radius: f64,
    },
    Line {
        from: Point,
        to: Point,
        color: u32,
        width: f64,
    },
    Text {
        position: Point,
        text: String,
        color: u32,
        size: f64,
        #[serde(default)]
        font: Option<String>,
    },
}
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "DesktopCanvasEventKind")]
pub enum EventKind {
    PointerDown,
    DragMove,
    Drop,
    KeyDown,
    Activate,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "DesktopCanvasRegion")]
pub struct Region {
    pub key: String,
    pub rect: Rect,
    /// Corner radius of mechanical outlines; the hit area remains `rect`.
    #[serde(default)]
    pub radius: f64,
    pub label: String,
    #[serde(default)]
    pub draggable: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub tab_index: i16,
    #[serde(default)]
    pub listen: BTreeSet<EventKind>,
    /// These defaults are decided from the committed region before guest input.
    #[serde(default = "native_default")]
    pub focus_on_pointer: bool,
    #[serde(default = "native_default")]
    pub drag_preview: bool,
    #[serde(default = "native_default")]
    pub tab_navigation: bool,
    #[serde(default = "native_default")]
    pub native_copy: bool,
    #[serde(default = "native_default")]
    pub bubble_input: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "DesktopCanvasContents")]
pub struct Contents {
    pub commands: Vec<Command>,
    pub regions: Vec<Region>,
    pub focus_color: Option<u32>,
    pub drag_color: Option<u32>,
}
fn native_default() -> bool {
    true
}
impl Contents {
    pub fn preflight(source: &Value, work: &mut usize) -> Result<(), String> {
        let mut text_bytes = 0;
        for (field, limit) in [("commands", 1024), ("regions", 256)] {
            let Some(items) = source.get(field).and_then(Value::as_array) else {
                continue;
            };
            *work += items.len();
            if items.len() > limit || *work > 4096 {
                return Err("canvas work limit".into());
            }
            for item in items {
                for (name, max) in [("text", 4096), ("label", 4096), ("key", 128), ("font", 128)] {
                    if item
                        .get(name)
                        .and_then(Value::as_str)
                        .is_some_and(|s| s.len() > max)
                    {
                        return Err("canvas string limit".into());
                    }
                }
                text_bytes += item.get("text").and_then(Value::as_str).map_or(0, str::len);
                if text_bytes > 64 * 1024 {
                    return Err("canvas text limit".into());
                }
                if item
                    .get("listen")
                    .and_then(Value::as_array)
                    .is_some_and(|s| s.len() > 5)
                {
                    return Err("canvas subscription limit".into());
                }
            }
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.commands.len() > 1024 || self.regions.len() > 256 {
            return Err("canvas work limit".into());
        }
        let mut text_bytes = 0;
        for command in &self.commands {
            match command {
                Command::Rect { rect, radius, .. } => {
                    rect.validate()?;
                    if !radius.is_finite() || !(0.0..=8000.0).contains(radius) {
                        return Err("invalid canvas radius".into());
                    }
                }
                Command::Line {
                    from, to, width, ..
                } => {
                    from.validate()?;
                    to.validate()?;
                    if !width.is_finite() || !(0.0..=128.0).contains(width) {
                        return Err("invalid canvas stroke".into());
                    }
                }
                Command::Text {
                    position,
                    text,
                    size,
                    font,
                    ..
                } => {
                    position.validate()?;
                    text_bytes += text.len();
                    if text.len() > 4096
                        || text_bytes > 64 * 1024
                        || text.contains(['\n', '\r'])
                        || !size.is_finite()
                        || !(1.0..=256.0).contains(size)
                        || font.as_ref().is_some_and(|s| s.is_empty() || s.len() > 128)
                    {
                        return Err("invalid canvas text".into());
                    }
                }
            }
        }
        let mut keys = BTreeSet::new();
        for region in &self.regions {
            region.rect.validate()?;
            if !region.radius.is_finite() || !(0.0..=8000.0).contains(&region.radius) {
                return Err("invalid canvas region radius".into());
            }
            if region.key.is_empty()
                || region.key.len() > 128
                || !keys.insert(&region.key)
                || region.label.len() > 4096
                || region.tab_index < -1
            {
                return Err("invalid canvas region".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "DesktopCanvasModifiers")]
pub struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub platform: bool,
    pub function: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "DesktopCanvasInput")]
pub enum CanvasInput {
    PointerDown {
        region: String,
        position: Point,
        gesture: Option<u32>,
    },
    DragMove {
        region: String,
        gesture: u32,
        start: Point,
        position: Point,
        delta: Point,
    },
    Drop {
        region: String,
        gesture: u32,
        start: Point,
        position: Point,
        delta: Point,
        target: Option<String>,
    },
    KeyDown {
        region: String,
        key: String,
        modifiers: Modifiers,
    },
    Activate {
        region: String,
    },
}
