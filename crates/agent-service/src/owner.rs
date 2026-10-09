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
    Configuration, IMPLEMENTATION, NativeContext, configuration_schema, contract::*, workspace,
};
use futures_util::future::BoxFuture;
use maka_agent::{Engine, RunError, RunInput, RunWork};

mod interactions;
mod queue;
use maka_config::ConfigurationStore;
use maka_event_log::{EventLog, sessions::SessionRecord};
use maka_model::ModelExecutor;
use maka_runtime::{
    composition::{SourceKind, SourceRevision},
    event::Invocation,
    execution::*,
    input::{DeliveredMessage, MessageInput},
    message::*,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use uuid::Uuid;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionConfiguration {
    pub title: String,
    pub labels: Vec<String>,
    pub workspace: Workspace,
    pub permissions: WorkspacePermissions,
    /// Original canonical permission-grant fence, not a second grant ledger.
    #[serde(rename = "boundary_revision")]
    pub boundary_revision: u64,
}
/// Frozen accepted message prerequisites, interpreted only by this Agent.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MessageBasis {
    model: crate::model::AcceptedModel,
    workspace: Workspace,
    fingerprint: String,
    tools: Vec<crate::tools::SelectedTool>,
    max_steps: usize,
}
struct Control {
    invocation: Invocation,
    basis: MessageBasis,
    cancellation: CancellationToken,
    failure: Option<&'static str>,
}
#[derive(Default)]
struct Runs {
    closing: bool,
    controls: BTreeMap<String, Control>,
    /// Unconfirmed work retains actual capacity, never a reusable slot.
    quarantined: Vec<OwnedSemaphorePermit>,
}
pub(crate) struct Owner {
    pub store: Arc<EventLog>,
    pub settings: Configuration,
    pub prefix: String,
    configuration: Arc<ConfigurationStore>,
    revision: SourceRevision,
    pub workspace_roots: Vec<workspace::Root>,
    managed_worktrees: std::path::PathBuf,
    engine: Engine,
    tools: maka_tools::ToolCatalog,
    tool_basis: Vec<crate::tools::SelectedTool>,
    runs: Mutex<Runs>,
    permits: Arc<Semaphore>,
    workers: TaskTracker,
    stop: CancellationToken,
}
impl Owner {
    pub async fn open(context: NativeContext, input: Value) -> Result<Arc<Self>, String> {
        if !jsonschema_valid(&input) {
            return Err("agent_configuration_invalid".into());
        }
        let settings: Configuration =
            serde_json::from_value(input).map_err(|_| "agent_configuration_invalid")?;
        let persistence = context.persistence().ok_or("agent_requires_persistence")?;
        let (tools, tool_basis) = crate::tools::catalog(
            context.services(),
            persistence.store().clone(),
            &settings.tools,
        )?;
        let root = persistence
            .store()
            .owned_root()
            .map_err(|_| "agent_root_unavailable")?;
        let prefix = namespace(
            &persistence.source().state_domain,
            persistence.source().deployment,
            context.activation().instance(),
        );
        let managed_worktrees = root.canonical_path().join("subagent-worktrees");
        let workspace_roots =
            workspace::open(settings.workspace_roots.clone(), &managed_worktrees)?;
        let models = ModelExecutor::new(1, Duration::from_secs(60))
            .map_err(|_| "agent_executor_unavailable")?;
        let cells = maka_js_runtime::CodeExecutor::new(1, maka_js_runtime::CellLimits::default())
            .map_err(|_| "agent_executor_unavailable")?;
        let engine = Engine::new(persistence.store().clone(), models, cells);
        let revision = SourceRevision {
            kind: SourceKind::ModelProvider,
            name: IMPLEMENTATION.into(),
            package_id: context.package().id.clone(),
            entry_id: IMPLEMENTATION.into(),
            activation: context.activation().epoch().to_string(),
            revision: context.package().revision.clone(),
        };
        // Acquire the only asynchronous persistent child last. No fallible
        // export preparation remains after the actual native owner is created.
        let configuration = Arc::new(
            ConfigurationStore::for_root(root)
                .await
                .map_err(|_| "agent_configuration_unavailable")?,
        );
        Ok(Arc::new(Self {
            store: persistence.store().clone(),
            prefix,
            workspace_roots,
            managed_worktrees,
            revision,
            permits: Arc::new(Semaphore::new(settings.max_active_runs)),
            settings,
            configuration,
            engine,
            tools,
            tool_basis,
            runs: Mutex::default(),
            workers: TaskTracker::new(),
            stop: CancellationToken::new(),
        }))
    }
    pub fn operation_id(&self, operation: &maka_plugin_core::operation::RecoveryRef) -> String {
        // Use the actual complete original reference. UUIDs alone do not name
        // operations across provider namespaces or deployments. The Host journal
        // still rejects conflicting reuse inside its original operation namespace.
        let bytes = serde_json::to_vec(operation).expect("admitted operation reference");
        let digest = format!("{:x}", Sha256::digest(bytes));
        format!("{}{}", self.prefix, &digest[..32])
    }
    pub fn owns(&self, id: &str) -> bool {
        id.strip_prefix(&self.prefix).is_some_and(|tail| {
            tail.len() == 32
                && tail
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
    }
    pub async fn session(&self, id: &str) -> Result<SessionRecord<SessionConfiguration>, String> {
        if !self.owns(id) {
            return Err("agent_session_unavailable".into());
        }
        self.store
            .get_session(id)
            .await
            .map_err(|_| "agent_store_unavailable".to_owned())?
            .ok_or("agent_session_unavailable".into())
    }
    pub async fn create(
        &self,
        input: Create,
        operation: &maka_plugin_core::operation::RecoveryRef,
    ) -> Result<Created, String> {
        if let Some(reason) = metadata_rejection(&input.title, &input.labels) {
            return Ok(Created::Rejected { reason });
        }
        let runs = self.runs.lock().await;
        if runs.closing {
            return Ok(Created::Rejected {
                reason: Rejection::Closed,
            });
        }
        let selected = match input.workspace {
            WorkspaceSelection::Directory { selection } => {
                let Some(workspace) = workspace::initialize(
                    &self.workspace_roots,
                    selection,
                    &self.managed_worktrees,
                )
                .await?
                else {
                    return Ok(Created::Rejected {
                        reason: Rejection::WorkspaceUnavailable,
                    });
                };
                workspace
            }
            WorkspaceSelection::Session { session_id } => {
                if !self.owns(&session_id) {
                    return Ok(Created::Rejected {
                        reason: Rejection::SessionUnavailable,
                    });
                }
                let Some(session) = self
                    .store
                    .get_session::<SessionConfiguration>(&session_id)
                    .await
                    .map_err(|_| "agent_store_unavailable")?
                else {
                    return Ok(Created::Rejected {
                        reason: Rejection::SessionUnavailable,
                    });
                };
                if session.archived {
                    return Ok(Created::Rejected {
                        reason: Rejection::SessionUnavailable,
                    });
                }
                if workspace::validate(
                    &self.workspace_roots,
                    &session.configuration.workspace,
                    &self.managed_worktrees,
                )
                .await
                .is_err()
                {
                    return Ok(Created::Rejected {
                        reason: Rejection::WorkspaceUnavailable,
                    });
                }
                session.configuration.workspace
            }
        };
        let id = self.operation_id(operation);
        let record = self
            .store
            .create_session(
                &id,
                &operation.fingerprint,
                &SessionConfiguration {
                    title: input.title,
                    labels: input.labels,
                    workspace: selected,
                    permissions: self.settings.permissions.clone(),
                    boundary_revision: 0,
                },
                now(),
            )
            .await
            .map_err(|_| "agent_creation_failed")?;
        Ok(Created::Created {
            session: crate::service::summary(record),
        })
    }
    pub async fn metadata(&self, input: Metadata) -> Result<MetadataResult, String> {
        if let Some(reason) = metadata_rejection(&input.title, &input.labels) {
            return Ok(MetadataResult::Rejected { reason });
        }
        let runs = self.runs.lock().await;
        if runs.closing {
            return Ok(MetadataResult::Rejected {
                reason: Rejection::Closed,
            });
        }
        if !self.owns(&input.session_id) {
            return Ok(MetadataResult::Rejected {
                reason: Rejection::SessionUnavailable,
            });
        }
        let Some(session) = self
            .store
            .get_session::<SessionConfiguration>(&input.session_id)
            .await
            .map_err(|_| "agent_store_unavailable")?
        else {
            return Ok(MetadataResult::Rejected {
                reason: Rejection::SessionUnavailable,
            });
        };
        if session.archived {
            return Ok(MetadataResult::Rejected {
                reason: Rejection::SessionUnavailable,
            });
        }
        use maka_event_log::sessions::SessionMutation;
        let permissions = input.permissions.is_some();
        let update = move |configuration: &mut SessionConfiguration| {
            configuration.title = input.title;
            configuration.labels = input.labels;
            if let Some(change) = input.permissions
                && (change.settings != configuration.permissions || change.revoke_grants)
            {
                configuration.boundary_revision = configuration
                    .boundary_revision
                    .checked_add(1)
                    .filter(|revision| *revision <= maka_runtime::interaction::MAX_SAFE_INTEGER)
                    .ok_or_else(|| {
                        maka_event_log::StoreError::InvalidTransition(
                            "permission boundary revision exhausted".into(),
                        )
                    })?;
                configuration.permissions = change.settings;
            }
            Ok(())
        };
        let changed = if permissions {
            self.store
                .update_session_permissions(
                    &input.session_id,
                    input.expected_revision,
                    now(),
                    update,
                )
                .await
        } else {
            self.store
                .update_session_metadata(&input.session_id, input.expected_revision, update)
                .await
        }
        .map_err(|_| "agent_metadata_failed")?;
        match changed {
            SessionMutation::Committed(record) => Ok(MetadataResult::Committed {
                session: crate::service::summary(record),
            }),
            SessionMutation::RevisionConflict { expected, actual } => {
                Ok(MetadataResult::RevisionConflict { expected, actual })
            }
        }
    }
    pub async fn send(
        self: &Arc<Self>,
        id: String,
        text: String,
        selection: crate::ModelSelection,
        thinking_level: Option<maka_runtime::execution::ThinkingLevel>,
        operation: &maka_plugin_core::operation::RecoveryRef,
    ) -> Result<crate::contract::SendResult, String> {
        if text.trim().is_empty() || text.len() > 64 * 1024 {
            return Err("agent_message_invalid".into());
        }
        let mut runs = self.runs.lock().await;
        if runs.closing {
            return Err("agent_closed".into());
        }
        let current = runs.controls.get(&id);
        if let Some(reason) = current.and_then(|control| control.failure) {
            return Err(reason.into());
        }
        let queued = current.map(|control| control.invocation.clone());
        // An existing worker already holds this Session's capacity through
        // physical cleanup and the final queue read. Queueing needs no new slot.
        let permit = if queued.is_none() {
            Some(
                self.permits
                    .clone()
                    .try_acquire_owned()
                    .map_err(|_| "agent_capacity_exhausted")?,
            )
        } else {
            None
        };
        let session = self.session(&id).await?;
        if session.archived {
            return Err("agent_session_configuration_changed".into());
        }
        let (source, prepared) = crate::model::Source::select(
            self.configuration.clone(),
            selection,
            thinking_level,
            id.clone(),
            self.revision.clone(),
        )
        .await
        .map_err(public_error)?;
        workspace::validate(
            &self.workspace_roots,
            &session.configuration.workspace,
            &self.managed_worktrees,
        )
        .await?;
        let basis = MessageBasis {
            model: source.accepted.clone(),
            workspace: session.configuration.workspace.clone(),
            fingerprint: operation.fingerprint.clone(),
            tools: self.tool_basis.clone(),
            max_steps: self.settings.max_steps,
        };
        let identity = self.operation_id(operation);
        let invocation = queued.clone().unwrap_or_else(|| Invocation {
            session_id: id.clone(),
            turn_id: format!("t_{identity}"),
            run_id: format!("r_{identity}"),
            invocation_id: format!("i_{identity}"),
        });
        let message: MessageInput = text.into();
        let delivered = RootSourceMessage {
            message: DeliveredMessage {
                message_id: format!("m_{identity}"),
                submitted_content_digest: message
                    .content_digest()
                    .map_err(|_| "agent_message_invalid")?,
                content: message.clone(),
            },
            unprepared_content: message.clone(),
            submitted_placement: if queued.is_some() {
                Placement::NextTurn
            } else {
                Placement::CurrentTurn
            },
            disposition: if queued.is_some() {
                MessageDisposition::Followup
            } else {
                MessageDisposition::TurnStarted
            },
            submitted_intent: None,
        };
        let admission = maka_event_log::message_admissions::PendingMessageAdmission {
            execution: maka_event_log::message_admissions::PendingExecution {
                owner: self.prefix.clone(),
                basis: serde_json::to_value(&basis).expect("accepted message basis"),
            },
            invocation: invocation.clone(),
            steering_invocation: None,
            source: delivered.clone(),
            required_tools: Default::default(),
            admitted_at: now(),
        };
        if queued.is_some() {
            loop {
                let revision = self
                    .store
                    .message_queue_revision(&id)
                    .await
                    .map_err(|_| "agent_queue_unavailable")?;
                match self
                    .store
                    .admit_queued_message(&self.revision.activation, revision, admission.clone())
                    .await
                {
                    Ok(receipt) => {
                        return Ok(SendResult::Queued {
                            session_id: id,
                            message_id: delivered.message.message_id,
                            revision: receipt.queue_revision,
                        });
                    }
                    Err(maka_event_log::StoreError::RevisionConflict { .. }) => continue,
                    Err(_) => return Err("agent_admission_failed".into()),
                }
            }
        }
        let permit = permit.expect("new worker owns capacity");
        self.store
            .admit_message(admission)
            .await
            .map_err(|_| "agent_admission_failed")?;
        let cancellation = self.stop.child_token();
        let input = self.input(
            invocation.clone(),
            source,
            prepared,
            basis.clone(),
            delivered,
        );
        let running = match self.engine.start(input, cancellation.clone()).await {
            Ok(running) => running,
            Err(error) => {
                if error.is_cleanup_unconfirmed() {
                    runs.quarantined.push(permit);
                }
                return Err(public_error(error));
            }
        };
        runs.controls.insert(
            id.clone(),
            Control {
                invocation: invocation.clone(),
                basis,
                cancellation,
                failure: None,
            },
        );
        let owner = self.clone();
        self.workers
            .spawn(async move { owner.drive(running, permit).await });
        Ok(crate::contract::SendResult::Started {
            session_id: invocation.session_id.clone(),
            invocation_id: invocation.invocation_id,
        })
    }
}
fn metadata_rejection(title: &str, labels: &[String]) -> Option<Rejection> {
    if title.trim().is_empty()
        || title.chars().count() > 128
        || title.len() > 512
        || title.chars().any(char::is_control)
    {
        Some(Rejection::TitleInvalid)
    } else if labels.len() > 32
        || labels.iter().any(|label| {
            label.trim().is_empty() || label.len() > 128 || label.chars().any(char::is_control)
        })
    {
        Some(Rejection::LabelsInvalid)
    } else {
        None
    }
}
impl Owner {
    pub fn close(self: &Arc<Self>) -> BoxFuture<'static, Result<(), String>> {
        let owner = self.clone();
        Box::pin(async move {
            {
                let mut runs = owner.runs.lock().await;
                runs.closing = true;
                owner.stop.cancel();
                owner.workers.close();
            }
            owner.workers.wait().await;
            owner.engine.drain().await;
            let configuration = owner.configuration.shutdown().await;
            let runs = owner.runs.lock().await;
            if !runs.quarantined.is_empty() {
                return Err("agent_cleanup_unconfirmed".into());
            }
            configuration.map_err(|_| "agent_configuration_cleanup_unconfirmed".into())
        })
    }
}
pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_millis() as u64
}
fn jsonschema_valid(input: &Value) -> bool {
    serde_json::to_vec(input).is_ok_and(|bytes| bytes.len() <= 64 * 1024)
        && jsonschema::validator_for(&configuration_schema())
            .expect("configuration schema")
            .is_valid(input)
}
pub(crate) fn namespace(
    domain: &str,
    deployment: Uuid,
    instance: &maka_plugin_core::composition::InstanceId,
) -> String {
    let bytes =
        serde_json::to_vec(&(domain, deployment, instance)).expect("native owner namespace");
    format!("ma_{:x}_", Sha256::digest(bytes))
}
fn public_error(error: RunError) -> String {
    match error {
        RunError::Busy => "agent_session_busy",
        RunError::Cancelled => "agent_cancelled",
        RunError::CleanupUnconfirmed => "agent_cleanup_unconfirmed",
        _ => "agent_execution_unavailable",
    }
    .into()
}

#[cfg(test)]
#[path = "owner/tests.rs"]
mod tests;
