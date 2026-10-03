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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelSelection {
    #[schemars(length(min = 1, max = 128))]
    pub connection_id: String,
    #[schemars(length(min = 1, max = 128))]
    pub connection_slug: String,
    #[schemars(length(min = 1, max = 512))]
    pub model: String,
}
impl ModelSelection {
    pub(crate) fn binding(&self) -> maka_runtime::execution::ModelBinding {
        maka_runtime::execution::ModelBinding {
            connection_id: self.connection_id.clone(),
            connection_slug: self.connection_slug.clone(),
            model: self.model.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Configuration {
    pub model: ModelSelection,
    /// Compiled API provider; never an endpoint, credential or mutable fallback.
    #[schemars(length(min = 1, max = 128))]
    pub provider: String,
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
pub(crate) struct List {
    pub revision: Option<String>,
    #[schemars(length(min = 1, max = 128))]
    pub cursor: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Create {
    #[schemars(length(min = 1, max = 128))]
    pub title: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Send {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(length(min = 1, max = 65536))]
    pub message: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Stop {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(length(min = 1, max = 128))]
    pub invocation_id: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Watch {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub after: u64,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Session {
    pub id: String,
    pub title: String,
    pub revision: u64,
    pub updated_at: u64,
    pub archived: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionPage {
    pub revision: String,
    pub sessions: Vec<Session>,
    pub next_cursor: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Started {
    pub session_id: String,
    pub invocation_id: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Stopped {
    pub requested: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Delivery {
    pub sequence: u64,
    pub invocation_id: String,
    pub kind: DeliveryKind,
    pub text: Option<String>,
    pub part_id: Option<String>,
    pub failed: Option<bool>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeliveryKind {
    Opened,
    PartStarted,
    Delta,
    PartFinished,
    Ended,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DeliveryPage {
    pub through: u64,
    pub next_after: u64,
    pub events: Vec<Delivery>,
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
