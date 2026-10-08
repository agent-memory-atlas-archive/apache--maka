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

use futures_util::future::BoxFuture;
use maka_agent::{ModelSource, PreparedModel, RunError};
use maka_config::{ConfigurationStore, ConnectionObservation};
use maka_model::{ProviderAuth, ProviderConfig};
use maka_plugins::provider::{Connection, Definition, Model, Resolve};
use maka_runtime::{
    composition::SourceRevision,
    configuration::{ModelCapabilities, ModelInfo, ModelSelection},
    execution::ModelBinding,
    scope::Scope,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Only parameters actually supplied to execution. Catalog labels, tests and
/// other models are observations, not a mutable authority over an accepted run.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionBasis {
    adapter: String,
    protocol: maka_runtime::model::request::ProviderKind,
    base_url: String,
    capabilities: ModelCapabilities,
    options: Value,
    main_output_limit: Option<u64>,
    body_overlay: Option<Value>,
}
impl ExecutionBasis {
    fn new(model: &Model, row: &maka_runtime::configuration::ConnectionCatalogEntry) -> Self {
        Self {
            adapter: model.adapter.clone(),
            protocol: model.protocol.clone(),
            base_url: model.base_url.clone(),
            capabilities: model.info.capabilities.unwrap_or_default(),
            options: model.provider_options.clone(),
            main_output_limit: model.main_output_limit,
            body_overlay: row.request_body_overlay.clone(),
        }
    }
}
/// Accepted route facts, without credentials or live activation handles.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AcceptedModel {
    pub binding: ModelBinding,
    provider: maka_runtime::provider::Identity,
    provider_configuration: Value,
    basis: ExecutionBasis,
}
pub(crate) struct Source {
    configuration: Arc<ConfigurationStore>,
    pub accepted: AcceptedModel,
    definition: Arc<Definition>,
    session: String,
    revision: SourceRevision,
}
impl Source {
    pub async fn select(
        configuration: Arc<ConfigurationStore>,
        selection: ModelSelection,
        session: String,
        mut revision: SourceRevision,
    ) -> Result<(Self, PreparedModel), RunError> {
        let material = configuration
            .observe_selected_model(selection.clone())
            .await
            .map_err(|_| unavailable())?
            .ok_or_else(unavailable)?;
        let row = &material.connection;
        if row.provider.package_id != maka_providers::api::ID
            || row.provider.entry_id != maka_providers::api::ID
            || row.provider.scope != Scope::Profile
        {
            return Err(unavailable());
        }
        let definition = Arc::new(
            maka_providers::api::ApiProviders::definition(&row.provider.name)
                .map_err(|_| unavailable())?,
        );
        let model = resolve(&definition, row, &selection.model_id).await?;
        revision.name = row.provider.name.clone();
        let source = Self {
            configuration,
            accepted: AcceptedModel {
                binding: ModelBinding {
                    connection_id: row.connection_id.clone(),
                    connection_slug: row.slug.clone(),
                    model: selection.model_id,
                },
                provider: row.provider.clone(),
                provider_configuration: row.configuration.clone(),
                basis: ExecutionBasis::new(&model, row),
            },
            definition,
            session,
            revision,
        };
        let prepared = source.prepare(material, model).await?;
        Ok((source, prepared))
    }
    async fn prepare(
        &self,
        material: ConnectionObservation,
        model: Model,
    ) -> Result<PreparedModel, RunError> {
        let capabilities = model.info.capabilities.unwrap_or_default();
        if capabilities.chat == Some(false) {
            return Err(unavailable());
        }
        let row = &material.connection;
        let credentials = self
            .definition
            .authorize(
                connection(row),
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
            provider_id: self.accepted.provider.name.clone(),
            provider: ProviderConfig {
                adapter: Some(model.adapter),
                capabilities,
                kind: model.protocol,
                model: self.accepted.binding.model.clone(),
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
    }
}
impl ModelSource for Source {
    fn capture(&self) -> BoxFuture<'_, Result<PreparedModel, RunError>> {
        Box::pin(async move {
            let material = self
                .configuration
                .observe_model(self.accepted.binding.clone())
                .await
                .map_err(|_| unavailable())?
                .ok_or_else(unavailable)?;
            let row = &material.connection;
            if row.provider != self.accepted.provider
                || row.configuration != self.accepted.provider_configuration
            {
                return Err(unavailable());
            }
            let model = resolve(&self.definition, row, &self.accepted.binding.model).await?;
            if ExecutionBasis::new(&model, row) != self.accepted.basis {
                return Err(unavailable());
            }
            self.prepare(material, model).await
        })
    }
}
async fn resolve(
    definition: &Definition,
    row: &maka_runtime::configuration::ConnectionCatalogEntry,
    model: &str,
) -> Result<Model, RunError> {
    definition
        .prepare(Resolve {
            connection: connection(row),
            model: row
                .models
                .iter()
                .find(|info| info.id == model)
                .cloned()
                .unwrap_or_else(|| ModelInfo::new(model)),
            overrides: row
                .model_overrides
                .as_ref()
                .and_then(|models| models.get(model))
                .cloned(),
            thinking_level: None,
        })
        .await
        .map_err(|_| unavailable())
}
fn connection(row: &maka_runtime::configuration::ConnectionCatalogEntry) -> Connection {
    Connection {
        id: row.connection_id.clone(),
        revision: row.revision,
        configuration: row.configuration.clone(),
    }
}
fn unavailable() -> RunError {
    RunError::InvalidInput("agent_model_unavailable".into())
}
