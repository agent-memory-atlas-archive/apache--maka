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

use crate::{
    contract::ServiceTool,
    interaction_wait::{wait, withdraw},
};
use maka_event_log::EventLog;
use maka_runtime::{
    interaction::{Decision, InteractionOutcome, InteractionRecord, InteractionRequest},
    tools::{ToolCallContext, ToolError},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

pub(super) async fn confirm(
    store: &EventLog,
    context: &ToolCallContext,
    selection: &ServiceTool,
    binding: &str,
    service: String,
    input: &Value,
    cancellation: &CancellationToken,
) -> Result<(), ToolError> {
    if cancellation.is_cancelled() {
        return Err(ToolError::Failed("service confirmation cancelled".into()));
    }
    let through = store
        .tool_dispatch_fence(context, &selection.name, input)
        .await
        .map_err(|_| ToolError::Failed("service confirmation source unavailable".into()))?;
    let record = InteractionRecord {
        session_id: context.invocation.session_id.clone(),
        turn_id: context.invocation.turn_id.clone(),
        run_id: context.invocation.run_id.clone(),
        request_id: format!(
            "service_{:x}",
            Sha256::digest(
                serde_json::to_vec(&(&context.invocation, &context.operation_id, binding))
                    .expect("original service call identity")
            )
        ),
        created_at: crate::owner::now(),
        request: InteractionRequest::ServiceApproval {
            tool_use_id: context.tool_use_id(),
            tool_name: selection.name.clone(),
            service,
            method: selection.method.clone(),
            binding: binding.into(),
            through,
        },
        outcome: None,
    };
    let result = async {
        let committed = store
            .establish_run_interaction(&context.invocation, &record)
            .await
            .map_err(|_| "service confirmation publication unavailable")?;
        if !committed.matches {
            return Err("service confirmation identity conflict");
        }
        wait(store, &record.request_id, cancellation).await
    }
    .await;
    let outcome = match result {
        Ok(outcome) => outcome,
        Err(error) => match withdraw(store, &record).await {
            Ok(Some(outcome)) => outcome,
            Ok(None) => return Err(ToolError::Failed(error.into())),
            Err(cleanup) => return Err(ToolError::CleanupUnconfirmed(cleanup.into())),
        },
    };
    match outcome {
        InteractionOutcome::ServiceApprovalDecision {
            decision: Decision::Allow,
            ..
        } if !cancellation.is_cancelled() => Ok(()),
        InteractionOutcome::ServiceApprovalDecision { .. } | InteractionOutcome::Closure { .. } => {
            Err(ToolError::Failed("service call was not approved".into()))
        }
        _ => Err(ToolError::Failed(
            "service confirmation outcome invalid".into(),
        )),
    }
}
