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
    /// Enable installation into this instance's own private collection.
    #[serde(default)]
    pub manage: bool,
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
    pub installed: bool,
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
        writable: bool,
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
        writable: bool,
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

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "SkillLibraryChangeKind")]
pub enum ChangeKind {
    Install,
    Update,
    Delete,
}
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "SkillLibraryChange")]
pub struct Change {
    pub kind: ChangeKind,
    #[schemars(length(min = 1, max = 512))]
    pub reference: String,
    #[schemars(length(min = 1, max = 128))]
    pub revision: String,
}
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "SkillLibraryReview")]
pub struct Review {
    pub change: Change,
    #[schemars(length(min = 71, max = 71))]
    pub digest: String,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "SkillLibraryPreview")]
pub enum Preview {
    Ready {
        review: Review,
        target: String,
        entries: usize,
        paths: Vec<String>,
        paths_truncated: bool,
        locally_modified: bool,
    },
    Changed,
    Rejected {
        reason: String,
    },
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "SkillLibraryChangeResult")]
pub enum ChangeResult {
    Applied { reference: String },
    Changed,
    Rejected { reason: String },
}
/// Canonical publication evidence alongside the unmodified original Host facts.
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "SkillLibraryOutcome")]
pub struct Outcome {
    pub original: maka_plugin_core::operation::OperationReply,
    pub published: Option<ChangeResult>,
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
                    "outcome".into(),
                    Method {
                        mode: MethodMode::Query,
                        input: schemars::schema_for!(maka_plugin_core::operation::RecoveryRef)
                            .into(),
                        output: schemars::schema_for!(Outcome).into(),
                        resources: BTreeMap::new(),
                    },
                ),
                (
                    "preview".into(),
                    Method {
                        mode: MethodMode::Query,
                        input: schemars::schema_for!(Change).into(),
                        output: schemars::schema_for!(Preview).into(),
                        resources: BTreeMap::new(),
                    },
                ),
                (
                    "apply".into(),
                    Method {
                        mode: MethodMode::Mutation,
                        input: schemars::schema_for!(Review).into(),
                        output: schemars::schema_for!(ChangeResult).into(),
                        resources: BTreeMap::new(),
                    },
                ),
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
