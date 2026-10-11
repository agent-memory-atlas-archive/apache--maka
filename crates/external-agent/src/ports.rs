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

//! Narrow host ports shared by ordinary service and legacy plugin embeddings.
use futures_util::future::BoxFuture;
use maka_plugins::{
    call::{ReadResource, Scope},
    execution::{Access, OfferInteraction},
    filesystem::{Files, Operation, Output},
    process::Processes,
    storage::Store,
};
use maka_runtime::{capability::FormResult, interaction::InteractionOutcome};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Ports {
    pub processes: Arc<dyn Processes>,
    pub storage: Arc<dyn Store>,
    pub callbacks: Arc<dyn Callbacks>,
}
/// An immutable attachment from the admitted request.
pub struct PromptAttachment {
    pub name: String,
    pub mime_type: String,
    pub resource: Arc<dyn ReadResource>,
}
/// Each callback identity belongs to one accepted protocol callback. Repeated
/// pages retain it; separate identical requests are still separate operations.
pub trait Callbacks: Send + Sync {
    /// Actual immutable resources from this admitted call, never filesystem locators.
    fn prompt_attachments(&self, _scope: &Scope) -> Result<Vec<PromptAttachment>, String> {
        Ok(vec![])
    }

    fn file(
        &self,
        scope: Scope,
        identity: String,
        operation: Operation,
        cancellation: CancellationToken,
    ) -> BoxFuture<'_, Result<Output, String>>;
    fn form(
        &self,
        scope: Scope,
        offer: OfferInteraction,
        cancellation: CancellationToken,
    ) -> BoxFuture<'_, Result<Option<FormResult>, String>>;
}
impl From<maka_plugins::host::Services> for Ports {
    fn from(host: maka_plugins::host::Services) -> Self {
        Self {
            processes: host.processes,
            storage: host.storage,
            callbacks: Arc::new(Legacy {
                files: host.files,
                executions: host.executions,
            }),
        }
    }
}
struct Legacy {
    files: Arc<dyn Files>,
    executions: Arc<dyn Access>,
}
impl Callbacks for Legacy {
    fn file(
        &self,
        scope: Scope,
        _: String,
        operation: Operation,
        _: CancellationToken,
    ) -> BoxFuture<'_, Result<Output, String>> {
        Box::pin(async move {
            self.files
                .invoke(scope, operation)
                .await
                .map_err(|e| e.to_string())
        })
    }
    fn form(
        &self,
        scope: Scope,
        offer: OfferInteraction,
        cancellation: CancellationToken,
    ) -> BoxFuture<'_, Result<Option<FormResult>, String>> {
        Box::pin(async move {
            let operation = offer.operation_id.clone();
            let commands = self
                .executions
                .acquire(scope)
                .await
                .map_err(|e| e.to_string())?;
            // An accepted offer must settle even if cancellation arrives.
            commands
                .offer_interaction(offer)
                .await
                .map_err(|e| e.to_string())?;
            let result = tokio::select! {
                biased;
                () = cancellation.cancelled() => None,
                result = commands.wait_interaction(operation.clone()) => Some(result),
            };
            if result.as_ref().is_none_or(|r| r.is_err()) {
                commands
                    .close_interaction(operation)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            match result.transpose().map_err(|e| e.to_string())? {
                Some(InteractionOutcome::FormAnswer { result, .. }) => Ok(Some(result)),
                None | Some(InteractionOutcome::Closure { .. }) => Ok(None),
                _ => Err("Unexpected permission interaction outcome".into()),
            }
        })
    }
}
