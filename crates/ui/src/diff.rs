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

//! Read-only annotated source; rows carry display ranges, never file authority.
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
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiffAppearance {
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
    #[schemars(length(max = 65536))]
    pub source: String,
    #[schemars(length(max = 8192))]
    pub rows: Vec<DiffRow>,
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
