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
pub use maka_external_agent::setup::{Action, Outcome, Progress};
use maka_runtime_host::application::Context;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "ExecutorSetupBinding")]
pub struct Binding {
    #[schemars(length(min = 1, max = 256))]
    pub import: String,
    #[serde(default)]
    pub member: usize,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "AgentExecutorSetup")]
pub struct Input {
    #[schemars(length(min = 1, max = 128))]
    pub executor: String,
    pub action: Action,
}
pub fn contract() -> Contract {
    Contract {
        name: "maka.agent.ExecutorSetup".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([(
                "setup".into(),
                Method {
                    mode: MethodMode::Mutation,
                    input: schemars::schema_for!(Action).into(),
                    output: schemars::schema_for!(Outcome).into(),
                    progress: Some(schemars::schema_for!(Progress).into()),
                    resources: BTreeMap::new(),
                },
            )]),
        },
    }
}
impl Executors {
    pub(crate) async fn setup(&self, input: Input, context: Context) -> Result<Value, String> {
        if !matches!(context.scope().identity, Identity::Remote { .. }) {
            return Err("executor_setup_requires_application_request".into());
        }
        let operation = context
            .operation()
            .ok_or("executor_setup_requires_mutation")?
            .operation;
        let backend = self
            .0
            .get(&input.executor)
            .ok_or("agent_executor_unavailable")?;
        let setup = backend
            .setup
            .as_ref()
            .ok_or("agent_executor_setup_unavailable")?;
        let request = backend.services.request(
            Identity::Remote {
                request_id: operation,
            },
            context.scope().cancellation.clone(),
        )?;
        let mut ticket = context
            .scope()
            .resources
            .reserve()
            .map_err(|e| e.to_string())?;
        ticket.start();
        let result =
            async {
                let mut mutation = request
                    .begin_mutation(
                        &setup.import,
                        setup.member,
                        "setup".into(),
                        serde_json::to_value(input.action).map_err(|e| e.to_string())?,
                        operation,
                        maka_plugins::call::Attachments::default(),
                    )?
                    .await?;
                while let Some(value) = mutation.recv().await {
                    context.emit(value).await?;
                }
                let completed = mutation.completion().await;
                completed.cleanup?;
                match completed.reply.receipt {
                    Receipt::Recorded {
                        outcome: OperationOutcome::Returned { value },
                        cleanup: OperationCleanup::Confirmed,
                    } => Ok(value),
                    Receipt::Recorded {
                        outcome: OperationOutcome::Unknown { diagnostic },
                        cleanup: OperationCleanup::Confirmed,
                    } => serde_json::to_value(Outcome::Unresolved {
                        diagnostic: diagnostic.chars().take(2048).collect(),
                    })
                    .map_err(|e| e.to_string()),
                    _ => serde_json::to_value(Outcome::Unresolved {
                        diagnostic:
                            "Original setup outcome is unconfirmed; do not repeat authentication"
                                .into(),
                    })
                    .map_err(|e| e.to_string()),
                }
            }
            .await;
        let cleanup = request.finish().await;
        ticket.complete(cleanup.clone());
        cleanup?;
        result
    }
}
