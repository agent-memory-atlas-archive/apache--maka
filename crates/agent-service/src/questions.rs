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

//! Explicitly bound question producer. Canonical records belong to the Run;
//! the original admitted Host call owns waiting and durable withdrawal.
use futures_util::future::BoxFuture;
use maka_event_log::EventLog;
use maka_plugin_core::contract::{Contract, Interface, Method, MethodMode, Transport};
use maka_runtime::interaction::{
    ClosureReason, InteractionOutcome, InteractionQuestion, InteractionRecord,
};
use maka_runtime_host::application::{
    Context, Handler, NativeActivation, NativeOwner, NativeRegistry,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};

pub const IMPLEMENTATION: &str = "maka.interactions.questions";
pub const EXPORT: &str = "questions";

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "QuestionAsk")]
pub struct Ask {
    #[schemars(length(min = 1, max = 3))]
    pub questions: Vec<InteractionQuestion>,
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "QuestionResult")]
pub enum Answer {
    Answered { answers: Vec<Response> },
    Closed { reason: ClosureReason },
    Rejected { reason: Rejection },
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "QuestionRejection")]
pub enum Rejection {
    InvalidInput,
    Unavailable,
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "QuestionResponse")]
pub struct Response {
    pub question: String,
    #[schemars(with = "maka_runtime::schema::RequiredNullable<String>")]
    pub answer: Option<String>,
}
pub fn contract() -> Contract {
    Contract {
        name: "maka.interactions.Questions".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([(
                "ask".into(),
                Method {
                    progress: None,
                    mode: MethodMode::Mutation,
                    input: schemars::schema_for!(Ask).into(),
                    output: schemars::schema_for!(Answer).into(),
                    resources: BTreeMap::new(),
                },
            )]),
        },
    }
}
pub(crate) fn provide(registry: &mut NativeRegistry) -> Result<(), String> {
    registry.insert(
        IMPLEMENTATION.into(),
        Arc::new(|context, _| {
            Box::pin(async move {
                let Some(persistence) = context.persistence() else {
                    return NativeActivation {
                        exports: Err("questions_require_persistence".into()),
                        owner: Arc::new(super::Uninitialized),
                    };
                };
                let owner = Arc::new(Questions(persistence.store().clone()));
                NativeActivation {
                    exports: Ok(BTreeMap::from([(
                        EXPORT.into(),
                        maka_plugin_core::runtime::Published {
                            contract: contract(),
                            value: owner.clone() as Arc<dyn Handler>,
                        },
                    )])),
                    owner,
                }
            })
        }),
    )
}
struct Questions(Arc<EventLog>);
impl Handler for Questions {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let store = self.0.clone();
        Box::pin(async move {
            if method != "ask" {
                return Err("question_method_unavailable".into());
            }
            let invocation = context
                .scope()
                .identity
                .agent()
                .ok_or("question_requires_agent")?
                .clone();
            let operation_id = context
                .scope()
                .identity
                .operation_id()
                .ok_or("question_requires_tool")?;
            let operation = context.operation().ok_or("question_requires_mutation")?;
            let tool = maka_runtime::tools::ToolCallContext {
                invocation: invocation.clone(),
                operation_id: operation_id.into(),
            };
            let request = match maka_protocol::interaction::project_question_request(
                &tool.tool_use_id(),
                &input["questions"],
            ) {
                Ok(request) => request,
                Err(_) => {
                    return encode(Answer::Rejected {
                        reason: Rejection::InvalidInput,
                    });
                }
            };
            let input: Ask = serde_json::from_value(input).map_err(|_| "question_input_invalid")?;
            if context.scope().cancellation.is_cancelled() {
                return encode(Answer::Rejected {
                    reason: Rejection::Unavailable,
                });
            }
            let record = InteractionRecord {
                session_id: invocation.session_id.clone(),
                turn_id: invocation.turn_id.clone(),
                run_id: invocation.run_id.clone(),
                request_id: format!(
                    "question_{:x}",
                    Sha256::digest(serde_json::to_vec(operation).expect("operation identity"))
                ),
                created_at: super::owner::now(),
                request,
                outcome: None,
            };
            let Some(result) =
                crate::interaction_wait::publish(&store, &context, &invocation, &record).await?
            else {
                return encode(Answer::Rejected {
                    reason: Rejection::Unavailable,
                });
            };
            let answer = match result {
                InteractionOutcome::QuestionAnswer { answers, .. } => Answer::Answered {
                    answers: input
                        .questions
                        .into_iter()
                        .zip(answers)
                        .map(|(question, answer)| Response {
                            question: question.question,
                            answer,
                        })
                        .collect(),
                },
                InteractionOutcome::Closure { reason, .. } => Answer::Closed { reason },
                _ => return Err("question_outcome_invalid".into()),
            };
            encode(answer)
        })
    }
}
fn encode(answer: Answer) -> Result<Value, String> {
    serde_json::to_value(answer).map_err(|_| "question_output_unavailable".into())
}
impl NativeOwner for Questions {
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        // No detached work: native Host calls retain actual request custody and
        // revoke/settle their Scope before the provider can finish retirement.
        Box::pin(async { Ok(()) })
    }
}
