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

use crate::{Configuration, IMPLEMENTATION, NativeContext, configuration_schema};
use futures_util::{FutureExt, future::BoxFuture};
use maka_agent::{Engine, ModelSource, RunError, RunInput, RunWork};
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
    pub schema_version: u32,
    pub title: String,
    pub model: crate::ModelSelection,
    pub provider: String,
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
    definition: Arc<maka_plugins::provider::Definition>,
    revision: SourceRevision,
    cwd: String,
    engine: Engine,
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
        let root = persistence
            .store()
            .owned_root()
            .map_err(|_| "agent_root_unavailable")?;
        let prefix = namespace(
            &persistence.source().state_domain,
            persistence.source().deployment,
            context.activation().instance(),
        );
        let cwd = root
            .canonical_path()
            .to_str()
            .ok_or("agent_root_path_invalid")?
            .to_owned();
        let definition = Arc::new(
            maka_providers::api::ApiProviders::definition(&settings.provider)
                .map_err(|_| "agent_provider_unavailable")?,
        );
        let models = ModelExecutor::new(1, Duration::from_secs(60))
            .map_err(|_| "agent_executor_unavailable")?;
        let cells = maka_js_runtime::CodeExecutor::new(1, maka_js_runtime::CellLimits::default())
            .map_err(|_| "agent_executor_unavailable")?;
        let engine = Engine::new(persistence.store().clone(), models, cells);
        let revision = SourceRevision {
            kind: SourceKind::ModelProvider,
            name: settings.provider.clone(),
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
            cwd,
            definition,
            revision,
            permits: Arc::new(Semaphore::new(settings.max_active_runs)),
            settings,
            configuration,
            engine,
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
    pub async fn send(
        self: &Arc<Self>,
        id: String,
        text: String,
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
        if session.archived
            || session.configuration.schema_version != 1
            || session.configuration.model != self.settings.model
            || session.configuration.provider != self.settings.provider
        {
            return Err("agent_session_configuration_changed".into());
        }
        let source = Arc::new(crate::model::Source {
            configuration: self.configuration.clone(),
            settings: self.settings.clone(),
            definition: self.definition.clone(),
            session: id.clone(),
            revision: self.revision.clone(),
        });
        let prepared = source.capture().await.map_err(public_error)?;
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
                cwd: self.cwd.clone(),
                workspace_identity: None,
                workspace_origin: WorkspaceOrigin::Selected,
                sandbox_mode: SandboxMode::WorkspaceWrite,
                approval_policy: ApprovalPolicy::OnRequest,
                boundary_revision: 0,
                collaboration_mode: CollaborationMode::Agent,
                orchestration_mode: BehaviorId::default(),
                tool_mode: ToolMode::Direct,
                model: Some(self.settings.model.binding()),
                thinking_level: None,
                system_prompt: None,
                tool_composition: None,
            },
            work: RunWork::Message {
                message,
                source_messages: vec![delivered],
                tools: maka_tools::ToolCatalog::new([]).expect("empty tool catalog"),
                max_steps: self.settings.max_steps,
                allow_prior_unknown: false,
            },
        };
        let running = match self.engine.start(input, self.stop.clone()).await {
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
                cancellation: running.cancellation(),
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
