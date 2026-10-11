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

mod controls;

use super::*;
use futures_util::FutureExt;
use maka_agent::{ModelSource, RunningInvocation};
use maka_event_log::{
    StoreError,
    message_admissions::PendingMessageAdmission,
    message_interrupts::{InterruptCommand, InterruptReceipt},
};
use maka_runtime::event::{EventWrite, Fact, InvocationInput, InvocationOutcome, RuntimeEvent};

impl Owner {
    pub fn activated(self: &Arc<Self>) {
        let owner = self.clone();
        self.workers.spawn(async move { owner.schedule().await });
    }

    // SQL is the only backlog. Every released capacity slot revisits the bounded
    // owner-scoped scan, including Sessions not reached during activation.
    async fn schedule(self: &Arc<Self>) {
        let mut runs = self.runs.lock().await;
        if runs.closing {
            return;
        }
        if self.schedule_locked(&mut runs).await.is_err() {
            runs.closing = true;
            self.stop.cancel();
        }
    }
    async fn schedule_locked(self: &Arc<Self>, runs: &mut Runs) -> Result<(), RunError> {
        let mut after = None;
        loop {
            if self.permits.available_permits() == 0 || self.stop.is_cancelled() {
                return Ok(());
            }
            let sessions = self
                .store
                .pending_message_sessions(&self.prefix, after.as_deref())
                .await?;
            if sessions.is_empty() {
                return Ok(());
            }
            for session in &sessions {
                if !self.owns(session) {
                    return Err(RunError::InvalidInput("agent_queue_owner_invalid".into()));
                }
                if runs.controls.contains_key(session) {
                    continue;
                }
                let Ok(permit) = self.permits.clone().try_acquire_owned() else {
                    return Ok(());
                };
                match self.next(session).await {
                    Ok(Some((running, basis))) => {
                        Self::publish(runs, &running, basis);
                        self.workers.spawn(self.clone().drive(running, permit));
                    }
                    Ok(None) => drop(permit),
                    Err(error) => {
                        if error.is_cleanup_unconfirmed() {
                            runs.quarantined.push(permit);
                        }
                        return Err(error);
                    }
                }
            }
            after = sessions.last().cloned();
        }
    }
    fn publish(runs: &mut Runs, running: &RunningInvocation, basis: MessageBasis) {
        runs.controls.insert(
            running.invocation().session_id.clone(),
            Control {
                invocation: running.invocation().clone(),
                basis,
                cancellation: running.cancellation(),
                failure: None,
            },
        );
    }
    // A boxed task breaks the type recursion between scheduling and a completed
    // worker's capacity handoff; all work remains in the original TaskTracker.
    pub(super) fn drive(
        self: Arc<Self>,
        mut running: RunningInvocation,
        permit: OwnedSemaphorePermit,
    ) -> BoxFuture<'static, ()> {
        Box::pin(async move {
            let session = running.invocation().session_id.clone();
            loop {
                let completion = std::panic::AssertUnwindSafe(running.completion())
                    .catch_unwind()
                    .await;
                let mut runs = self.runs.lock().await;
                if !matches!(completion, Ok(ref value) if value.cleanup.is_ok()) {
                    runs.controls
                        .get_mut(&session)
                        .expect("original worker control")
                        .failure = Some("agent_cleanup_unconfirmed");
                    runs.quarantined.push(permit);
                    return;
                }
                let next = if runs.closing {
                    Ok(None)
                } else {
                    self.next(&session).await
                };
                match next {
                    Ok(Some((next, basis))) => {
                        Self::publish(&mut runs, &next, basis);
                        running = next;
                    }
                    Ok(None) => {
                        runs.controls.remove(&session);
                        self.engine.release_session_state(&session);
                        drop(permit);
                        drop(runs);
                        self.schedule().await;
                        return;
                    }
                    Err(error) => {
                        runs.controls
                            .get_mut(&session)
                            .expect("original worker control")
                            .failure = Some("agent_execution_unavailable");
                        if error.is_cleanup_unconfirmed() {
                            runs.quarantined.push(permit);
                        }
                        runs.closing = true;
                        self.stop.cancel();
                        return;
                    }
                }
            }
        })
    }
    async fn next(
        &self,
        session: &str,
    ) -> Result<Option<(RunningInvocation, MessageBasis)>, RunError> {
        loop {
            if self.stop.is_cancelled() {
                return Ok(None);
            }
            let queue = self.store.message_queue(session).await?;
            let Some(entry) = queue
                .entries
                .iter()
                .find(|entry| entry.source.disposition == MessageDisposition::Steering)
                .or_else(|| queue.entries.first())
            else {
                return Ok(None);
            };
            if entry.execution.owner != self.prefix || !self.owns(session) {
                return Err(RunError::InvalidInput("agent_queue_owner_invalid".into()));
            }
            let invocation = if entry.source.disposition == MessageDisposition::TurnStarted {
                entry.invocation.clone()
            } else {
                let identity = format!(
                    "{:x}",
                    Sha256::digest(entry.source.message.message_id.as_bytes())
                );
                Invocation {
                    session_id: session.into(),
                    turn_id: format!("t_{identity}"),
                    run_id: format!("r_{identity}"),
                    invocation_id: format!("i_{identity}"),
                }
            };
            let prepared = self.prepare_pending(entry, invocation.clone()).await;
            if self.stop.is_cancelled() {
                return Ok(None);
            }
            let result = match prepared {
                Ok((input, basis)) => self
                    .engine
                    .start(input, self.stop.child_token())
                    .await
                    .map(|running| (running, basis)),
                Err(reason) => {
                    self.fail_pending(invocation, entry, reason).await?;
                    continue;
                }
            };
            match result {
                Ok(running) => return Ok(Some(running)),
                Err(error @ (RunError::InvalidInput(_) | RunError::ReconciliationRequired(_))) => {
                    self.fail_pending(invocation, entry, public_error(error))
                        .await?;
                }
                // SQL/commit uncertainty and physical cleanup failures do not
                // establish a deterministic rejection of the accepted message.
                Err(error) => return Err(error),
            }
        }
    }
    async fn prepare_pending(
        &self,
        entry: &PendingMessageAdmission,
        invocation: Invocation,
    ) -> Result<(RunInput, MessageBasis), String> {
        let basis: MessageBasis = serde_json::from_value(entry.execution.basis.clone())
            .map_err(|_| "agent_message_basis_invalid")?;
        if basis.tools != self.tool_basis
            || basis.internal_tools != self.internal_tools
            || basis.internal_bindings != self.internal_bindings
        {
            return Err("agent_tools_changed".into());
        }
        let session = self.session(&invocation.session_id).await?;
        if session.archived || session.configuration.workspace != basis.workspace {
            return Err("agent_session_configuration_changed".into());
        }
        workspace::validate(
            &self.workspace_roots,
            &basis.workspace,
            &self.managed_worktrees,
        )
        .await?;
        let source = crate::model::Source::restore(
            self.configuration.clone(),
            self.credentials.clone(),
            basis.model.clone(),
            invocation.session_id.clone(),
            self.revision.clone(),
        )
        .map_err(public_error)?;
        let prepared = source.capture().await.map_err(public_error)?;
        Ok((
            self.input(
                invocation,
                source,
                prepared,
                basis.clone(),
                entry.source.clone(),
            )?,
            basis,
        ))
    }
    pub(super) fn input(
        &self,
        invocation: Invocation,
        source: crate::model::Source,
        prepared: maka_agent::PreparedModel,
        basis: MessageBasis,
        delivered: RootSourceMessage,
    ) -> Result<RunInput, String> {
        let binding = source.accepted.binding.clone();
        let thinking_level = source.accepted.thinking_level;
        let system_prompt = match (
            basis.behavior.clone(),
            skills::prompt(&delivered.message.content)?,
        ) {
            (Some(mut behavior), Some(skills)) => {
                behavior.text.push_str("\n\n");
                behavior.text.push_str(&skills.text);
                behavior.sources.extend(skills.sources);
                Some(behavior)
            }
            (behavior, skills) => behavior.or(skills),
        };
        if let Some(prompt) = &system_prompt {
            prompt.validate().map_err(|_| "agent_prompt_invalid")?;
        }
        Ok(RunInput {
            model_source: Some(Arc::new(source)),
            model_revision: Some(prepared.revision),
            provider_id: prepared.provider_id,
            invocation,
            context: prepared.context,
            provider: prepared.provider,
            provider_options: prepared.options,
            main_output_limit: prepared.main_output_limit,
            supports_vision: prepared.supports_vision,
            request_fingerprint: Some(basis.fingerprint),
            configuration: InvocationConfiguration {
                cwd: basis.workspace.path,
                workspace_identity: Some(basis.workspace.identity),
                workspace_origin: WorkspaceOrigin::Selected,
                sandbox_mode: SandboxMode::WorkspaceWrite,
                approval_policy: ApprovalPolicy::OnRequest,
                boundary_revision: 0,
                collaboration_mode: CollaborationMode::Agent,
                orchestration_mode: BehaviorId::default(),
                tool_mode: ToolMode::Direct,
                model: Some(binding),
                thinking_level,
                system_prompt,
                tool_composition: None,
            },
            work: RunWork::Message {
                message: delivered.message.content.clone(),
                source_messages: vec![delivered],
                tools: self.tools.clone(),
                max_steps: basis.max_steps,
                allow_prior_unknown: false,
            },
        })
    }
    async fn fail_pending(
        &self,
        invocation: Invocation,
        entry: &PendingMessageAdmission,
        reason: String,
    ) -> Result<(), RunError> {
        let events = [
            Fact::InvocationOpened {
                configuration: None,
                input: InvocationInput::Message {
                    content: entry.source.message.content.clone(),
                    source_messages: vec![entry.source.clone()],
                    request_fingerprint: None,
                },
            },
            Fact::InvocationEnded {
                outcome: InvocationOutcome::Failed {
                    class: "message_preparation_failed".into(),
                    message: Some(reason),
                },
            },
        ]
        .into_iter()
        .map(|fact| EventWrite::plain(RuntimeEvent::new(invocation.clone(), fact)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| RunError::InvalidInput(error.to_string()))?;
        self.store.append_batch(&events).await?;
        Ok(())
    }
    pub async fn stop_run(
        &self,
        session: &str,
        invocation: &str,
        operation: &maka_plugin_core::operation::RecoveryRef,
    ) -> Result<bool, String> {
        self.session(session).await?;
        let runs = self.runs.lock().await;
        let Some(control) = runs
            .controls
            .get(session)
            .filter(|control| control.invocation.invocation_id == invocation)
        else {
            return Ok(false);
        };
        let command = InterruptCommand {
            host_epoch: self.revision.activation.clone(),
            session_id: session.into(),
            interrupt_id: self.operation_id(operation),
            turn_id: control.invocation.turn_id.clone(),
            run_id: control.invocation.run_id.clone(),
        };
        loop {
            let revision = self
                .store
                .message_queue_revision(session)
                .await
                .map_err(|_| "agent_queue_unavailable")?;
            match self
                .store
                .interrupt_message_queue(&command, revision, Some(&control.invocation))
                .await
            {
                Ok(InterruptReceipt::Fenced(_)) => {
                    control.cancellation.cancel();
                    return Ok(true);
                }
                Ok(InterruptReceipt::Conflict) => return Ok(false),
                Err(StoreError::RevisionConflict { .. }) => continue,
                Err(_) => return Err("agent_queue_unavailable".into()),
            }
        }
    }
}
