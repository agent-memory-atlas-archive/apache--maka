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

//! Native editor protocol. Snapshots are observations, never writable buffers.
use maka_plugin_core::contract::{Contract, Interface, Transport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorProps {
    #[serde(default)]
    #[schemars(length(max = 65536))]
    pub initial_text: String,
    #[serde(default)]
    #[schemars(length(max = 512))]
    pub placeholder: String,
    #[serde(default)]
    pub command: Option<EditorCommand>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorCommand {
    #[schemars(range(min = 1, max = 9007199254740991u64))]
    pub sequence: u64,
    #[schemars(range(max = 9007199254740991u64))]
    pub based_on: u64,
    pub edit: EditorEdit,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EditorEdit {
    Replace {
        #[schemars(length(max = 8192))]
        text: String,
    },
    Reset {
        #[schemars(length(max = 8192))]
        text: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditorTextRef {
    pub owner: String,
    pub revision: u64,
    pub bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditorSnapshot {
    pub text: EditorTextRef,
    pub selection: TextRange,
    pub reversed: bool,
    pub marked: Option<TextRange>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EditorConflict {
    Revision,
    Composition,
    Sequence,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum EditorOutcome {
    Applied,
    Conflict { reason: EditorConflict },
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditorReceipt {
    pub sequence: u64,
    pub revision: u64,
    pub outcome: EditorOutcome,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditorObservation {
    pub revision: u64,
    pub snapshot: EditorSnapshot,
    pub receipt: Option<EditorReceipt>,
}
pub fn editor_contract() -> Contract {
    Contract {
        name: "maka.ui.Editor".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Component {
            props: serde_json::to_value(schemars::schema_for!(EditorProps))
                .expect("editor props schema"),
            events: BTreeMap::new(),
        },
    }
}

pub fn observation_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(EditorObservation))
        .expect("editor observation schema")
}

#[derive(Debug, thiserror::Error)]
pub enum EditorReadError {
    #[error("editor resource is closed or belongs to another owner")]
    Owner,
    #[error("editor revision changed")]
    Revision,
    #[error("invalid editor UTF-8 byte range or range exceeds 64 KiB")]
    Range,
}
