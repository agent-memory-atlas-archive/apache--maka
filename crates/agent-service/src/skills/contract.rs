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

use maka_plugin_core::contract::{Contract, Interface, Method, MethodMode, Transport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Installation-selected skill collections. No ambient home or workspace scan.
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "SkillLibraryConfiguration")]
pub struct Configuration {
    #[schemars(length(max = 8))]
    pub sources: Vec<Source>,
}
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "SkillLibrarySource")]
pub struct Source {
    #[schemars(regex(pattern = "^[a-z0-9][a-z0-9_-]{0,63}$"))]
    pub id: String,
    #[schemars(length(min = 1, max = 128))]
    pub label: String,
    /// Absolute directory containing skill directories. Captured at activation.
    #[schemars(length(min = 1, max = 4096))]
    pub path: String,
}
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "SkillLibraryCursor")]
pub struct Cursor {
    #[schemars(length(min = 1, max = 128))]
    pub revision: String,
    #[schemars(length(max = 256))]
    pub query: String,
    #[schemars(range(max = 16384))]
    pub offset: usize,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "SkillLibraryList")]
pub struct List {
    #[schemars(length(max = 256))]
    pub query: String,
    pub cursor: Option<Cursor>,
}
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "SkillLibraryEntry")]
pub struct Entry {
    pub reference: String,
    pub source: String,
    pub name: String,
    pub description: String,
    pub valid: bool,
    pub shadowed_by: Option<String>,
    pub metadata_truncated: bool,
    pub issues: Vec<String>,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "SkillLibraryPage")]
pub enum Page {
    Page {
        revision: String,
        entries: Vec<Entry>,
        after: Option<Cursor>,
        diagnostics: Vec<String>,
    },
    Changed,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "SkillLibraryRead")]
pub struct Read {
    #[schemars(length(min = 1, max = 512))]
    pub reference: String,
    /// Complete catalog revision from the list/read response.
    #[schemars(length(min = 1, max = 128))]
    pub revision: String,
    #[schemars(range(max = 16777216))]
    pub offset: usize,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "SkillLibraryDocument")]
pub enum Document {
    Page {
        entry: Box<Entry>,
        revision: String,
        text: String,
        offset: usize,
        before: Option<usize>,
        after: Option<usize>,
        total_bytes: usize,
    },
    Changed,
    Unavailable,
}
pub fn configuration_schema() -> Value {
    schemars::schema_for!(Configuration).into()
}
pub fn contract() -> Contract {
    Contract {
        name: "maka.skills.Library".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([
                (
                    "list".into(),
                    Method {
                        mode: MethodMode::Query,
                        input: schemars::schema_for!(List).into(),
                        output: schemars::schema_for!(Page).into(),
                        resources: BTreeMap::new(),
                    },
                ),
                (
                    "read".into(),
                    Method {
                        mode: MethodMode::Query,
                        input: schemars::schema_for!(Read).into(),
                        output: schemars::schema_for!(Document).into(),
                        resources: BTreeMap::new(),
                    },
                ),
            ]),
        },
    }
}
