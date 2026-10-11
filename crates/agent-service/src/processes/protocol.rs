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

//! Exact bidirectional protocol transport. The run mutation owns the process
//! until native exit; its progress reader is neither a handle lease nor a receipt.
use super::*;
use maka_plugin_core::runtime::Activation;
use maka_runtime::interaction::{PermissionCommand, PermissionRequest};
use std::sync::Mutex;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

mod worker;
pub const IMPLEMENTATION: &str = "maka.workspace.protocol-processes";
pub const EXPORT: &str = "protocol";
const CHUNK_BYTES: usize = 8192;

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "ProtocolProcessRun")]
pub struct Run {
    #[schemars(length(min = 1, max = 4096))]
    pub executable: String,
    #[serde(default)]
    #[schemars(length(max = 256))]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub additional_permissions: Option<maka_sandbox::grant::Permissions>,
    #[schemars(length(min = 1, max = 2048))]
    pub justification: Option<String>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "ProtocolProcessInput")]
pub struct Input {
    pub id: Uuid,
    /// Null closes stdin. Empty data is a successful zero-byte write.
    #[schemars(length(max = CHUNK_BYTES))]
    pub bytes: Option<Vec<u8>>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "ProtocolProcessProgress")]
pub enum Progress {
    Started {
        id: Uuid,
    },
    Output {
        stream: Stream,
        #[schemars(length(max = CHUNK_BYTES))]
        bytes: Vec<u8>,
    },
}
#[derive(Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "ProtocolProcessStream")]
pub enum Stream {
    Stdout,
    Stderr,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "ProtocolProcessCompletion")]
pub enum Completion {
    Exited {
        code: Option<i32>,
        success: bool,
        stopped: bool,
        error: Option<String>,
    },
    Failed {
        message: String,
    },
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "ProtocolProcessWritten")]
pub struct Written {
    pub bytes: usize,
    pub closed: bool,
}

pub fn contract() -> Contract {
    Contract {
        name: "maka.workspace.ProtocolProcesses".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([
                (
                    "run".into(),
                    Method {
                        mode: MethodMode::Mutation,
                        input: schemars::schema_for!(Run).into(),
                        output: schemars::schema_for!(Completion).into(),
                        progress: Some(schemars::schema_for!(Progress).into()),
                        resources: BTreeMap::new(),
                    },
                ),
                (
                    "input".into(),
                    Method {
                        mode: MethodMode::Mutation,
                        input: schemars::schema_for!(Input).into(),
                        output: schemars::schema_for!(Written).into(),
                        progress: None,
                        resources: BTreeMap::new(),
                    },
                ),
            ]),
        },
    }
}
pub(super) fn provide(registry: &mut NativeRegistry) -> Result<(), String> {
    registry.insert(
        IMPLEMENTATION.into(),
        Arc::new(|context, input| {
            Box::pin(async move {
                match Processes::open(context, input).await {
                    Ok(owner) => NativeActivation {
                        exports: Ok(BTreeMap::from([(
                            EXPORT.into(),
                            maka_plugin_core::runtime::Published {
                                contract: contract(),
                                value: Arc::new(Service(owner.clone())) as Arc<dyn Handler>,
                            },
                        )])),
                        owner: Arc::new(super::Service(owner)),
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
struct Write {
    bytes: Option<Vec<u8>>,
    reply: oneshot::Sender<Result<Written, String>>,
}
struct Control {
    consumer: Activation,
    session: String,
    configuration: SessionConfiguration,
    sandbox: Sandbox,
    input: mpsc::Sender<Write>,
}
#[derive(Default)]
pub(super) struct Controls(Mutex<BTreeMap<Uuid, Arc<Control>>>);
pub(super) struct Service(pub(super) Arc<Processes>);
impl Handler for Service {
    fn call(
        &self,
        method: String,
        value: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let owner = self.0.clone();
        Box::pin(async move {
            let invocation = context
                .scope()
                .identity
                .agent()
                .ok_or("protocol_requires_agent")?
                .clone();
            let operation_id = context
                .scope()
                .identity
                .operation_id()
                .ok_or("protocol_requires_operation")?
                .into();
            context.operation().ok_or("protocol_requires_mutation")?;
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
            let result = match method.as_str() {
                "run" => match serde_json::from_value(value) {
                    Ok(input) => owner
                        .run(input, &tool, &context)
                        .await
                        .and_then(|v| serde_json::to_value(v).map_err(failed)),
                    Err(error) => Err(failed(error)),
                },
                "input" => match serde_json::from_value(value) {
                    Ok(input) => owner
                        .protocol_input(input, &tool, &context)
                        .await
                        .and_then(|v| serde_json::to_value(v).map_err(failed)),
                    Err(error) => Err(failed(error)),
                },
                _ => Err(failed("protocol_method_unavailable")),
            };
            let cleanup = match &result {
                Err(ToolError::CleanupUnconfirmed(message)) => {
                    owner.drain.cancel();
                    Err(message.clone())
                }
                _ => Ok(()),
            };
            ticket.complete(cleanup);
            result.map_err(|e| e.to_string())
        })
    }
}
impl Processes {
    async fn run(
        &self,
        input: Run,
        tool: &ToolCallContext,
        context: &Context,
    ) -> Result<Completion, ToolError> {
        cancelled(&context.scope().cancellation)?;
        if self.drain.is_cancelled() {
            return Err(failed("Process owner is draining"));
        }
        let command = maka_plugins::process::Command {
            executable: input.executable,
            args: input.args,
            env: input.env,
            lifetime: maka_plugins::process::Lifetime::Invocation,
        };
        command.validate().map_err(failed)?;
        if input.additional_permissions.is_some()
            && input
                .justification
                .as_ref()
                .is_none_or(|s| s.trim().is_empty() || s.len() > 2048)
        {
            return Err(failed(
                "Protocol process permissions require a justification",
            ));
        }
        let configuration = crate::permissions::live(&self.store, &tool.invocation).await?;
        let requested = input
            .additional_permissions
            .map(|permissions| PermissionRequest {
                reason: input.justification.expect("validated justification"),
                command: Some(PermissionCommand {
                    command: serde_json::to_string(&(&command.executable, &command.args))
                        .expect("command"),
                    cwd: configuration.workspace.path.clone(),
                }),
                permissions,
            });
        let captured = self
            .prepare(
                tool,
                &configuration,
                requested,
                &context.scope().cancellation,
            )
            .await?;
        let sandbox = captured.executor.sandbox().clone();
        let plan = tokio::task::spawn_blocking(move || {
            captured
                .executor
                .command_program(command.executable, &command.args, &command.env)
        })
        .await
        .map_err(failed)??;
        let id = Uuid::new_v4();
        let (input, receive) = mpsc::channel(8);
        let control = Arc::new(Control {
            consumer: context.requester().clone(),
            session: tool.invocation.session_id.clone(),
            configuration: configuration.clone(),
            sandbox,
            input,
        });
        {
            let mut controls = self.protocol.0.lock().unwrap();
            if controls.len() >= 32 {
                return Err(failed("Protocol process capacity exhausted"));
            }
            controls.insert(id, control);
        }
        let result = async {
            // The same SQL transaction checks the live invocation and captured
            // configuration, and retains Session removal until actual cleanup.
            self.store
                .admit_session_process_for(&tool.invocation, id, &configuration)
                .await
                .map_err(persistence)?;
            let result = worker::run(self, context, id, plan, receive, &configuration).await;
            if !matches!(&result, Err(ToolError::CleanupUnconfirmed(_))) {
                self.store
                    .clean_session_process(id)
                    .await
                    .map_err(|e| ToolError::CleanupUnconfirmed(e.to_string()))?;
            }
            result
        }
        .await;
        self.protocol.0.lock().unwrap().remove(&id);
        result
    }
    async fn protocol_input(
        &self,
        input: Input,
        tool: &ToolCallContext,
        context: &Context,
    ) -> Result<Written, ToolError> {
        if input.bytes.as_ref().is_some_and(|b| b.len() > CHUNK_BYTES) {
            return Err(failed("Protocol input exceeds 8 KiB"));
        }
        let control = self
            .protocol
            .0
            .lock()
            .unwrap()
            .get(&input.id)
            .cloned()
            .ok_or_else(|| failed("Protocol process unavailable"))?;
        if &control.consumer != context.requester() || control.session != tool.invocation.session_id
        {
            return Err(failed(
                "Protocol process belongs to another caller or Session",
            ));
        }
        let capacity = tokio::select! {
            biased;
            () = context.scope().cancellation.cancelled() => return Err(failed("Protocol input cancelled")),
            capacity = control.input.clone().reserve_owned() => capacity.map_err(failed)?,
        };
        let current = crate::permissions::live(&self.store, &tool.invocation).await?;
        if current.boundary_revision != control.configuration.boundary_revision
            || current.workspace != control.configuration.workspace
        {
            return Err(failed("Protocol process permissions changed"));
        }
        let captured = self
            .prepare(tool, &current, None, &context.scope().cancellation)
            .await?;
        if !captured
            .executor
            .sandbox()
            .contains(&control.sandbox)
            .map_err(failed)?
        {
            return Err(failed(
                "Protocol process launch permissions are no longer available",
            ));
        }
        let latest = crate::permissions::live(&self.store, &tool.invocation).await?;
        if latest.boundary_revision != current.boundary_revision
            || latest.workspace != current.workspace
        {
            return Err(failed(
                "Protocol process permissions changed during input admission",
            ));
        }
        cancelled(&context.scope().cancellation)?;
        let (reply, result) = oneshot::channel();
        capacity.send(Write {
            bytes: input.bytes,
            reply,
        });
        // Once enqueued, await the writer even on cancellation: a partial native
        // write is not a safe refusal and must retain the original operation.
        result.await.map_err(failed)?.map_err(failed)
    }
}
