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

//! JavaScript component ABI. These are the actual activation and input values.
use maka_plugin_core::contract::Contract;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Actual exports from a local JavaScript entry. Values are immutable for this
/// activation; component paths are captured before publication.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UiExport {
    Component {
        contract: Contract,
        component: Vec<String>,
    },
    Value {
        contract: Contract,
        value: Value,
    },
}

/// Read-only data copied from this activation's captured value imports.
/// The keys and member order come from its checked plan, never guest routing.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct ValueBindings(pub std::collections::BTreeMap<String, Vec<Value>>);
impl ValueBindings {
    pub fn get(&self, import: &str) -> Option<&[Value]> {
        self.0.get(import).map(Vec::as_slice)
    }
}

/// Input origin is assigned by the application, never by plugin event payloads.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComponentInput {
    Raw {
        key: String,
        event: Value,
    },
    Action {
        key: String,
        event: Value,
    },
    Component {
        key: String,
        child: String,
        name: String,
        event: Value,
    },
}
