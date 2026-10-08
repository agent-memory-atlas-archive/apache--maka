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
use futures_util::{FutureExt, future::BoxFuture};
use maka_agent::{Engine, RunError, RunInput, RunWork};
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
}
struct Control {
    invocation: String,
    cancellation: CancellationToken,
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
        let tools = crate::tools::catalog(context.services(), &settings.tools)?;
        let persistence = context.persistence().ok_or("agent_requires_persistence")?;
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
        match self
            .store
            .update_session_metadata::<SessionConfiguration, _>(
                &input.session_id,
                input.expected_revision,
                move |configuration| {
                    configuration.title = input.title;
                    configuration.labels = input.labels;
                    Ok(())
                },
            )
            .await
            .map_err(|_| "agent_metadata_failed")?
        {
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
        operation: &maka_plugin_core::operation::RecoveryRef,
    ) -> Result<crate::contract::Started, String> {
        if text.trim().is_empty() || text.len() > 64 * 1024 {
            return Err("agent_message_invalid".into());
        }
        // No waiting queue. Capacity covers preparation, durable admission and
        // the original complete run, independently of request/observer lifetime.
        let permit = self
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| "agent_capacity_exhausted")?;
        let mut runs = self.runs.lock().await;
        if runs.closing {
            return Err("agent_closed".into());
        }
        if runs.controls.contains_key(&id) {
            return Err("agent_session_busy".into());
        }
        let session = self.session(&id).await?;
        if session.archived {
            return Err("agent_session_configuration_changed".into());
        }
        let (source, prepared) = crate::model::Source::select(
            self.configuration.clone(),
            selection,
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
        let binding = source.binding.clone();
        let source = Arc::new(source);
        let identity = self.operation_id(operation);
        let invocation = Invocation {
            session_id: id.clone(),
            turn_id: format!("t_{identity}"),
            run_id: format!("r_{identity}"),
            invocation_id: format!("i_{identity}"),
        };
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
            submitted_placement: Placement::CurrentTurn,
            disposition: MessageDisposition::TurnStarted,
            submitted_intent: None,
        };
        self.store
            .admit_message(
                maka_event_log::message_admissions::PendingMessageAdmission {
                    invocation: invocation.clone(),
                    steering_invocation: None,
                    source: delivered.clone(),
                    required_tools: Default::default(),
                    admitted_at: now(),
                },
            )
            .await
            .map_err(|_| "agent_admission_failed")?;
        let cancellation = self.stop.child_token();
        let input = RunInput {
            model_source: Some(source),
            model_revision: Some(prepared.revision),
            provider_id: prepared.provider_id,
            invocation: invocation.clone(),
            context: prepared.context,
            provider: prepared.provider,
            provider_options: prepared.options,
            main_output_limit: prepared.main_output_limit,
            supports_vision: prepared.supports_vision,
            request_fingerprint: Some(operation.fingerprint.clone()),
            configuration: InvocationConfiguration {
                cwd: session.configuration.workspace.path,
                workspace_identity: Some(session.configuration.workspace.identity),
                workspace_origin: WorkspaceOrigin::Selected,
                sandbox_mode: SandboxMode::WorkspaceWrite,
                approval_policy: ApprovalPolicy::OnRequest,
                boundary_revision: 0,
                collaboration_mode: CollaborationMode::Agent,
                orchestration_mode: BehaviorId::default(),
                tool_mode: ToolMode::Direct,
                model: Some(binding),
                thinking_level: None,
                system_prompt: None,
                tool_composition: None,
            },
            work: RunWork::Message {
                message,
                source_messages: vec![delivered],
                tools: self.tools.clone(),
                max_steps: self.settings.max_steps,
                allow_prior_unknown: false,
            },
        };
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
                invocation: invocation.invocation_id.clone(),
                cancellation,
            },
        );
        let owner = self.clone();
        // Publish original-handle custody before returning Started. A closed UI
        // or finished mutation Scope cannot cancel or acknowledge this domain run.
        self.workers.spawn(async move {
            let completion = std::panic::AssertUnwindSafe(running.completion())
                .catch_unwind()
                .await;
            let mut runs = owner.runs.lock().await;
            let clean = matches!(completion, Ok(ref completion) if completion.cleanup.is_ok());
            if clean {
                runs.controls.remove(&id);
                owner.engine.release_session_state(&id);
                drop(permit);
            } else {
                runs.quarantined.push(permit);
            }
        });
        Ok(crate::contract::Started {
            session_id: invocation.session_id.clone(),
            invocation_id: invocation.invocation_id,
        })
    }
    pub async fn stop_run(&self, session: &str, invocation: &str) -> Result<bool, String> {
        self.session(session).await?;
        let runs = self.runs.lock().await;
        if let Some(control) = runs.controls.get(session)
            && control.invocation == invocation
        {
            control.cancellation.cancel();
            return Ok(true);
        }
        Ok(false)
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
            if !owner.runs.lock().await.quarantined.is_empty() {
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
