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

fn rejected(reason: ConfigurationRejection) -> Result<Value, String> {
    super::connections::encode(CatalogMutationResult::Rejected { reason })
}

fn catalog_result(result: maka_config::Result<CatalogMutationResult>) -> Result<Value, String> {
    match result {
        Ok(result) => super::connections::encode(result),
        // ConfigurationStore returns Invalid either before admission to SQL or
        // after confirmed rollback of its single transaction. Commit, rollback
        // and owner uncertainty must remain the original operation's Unknown.
        Err(maka_config::ConfigError::Invalid(_)) => rejected(ConfigurationRejection::InvalidInput),
        Err(_) => Err("settings_effect_unknown".into()),
    }
}

fn credential_result(
    result: maka_config::Result<CredentialMutationResult>,
) -> Result<Value, String> {
    match result {
        Ok(result) => super::connections::encode(result),
        Err(maka_config::ConfigError::Invalid(_)) => {
            super::connections::encode(CredentialMutationResult::Rejected {
                reason: ConfigurationRejection::InvalidInput,
            })
        }
        Err(_) => Err("settings_effect_unknown".into()),
    }
}
fn selectable(row: &ConnectionCatalogEntry) -> bool {
    row.enabled
        && row.provider.scope == Scope::Profile
        && row.provider.package_id == maka_providers::api::ID
        && row.provider.entry_id == maka_providers::api::ID
}
fn model_choice(row: &ConnectionCatalogEntry, model: &str) -> ModelChoice {
    let info = row
        .models
        .iter()
        .find(|info| info.id == model)
        .cloned()
        .unwrap_or_else(|| ModelInfo::new(model));
    let info = row
        .model_overrides
        .as_ref()
        .and_then(|all| all.get(model))
        .map_or_else(|| info.clone(), |overrides| overrides.apply(&info));
    ModelChoice {
        selection: ModelSelection {
            connection_id: row.connection_id.clone(),
            revision: row.revision,
            model_id: model.into(),
        },
        connection_name: row.name.clone(),
        label: info.display_name.unwrap_or_else(|| model.into()),
    }
}
fn default_model(catalog: &ConnectionCatalogSnapshot) -> DefaultModel {
    match &catalog.default_target {
        None => DefaultModel::None,
        Some(target) => match catalog.connections.iter().find(|row| {
            row.connection_id == target.connection_id
                && selectable(row)
                && row.enabled_model_ids.contains(&target.model_id)
        }) {
            Some(row) => DefaultModel::Ready {
                choice: model_choice(row, &target.model_id),
            },
            None => DefaultModel::Unavailable {
                target: target.clone(),
            },
        },
    }
}
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
                    let Ok(mut input) =
                        serde_json::from_value::<CreateCatalogConnectionInput>(input)
                    else {
                        return rejected(ConfigurationRejection::InvalidInput);
                    };
                    let Ok(definition) = super::connections::definition(&input.connection.provider)
                    else {
                        return rejected(ConfigurationRejection::ProviderUnavailable);
                    };
                    let Ok(configuration) = definition.configure(input.connection.configuration)
                    else {
                        return rejected(ConfigurationRejection::InvalidConfiguration);
                    };
                    input.connection.configuration = configuration;
                    catalog_result(store.create_connection(input).await)
                }
                "update" => {
                    context.operation().ok_or("settings_mutation_required")?;
                    let Ok(input) = serde_json::from_value::<UpdateCatalogConnectionInput>(input)
                    else {
                        return rejected(ConfigurationRejection::InvalidInput);
                    };
                    if let Some(configuration) = &input.changes.configuration {
                        let catalog = store
                            .catalog()
                            .await
                            .map_err(|_| "settings_store_unavailable")?;
                        if let Some(row) = catalog.connections.iter().find(|row| {
                            row.connection_id == input.expected.connection_id
                                && row.revision == input.expected.revision
                        }) {
                            let Ok(definition) = super::connections::definition(&row.provider)
                            else {
                                return rejected(ConfigurationRejection::ProviderUnavailable);
                            };
                            if definition.validate_configuration(configuration).is_err() {
                                return rejected(ConfigurationRejection::InvalidConfiguration);
                            }
                        }
                    }
                    catalog_result(store.update_connection(input).await)
                }
                "remove" => {
                    context.operation().ok_or("settings_mutation_required")?;
                    let Ok(input) = serde_json::from_value(input) else {
                        return rejected(ConfigurationRejection::InvalidInput);
                    };
                    catalog_result(store.remove_connection(input).await)
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
                    let Ok(input) = serde_json::from_value(input) else {
                        return super::connections::encode(CredentialMutationResult::Rejected {
                            reason: ConfigurationRejection::InvalidInput,
                        });
                    };
                    credential_result(store.delete_credential(input).await)
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
                        if !selectable(row) {
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
                            choices.push(model_choice(row, model));
                        }
                    }
                    serde_json::to_value(ModelsPage::Ready {
                        revision: catalog.revision,
                        default_model: default_model(&catalog),
                        choices,
                        next,
                    })
                    .map_err(|_| "settings_output_invalid".into())
                }
                "setDefault" => {
                    context.operation().ok_or("settings_mutation_required")?;
                    let Ok(input) = serde_json::from_value(input) else {
                        return rejected(ConfigurationRejection::InvalidInput);
                    };
                    catalog_result(store.set_default_target(input).await)
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
