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

//! Read-only source differences; content carries display evidence, never file authority.
use crate::appearance::{Color, TextAppearance};
use maka_plugin_core::contract::{Contract, Interface, Transport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    Removed,
    Added,
    Context,
    Content,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiffRow {
    /// Half-open UTF-8 byte range in source. Rows are ordered and disjoint.
    #[schemars(range(max = 65536))]
    pub start: u32,
    #[schemars(range(max = 65536))]
    pub end: u32,
    pub kind: DiffKind,
    #[schemars(length(max = 128))]
    pub language: Option<String>,
}
/// Complete bounded comparisons and structured patches reuse native parsers.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiffContent {
    Annotated {
        #[schemars(length(max = 65536))]
        source: String,
        #[schemars(length(max = 8192))]
        rows: Vec<DiffRow>,
    },
    /// Combined inputs must be <= 8192 UTF-8 bytes and <= 256 lines, without NUL.
    Comparison {
        #[schemars(length(max = 8192))]
        before: String,
        #[schemars(length(max = 8192))]
        after: String,
        #[schemars(length(max = 128))]
        language: Option<String>,
    },
    /// Invalid, binary or oversized patches display literally, without diff gutters.
    Patch {
        #[schemars(length(max = 65536))]
        source: String,
        operation: DiffPatch,
        #[schemars(length(max = 128))]
        language: Option<String>,
    },
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiffPatch {
    Create,
    Update,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiffAppearance {
    #[serde(default)]
    pub terminal: Option<crate::scene::terminal::Palette>,
    pub text: TextAppearance,
    pub background: Color,
    pub added: Color,
    pub removed: Color,
    pub selection: Color,
    pub dark: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiffProps {
    pub content: DiffContent,
    pub appearance: DiffAppearance,
    pub ascii: bool,
    pub disabled: bool,
    pub focus_on_pointer: bool,
}
pub fn diff_contract() -> Contract {
    Contract {
        name: "maka.ui.Diff".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Component {
            props: serde_json::to_value(schemars::schema_for!(DiffProps)).expect("Diff props"),
            events: BTreeMap::new(),
            captures: BTreeMap::new(),
        },
    }
}
