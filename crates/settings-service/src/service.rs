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

use crate::contract::*;
use futures_util::future::BoxFuture;
use maka_runtime::{configuration::*, scope::Scope};
use maka_runtime_host::application::{Context, Handler, NativeOwner};
use serde_json::Value;
use std::sync::Arc;
pub(crate) struct Settings(pub Arc<maka_config::ConfigurationStore>);
impl Handler for Settings {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let store = self.0.clone();
        Box::pin(async move {
            match method.as_str() {
                "connections" => super::connections::page(&store, input).await,
                "connection" => super::connections::detail(&store, input).await,
                "providers" => super::connections::providers(input),
                "create" => {
                    context.operation().ok_or("settings_mutation_required")?;
                    let mut input: CreateCatalogConnectionInput =
                        serde_json::from_value(input).map_err(|_| "settings_input_invalid")?;
                    let definition = super::connections::definition(&input.connection.provider)?;
                    input.connection.configuration = definition
                        .configure(input.connection.configuration)
                        .map_err(|_| "settings_configuration_invalid")?;
                    super::connections::encode(
                        store
                            .create_connection(input)
                            .await
                            .map_err(|_| "settings_effect_unknown")?,
                    )
                }
                "update" => {
                    context.operation().ok_or("settings_mutation_required")?;
                    let input: UpdateCatalogConnectionInput =
                        serde_json::from_value(input).map_err(|_| "settings_input_invalid")?;
                    if let Some(configuration) = &input.changes.configuration {
                        let catalog = store
                            .catalog()
                            .await
                            .map_err(|_| "settings_store_unavailable")?;
                        if let Some(row) = catalog.connections.iter().find(|row| {
                            row.connection_id == input.expected.connection_id
                                && row.revision == input.expected.revision
                        }) {
                            super::connections::definition(&row.provider)?
                                .validate_configuration(configuration)
                                .map_err(|_| "settings_configuration_invalid")?;
                        }
                    }
                    super::connections::encode(
                        store
                            .update_connection(input)
                            .await
                            .map_err(|_| "settings_effect_unknown")?,
                    )
                }
                "remove" => {
                    context.operation().ok_or("settings_mutation_required")?;
                    let input =
                        serde_json::from_value(input).map_err(|_| "settings_input_invalid")?;
                    super::connections::encode(
                        store
                            .remove_connection(input)
                            .await
                            .map_err(|_| "settings_effect_unknown")?,
                    )
                }
                "credentialStatus" => {
                    let input: CredentialVaultQueryInput =
                        serde_json::from_value(input).map_err(|_| "settings_input_invalid")?;
                    super::connections::encode(
                        store
                            .credential_status(input.locator)
                            .await
                            .map_err(|_| "settings_store_unavailable")?,
                    )
                }
                "deleteCredential" => {
                    context.operation().ok_or("settings_mutation_required")?;
                    let input =
                        serde_json::from_value(input).map_err(|_| "settings_input_invalid")?;
                    super::connections::encode(
                        store
                            .delete_credential(input)
                            .await
                            .map_err(|_| "settings_effect_unknown")?,
                    )
                }
                "saveCredential" => super::credential::save(&store, input, context).await,
                "models" => {
                    let input: ModelsRead =
                        serde_json::from_value(input).map_err(|_| "settings_input_invalid")?;
                    let catalog = store
                        .catalog()
                        .await
                        .map_err(|_| "settings_store_unavailable")?;
                    let start = match input {
                        ModelsRead::Start => ModelCursor {
                            connection: 0,
                            model: 0,
                        },
                        ModelsRead::Continue { revision, cursor } => {
                            if revision != catalog.revision {
                                return serde_json::to_value(ModelsPage::Changed {
                                    revision: catalog.revision,
                                })
                                .map_err(|_| "settings_output_invalid".into());
                            }
                            if cursor.connection >= catalog.connections.len()
                                || cursor.model
                                    >= catalog.connections[cursor.connection]
                                        .enabled_model_ids
                                        .len()
                            {
                                return Err("settings_cursor_invalid".into());
                            }
                            cursor
                        }
                    };
                    let mut choices = Vec::new();
                    let mut next = None;
                    'rows: for (index, row) in catalog
                        .connections
                        .iter()
                        .enumerate()
                        .skip(start.connection)
                    {
                        if !row.enabled
                            || row.provider.scope != Scope::Profile
                            || row.provider.package_id != maka_providers::api::ID
                            || row.provider.entry_id != maka_providers::api::ID
                        {
                            continue;
                        }
                        for (at, model) in row.enabled_model_ids.iter().enumerate().skip(
                            if index == start.connection {
                                start.model
                            } else {
                                0
                            },
                        ) {
                            if choices.len() == 16 {
                                next = Some(ModelCursor {
                                    connection: index,
                                    model: at,
                                });
                                break 'rows;
                            }
                            let info = row
                                .models
                                .iter()
                                .find(|info| &info.id == model)
                                .cloned()
                                .unwrap_or_else(|| ModelInfo::new(model));
                            let info = row
                                .model_overrides
                                .as_ref()
                                .and_then(|all| all.get(model))
                                .map_or_else(|| info.clone(), |overrides| overrides.apply(&info));
                            choices.push(ModelChoice {
                                selection: ModelSelection {
                                    connection_id: row.connection_id.clone(),
                                    revision: row.revision,
                                    model_id: model.clone(),
                                },
                                connection_name: row.name.clone(),
                                label: info.display_name.unwrap_or_else(|| model.clone()),
                                is_default: catalog.default_target.as_ref().is_some_and(|target| {
                                    target.connection_id == row.connection_id
                                        && target.model_id == *model
                                }),
                            });
                        }
                    }
                    serde_json::to_value(ModelsPage::Ready {
                        revision: catalog.revision,
                        choices,
                        next,
                    })
                    .map_err(|_| "settings_output_invalid".into())
                }
                "setDefault" => {
                    context.operation().ok_or("settings_mutation_required")?;
                    let input =
                        serde_json::from_value(input).map_err(|_| "settings_input_invalid")?;
                    let result = store
                        .set_default_target(input)
                        .await
                        .map_err(|_| "settings_effect_unknown")?;
                    serde_json::to_value(result).map_err(|_| "settings_output_invalid".into())
                }
                _ => Err("settings_method_unknown".into()),
            }
        })
    }
}
impl NativeOwner for Settings {
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        let store = self.0.clone();
        Box::pin(async move {
            store
                .shutdown()
                .await
                .map_err(|_| "settings_cleanup_unconfirmed".into())
        })
    }
}
