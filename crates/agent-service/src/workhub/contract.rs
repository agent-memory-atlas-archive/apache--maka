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
