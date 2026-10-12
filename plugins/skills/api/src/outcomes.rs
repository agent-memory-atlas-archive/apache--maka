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

use maka_plugin_core::{
    contract::{Interface, Method, MethodMode, Transport},
    operation::RecoveryRef,
};
use std::collections::BTreeMap;

pub const IMPLEMENTATION: &str = "maka.skills.outcomes";
pub const EXPORT: &str = "Outcomes";

pub fn contract() -> maka_plugin_core::contract::Contract {
    maka_plugin_core::contract::Contract {
        name: "maka.skills.Outcomes".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([(
                "read".into(),
                Method {
                    progress: None,
                    mode: MethodMode::Query,
                    input: schemars::schema_for!(RecoveryRef).into(),
                    output: schemars::schema_for!(crate::contract::Outcome).into(),
                    resources: BTreeMap::new(),
                },
            )]),
        },
    }
}

pub fn native() -> maka_plugin_core::package::NativeDefinition {
    maka_plugin_core::package::NativeDefinition::new(
        IMPLEMENTATION,
        serde_json::json!({"type":"object","properties":{},"required":[],"additionalProperties":false}),
        [(EXPORT, contract())],
    )
}
