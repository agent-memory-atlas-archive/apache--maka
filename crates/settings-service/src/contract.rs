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
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "SettingsDefaultModel")]
pub enum DefaultModel {
    None,
    Unavailable {
        target: maka_runtime::configuration::ConnectionTarget,
    },
    Ready {
        choice: ModelChoice,
    },
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
        #[serde(rename = "defaultModel")]
        default_model: DefaultModel,
        #[schemars(with = "maka_runtime::schema::RequiredNullable<ModelCursor>")]
        next: Option<ModelCursor>,
    },
}
#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "SettingsConnectionsRead")]
pub enum ConnectionsRead {
    Start,
    Continue {
        revision: u64,
        #[schemars(range(max = 1024))]
        offset: usize,
    },
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "SettingsConnectionSummary")]
pub struct ConnectionSummary {
    pub basis: maka_runtime::configuration::ConnectionVersionBasis,
    pub slug: String,
    pub name: String,
    pub provider: maka_runtime::provider::Identity,
    pub enabled: bool,
    pub enabled_model_count: usize,
}
#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "SettingsConnectionsPage")]
pub enum ConnectionsPage {
    Changed {
        revision: u64,
    },
    Ready {
        revision: u64,
        #[schemars(
            with = "maka_runtime::schema::RequiredNullable<maka_runtime::configuration::ConnectionTarget>"
        )]
        default_target: Option<maka_runtime::configuration::ConnectionTarget>,
        #[schemars(length(max = 16))]
        connections: Vec<ConnectionSummary>,
        #[schemars(with = "maka_runtime::schema::RequiredNullable<usize>")]
        next: Option<usize>,
    },
}
#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "SettingsConnectionDetail")]
pub enum ConnectionDetail {
    Stale {
        #[schemars(
            with = "maka_runtime::schema::RequiredNullable<maka_runtime::configuration::ConnectionVersionBasis>"
        )]
        actual: Option<maka_runtime::configuration::ConnectionVersionBasis>,
    },
    Ready {
        target: Box<maka_runtime::configuration::ConnectionCredentialTarget>,
        name: String,
        enabled: bool,
        enabled_model_count: usize,
    },
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "SettingsProvidersRead")]
pub struct ProvidersRead {
    #[schemars(range(max = 1024))]
    pub offset: usize,
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "SettingsProvider")]
pub struct Provider {
    pub identity: maka_runtime::provider::Identity,
    pub descriptor: maka_plugins::provider::Descriptor,
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "SettingsProvidersPage")]
pub struct ProvidersPage {
    #[schemars(length(max = 8))]
    pub providers: Vec<Provider>,
    #[schemars(with = "maka_runtime::schema::RequiredNullable<usize>")]
    pub next: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "SettingsSaveCredential")]
pub struct SaveCredential {
    pub expected: maka_runtime::configuration::ConnectionCredentialTarget,
    #[serde(deserialize_with = "Option::deserialize")]
    #[schemars(
        with = "maka_runtime::schema::RequiredNullable<maka_runtime::configuration::CredentialVersionBasis>"
    )]
    pub expected_credential: Option<maka_runtime::configuration::CredentialVersionBasis>,
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "SettingsCredentialSaveResult")]
pub enum CredentialSaveResult {
    Saved {
        connection: maka_runtime::oauth::ConnectionIdentity,
    },
    Rejected {
        reason: CredentialRejection,
    },
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "SettingsCredentialRejection")]
pub enum CredentialRejection {
    InvalidInput,
    InvalidSecret,
    SecretUnavailable,
    ProviderUnavailable,
    AuthenticationUnavailable,
    AttemptConflict,
    ConnectionChanged,
    CredentialChanged,
    Capacity,
    SlugTaken,
    Authentication,
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
                    "connections".into(),
                    method::<ConnectionsRead, ConnectionsPage>(MethodMode::Query),
                ),
                (
                    "connection".into(),
                    method::<maka_runtime::configuration::ConnectionVersionBasis, ConnectionDetail>(
                        MethodMode::Query,
                    ),
                ),
                (
                    "providers".into(),
                    method::<ProvidersRead, ProvidersPage>(MethodMode::Query),
                ),
                (
                    "create".into(),
                    method::<
                        maka_runtime::configuration::CreateCatalogConnectionInput,
                        CatalogMutationResult,
                    >(MethodMode::Mutation),
                ),
                (
                    "update".into(),
                    method::<
                        maka_runtime::configuration::UpdateCatalogConnectionInput,
                        CatalogMutationResult,
                    >(MethodMode::Mutation),
                ),
                (
                    "remove".into(),
                    method::<
                        maka_runtime::configuration::RemoveCatalogConnectionInput,
                        CatalogMutationResult,
                    >(MethodMode::Mutation),
                ),
                (
                    "credentialStatus".into(),
                    method::<
                        maka_runtime::configuration::CredentialVaultQueryInput,
                        maka_runtime::configuration::CredentialVaultQueryResult,
                    >(MethodMode::Query),
                ),
                (
                    "deleteCredential".into(),
                    method::<
                        maka_runtime::configuration::DeleteCredentialInput,
                        maka_runtime::configuration::CredentialMutationResult,
                    >(MethodMode::Mutation),
                ),
                (
                    "saveCredential".into(),
                    Method {
                        resources: BTreeMap::from([(
                            "secret".into(),
                            maka_plugin_core::contract::ResourceType {
                                name: "maka.text.utf8".into(),
                                version: 1,
                            },
                        )]),
                        ..method::<SaveCredential, CredentialSaveResult>(MethodMode::Mutation)
                    },
                ),
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
