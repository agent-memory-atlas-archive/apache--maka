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

use crate::{settings::Manager, transport::Connection};
use futures_util::future::BoxFuture;
use maka_plugins::{
    authorization, fiber,
    remote::{self, Error},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

mod auth;
mod protocol;
pub use protocol::{Action, Info, Method, Outcome, Output, Progress, execute};

pub(crate) struct Provider {
    pub manager: Arc<Manager>,
    pub context: fiber::Context,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Input {
    Check {
        agent_id: String,
        expected_revision: Option<u64>,
        #[schemars(with = "String")]
        operation_id: Uuid,
    },
    Authenticate {
        agent_id: String,
        expected_revision: Option<u64>,
        method_id: String,
        #[schemars(with = "String")]
        operation_id: Uuid,
    },
    InstallAntigravity {
        #[schemars(with = "String")]
        operation_id: Uuid,
    },
}
impl Input {
    fn operation_id(&self) -> Uuid {
        match self {
            Self::Check { operation_id, .. }
            | Self::Authenticate { operation_id, .. }
            | Self::InstallAntigravity { operation_id } => *operation_id,
        }
    }
}

impl remote::StreamProvider for Provider {
    fn open(
        &self,
        input: Value,
        caller: remote::Caller,
    ) -> BoxFuture<'static, Result<Box<dyn remote::Stream>, Error>> {
        let manager = self.manager.clone();
        let context = self.context.clone();
        Box::pin(async move {
            let input: Input =
                serde_json::from_value(input).map_err(|e| Error::Invalid(e.to_string()))?;
            if let Input::Authenticate { method_id, .. } = &input
                && (method_id.is_empty()
                    || method_id.len() > 1024
                    || method_id.chars().any(char::is_control))
            {
                return Err(Error::Invalid("Invalid authentication method".into()));
            }
            // This stream owns only its work. The authorized scope below settles
            // pending consent/processes; closing a nested stream must not cancel
            // the Remote method that is consuming its result.
            let cancellation = caller.cancellation.child_token();
            let worker_cancel = cancellation.clone();
            let (send, receive) = mpsc::channel(8);
            let done = context
                .spawn_resource("external agent setup", move |stopping| async move {
                    let result =
                        work(manager, input, caller, &send, &worker_cancel, &stopping).await;
                    let cleanup_failed = matches!(result, Err(Error::CleanupUnconfirmed));
                    if let Err(error) = result {
                        tokio::select! {
                            biased;
                            _ = worker_cancel.cancelled() => {},
                            _ = stopping.cancelled() => {},
                            _ = send.send(Err(error)) => {},
                        }
                    }
                    if cleanup_failed {
                        Err("external agent setup cleanup is unconfirmed".into())
                    } else {
                        Ok(())
                    }
                })
                .map_err(|_| Error::Retired)?;
            Ok(Box::new(SetupStream {
                receive: Mutex::new(receive),
                cancellation,
                done,
            }) as Box<dyn remote::Stream>)
        })
    }
}

struct SetupStream {
    receive: Mutex<mpsc::Receiver<Result<Value, Error>>>,
    cancellation: CancellationToken,
    done: oneshot::Receiver<Result<(), String>>,
}
impl remote::Stream for SetupStream {
    fn next(&self) -> BoxFuture<'_, Result<Option<Value>, Error>> {
        Box::pin(async move {
            tokio::select! {
                biased;
                _ = self.cancellation.cancelled() => Err(Error::Cancelled),
                item = async { self.receive.lock().await.recv().await } => item.transpose(),
            }
        })
    }
    fn cancel(&self) {
        self.cancellation.cancel();
    }
    fn close(self: Box<Self>) -> BoxFuture<'static, Result<(), Error>> {
        Box::pin(async move {
            self.cancellation.cancel();
            match tokio::time::timeout(Duration::from_secs(5), self.done).await {
                Ok(Ok(Ok(()))) => Ok(()),
                _ => Err(Error::CleanupUnconfirmed),
            }
        })
    }
}

async fn work(
    manager: Arc<Manager>,
    input: Input,
    caller: remote::Caller,
    send: &mpsc::Sender<Result<Value, Error>>,
    cancellation: &CancellationToken,
    stopping: &CancellationToken,
) -> Result<(), Error> {
    if cancellation.is_cancelled() || stopping.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let (title, agent) = match &input {
        Input::Check {
            agent_id,
            expected_revision,
            ..
        } => (
            "Check external agent",
            Some(
                manager
                    .agent_at_revision(agent_id, *expected_revision)
                    .await
                    .map_err(Error::Invalid)?,
            ),
        ),
        Input::Authenticate {
            agent_id,
            expected_revision,
            ..
        } => (
            "Authenticate external agent",
            Some(
                manager
                    .agent_at_revision(agent_id, *expected_revision)
                    .await
                    .map_err(Error::Invalid)?,
            ),
        ),
        Input::InstallAntigravity { .. } => ("Download and install Antigravity from Google", None),
    };
    let authorization = authorization::Request {
        operation_id: input.operation_id(),
        title: agent
            .as_ref()
            .map_or_else(|| title.into(), |agent| format!("{title}: {}", agent.id)),
        target: authorization::Target::PluginWorkspace {
            sandbox_mode: if agent.is_some() {
                maka_runtime::execution::SandboxMode::DangerFullAccess
            } else {
                maka_runtime::execution::SandboxMode::WorkspaceWrite
            },
        },
        capabilities: [if agent.is_some() {
            authorization::Capability::Processes
        } else {
            authorization::Capability::Network
        }]
        .into(),
    };
    let owned = tokio::select! {
        biased;
        _ = cancellation.cancelled() => return Err(Error::Cancelled),
        _ = stopping.cancelled() => {
            cancellation.cancel();
            return Err(Error::Cancelled);
        },
        result = caller.views.authorize(authorization) => result?,
    };
    let scope = owned.scope();
    if matches!(input, Input::InstallAntigravity { .. }) {
        let data = manager
            .data
            .as_ref()
            .ok_or_else(|| Error::Provider("Plugin private storage unavailable".into()));
        let result = match data {
            Ok(data) => {
                let install = crate::install::antigravity(&manager.host, data, scope, cancellation);
                tokio::pin!(install);
                tokio::select! {
                    biased;
                    _ = stopping.cancelled() => {
                        cancellation.cancel();
                        install.await
                    },
                    result = &mut install => result,
                }
                .map_err(|error| match error {
                    crate::install::Error::Cleanup => Error::CleanupUnconfirmed,
                    crate::install::Error::Cancelled => Error::Cancelled,
                    crate::install::Error::Storage(
                        maka_plugins::storage::StoreError::OutcomeUnknown(reason),
                    ) => Error::OutcomeUnknown(reason),
                    other => provider(other),
                })
            }
            Err(error) => Err(error),
        };
        owned
            .finish()
            .await
            .map_err(|_| Error::CleanupUnconfirmed)?;
        return emit(send, json!({"kind":"installed", "agent":result?})).await;
    }
    let action = match input {
        Input::Check { .. } => Action::Check,
        Input::Authenticate { method_id, .. } => Action::Authenticate { method_id },
        Input::InstallAntigravity { .. } => unreachable!(),
    };
    let output = LegacyOutput(send);
    let agent = agent.expect("process setup has an agent");
    let setup = execute(&agent, manager.host.clone().into(), scope, action, &output);
    tokio::pin!(setup);
    let result = tokio::select! { biased;
        () = cancellation.cancelled() => { owned.scope().cancellation.cancel(); setup.await },
        () = stopping.cancelled() => { owned.scope().cancellation.cancel(); setup.await },
        result = &mut setup => result,
    };
    owned
        .finish()
        .await
        .map_err(|_| Error::CleanupUnconfirmed)?;
    let event = match result? {
        Outcome::Checked { info, methods } => json!({"kind":"initialized", "agentInfo":info,
            "authMethods":methods.into_iter().map(|m| json!({"id":m.id,"name":m.name,"description":m.description,"type":if m.supported {"agent"} else {"unsupported"}})).collect::<Vec<_>>() }),
        Outcome::Authenticated => json!({"kind":"authenticated"}),
        Outcome::Unresolved { diagnostic } => return Err(Error::OutcomeUnknown(diagnostic)),
    };
    emit(send, event).await
}
struct LegacyOutput<'a>(&'a mpsc::Sender<Result<Value, Error>>);
impl Output for LegacyOutput<'_> {
    fn emit(&self, progress: Progress) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(
            async move { emit(self.0, serde_json::to_value(progress).map_err(provider)?).await },
        )
    }
}

async fn emit(send: &mpsc::Sender<Result<Value, Error>>, event: Value) -> Result<(), Error> {
    remote::validate_payload(&event)?;
    send.send(Ok(event)).await.map_err(|_| Error::Cancelled)
}
fn provider(error: impl std::fmt::Display) -> Error {
    Error::Provider(error.to_string())
}
