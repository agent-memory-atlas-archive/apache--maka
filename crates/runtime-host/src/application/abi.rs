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

//! Actual JavaScript service activation and request-context ABI.
use maka_plugin_core::contract::{Contract, ResourceType};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HostActivation {
    pub contract: Contract,
    pub service: Vec<String>,
}

/// Resource names select slots in the current request, not durable grants.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
pub enum HostRequest {
    Call {
        import: String,
        provider: usize,
        method: String,
        input: Value,
        attachments: BTreeMap<String, String>,
    },
    Metadata {
        slot: String,
    },
    Read {
        slot: String,
        start: usize,
        end: usize,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceMetadata {
    #[serde(rename = "type")]
    pub resource_type: ResourceType,
    pub bytes: usize,
    pub metadata: Value,
}
