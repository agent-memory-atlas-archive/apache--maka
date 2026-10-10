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

//! Native reading evidence and commands. Layout remains the selected scene language.
use maka_plugin_core::contract::{Capture, CaptureOutput, Contract, Interface, Transport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadingProps {
    /// The presenter's scene language; each requires a scrolling column Stack.
    /// Its scroll policy and layout are the only presentation authority.
    pub content: Value,
    /// Omission retains an accepted command; numbers increase per original mount.
    pub command: Option<ReadingCommand>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadingCommand {
    #[schemars(range(min = 1))]
    pub sequence: u64,
    pub target: ReadingTarget,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReadingTarget {
    Start,
    End,
    /// Find over native readable descendants, including folded slots. SourceText uses
    /// original literal bytes; Markdown uses its width-independent rendered text.
    Find {
        #[schemars(length(max = 512))]
        query: String,
        step: FindStep,
    },
    ClearFind,
    /// Move the original terminal viewport by physical rows.
    ScrollRows {
        #[schemars(range(min = -65536, max = 65536))]
        rows: i32,
    },
    /// Reveal a UTF-8 byte in the descendant's native readable text, as returned by Find.
    RevealSource {
        #[schemars(length(min = 1, max = 32))]
        path: Vec<String>,
        offset: u32,
        /// If supplied, reveal only while this accepted find retains its original physical intent.
        #[serde(default)]
        find_sequence: Option<u64>,
    },
    RevealChild {
        #[schemars(length(min = 1, max = 128))]
        key: String,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FindStep {
    First,
    Next,
    Previous,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadingMatch {
    #[schemars(length(min = 1, max = 32))]
    pub path: Vec<String>,
    pub offset: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadingFind {
    pub sequence: u64,
    #[schemars(range(max = 4096))]
    pub count: u32,
    pub limited: bool,
    /// One-based current match, or null when no match remains.
    pub index: Option<u32>,
    pub active: Option<ReadingMatch>,
}
impl ReadingCommand {
    pub fn valid_target(&self) -> bool {
        match &self.target {
            ReadingTarget::Find { query, .. } => query.len() <= 512,
            ReadingTarget::ScrollRows { rows } => (-65536..=65536).contains(rows),
            ReadingTarget::RevealSource {
                path,
                offset,
                find_sequence,
            } => {
                find_sequence.is_none_or(|sequence| sequence > 0)
                    && !path.is_empty()
                    && path.len() <= 32
                    && *offset <= 65536
                    && path.iter().all(|key| !key.is_empty() && key.len() <= 128)
            }
            _ => true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReadingStatus {
    Applied,
    Cancelled,
    Rejected,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadingReceipt {
    pub sequence: u64,
    pub status: ReadingStatus,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadingViewport {
    /// Explicit reading intent used by the actually painted children.
    pub reading_revision: u64,
    pub at_end: bool,
    #[schemars(length(max = 4096))]
    pub visible_child_keys: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadingObservation {
    /// Immediate native intent; can be newer than the painted viewport.
    pub reading_revision: u64,
    pub viewport: Option<ReadingViewport>,
    /// Direct content child containing the native selection focus. Renderers
    /// without this observation return null. This key conveys no read authority.
    #[schemars(length(max = 256))]
    pub selected_child_key: Option<String>,
    pub pending_command: Option<u64>,
    /// Only the most recently settled command is retained, without a ledger.
    pub receipt: Option<ReadingReceipt>,
    pub find: Option<ReadingFind>,
}
pub fn observation_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(ReadingObservation)).expect("reading observation")
}
pub fn reading_contract() -> Contract {
    Contract {
        name: "maka.ui.ReadingArea".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Component {
            props: serde_json::to_value(schemars::schema_for!(ReadingProps))
                .expect("reading props"),
            events: BTreeMap::from([
                ("observed".into(), observation_schema()),
                (
                    "input".into(),
                    serde_json::to_value(schemars::schema_for!(crate::abi::ComponentInput))
                        .expect("reading input"),
                ),
            ]),
            captures: BTreeMap::from([(
                "tail".into(),
                Capture {
                    input: serde_json::json!({"type":"null"}),
                    output: CaptureOutput::Continuation,
                },
            )]),
        },
    }
}
