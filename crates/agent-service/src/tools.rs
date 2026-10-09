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

//! Explicit service-to-tool projection. Imports describe bindings, not grants.
mod approval;
pub(crate) mod image;
use crate::contract::{ServiceTool, ToolConfirmation, ToolResult};
use futures_util::FutureExt;
use maka_event_log::EventLog;
use maka_plugin_core::{
    contract::{Interface, MethodMode},
    operation::{OperationCleanup, OperationOutcome, OperationReply, Receipt},
};
use maka_plugins::call::{Attachments, Identity};
use maka_runtime::{
    tool_call::ToolRejection,
    tool_output::{ToolOutput, ToolSuccess},
    tools::{
        PreparationFuture, PreparedEffect, ToolCallContext, ToolDefinition, ToolError, ToolHandler,
        ToolNesting, ToolPreparer, ToolRegistration, ToolSemantics,
    },
};
use maka_runtime_host::application::NativeServices;
use maka_tools::ToolCatalog;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{panic::AssertUnwindSafe, sync::Arc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SelectedTool {
    selection: ServiceTool,
    binding: String,
}

pub(crate) fn catalog(
    services: NativeServices,
    store: Arc<EventLog>,
    selected: &[ServiceTool],
) -> Result<(ToolCatalog, Vec<SelectedTool>), String> {
    let mut basis = Vec::new();
    let registrations = selected
        .iter()
        .map(|selection| {
            let bindings = services
                .imports()
                .find(|(name, _)| *name == selection.import)
                .map(|(_, bindings)| bindings)
                .ok_or("agent_tool_import_unavailable")?;
            let [binding] = bindings else {
                return Err("agent_tool_import_ambiguous".to_owned());
            };
            let Interface::Service { methods } = &binding.contract().interface else {
                return Err("agent_tool_contract_invalid".to_owned());
            };
            let method = methods
                .get(&selection.method)
                .ok_or("agent_tool_method_unavailable")?;
            if !method.resources.is_empty()
                || match selection.result {
                    ToolResult::Json => method.mode == MethodMode::Subscription,
                    ToolResult::Image => method.mode != MethodMode::Subscription,
                }
            {
                return Err("agent_tool_method_unsupported".to_owned());
            }
            if selection.description.trim().is_empty() {
                return Err("agent_tool_description_invalid".to_owned());
            }
            basis.push(SelectedTool {
                selection: selection.clone(),
                binding: binding.execution_basis().into(),
            });
            Ok(ToolRegistration {
                definition: ToolDefinition {
                    provider: None,
                    name: selection.name.clone(),
                    description: selection.description.clone(),
                    input_schema: method.input.clone(),
                    output_schema: match selection.result {
                        ToolResult::Json => Some(method.output.clone()),
                        ToolResult::Image => None,
                    },
                    freeform: None,
                },
                nesting: ToolNesting::DirectOnly,
                semantics: if method.mode == MethodMode::Mutation {
                    ToolSemantics::ExclusiveStep
                } else {
                    ToolSemantics::Parallel
                },
                handler: ToolHandler::Prepared(Arc::new(Service {
                    services: services.clone(),
                    store: store.clone(),
                    binding: binding.execution_basis().into(),
                    target: format!(
                        "{} · {} v{} · {} / {} · {}",
                        selection.import,
                        binding.contract().name,
                        binding.contract().version,
                        serde_json::to_string(binding.provider().instance())
                            .expect("instance identity"),
                        binding.export(),
                        binding
                            .provider()
                            .remote_target()
                            .map_or("local", |target| target.deployment.as_str())
                    ),
                    selection: selection.clone(),
                    mode: method.mode,
                })),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let catalog = ToolCatalog::new(registrations).map_err(|_| "agent_tool_catalog_invalid")?;
    Ok((catalog, basis))
}
struct Service {
    services: NativeServices,
    store: Arc<EventLog>,
    binding: String,
    target: String,
    selection: ServiceTool,
    mode: MethodMode,
}
impl ToolPreparer for Service {
    fn names(&self) -> Vec<String> {
        vec![self.selection.name.clone()]
    }
    fn prepare(
        &self,
        name: String,
        input: Value,
        context: ToolCallContext,
        cancellation: CancellationToken,
    ) -> PreparationFuture {
        let services = self.services.clone();
        let store = self.store.clone();
        let binding = self.binding.clone();
        let target = self.target.clone();
        let selection = self.selection.clone();
        let mode = self.mode;
        Box::pin(async move {
            if name != selection.name {
                return Err(ToolRejection::Unavailable);
            }
            if cancellation.is_cancelled() {
                return Err(ToolRejection::Cancelled);
            }
            // No request or effect is admitted until the existing ToolJournal T1.
            Ok(PreparedEffect::new(move |cancellation| {
                Box::pin(async move {
                    if selection.confirmation == ToolConfirmation::Required {
                        approval::confirm(
                            &store,
                            &context,
                            &selection,
                            &binding,
                            target,
                            &input,
                            &cancellation,
                        )
                        .await?;
                    }
                    if cancellation.is_cancelled() {
                        return Err(ToolError::Failed(
                            "service call cancelled before admission".into(),
                        ));
                    }
                    let operation = operation_id(&context);
                    let request = services
                        .request(
                            Identity::Agent {
                                invocation: context.invocation,
                                operation_id: Some(context.operation_id),
                            },
                            cancellation.clone(),
                        )
                        .map_err(ToolError::Failed)?;
                    // Keep original request custody even if a native descriptor panics.
                    let mut observation = None;
                    let result = {
                        let mut business = Box::pin(
                            AssertUnwindSafe(async {
                                match mode {
                                    MethodMode::Query => {
                                        let result = request
                                            .call(
                                                &selection.import,
                                                0,
                                                selection.method,
                                                input,
                                                Attachments::default(),
                                            )
                                            .map_err(ToolError::Failed)?
                                            .await
                                            .map_err(ToolError::Failed)?;
                                        Ok(ToolSuccess::from(result))
                                    }
                                    MethodMode::Mutation => {
                                        let observer = request
                                            .mutate(
                                                &selection.import,
                                                0,
                                                selection.method,
                                                input,
                                                operation,
                                                Attachments::default(),
                                            )
                                            .map_err(ToolError::Failed)?;
                                        // This outer error is admission refusal, not an accepted
                                        // effect result. Scope.finish still must confirm cleanup;
                                        // every accepted mutation returns its original typed receipt.
                                        let reply = observer.await.map_err(ToolError::Failed)?;
                                        observation = Some(Box::new(reply.clone()));
                                        mutation_result(reply)
                                    }
                                    MethodMode::Subscription => {
                                        image::read(
                                            &request,
                                            &selection.import,
                                            selection.method.clone(),
                                            input,
                                            &cancellation,
                                        )
                                        .await
                                    }
                                }
                            })
                            .catch_unwind(),
                        );
                        tokio::select! {
                            biased;
                            result = &mut business => result,
                            _ = request.scope().cancellation.cancelled() => {
                                // Reach the original cleanup budget even when the native
                                // callback has not returned. Dropping this observer never
                                // drops the Host's actual admitted job or its fence.
                                request.finish().await.map_err(ToolError::CleanupUnconfirmed)?;
                                // A completed parent Ticket precedes the original Host
                                // reply in the same owner turn. Keep that exact observer.
                                business.await
                            }
                        }
                    }
                    .unwrap_or_else(|_| {
                        Err(ToolError::OutcomeUnknown(
                            "native tool observation panicked".into(),
                        ))
                    });
                    if let Err(message) = request.finish().await {
                        return Err(match observation {
                            Some(observation) => ToolError::OperationCleanupUnconfirmed {
                                message,
                                observation,
                            },
                            None => ToolError::CleanupUnconfirmed(message),
                        });
                    }
                    // finish() cancels the request itself; only the original
                    // Tool token can distinguish a stopped run at publication.
                    if selection.result == ToolResult::Image
                        && result.is_ok()
                        && cancellation.is_cancelled()
                    {
                        return Err(ToolError::Failed(
                            "Image tool cancelled before publication".into(),
                        ));
                    }
                    match (result, observation) {
                        (Err(ToolError::CleanupUnconfirmed(message)), Some(observation)) => {
                            Err(ToolError::OperationCleanupUnconfirmed {
                                message,
                                observation,
                            })
                        }
                        (result, _) => result,
                    }
                })
            }))
        })
    }
}
fn operation_id(context: &ToolCallContext) -> Uuid {
    let bytes = serde_json::to_vec(&(&context.invocation, &context.operation_id))
        .expect("Host tool identity");
    let digest = Sha256::digest(bytes);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    // A deterministic nonnil UUID in the native Agent's actual caller namespace.
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}
fn mutation_result(reply: OperationReply) -> Result<ToolSuccess, ToolError> {
    match &reply.receipt {
        Receipt::Recorded { outcome, cleanup } => {
            if *cleanup != OperationCleanup::Confirmed {
                return Err(ToolError::CleanupUnconfirmed(
                    "native service cleanup is unconfirmed".into(),
                ));
            }
            match outcome {
                OperationOutcome::Returned { .. } => Ok(ToolOutput::Operation(Box::new(reply)).into()),
                OperationOutcome::Unknown { .. } | OperationOutcome::Unresolved =>
                    Err(ToolError::OperationUnknown { message: "Native service result is unknown; inspect its original receipt before retrying.".into(), observation: Box::new(reply) }),
            }
        }
        Receipt::Unavailable { .. } => Err(ToolError::OperationUnknown {
            message:
                "Native service result is unknown; inspect its original receipt before retrying."
                    .into(),
            observation: Box::new(reply),
        }),
        Receipt::NotRecorded => Err(ToolError::Failed(
            "native service operation was not recorded".into(),
        )),
    }
}
