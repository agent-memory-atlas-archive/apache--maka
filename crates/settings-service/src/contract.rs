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
use maka_runtime::configuration::{
    CatalogMutationResult, ModelSelection, SetDefaultConnectionTargetInput,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "SettingsModelsRead")]
pub enum ModelsRead {
    Start,
    Continue {
        #[schemars(range(max = 9_007_199_254_740_991u64))]
        revision: u64,
        cursor: ModelCursor,
    },
}
#[derive(Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "SettingsModelCursor")]
pub struct ModelCursor {
    #[schemars(range(max = 1024))]
    pub connection: usize,
    #[schemars(range(max = 4096))]
    pub model: usize,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "SettingsModelChoice")]
pub struct ModelChoice {
    pub selection: ModelSelection,
    #[schemars(length(max = 256))]
    pub connection_name: String,
    #[schemars(length(max = 512))]
    pub label: String,
    pub is_default: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "SettingsModelsPage")]
pub enum ModelsPage {
    Changed {
        revision: u64,
    },
    Ready {
        revision: u64,
        #[schemars(length(max = 16))]
        choices: Vec<ModelChoice>,
        #[schemars(with = "maka_runtime::schema::RequiredNullable<ModelCursor>")]
        next: Option<ModelCursor>,
    },
}
pub fn contract() -> Contract {
    fn schema<T: JsonSchema>() -> serde_json::Value {
        let settings = schemars::generate::SchemaSettings::default()
            .with(|settings| settings.contract = schemars::generate::Contract::Serialize);
        serde_json::to_value(settings.into_generator().into_root_schema_for::<T>())
            .expect("settings schema")
    }
    fn method<I: JsonSchema, O: JsonSchema>(mode: MethodMode) -> Method {
        Method {
            mode,
            input: schema::<I>(),
            output: schema::<O>(),
            resources: BTreeMap::new(),
        }
    }
    Contract {
        name: "maka.settings.Settings".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([
                (
                    "models".into(),
                    method::<ModelsRead, ModelsPage>(MethodMode::Query),
                ),
                (
                    "setDefault".into(),
                    method::<SetDefaultConnectionTargetInput, CatalogMutationResult>(
                        MethodMode::Mutation,
                    ),
                ),
            ]),
        },
    }
}
