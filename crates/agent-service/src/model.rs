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

use crate::Configuration;
use futures_util::future::BoxFuture;
use maka_agent::{ModelSource, PreparedModel, RunError};
use maka_config::ConfigurationStore;
use maka_model::{ProviderAuth, ProviderConfig};
use maka_plugins::provider::{Connection, Definition, Resolve};
use maka_runtime::{composition::SourceRevision, configuration::ModelInfo, scope::Scope};
use std::sync::Arc;

pub(crate) struct Source {
    pub configuration: Arc<ConfigurationStore>,
    pub settings: Configuration,
    pub definition: Arc<Definition>,
    pub session: String,
    pub revision: SourceRevision,
}
impl ModelSource for Source {
    fn capture(&self) -> BoxFuture<'_, Result<PreparedModel, RunError>> {
        Box::pin(async move {
            // Every logical step observes the actual selected Root in one SQL
            // snapshot. Disabled, removed or replaced providers never fall back.
            let material = self
                .configuration
                .observe_model(self.settings.model.binding())
                .await
                .map_err(|_| unavailable())?
                .ok_or_else(unavailable)?;
            let row = &material.connection;
            if row.provider.package_id != maka_providers::api::ID
                || row.provider.entry_id != maka_providers::api::ID
                || row.provider.scope != Scope::Profile
                || row.provider.name != self.settings.provider
            {
                return Err(unavailable());
            }
            let connection = Connection {
                id: row.connection_id.clone(),
                revision: row.revision,
                configuration: row.configuration.clone(),
            };
            let model = self
                .definition
                .prepare(Resolve {
                    connection: connection.clone(),
                    model: row
                        .models
                        .iter()
                        .find(|model| model.id == self.settings.model.model)
                        .cloned()
                        .unwrap_or_else(|| ModelInfo::new(&self.settings.model.model)),
                    overrides: row
                        .model_overrides
                        .as_ref()
                        .and_then(|models| models.get(&self.settings.model.model))
                        .cloned(),
                    thinking_level: None,
                })
                .await
                .map_err(|_| unavailable())?;
            let capabilities = model.info.capabilities.unwrap_or_default();
            if capabilities.chat == Some(false) {
                return Err(unavailable());
            }
            let credentials = self
                .definition
                .authorize(
                    connection,
                    material
                        .credential
                        .as_ref()
                        .map(|value| value.credential().clone()),
                    self.session.clone(),
                )
                .await
                .map_err(|_| RunError::InvalidInput("agent_authentication_unavailable".into()))?;
            let auth = match credentials {
                maka_plugins::model::Credentials::ApiKey(key) => ProviderAuth::ApiKey(key),
                maka_plugins::model::Credentials::RequestHeaders(headers) => {
                    ProviderAuth::RequestHeaders(headers)
                }
            };
            let network = maka_network::Policy::from_host_settings(
                &material.network.proxy,
                material.network.password.as_deref(),
            )
            .map_err(|_| unavailable())?;
            Ok(PreparedModel {
                provider_id: self.settings.provider.clone(),
                provider: ProviderConfig {
                    adapter: Some(model.adapter),
                    capabilities,
                    kind: model.protocol,
                    model: self.settings.model.model.clone(),
                    base_url: model.base_url,
                    auth,
                    headers: material.request_headers,
                    network,
                    body_overlay: row
                        .request_body_overlay
                        .as_ref()
                        .map(|value| value.as_object().expect("validated overlay").clone()),
                },
                options: model.provider_options,
                context: None,
                main_output_limit: model.main_output_limit,
                supports_vision: capabilities.vision.unwrap_or(false),
                revision: self.revision.clone(),
            })
        })
    }
}
fn unavailable() -> RunError {
    RunError::InvalidInput("agent_model_unavailable".into())
}
