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
use maka_config::ConfigurationStore;
use maka_plugins::provider::Definition;
use maka_runtime::{configuration::*, provider::Identity, scope::Scope};
use serde::Serialize;
use serde_json::Value;

pub(crate) fn encode(value: impl Serialize) -> Result<Value, String> {
    let value = serde_json::to_value(value).map_err(|_| "settings_output_invalid")?;
    if serde_json::to_vec(&value)
        .map_err(|_| "settings_output_invalid")?
        .len()
        > 128 * 1024
    {
        return Err("settings_output_budget".into());
    }
    Ok(value)
}
pub(crate) fn definition(identity: &Identity) -> Result<Definition, String> {
    if identity.package_id != maka_providers::api::ID
        || identity.entry_id != maka_providers::api::ID
        || identity.scope != Scope::Profile
    {
        return Err("settings_provider_unavailable".into());
    }
    maka_providers::api::ApiProviders::definition(&identity.name)
        .map_err(|_| "settings_provider_unavailable".into())
}
pub(crate) async fn page(store: &ConfigurationStore, input: Value) -> Result<Value, String> {
    let input: ConnectionsRead =
        serde_json::from_value(input).map_err(|_| "settings_input_invalid")?;
    let catalog = store
        .catalog()
        .await
        .map_err(|_| "settings_store_unavailable")?;
    let offset = match input {
        ConnectionsRead::Start => 0,
        ConnectionsRead::Continue { revision, offset } => {
            if revision != catalog.revision {
                return encode(ConnectionsPage::Changed {
                    revision: catalog.revision,
                });
            }
            if offset >= catalog.connections.len() {
                return Err("settings_cursor_invalid".into());
            }
            offset
        }
    };
    let end = (offset + 16).min(catalog.connections.len());
    let connections = catalog.connections[offset..end]
        .iter()
        .map(|row| ConnectionSummary {
            basis: ConnectionVersionBasis {
                connection_id: row.connection_id.clone(),
                revision: row.revision,
            },
            slug: row.slug.clone(),
            name: row.name.clone(),
            provider: row.provider.clone(),
            enabled: row.enabled,
            enabled_model_count: row.enabled_model_ids.len(),
        })
        .collect();
    encode(ConnectionsPage::Ready {
        revision: catalog.revision,
        default_target: catalog.default_target,
        connections,
        next: (end < catalog.connections.len()).then_some(end),
    })
}
pub(crate) async fn detail(store: &ConfigurationStore, input: Value) -> Result<Value, String> {
    let expected: ConnectionVersionBasis =
        serde_json::from_value(input).map_err(|_| "settings_input_invalid")?;
    let catalog = store
        .catalog()
        .await
        .map_err(|_| "settings_store_unavailable")?;
    let row = catalog
        .connections
        .into_iter()
        .find(|row| row.connection_id == expected.connection_id);
    match row {
        Some(row) if row.revision == expected.revision => encode(ConnectionDetail::Ready {
            target: Box::new(ConnectionCredentialTarget {
                connection_id: row.connection_id,
                revision: row.revision,
                slug: row.slug,
                provider: row.provider,
                configuration: row.configuration,
            }),
            name: row.name,
            enabled: row.enabled,
            enabled_model_count: row.enabled_model_ids.len(),
        }),
        actual => encode(ConnectionDetail::Stale {
            actual: actual.map(|row| ConnectionVersionBasis {
                connection_id: row.connection_id,
                revision: row.revision,
            }),
        }),
    }
}
pub(crate) fn providers(input: Value) -> Result<Value, String> {
    let input: ProvidersRead =
        serde_json::from_value(input).map_err(|_| "settings_input_invalid")?;
    let names =
        maka_providers::api::ApiProviders::names().map_err(|_| "settings_provider_unavailable")?;
    if input.offset >= names.len() && input.offset != 0 {
        return Err("settings_cursor_invalid".into());
    }
    let mut providers = Vec::new();
    let mut next = None;
    for (offset, name) in names.into_iter().enumerate().skip(input.offset) {
        let identity = Identity {
            package_id: maka_providers::api::ID.into(),
            entry_id: maka_providers::api::ID.into(),
            scope: Scope::Profile,
            name,
        };
        let descriptor = definition(&identity)?.descriptor().clone();
        providers.push(Provider {
            identity,
            descriptor,
        });
        // Include the envelope and largest bounded cursor in the byte budget.
        let projected = serde_json::to_value(ProvidersPage {
            providers: Vec::new(),
            next: Some(offset + 1),
        })
        .map_err(|_| "settings_output_invalid")?;
        let bytes = serde_json::to_vec(&providers)
            .map_err(|_| "settings_output_invalid")?
            .len()
            + serde_json::to_vec(&projected)
                .map_err(|_| "settings_output_invalid")?
                .len();
        if providers.len() > 8 || bytes > 128 * 1024 {
            providers.pop();
            next = Some(offset);
            break;
        }
    }
    encode(ProvidersPage { providers, next })
}
