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

mod assistant;
mod attachments;
mod execution;
mod imports;
mod interactions;
mod notifications;
mod projects;
pub(crate) use notifications::ReturnRoute;
mod patches;
mod queue;
mod recall;
mod scheduler;
mod skills;
mod work;
mod worktrees;
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
    #[serde(default)]
    pub tools: Option<Vec<String>>,
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
    #[serde(default)]
    allowed_tools: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    background: Option<ScheduledTarget>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    internal_tools: Vec<maka_runtime::tools::ToolDefinition>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    internal_bindings: BTreeMap<String, Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    behavior: Option<SystemPrompt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<crate::model::AcceptedModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    executor: Option<crate::executor::Accepted>,
    workspace: Workspace,
    fingerprint: String,
    tools: Vec<crate::tools::SelectedTool>,
    max_steps: usize,
}
enum Selection {
    Model(ModelSelection, Option<ThinkingLevel>),
    Executor(crate::executor::Selection),
}
struct Submission {
    id: String,
    text: String,
    attachments: Vec<String>,
    selection: Selection,
}
enum PreparedBackend {
    Model(Box<(crate::model::Source, maka_agent::PreparedModel)>),
    Executor(Box<crate::executor::Backend>),
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
    pub recall: Arc<recall::Recall>,
    scheduler: Arc<scheduler::Scheduler>,
    pub settings: Configuration,
    pub prefix: String,
    pub data_source: DataSource,
    configuration: Arc<ConfigurationStore>,
    credentials: crate::model::Credentials,
    revision: SourceRevision,
    behavior: Option<SystemPrompt>,
    pub workspace_roots: Vec<workspace::Root>,
    instruction_home: Option<Arc<cap_std::fs::Dir>>,
    pub import_roots: Vec<workspace::Root>,
    import_capacity: Arc<Semaphore>,
    internal_tools: Vec<maka_runtime::tools::ToolDefinition>,
    internal_bindings: BTreeMap<String, Option<String>>,
    managed_worktrees: std::path::PathBuf,
    engine: Engine,
    tools: maka_tools::ToolCatalog,
    tool_basis: Vec<crate::tools::SelectedTool>,
    skills: Option<skills::Library>,
    executors: crate::executor::Executors,
    runs: Mutex<Runs>,
    permits: Arc<Semaphore>,
    upload_capacity: Arc<Semaphore>,
    workers: TaskTracker,
    stop: CancellationToken,
}
impl Owner {
    pub(crate) async fn executor_setup(
        &self,
        input: crate::executor::setup::Input,
        context: maka_runtime_host::application::Context,
    ) -> Result<Value, String> {
        self.executors.setup(input, context).await
    }

    pub async fn open(context: NativeContext, input: Value) -> Result<Arc<Self>, String> {
        Self::open_behavior(
            context,
            input,
            IMPLEMENTATION,
            None,
            vec![],
            BTreeMap::new(),
        )
        .await
    }
    pub(crate) async fn open_behavior(
        context: NativeContext,
        input: Value,
        implementation: &'static str,
        prompt: Option<&'static str>,
        mut additional: Vec<maka_runtime::tools::ToolRegistration>,
        internal_bindings: BTreeMap<String, Option<String>>,
    ) -> Result<Arc<Self>, String> {
        if !jsonschema_valid(&input) {
            return Err("agent_configuration_invalid".into());
        }
        let settings: Configuration =
            serde_json::from_value(input).map_err(|_| "agent_configuration_invalid")?;
        let persistence = context.persistence().ok_or("agent_requires_persistence")?;
        let root = persistence
            .store()
            .owned_root()
            .map_err(|_| "agent_root_unavailable")?;
        let data_source = DataSource {
            state_domain: persistence.source().state_domain.clone(),
            deployment: persistence.source().deployment,
            namespace: context.activation().instance().clone(),
        };
        let prefix = session_namespace(
            &persistence.source().state_domain,
            persistence.source().deployment,
            context.activation().instance(),
            implementation != IMPLEMENTATION,
        );
        let managed_worktrees = root.canonical_path().join("subagent-worktrees");
        let workspace_roots =
            workspace::open(settings.workspace_roots.clone(), &managed_worktrees)?;
        let import_roots = if settings.import_roots.is_empty() {
            Vec::new()
        } else {
            workspace::open(settings.import_roots.clone(), root.canonical_path())?
        };
        let models = ModelExecutor::new(1, Duration::from_secs(60))
            .map_err(|_| "agent_executor_unavailable")?;
        let cells = maka_js_runtime::CodeExecutor::new(1, maka_js_runtime::CellLimits::default())
            .map_err(|_| "agent_executor_unavailable")?;
        let workers = TaskTracker::new();
        let stop = CancellationToken::new();
        let revision = SourceRevision {
            kind: SourceKind::ModelProvider,
            name: implementation.into(),
            package_id: context.package().id.clone(),
            entry_id: implementation.into(),
            activation: context.activation().epoch().to_string(),
            revision: context.package().revision.clone(),
        };
        let behavior = prompt.map(|text| {
            let mut source = revision.clone();
            source.kind = SourceKind::PromptSection;
            SystemPrompt {
                text: text.into(),
                policy_revision: 0,
                sources: vec![source],
            }
        });
        let skills = skills::Library::select(&context, &settings, revision.clone())?;
        let instruction_home = assistant::open_home(settings.assistant.as_ref())?;
        let executors = crate::executor::Executors::capture(&context, &settings.executors)?;
        let configuration = Arc::new(
            ConfigurationStore::for_root(root)
                .await
                .map_err(|_| "agent_configuration_unavailable")?,
        );
        let recall = Arc::new(recall::Recall {
            store: persistence.store().clone(),
            configuration: configuration.clone(),
            prefix: prefix.clone(),
            data_source: data_source.clone(),
            enabled: settings.recall,
            stop: stop.clone(),
            workers: workers.clone(),
            services: context.services(),
            capacity: Arc::new(Semaphore::new(2)),
        });
        if settings.recall {
            additional.extend(recall.tools());
        }
        let scheduler = Arc::new(scheduler::Scheduler::new(
            persistence.store().clone(),
            prefix.clone(),
        ));
        if settings.scheduled_tasks {
            additional.extend(scheduler.tools());
        }
        let internal_tools = additional
            .iter()
            .map(|tool| tool.definition.clone())
            .collect();
        let prepared_tools = crate::tools::catalog(
            context.services(),
            persistence.store().clone(),
            &settings.tools,
            additional,
            workers.clone(),
        )
        .and_then(|(tools, basis)| {
            let tools = if settings.recall {
                tools
                    .with_request_binding(
                        maka_runtime::tools::ToolPreparer::names(recall.as_ref())
                            .into_iter()
                            .collect(),
                        recall.clone(),
                    )
                    .map_err(|error| error.to_string())?
            } else {
                tools
            };
            Ok((tools, basis))
        });
        let (tools, tool_basis) = match prepared_tools {
            Ok(tools) => tools,
            Err(error) => {
                configuration
                    .shutdown()
                    .await
                    .map_err(|_| "agent_configuration_cleanup_unconfirmed")?;
                return Err(error);
            }
        };
        let credentials = crate::model::Credentials {
            executor: models.clone(),
            refresh: Arc::new(maka_runtime_host::oauth::Authority::new(
                workers.clone(),
                stop.clone(),
            )),
        };
        let engine = Engine::with_pricing(
            persistence.store().clone(),
            models,
            cells,
            Arc::new(crate::model::Prices(configuration.clone())),
        );
        let owner = Arc::new(Self {
            scheduler,
            recall,
            store: persistence.store().clone(),
            prefix,
            data_source,
            workspace_roots,
            import_roots,
            import_capacity: Arc::new(Semaphore::new(2)),
            managed_worktrees,
            revision,
            behavior,
            instruction_home,
            internal_tools,
            internal_bindings,
            permits: Arc::new(Semaphore::new(settings.max_active_runs)),
            upload_capacity: Arc::new(Semaphore::new(1)),
            settings,
            configuration,
            credentials,
            engine,
            tools,
            tool_basis,
            skills,
            executors,
            runs: Mutex::default(),
            workers,
            stop,
        });
        Ok(owner)
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
        owns_session(&self.prefix, id)
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
        self: &Arc<Self>,
        input: Create,
        operation: &maka_plugin_core::operation::RecoveryRef,
        scope: &maka_plugins::call::Scope,
    ) -> Result<Created, String> {
        let operation = operation.clone();
        self.resource_work(scope, move |owner, scope| {
            Box::pin(async move { owner.create_inner(input, &operation, &scope).await })
        })
        .await
    }
    async fn create_inner(
        &self,
        input: Create,
        operation: &maka_plugin_core::operation::RecoveryRef,
        scope: &maka_plugins::call::Scope,
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
        let mut tools = input.tools;
        if let Some(names) = &mut tools {
            names.sort();
            if names.len() > 128
                || names.windows(2).any(|pair| pair[0] == pair[1])
                || names.iter().any(|name| !self.tools.contains(name))
            {
                return Ok(Created::Rejected {
                    reason: Rejection::ToolsUnavailable,
                });
            }
        }
        let selected = match input.workspace {
            WorkspaceSelection::IsolatedGit { selection } => {
                let Some(workspace) = self.isolated_workspace(selection, operation, scope).await?
                else {
                    return Ok(Created::Rejected {
                        reason: Rejection::WorkspaceUnavailable,
                    });
                };
                workspace
            }
            WorkspaceSelection::Project { project, selection } => {
                let Some(workspace) = self.select_project(project, selection).await? else {
                    return Ok(Created::Rejected {
                        reason: Rejection::WorkspaceUnavailable,
                    });
                };
                workspace
            }
            WorkspaceSelection::Directory { selection } => {
                let Some(workspace) = self.create_project_workspace(selection).await? else {
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
                if session.archived
                    || !self
                        .project_accepts_session(&session.configuration.workspace)
                        .await?
                {
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
                if let Some(parent) = session.configuration.tools {
                    tools = Some(match tools {
                        None => parent,
                        Some(names) => names
                            .into_iter()
                            .filter(|name| parent.contains(name))
                            .collect(),
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
                    tools,
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
            session: Box::new(crate::service::summary(self, record)),
        })
    }
    pub async fn mark_read(&self, input: MarkRead) -> Result<MarkReadResult, String> {
        let runs = self.runs.lock().await;
        if runs.closing {
            return Ok(MarkReadResult::Rejected {
                reason: Rejection::Closed,
            });
        }
        if !self.owns(&input.session_id) {
            return Ok(MarkReadResult::Rejected {
                reason: Rejection::SessionUnavailable,
            });
        }
        let session = self
            .store
            .get_session::<SessionConfiguration>(&input.session_id)
            .await
            .map_err(|_| "agent_store_unavailable")?;
        if session.is_none_or(|session| session.archived) {
            return Ok(MarkReadResult::Rejected {
                reason: Rejection::SessionUnavailable,
            });
        }
        // The original store compares the exposed message with the durable tail
        // inside its transaction. A stale reader cannot clear a newer message.
        let record = self
            .store
            .set_session_read_marker::<SessionConfiguration>(&input.session_id, &input.message_id)
            .await
            .map_err(|_| "agent_read_marker_failed")?;
        Ok(MarkReadResult::Observed {
            session: Box::new(crate::service::summary(self, record)),
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
                session: Box::new(crate::service::summary(self, record)),
            }),
            SessionMutation::RevisionConflict { expected, actual } => {
                Ok(MetadataResult::RevisionConflict { expected, actual })
            }
        }
    }
    pub(super) fn tool_names(&self) -> crate::contract::ToolNames {
        crate::contract::ToolNames {
            names: self.tools.names(),
        }
    }
    fn selected_tools(&self, allowed: Option<&[String]>) -> maka_tools::ToolCatalog {
        match allowed {
            None => self.tools.clone(),
            Some(allowed) => {
                let names = self.tools.names();
                let excluded: Vec<_> = names
                    .iter()
                    .filter(|name| !allowed.contains(name))
                    .map(String::as_str)
                    .collect();
                self.tools.excluding(&excluded)
            }
        }
    }
    pub async fn send(
        self: &Arc<Self>,
        input: Send,
        operation: &maka_plugin_core::operation::RecoveryRef,
        parent: &maka_plugins::call::Scope,
    ) -> Result<crate::contract::SendResult, String> {
        let Send {
            session_id: id,
            message: text,
            model: selection,
            thinking_level,
            attachments,
        } = input;
        self.submit(
            Submission {
                id,
                text,
                attachments,
                selection: Selection::Model(selection, thinking_level),
            },
            operation,
            parent,
        )
        .await
    }
    pub async fn execute(
        self: &Arc<Self>,
        input: Execute,
        operation: &maka_plugin_core::operation::RecoveryRef,
        parent: &maka_plugins::call::Scope,
    ) -> Result<SendResult, String> {
        self.submit(
            Submission {
                id: input.session_id,
                text: input.message,
                attachments: input.attachments,
                selection: Selection::Executor(input.executor),
            },
            operation,
            parent,
        )
        .await
    }
    async fn submit(
        self: &Arc<Self>,
        input: Submission,
        operation: &maka_plugin_core::operation::RecoveryRef,
        parent: &maka_plugins::call::Scope,
    ) -> Result<SendResult, String> {
        let Submission {
            id,
            text,
            attachments,
            selection,
        } = input;
        if (text.trim().is_empty() && attachments.is_empty()) || text.len() > 64 * 1024 {
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
        let backend = match selection {
            Selection::Model(selection, thinking_level) => PreparedBackend::Model(Box::new(
                crate::model::Source::select(
                    self.configuration.clone(),
                    self.credentials.clone(),
                    selection,
                    thinking_level,
                    id.clone(),
                    self.revision.clone(),
                )
                .await
                .map_err(public_error)?,
            )),
            Selection::Executor(selection) => {
                if session.configuration.tools.is_some() {
                    return Err("agent_executor_cannot_enforce_tool_selection".into());
                }
                PreparedBackend::Executor(Box::new(self.executors.select(selection)?))
            }
        };
        let (model, executor) = match &backend {
            PreparedBackend::Model(value) => (Some(value.0.accepted.clone()), None),
            PreparedBackend::Executor(value) => (None, Some(value.accepted.clone())),
        };
        workspace::validate(
            &self.workspace_roots,
            &session.configuration.workspace,
            &self.managed_worktrees,
        )
        .await?;
        let external = executor.is_some();
        let basis = MessageBasis {
            allowed_tools: session.configuration.tools.clone(),
            background: None,
            internal_tools: if external {
                vec![]
            } else {
                self.internal_tools.clone()
            },
            internal_bindings: if external {
                BTreeMap::new()
            } else {
                self.internal_bindings.clone()
            },
            behavior: self
                .capture_behavior(&session.configuration.workspace)
                .await?,
            model,
            executor,
            workspace: session.configuration.workspace.clone(),
            fingerprint: operation.fingerprint.clone(),
            tools: if external {
                vec![]
            } else {
                self.tool_basis.clone()
            },
            max_steps: self.settings.max_steps,
        };
        let identity = self.operation_id(operation);
        let invocation = queued.clone().unwrap_or_else(|| Invocation {
            session_id: id.clone(),
            turn_id: format!("t_{identity}"),
            run_id: format!("r_{identity}"),
            invocation_id: format!("i_{identity}"),
        });
        let mut message: MessageInput = text.into();
        message.attachments = self.attachment_refs(&id, attachments).await?;
        let unprepared = message;
        let (message, required_tools) = match self
            .prepare_skills(
                unprepared.clone(),
                Some(session.configuration.workspace.clone()),
                parent,
                if basis.executor.is_some() {
                    vec![]
                } else {
                    self.selected_tools(session.configuration.tools.as_deref())
                        .names()
                },
            )
            .await?
        {
            skills::Input::Ready {
                message,
                required_tools,
            } => (message, required_tools),
            skills::Input::Rejected { reason, skills } => {
                return Ok(SendResult::Rejected {
                    session_id: id,
                    reason,
                    skills,
                });
            }
        };
        let delivered = RootSourceMessage {
            message: DeliveredMessage {
                message_id: format!("m_{identity}"),
                submitted_content_digest: unprepared
                    .content_digest()
                    .map_err(|_| "agent_message_invalid")?,
                content: message.clone(),
            },
            unprepared_content: unprepared,
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
            required_tools,
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
        let input = self.input(invocation.clone(), backend, basis.clone(), delivered)?;
        self.store
            .admit_message(admission)
            .await
            .map_err(|_| "agent_admission_failed")?;
        let cancellation = self.stop.child_token();
        let running = match self.start(input, cancellation.clone()).await {
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
            let scheduler = owner.scheduler.finish().await;
            let configuration = owner.configuration.shutdown().await;
            let runs = owner.runs.lock().await;
            if !runs.quarantined.is_empty() {
                return Err("agent_cleanup_unconfirmed".into());
            }
            scheduler?;
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
/// Data selection only; service grants remain independent of this prefix.
pub(crate) fn session_namespace(
    domain: &str,
    deployment: uuid::Uuid,
    instance: &maka_plugin_core::composition::InstanceId,
    coordinator: bool,
) -> String {
    let prefix = namespace(domain, deployment, instance);
    if coordinator {
        format!("wh_{}", &prefix[3..])
    } else {
        prefix
    }
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

pub(crate) fn owns_session(prefix: &str, id: &str) -> bool {
    id.strip_prefix(prefix).is_some_and(|tail| {
        tail.len() == 32
            && tail
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
