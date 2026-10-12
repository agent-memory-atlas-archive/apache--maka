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

//! Canonical permission decisions shared by native workspace executors.
use crate::interaction_wait::{wait, withdraw};
use maka_agent_api::session::SessionConfiguration;
use maka_agent_facts::interaction::InteractionOutcome;
use maka_agent_facts::interaction::InteractionRecord;
use maka_agent_facts::interaction::InteractionRequest;
use maka_agent_facts::interaction::PermissionRequest;
use maka_agent_facts::tools::ToolCallContext;
use maka_agent_facts::tools::ToolError;
use maka_event_log::{EventLog, interactions::PermissionGrant};
use maka_sandbox::grant::Decision;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

pub async fn request(
    store: &EventLog,
    configuration: &SessionConfiguration,
    request: PermissionRequest,
    context: &ToolCallContext,
    cancellation: &CancellationToken,
) -> Result<PermissionGrant, ToolError> {
    if !configuration.permissions.request_permissions {
        return Err(failed(
            "Workspace policy does not allow requesting additional permissions",
        ));
    }
    let record = InteractionRecord {
        session_id: context.invocation.session_id.clone(),
        turn_id: context.invocation.turn_id.clone(),
        run_id: context.invocation.run_id.clone(),
        request_id: format!(
            "permissions_{:x}",
            Sha256::digest(
                serde_json::to_vec(&(
                    &context.invocation,
                    &context.operation_id,
                    configuration.boundary_revision,
                    &request,
                ))
                .expect("tool identity")
            )
        ),
        created_at: crate::now(),
        request: InteractionRequest::Permissions {
            tool_use_id: Some(context.tool_use_id()),
            base_revision: configuration.boundary_revision,
            request,
        },
        outcome: None,
    };
    let observed = async {
        let committed = store
            .establish_run_interaction(&context.invocation, &record)
            .await
            .map_err(|_| "permission publication unavailable")?;
        if !committed.matches {
            return Err("permission identity conflict");
        }
        wait(store, &record.request_id, cancellation).await
    }
    .await;
    let outcome = match observed {
        Ok(outcome) => outcome,
        Err(error) => match withdraw(store, &record).await {
            Ok(Some(outcome)) => outcome,
            Ok(None) => return Err(failed(error)),
            Err(cleanup) => return Err(ToolError::CleanupUnconfirmed(cleanup.into())),
        },
    };
    if cancellation.is_cancelled() {
        return Err(failed("Permission request cancelled"));
    }
    if !matches!(
        outcome,
        InteractionOutcome::PermissionsDecision {
            decision: Decision::Allow { .. },
            ..
        }
    ) {
        return Err(failed("Additional permissions were not granted"));
    }
    store
        .permission_grants(
            &context.invocation,
            Some(&context.tool_use_id()),
            configuration.boundary_revision,
        )
        .await
        .map_err(|_| failed("Permission observation unavailable"))?
        .into_iter()
        .find(|grant| grant.request_id == record.request_id)
        .ok_or_else(|| failed("Approved permissions are no longer current"))
}

fn failed(message: impl Into<String>) -> ToolError {
    ToolError::Failed(message.into())
}

pub async fn live(
    store: &EventLog,
    invocation: &maka_agent_facts::event::Invocation,
) -> Result<SessionConfiguration, ToolError> {
    let projection = store
        .session_projection::<SessionConfiguration>(&invocation.session_id)
        .await
        .map_err(|_| failed("Workspace session is unavailable"))?
        .filter(|p| {
            !p.session.archived
                && p.root_turn.as_ref().is_some_and(|turn| {
                    turn.invocation == *invocation
                        && !matches!(
                            turn.state,
                            maka_event_log::turns::InvocationState::Ended { .. }
                        )
                })
        })
        .ok_or_else(|| failed("Workspace invocation is no longer active"))?;
    Ok(projection.session.configuration)
}
