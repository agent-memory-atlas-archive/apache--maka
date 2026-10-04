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
use maka_plugins::session::history::{Cursor, Role};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub use maka_runtime::configuration::ModelSelection;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentConfiguration")]
pub struct Configuration {
    #[schemars(range(min = 1, max = 16))]
    pub max_active_runs: usize,
    #[schemars(range(min = 1, max = 16))]
    pub max_steps: usize,
}
pub fn configuration_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(Configuration)).expect("configuration schema")
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentList")]
pub struct List {
    pub revision: Option<String>,
    #[schemars(length(min = 1, max = 128))]
    pub cursor: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "AgentCreate")]
pub struct Create {
    #[schemars(length(min = 1, max = 128))]
    pub title: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentSend")]
pub struct Send {
    pub model: ModelSelection,
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(length(min = 1, max = 65536))]
    pub message: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentStop")]
pub struct Stop {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(length(min = 1, max = 128))]
    pub invocation_id: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentWatch")]
pub struct Watch {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub after: u64,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentSession")]
pub struct Session {
    pub id: String,
    pub title: String,
    pub revision: u64,
    pub updated_at: u64,
    pub archived: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentSessionPage")]
pub struct SessionPage {
    pub revision: String,
    pub sessions: Vec<Session>,
    pub next_cursor: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentStarted")]
pub struct Started {
    pub session_id: String,
    pub invocation_id: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentStopped")]
pub struct Stopped {
    pub requested: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentDelivery")]
pub struct Delivery {
    pub sequence: u64,
    pub invocation_id: String,
    pub fact: DeliveryFact,
}
#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentDeliveryFact")]
pub enum DeliveryFact {
    Opened,
    /// Canonical transcript facts changed; clients query bounded history.
    TranscriptChanged,
    StepEnded {
        step_id: String,
    },
    PartStarted {
        message_id: String,
        step_id: String,
        part_id: String,
        text_kind: ContentKind,
    },
    Delta {
        step_id: String,
        part_id: String,
        text: String,
    },
    PartFinished {
        step_id: String,
        part_id: String,
    },
    Ended {
        failed: bool,
    },
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentContentKind")]
pub enum ContentKind {
    Text,
    Thinking,
}
impl From<maka_runtime::model::TextKind> for ContentKind {
    fn from(value: maka_runtime::model::TextKind) -> Self {
        match value {
            maka_runtime::model::TextKind::Text => Self::Text,
            maka_runtime::model::TextKind::Thinking => Self::Thinking,
        }
    }
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentSessionRead")]
pub struct SessionRead {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentObservation")]
pub struct Observation {
    pub session: Session,
    pub through: u64,
    pub run: Option<Run>,
    pub parts: Vec<Part>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentRun")]
pub struct Run {
    pub invocation_id: String,
    pub state: RunState,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentRunState")]
pub enum RunState {
    Admitted,
    Running,
    WaitingForUser,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentPart")]
pub struct Part {
    pub start_sequence: u64,
    pub message_id: String,
    pub step_id: String,
    pub part_id: String,
    pub text_kind: ContentKind,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentDeliveryPage")]
pub struct DeliveryPage {
    pub through: u64,
    pub next_after: u64,
    pub events: Vec<Delivery>,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentHistoryChunk")]
pub struct HistoryChunk {
    pub message_id: String,
    pub turn_id: String,
    pub timestamp: u64,
    pub role: Role,
    pub sequence: u64,
    pub offset: u64,
    pub total_bytes: u64,
    pub text: String,
}
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentHistoryRead")]
pub struct History {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub through: Option<u64>,
    pub position: HistoryPosition,
}
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "AgentHistoryPosition")]
pub enum HistoryPosition {
    Start,
    /// Inclusive event-log sequence. The service maps to its sparse transcript rows.
    Event {
        #[schemars(range(min = 1, max = 9_007_199_254_740_991u64))]
        sequence: u64,
    },
    Cursor {
        cursor: Cursor,
    },
}

#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentHistoryPage")]
pub enum HistoryPage {
    Preparing {
        through: u64,
    },
    Ready {
        through: u64,
        chunks: Vec<HistoryChunk>,
        next: Option<Cursor>,
    },
}

pub fn contract() -> Contract {
    fn method<I: JsonSchema, O: JsonSchema>(mode: MethodMode) -> Method {
        Method {
            mode,
            input: serde_json::to_value(schemars::schema_for!(I)).expect("input schema"),
            output: serde_json::to_value(schemars::schema_for!(O)).expect("output schema"),
            resources: BTreeMap::new(),
        }
    }
    Contract {
        name: "maka.agent.Agent".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([
                (
                    "list".into(),
                    method::<List, SessionPage>(MethodMode::Query),
                ),
                (
                    "observe".into(),
                    method::<SessionRead, Observation>(MethodMode::Query),
                ),
                (
                    "history".into(),
                    method::<History, HistoryPage>(MethodMode::Query),
                ),
                (
                    "create".into(),
                    method::<Create, Session>(MethodMode::Mutation),
                ),
                // Authorizes the entire bounded run against the immutable selected
                // account/model. A started reply is not a terminal business verdict.
                ("send".into(), method::<Send, Started>(MethodMode::Mutation)),
                ("stop".into(), method::<Stop, Stopped>(MethodMode::Mutation)),
                (
                    "watch".into(),
                    method::<Watch, DeliveryPage>(MethodMode::Subscription),
                ),
            ]),
        },
    }
}
