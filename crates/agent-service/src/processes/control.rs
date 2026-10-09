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

use super::*;
use maka_runtime::shell_run::{ShellPatch, ShellRun, ShellState, ShellVisibility};
use maka_runtime_host::shell::{ControlErrorKind, request::parse_ref, stdin};
use serde_json::json;

impl Processes {
    async fn read_model(&self, session: &str, id: &str) -> Result<ShellRun, ToolError> {
        self.store
            .read_shell_run(session, id)
            .await
            .map_err(persistence)?
            .filter(|record| {
                record.visibility == ShellVisibility::Model && record.id.starts_with(&self.prefix)
            })
            .ok_or_else(|| failed("Process resource not found in this Session and provider"))
    }
    pub(super) async fn project(
        &self,
        mut record: ShellRun,
        compact: bool,
    ) -> Result<Value, ToolError> {
        if matches!(
            record.state,
            ShellState::Terminal {
                observed_at: None,
                ..
            }
        ) {
            record = self
                .store
                .patch_shell_run(
                    &record.session_id,
                    &record.id,
                    ShellPatch {
                        observed_at: Some(crate::owner::now()),
                        ..Default::default()
                    },
                )
                .await
                .map_err(persistence)?;
        }
        let mut snapshot = maka_presentation::shell::local_update(record)
            .map_err(persistence)?
            .result;
        if compact {
            snapshot.output = None;
        }
        serde_json::to_value(snapshot).map_err(persistence)
    }
    pub(super) async fn control(
        &self,
        name: &str,
        value: Value,
        tool: &ToolCallContext,
        cancellation: &CancellationToken,
    ) -> Result<Value, ToolError> {
        let session = &tool.invocation.session_id;
        if name == "writeStdin" {
            return self
                .write(stdin::Input::parse(value)?, tool, cancellation)
                .await;
        }
        let input: Reference = serde_json::from_value(value).map_err(failed)?;
        let id =
            parse_ref(&input.reference).ok_or_else(|| failed("Invalid process resource ref"))?;
        let mut record = self.read_model(session, id).await?;
        cancelled(cancellation)?;
        if name == "read" {
            return self.project(record, false).await;
        }
        let applied = if let Some(mut handle) = self.resources.get(session, id) {
            // Stop bypasses input backpressure and remains available after revocation.
            let receipt = handle.stop_and_wait().await.map_err(worker_error)?;
            record = (*handle.drained().await.map_err(worker_error)?).clone();
            receipt.applied
        } else {
            if record.state.active() {
                record = self.read_model(session, id).await?;
                if record.state.active() {
                    return Err(ToolError::CleanupUnconfirmed(
                        "Active process has no native owner".into(),
                    ));
                }
            }
            false
        };
        let mut value = self.project(record, false).await?;
        value["operation"] = json!({"kind":"stop","applied":applied});
        Ok(value)
    }
    async fn write(
        &self,
        input: stdin::Input,
        tool: &ToolCallContext,
        cancellation: &CancellationToken,
    ) -> Result<Value, ToolError> {
        let session = &tool.invocation.session_id;
        let id = parse_ref(&input.reference).expect("validated ref");
        let mut record = self.read_model(session, id).await?;
        if !record.output.is_pty() {
            return Err(failed("writeStdin requires a PTY process ref"));
        }
        let mut accepted_bytes = 0;
        let mut resized = false;
        let mut resize_changed = false;
        let mut failure = None;
        if let Some(mut handle) = self.resources.get(session, id) {
            let gate = handle.control_gate();
            let _guard = tokio::select! {
                biased;
                _ = cancellation.cancelled() => return Err(failed("PTY input cancelled before admission")),
                guard = gate.lock() => guard,
            };
            record = (*handle.ready().await.map_err(worker_error)?).clone();
            if record.state.active() {
                if self.controllers.lock().unwrap().iter().any(|lease| {
                    lease.controller.target.session_id == *session
                        && lease.controller.target.reference == input.reference
                        && !lease.cancellation.is_cancelled()
                }) {
                    return Err(failed("PTY is controlled by a connected user"));
                }
                let current = crate::permissions::live(&self.store, &tool.invocation).await?;
                if current.boundary_revision != record.permissions.boundary_revision {
                    return Err(failed(
                        "PTY launch permissions no longer match this Session; start a new terminal",
                    ));
                }
                cancelled(cancellation)?;
                let result =
                    match handle.enqueue_control(input.data, input.size, cancellation.clone()) {
                        Ok(receipt) => receipt.await,
                        Err(error) => Err(error),
                    };
                match result {
                    Ok(receipt) => {
                        accepted_bytes = receipt.accepted_bytes;
                        resized = receipt.resized;
                        resize_changed = receipt.resize_changed;
                        record = (*receipt.record).clone();
                    }
                    Err(error) => match error.kind {
                        ControlErrorKind::Rejected => return Err(failed(error)),
                        ControlErrorKind::Unknown => {
                            return Err(ToolError::CleanupUnconfirmed(error.to_string()));
                        }
                        ControlErrorKind::Closed => {
                            handle.stop();
                            record = (*handle.drained().await.map_err(worker_error)?).clone();
                            accepted_bytes = error.accepted_bytes.expect("known input closure");
                            resized = error.resized.expect("known input closure");
                            resize_changed = error.resize_changed.expect("known input closure");
                            failure = Some(error.message);
                        }
                    },
                }
            }
        } else if record.state.active() {
            record = self.read_model(session, id).await?;
            if record.state.active() {
                return Err(ToolError::CleanupUnconfirmed(
                    "Active PTY has no native owner".into(),
                ));
            }
        }
        let mut value = self.project(record, false).await?;
        value["operation"] = json!({"kind":"pty_control", "acceptedBytes":accepted_bytes,
            "requestedBytes":input.bytes,"resized":resized,"resizeChanged":resize_changed,"failure":failure});
        Ok(value)
    }
}
