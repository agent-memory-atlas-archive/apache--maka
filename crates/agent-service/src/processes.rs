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

//! Explicit native process service; request data never supplies launch authority.
use crate::owner::SessionConfiguration;
use futures_util::future::BoxFuture;
use maka_config::ConfigurationStore;
use maka_event_log::EventLog;
use maka_plugin_core::contract::{Contract, Interface, Method, MethodMode, Transport};
use maka_runtime::tools::{ToolCallContext, ToolError};
use maka_runtime_host::{
    application::{Context, Handler, NativeActivation, NativeContext, NativeOwner, NativeRegistry},
    shell::{ShellError, ShellResources, request::Execute},
};
use maka_sandbox::{Network, Sandbox, filesystem::Policy};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;

mod control;
mod execute;
pub mod protocol;
pub mod terminal;

pub const IMPLEMENTATION: &str = "maka.workspace.processes";
pub const EXPORT: &str = "processes";

/// Immutable installation ceiling selected by trusted native composition.
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "WorkspaceProcessConfiguration")]
pub struct Configuration {
    /// Allow the selected native Session's verified owned worktree, within its
    /// own access policy. Other Host-private paths remain denied.
    #[serde(default)]
    pub isolated_workspaces: bool,
    pub filesystem: Policy,
    pub network: Network,
    /// Trusted CLI supporting the original platform sandbox bootstrap.
    pub helper: Option<PathBuf>,
    /// Explicit directory for application setup processes. Launch still needs
    /// its own grant and uses only the installation filesystem/network ceiling.
    #[serde(default)]
    pub setup_directory: Option<PathBuf>,
    /// Trusted Agent Session owner in this Root/deployment. This selects a data
    /// namespace only; Terminal endpoints still require their own explicit grants.
    #[serde(default)]
    pub session_owner: Option<maka_plugin_core::composition::InstanceId>,
    /// Explicit data namespace of the selected Session owner.
    #[serde(default)]
    pub session_kind: SessionKind,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "WorkspaceProcessSessionKind")]
pub enum SessionKind {
    #[default]
    Agent,
    Workhub,
}
pub fn configuration_schema() -> Value {
    schemars::schema_for!(Configuration).into()
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "WorkspaceProcessResult")]
pub enum ProcessResult {
    Completed { value: Value },
    Failed { message: String },
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Reference {
    #[serde(rename = "ref")]
    reference: String,
}
pub fn contract() -> Contract {
    let methods = [
        (
            "execute",
            MethodMode::Mutation,
            schemars::schema_for!(Execute).into(),
        ),
        (
            "read",
            MethodMode::Query,
            schemars::schema_for!(Reference).into(),
        ),
        (
            "writeStdin",
            MethodMode::Mutation,
            maka_runtime_host::shell::stdin::schema(),
        ),
        (
            "stop",
            MethodMode::Mutation,
            schemars::schema_for!(Reference).into(),
        ),
    ]
    .into_iter()
    .map(|(name, mode, input)| {
        (
            name.into(),
            Method {
                progress: None,
                mode,
                input,
                output: schemars::schema_for!(ProcessResult).into(),
                resources: BTreeMap::new(),
            },
        )
    })
    .collect();
    Contract {
        name: "maka.workspace.Processes".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service { methods },
    }
}
pub(crate) fn provide(registry: &mut NativeRegistry) -> Result<(), String> {
    protocol::provide(registry)?;
    registry.insert(
        IMPLEMENTATION.into(),
        Arc::new(|context, input| {
            Box::pin(async move {
                match Processes::open(context, input).await {
                    Ok(owner) => NativeActivation {
                        exports: Ok(BTreeMap::from([
                            (
                                EXPORT.into(),
                                maka_plugin_core::runtime::Published {
                                    contract: contract(),
                                    value: Arc::new(Service(owner.clone())) as Arc<dyn Handler>,
                                },
                            ),
                            (
                                terminal::EXPORT.into(),
                                maka_plugin_core::runtime::Published {
                                    contract: terminal::contract(),
                                    value: Arc::new(terminal::Service(owner.clone()))
                                        as Arc<dyn Handler>,
                                },
                            ),
                        ])),
                        owner: Arc::new(Service(owner)),
                    },
                    Err(error) => NativeActivation {
                        exports: Err(error),
                        owner: Arc::new(crate::Uninitialized),
                    },
                }
            })
        }),
    )
}
struct Processes {
    store: Arc<EventLog>,
    configuration: Arc<ConfigurationStore>,
    settings: Configuration,
    state_root: PathBuf,
    prefix: String,
    session_prefix: Option<String>,
    controllers: Arc<std::sync::Mutex<Vec<terminal::Lease>>>,
    resources: ShellResources,
    drain: CancellationToken,
    protocol: protocol::Controls,
    setup: Option<maka_process::ShellExecutor>,
}
impl Processes {
    async fn open(context: NativeContext, input: Value) -> Result<Arc<Self>, String> {
        if !serde_json::to_vec(&input).is_ok_and(|b| b.len() <= 64 * 1024)
            || !jsonschema::validator_for(&configuration_schema())
                .expect("process configuration schema")
                .is_valid(&input)
        {
            return Err("workspace_process_configuration_invalid".into());
        }
        let settings: Configuration =
            serde_json::from_value(input).map_err(|_| "workspace_process_configuration_invalid")?;
        if settings.session_owner.as_ref().is_some_and(|id| {
            id.0.is_empty() || id.0.len() > 32 || id.0.iter().any(uuid::Uuid::is_nil)
        }) {
            return Err("workspace_process_session_owner_invalid".into());
        }
        settings
            .filesystem
            .compile()
            .map_err(|_| "workspace_process_policy_invalid")?;
        maka_sandbox::grant::Permissions {
            filesystem: vec![],
            network: settings.network.clone(),
        }
        .validate()
        .map_err(|_| "workspace_process_network_invalid")?;
        if settings
            .helper
            .as_ref()
            .is_some_and(|p| !p.is_absolute() || !p.is_file())
        {
            return Err("workspace_process_helper_invalid".into());
        }
        #[cfg(windows)]
        if settings.helper.is_none() {
            return Err("workspace_process_helper_required".into());
        }
        let persistence = context
            .persistence()
            .ok_or("workspace_processes_require_persistence")?;
        let root = persistence
            .store()
            .owned_root()
            .map_err(|_| "workspace_process_root_unavailable")?;
        let state_root: PathBuf =
            maka_fs_tools::workspace::project::host_path(root.canonical_path())
                .map_err(|_| "workspace_process_root_unavailable")?
                .into();
        let setup = settings
            .setup_directory
            .as_ref()
            .map(|path| {
                if !path.is_absolute() {
                    return Err("protocol_setup_directory_invalid".to_owned());
                }
                let (path, identity) =
                    maka_fs_tools::directory::capture(path).map_err(|e| e.to_string())?;
                let directory =
                    maka_fs_tools::directory::open(&path, &identity).map_err(|e| e.to_string())?;
                let private = Sandbox::Managed {
                    filesystem: Policy {
                        default: maka_sandbox::filesystem::Access::Write,
                        rules: vec![maka_sandbox::filesystem::Rule::subtree(
                            &state_root,
                            maka_sandbox::filesystem::Access::Deny,
                        )],
                        deny_globs: vec![],
                    },
                    network: Network::Allowed,
                };
                let sandbox = Sandbox::Managed {
                    filesystem: settings.filesystem.clone(),
                    network: settings.network.clone(),
                }
                .intersect(&private)
                .map_err(|e| e.to_string())?;
                let executor = maka_process::ShellExecutor::from_directory(
                    path,
                    directory.into_std_file(),
                    sandbox,
                )
                .map_err(|e| e.to_string())?;
                Ok(execute::platform(executor, &settings, &state_root))
            })
            .transpose()?;
        let prefix = crate::owner::namespace(
            &persistence.source().state_domain,
            persistence.source().deployment,
            context.activation().instance(),
        );
        let session_prefix = settings.session_owner.as_ref().map(|instance| {
            crate::owner::session_namespace(
                &persistence.source().state_domain,
                persistence.source().deployment,
                instance,
                matches!(settings.session_kind, SessionKind::Workhub),
            )
        });
        let drain = CancellationToken::new();
        let resources = ShellResources::new(persistence.store().clone(), drain.clone());
        let configuration = Arc::new(
            ConfigurationStore::for_root(root)
                .await
                .map_err(|_| "workspace_process_configuration_unavailable")?,
        );
        Ok(Arc::new(Self {
            store: persistence.store().clone(),
            configuration,
            settings,
            state_root,
            prefix,
            session_prefix,
            controllers: Arc::default(),
            resources,
            drain,
            protocol: Default::default(),
            setup,
        }))
    }
}
struct Service(Arc<Processes>);
impl Handler for Service {
    fn call(
        &self,
        name: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let owner = self.0.clone();
        Box::pin(async move {
            if !matches!(name.as_str(), "execute" | "read" | "writeStdin" | "stop") {
                return Err("workspace_process_method_unavailable".into());
            }
            let invocation = context
                .scope()
                .identity
                .agent()
                .ok_or("workspace_processes_require_agent")?
                .clone();
            let operation_id = context
                .scope()
                .identity
                .operation_id()
                .ok_or("workspace_processes_require_tool")?
                .into();
            if name != "read" && context.operation().is_none() {
                return Err("workspace_process_mutation_required".into());
            }
            let tool = ToolCallContext {
                invocation,
                operation_id,
            };
            let mut ticket = context
                .scope()
                .resources
                .reserve()
                .map_err(|e| e.to_string())?;
            ticket.start();
            let result = owner
                .call(&name, input, &tool, &context.scope().cancellation)
                .await;
            let cleanup = match &result {
                Err(ToolError::CleanupUnconfirmed(message)) => {
                    owner.drain.cancel();
                    Err(message.clone())
                }
                _ => Ok(()),
            };
            ticket.complete(cleanup);
            let result = match result {
                Ok(value) => ProcessResult::Completed { value },
                Err(ToolError::Failed(message) | ToolError::Io { message, .. }) => {
                    ProcessResult::Failed { message }
                }
                Err(error) => return Err(error.to_string()),
            };
            serde_json::to_value(result).map_err(|_| "workspace_process_result_unavailable".into())
        })
    }
}
impl NativeOwner for Service {
    fn prepare_retirement(
        &self,
    ) -> Option<Box<dyn maka_runtime_host::application::NativeRetirement>> {
        (self.0.resources.active_count() == 0 && !self.0.drain.is_cancelled())
            .then(|| Box::new(()) as _)
    }
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        let owner = self.0.clone();
        Box::pin(async move {
            owner.resources.shutdown().await;
            let configuration = owner.configuration.shutdown().await;
            if owner.drain.is_cancelled() {
                return Err("workspace_process_cleanup_unconfirmed".into());
            }
            configuration.map_err(|_| "workspace_process_configuration_cleanup_unconfirmed".into())
        })
    }
}
impl Processes {
    async fn call(
        &self,
        name: &str,
        input: Value,
        tool: &ToolCallContext,
        cancellation: &CancellationToken,
    ) -> Result<Value, ToolError> {
        cancelled(cancellation)?;
        let configuration = crate::permissions::live(&self.store, &tool.invocation).await?;
        if name == "execute" {
            if self.drain.is_cancelled() {
                return Err(failed("Process owner is draining"));
            }
            self.execute(Execute::parse(input)?, tool, configuration, cancellation)
                .await
        } else {
            self.control(name, input, tool, cancellation).await
        }
    }
}
fn cancelled(cancellation: &CancellationToken) -> Result<(), ToolError> {
    if cancellation.is_cancelled() {
        Err(failed("Process operation cancelled"))
    } else {
        Ok(())
    }
}
fn failed(error: impl std::fmt::Display) -> ToolError {
    ToolError::Failed(error.to_string())
}
fn persistence(error: impl std::fmt::Display) -> ToolError {
    ToolError::Persistence(error.to_string())
}
fn worker_error(error: Arc<ShellError>) -> ToolError {
    match error.as_ref() {
        ShellError::CancelledBeforeAdmission => failed(error),
        ShellError::Store(error) => persistence(error),
        ShellError::Process(error) => error.clone(),
        _ => ToolError::CleanupUnconfirmed(error.to_string()),
    }
}
