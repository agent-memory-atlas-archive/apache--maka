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
    RevealChild {
        #[schemars(length(min = 1, max = 128))]
        key: String,
    },
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
    pub pending_command: Option<u64>,
    /// Only the most recently settled command is retained, without a ledger.
    pub receipt: Option<ReadingReceipt>,
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
