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

//! Persistent coordinator discovery; model and worker authority remain explicit.
use maka_plugin_core::contract::{Contract, Interface, Method, MethodMode, Transport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "WorkHubConfiguration")]
pub struct Configuration {
    pub agent: crate::Configuration,
    /// Exactly selected imports of Agent services. Native grants are independent.
    #[schemars(length(max = 8), extend("uniqueItems" = true))]
    pub workers: Vec<String>,
}
pub fn configuration_schema() -> serde_json::Value {
    schemars::schema_for!(Configuration).into()
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "WorkHubTasksRead")]
pub struct TasksRead {
    pub after: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "WorkHubTasks")]
pub struct Tasks {
    #[schemars(length(max = 16))]
    pub entries: Vec<TaskEntry>,
    pub after: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "WorkHubTaskEntry")]
pub struct TaskEntry {
    pub task_id: String,
    pub worker: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "WorkHubTaskRead")]
pub struct TaskRead {
    #[schemars(regex(pattern = "^wt_[a-f0-9]{64}$"))]
    pub task_id: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "WorkHubTask")]
pub struct Task {
    pub task_id: String,
    pub worker: String,
    pub operation: uuid::Uuid,
    pub status: TaskStatus,
}
#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "WorkHubTaskStatus")]
pub enum TaskStatus {
    Pending,
    Uncertain,
    Unavailable,
    Rejected {
        reason: String,
    },
    Started {
        session_id: String,
        invocation_id: String,
        progress: Option<crate::contract::ExecutionResult>,
    },
    Queued {
        session_id: String,
        message_id: String,
        progress: Option<crate::contract::MessageExecution>,
    },
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "WorkHubResolve")]
pub struct Resolve {
    /// Used only when freezing the first coordinator creation intent.
    pub workspace: crate::contract::DirectorySelection,
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "WorkHubCoordinator")]
pub enum Coordinator {
    Missing,
    Pending,
    Ready { session: crate::contract::Session },
    Rejected { reason: crate::contract::Rejection },
}
pub fn contract() -> Contract {
    Contract {
        name: "maka.workhub.Coordinator".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([
                (
                    "tasks".into(),
                    Method {
                        mode: MethodMode::Query,
                        input: schemars::schema_for!(TasksRead).into(),
                        output: schemars::schema_for!(Tasks).into(),
                        resources: BTreeMap::new(),
                    },
                ),
                (
                    "task".into(),
                    Method {
                        mode: MethodMode::Query,
                        input: schemars::schema_for!(TaskRead).into(),
                        output: schemars::schema_for!(Task).into(),
                        resources: BTreeMap::new(),
                    },
                ),
                (
                    "coordinator".into(),
                    Method {
                        mode: MethodMode::Query,
                        input: serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
                        output: schemars::schema_for!(Coordinator).into(),
                        resources: BTreeMap::new(),
                    },
                ),
                (
                    "resolve".into(),
                    Method {
                        mode: MethodMode::Mutation,
                        input: schemars::schema_for!(Resolve).into(),
                        output: schemars::schema_for!(Coordinator).into(),
                        resources: BTreeMap::new(),
                    },
                ),
            ]),
        },
    }
}
