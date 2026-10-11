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

//! Explicit Executor services, driven by the original Agent engine.
use futures_util::{FutureExt, future::BoxFuture};
use maka_event_log::EventLog;
use maka_plugin_core::{
    contract::{Contract, Interface, Method, MethodMode, Transport},
    operation::{OperationCleanup, OperationOutcome, Receipt},
};
use maka_plugins::{
    call::{Attachments, Identity},
    executor::{self, Admitted, Error, Outcome, OutputSink, Request, Settlement},
};
use maka_runtime_host::application::{Binding, NativeContext, NativeServices};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, panic::AssertUnwindSafe, sync::Arc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentServiceExecutor")]
pub struct ServiceExecutor {
    #[schemars(regex(pattern = "^[a-zA-Z][a-zA-Z0-9._:-]{0,127}$"))]
    pub name: String,
    #[schemars(length(min = 1, max = 256))]
    pub import: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentExecutorSelection")]
pub struct Selection {
    #[schemars(length(min = 1, max = 128))]
    pub executor: String,
    pub settings: maka_runtime::executor::Settings,
}
/// Observation and text only; no path, invocation or conversation key is authority.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "ExecutorExecute")]
pub struct Execute {
    pub invocation: maka_runtime::event::Invocation,
    #[schemars(length(min = 1, max = 256))]
    pub conversation_key: String,
    #[schemars(length(max = 65536))]
    pub message: String,
    #[schemars(length(min = 1, max = 4096))]
    pub cwd: String,
    #[schemars(length(max = 65536))]
    pub instructions: Option<String>,
    pub settings: maka_runtime::executor::Settings,
}
pub fn contract() -> Contract {
    Contract {
        name: "maka.agent.Executor".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([(
                "execute".into(),
                Method {
                    progress: Some(schemars::schema_for!(maka_runtime::executor::Output).into()),
                    mode: MethodMode::Mutation,
                    input: serde_json::to_value(schemars::schema_for!(Execute))
                        .expect("executor input"),
                    output: serde_json::to_value(schemars::schema_for!(Outcome))
                        .expect("executor output"),
                    resources: BTreeMap::new(),
                },
            )]),
        },
    }
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Accepted {
    pub selection: Selection,
    import: String,
    basis: String,
}
#[derive(Clone)]
pub(crate) struct Backend {
    services: NativeServices,
    store: Arc<EventLog>,
    deployment: Uuid,
    consumer: maka_plugin_core::composition::InstanceId,
    binding: Binding,
    pub accepted: Accepted,
}
#[derive(Default)]
pub(crate) struct Executors(BTreeMap<String, Backend>);
impl Executors {
    pub fn capture(context: &NativeContext, selected: &[ServiceExecutor]) -> Result<Self, String> {
        let mut result = BTreeMap::new();
        for selection in selected {
            maka_runtime::executor::ExecutorId::try_from(selection.name.clone())
                .map_err(|_| "agent_executor_name_invalid")?;
            let services = context.services();
            let bindings = services
                .imports()
                .find(|(name, _)| *name == selection.import)
                .map(|(_, bindings)| bindings)
                .ok_or("agent_executor_import_missing")?;
            let [binding] = bindings else {
                return Err("agent_executor_import_ambiguous".into());
            };
            if binding.contract() != &contract() {
                return Err("agent_executor_contract_invalid".into());
            }
            let persistence = context.persistence().ok_or("agent_requires_persistence")?;
            let backend = Backend {
                binding: binding.clone(),
                accepted: Accepted {
                    selection: Selection {
                        executor: selection.name.clone(),
                        settings: Default::default(),
                    },
                    import: selection.import.clone(),
                    basis: binding.execution_basis().into(),
                },
                services: services.clone(),
                store: persistence.store().clone(),
                deployment: persistence.source().deployment,
                consumer: context.activation().instance().clone(),
            };
            if result.insert(selection.name.clone(), backend).is_some() {
                return Err("agent_executor_name_ambiguous".into());
            }
        }
        Ok(Self(result))
    }
    pub fn select(&self, selection: Selection) -> Result<Backend, String> {
        selection
            .settings
            .validate()
            .map_err(|_| "agent_executor_settings_invalid")?;
        let mut backend = self
            .0
            .get(&selection.executor)
            .cloned()
            .ok_or("agent_executor_unavailable")?;
        backend.accepted.selection = selection;
        Ok(backend)
    }
    pub fn restore(&self, accepted: &Accepted) -> Result<Backend, String> {
        let backend = self.select(accepted.selection.clone())?;
        if &backend.accepted != accepted {
            return Err("agent_executor_changed".into());
        }
        Ok(backend)
    }
}
impl executor::Backend for Backend {
    fn admit_execution(&self, input: Request) -> Result<Admitted, Error> {
        input
            .settings
            .validate()
            .map_err(|error| Error::Invalid(error.into()))?;
        if input.settings != self.accepted.selection.settings
            || input
                .content
                .attachments
                .as_ref()
                .is_some_and(|items| !items.is_empty())
            || input
                .content
                .quotes
                .as_ref()
                .is_some_and(|items| !items.is_empty())
            || input
                .content
                .directory_references
                .as_ref()
                .is_some_and(|items| !items.is_empty())
        {
            return Err(Error::Invalid(
                "executor service requires owned text input".into(),
            ));
        }
        let identity = maka_runtime::executor::Binding {
            executor_id: self
                .accepted
                .selection
                .executor
                .clone()
                .try_into()
                .map_err(|error: &str| Error::Invalid(error.into()))?,
            package_id: self.binding.package().id.clone(),
            entry_id: "host".into(),
            activation: self.binding.provider().epoch().to_string(),
        };
        let operation = Uuid::from_bytes(
            Sha256::digest(
                serde_json::to_vec(&("maka.agent.executor", &input.invocation))
                    .expect("executor identity"),
            )[..16]
                .try_into()
                .expect("UUID digest"),
        );
        let body = Execute {
            invocation: input.invocation.clone(),
            conversation_key: input.conversation_key,
            message: input.content.text,
            cwd: input.cwd,
            instructions: input.instructions,
            settings: input.settings,
        };
        let value =
            serde_json::to_value(body).map_err(|error| Error::Invalid(error.to_string()))?;
        if !jsonschema::validator_for(
            &serde_json::to_value(schemars::schema_for!(Execute)).expect("executor input"),
        )
        .expect("executor schema")
        .is_valid(&value)
        {
            return Err(Error::Invalid("executor input is invalid".into()));
        }
        Ok(Admitted {
            identity,
            execution: Box::new(Call {
                backend: self.clone(),
                input: value,
                invocation: input.invocation,
                operation,
            }),
        })
    }
}
struct Call {
    backend: Backend,
    input: Value,
    invocation: maka_runtime::event::Invocation,
    operation: Uuid,
}
impl executor::Execution for Call {
    fn execute(
        self: Box<Self>,
        output: Arc<dyn OutputSink>,
        cancellation: CancellationToken,
    ) -> BoxFuture<'static, Settlement> {
        Box::pin(async move {
            let request = match self.backend.services.request(
                Identity::Agent {
                    invocation: self.invocation.clone(),
                    operation_id: None,
                },
                cancellation,
            ) {
                Ok(request) => request,
                Err(error) => return Settlement::new(Err(Error::Provider(error)), ()),
            };
            let mut result = {
                let business = async {
                    let original = self
                        .backend
                        .store
                        .application_intent(
                            self.backend.deployment,
                            self.backend.consumer.clone(),
                            self.operation,
                        )
                        .await
                        .map_err(|error| Error::Persistence(error.to_string()))?;
                    let receipt = if let Some(original) = original {
                        // An earlier uncertain call is observed at its actual source; never resubmit it.
                        request
                            .recover(original.reference)
                            .map_err(Error::Provider)?
                            .await
                            .map_err(Error::Provider)?
                            .map(|observation| Receipt::Recorded {
                                outcome: observation.outcome,
                                cleanup: observation.cleanup,
                            })
                            .ok_or(Error::Unknown)?
                    } else {
                        let mut mutation = request
                            .begin_mutation(
                                &self.backend.accepted.import,
                                0,
                                "execute".into(),
                                self.input,
                                self.operation,
                                Attachments::default(),
                            )
                            .map_err(Error::Provider)?
                            .await
                            .map_err(Error::Provider)?;
                        while let Some(value) = mutation.recv().await {
                            let observed = serde_json::from_value(value)
                                .map_err(|error| Error::Invalid(error.to_string()))?;
                            output.emit(observed).await?;
                        }
                        let completed = mutation.completion().await;
                        if completed.cleanup.is_err() {
                            return Err(Error::CleanupUnconfirmed);
                        }
                        completed.reply.receipt
                    };
                    match receipt {
                        Receipt::Recorded {
                            outcome: OperationOutcome::Returned { value },
                            cleanup: OperationCleanup::Confirmed,
                        } => serde_json::from_value(value)
                            .map_err(|error| Error::Invalid(error.to_string())),
                        Receipt::Recorded {
                            cleanup: OperationCleanup::Unconfirmed | OperationCleanup::Failed { .. },
                            ..
                        } => Err(Error::CleanupUnconfirmed),
                        _ => Err(Error::Unknown),
                    }
                };
                let mut business = Box::pin(AssertUnwindSafe(business).catch_unwind());
                tokio::select! {
                    biased;
                    result = &mut business => result.unwrap_or(Err(Error::Unknown)),
                    () = request.scope().cancellation.cancelled() => {
                        if request.finish().await.is_err() { Err(Error::CleanupUnconfirmed) }
                        else { let _ = business.await; Err(Error::Cancelled) }
                    }
                }
            };
            if request.finish().await.is_err() {
                result = Err(Error::CleanupUnconfirmed);
            }
            Settlement::new(result, request)
        })
    }
}
