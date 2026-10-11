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

mod controls;
mod preparation;
use super::*;
use contract::{Task, TaskStatus};
use futures_util::FutureExt;
use maka_plugin_core::{
    deployment::DeploymentRef,
    operation::{OperationCleanup, OperationOutcome, OperationReply, Receipt},
};
use maka_plugins::call::{Attachments, Identity};
use maka_runtime::{
    event::Invocation,
    tool_call::ToolRejection,
    tools::{
        PreparationFuture, PreparedEffect, ToolCallContext, ToolDefinition, ToolError, ToolHandler,
        ToolNesting, ToolPreparer, ToolRegistration, ToolSemantics,
    },
};
use maka_runtime_host::application::{Binding, NativeContext, NativeServices};
use preparation::{Target, Transfer};
use schemars::JsonSchema;
use sha2::{Digest, Sha256};
use std::{
    panic::AssertUnwindSafe,
    sync::{
        OnceLock, Weak,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const NAME: &str = "workhub_tasks";
type Link = Arc<OnceLock<Weak<Hub>>>;
#[derive(Clone)]
pub(super) struct Tasks {
    services: NativeServices,
    source: DeploymentRef,
    workers: BTreeMap<String, Option<Binding>>,
    failed: Arc<AtomicBool>,
}
impl Tasks {
    pub(super) fn bindings(&self) -> BTreeMap<String, Option<String>> {
        self.workers
            .iter()
            .map(|(name, binding)| {
                (
                    name.clone(),
                    binding
                        .as_ref()
                        .map(|binding| binding.execution_basis().into()),
                )
            })
            .collect()
    }
    pub fn cleanup_failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
}
#[derive(Deserialize, JsonSchema)]
#[serde(
    tag = "operation",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Input {
    Control {
        task_id: String,
        action: controls::Action,
    },
    Controls {
        after: Option<String>,
    },
    InspectControl {
        control_id: String,
    },
    AdvanceControl {
        control_id: String,
    },
    Workers,
    Assignments {
        after: Option<String>,
    },
    Sessions {
        worker: String,
        after: Option<String>,
    },
    Directories {
        worker: String,
        location: crate::contract::Directories,
    },
    Route {
        worker: String,
        target: Target,
        note: String,
        model: crate::ModelSelection,
        thinking_level: Option<maka_runtime::execution::ThinkingLevel>,
    },
    Wait {
        task_id: String,
    },
    Inspect {
        task_id: String,
        cursor: Option<crate::contract::AnswerCursor>,
    },
    Advance {
        task_id: String,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Assignment {
    source: Invocation,
    tool_operation: String,
    worker: String,
    binding: String,
    operation: Uuid,
    input: Value,
    target: Target,
    attachments: Vec<Transfer>,
}
struct Call<'a> {
    worker: &'a str,
    binding: &'a str,
    operation: Uuid,
    method: &'a str,
    input: &'a Value,
    attachment: Option<&'a Transfer>,
}
enum DispatchObservation {
    Pending,
    Unavailable,
    Reply(Box<OperationReply>),
}
pub(super) fn prepare(
    context: &NativeContext,
    value: Value,
) -> Result<(Value, Tasks, Vec<ToolRegistration>, Link), String> {
    if serde_json::to_vec(&value)
        .map_err(|_| "workhub_configuration_invalid")?
        .len()
        > 128 * 1024
        || !jsonschema::validator_for(&configuration_schema())
            .expect("WorkHub schema")
            .is_valid(&value)
    {
        return Err("workhub_configuration_invalid".into());
    }
    let settings: contract::Configuration =
        serde_json::from_value(value).map_err(|_| "workhub_configuration_invalid")?;
    let services = context.services();
    let mut workers = BTreeMap::new();
    for name in settings.workers {
        let bindings = services
            .imports()
            .find(|(alias, _)| *alias == name)
            .ok_or("workhub_worker_import_missing")?
            .1;
        if bindings.len() > 1
            || bindings
                .iter()
                .any(|binding| binding.contract() != &crate::contract())
            || workers.insert(name, bindings.first().cloned()).is_some()
        {
            return Err("workhub_worker_import_invalid".into());
        }
    }
    let link = Arc::new(OnceLock::new());
    let additional = if workers.is_empty() {
        vec![]
    } else {
        vec![ToolRegistration {
        definition: ToolDefinition {
            provider: None, name: NAME.into(),
            description: "Coordinate work using the explicitly selected Agent services. List workers and sessions before routing. Directories reads that worker’s published choices; pass complete selections to a new target. Route targets an existing session or creates a new one, transfers the original user attachments, then sends the original text and your note. Creation/upload preparations are not worker execution. Sessions browses the live catalog in ID order, returning display excerpts and an after cursor; pass that cursor unchanged to continue. Changes before the cursor require a fresh browse. Route preserves the original user request and adds your note. Admission is not completion. Inspect the returned taskId after uncertain replies; advance only resumes that same frozen dispatch and never redirects it. Wait observes an admitted task until it completes, needs input, pauses, or becomes unavailable; use it to receive the result during this conversation. Inspect returns bounded original answer pages; continue with its exact cursor. Control freezes stop/resume on that task's observed work; shared delivery cannot control the whole Run. Use controls and inspect_control after a missing reply, or advance_control for the same original operation. A requested stop is not confirmed termination. Preparation receipts stay with the original task. Unknown or an original resource capture requirement never authorizes a new route or recapture.".into(),
            input_schema: schemars::schema_for!(Input).into(), output_schema: None, freeform: None,
        },
        nesting: ToolNesting::DirectOnly, semantics: ToolSemantics::ExclusiveStep,
        handler: ToolHandler::Prepared(Arc::new(Tool(link.clone()))),
    }]
    };
    Ok((
        serde_json::to_value(settings.agent).map_err(|_| "workhub_configuration_invalid")?,
        Tasks {
            services,
            source: context
                .persistence()
                .ok_or("workhub_requires_persistence")?
                .source()
                .clone(),
            workers,
            failed: Arc::new(AtomicBool::new(false)),
        },
        additional,
        link,
    ))
}
struct Tool(Link);
impl ToolPreparer for Tool {
    fn names(&self) -> Vec<String> {
        vec![NAME.into()]
    }
    fn prepare(
        &self,
        name: String,
        input: Value,
        context: ToolCallContext,
        _: CancellationToken,
    ) -> PreparationFuture {
        let hub = self.0.get().and_then(Weak::upgrade);
        Box::pin(async move {
            let hub = hub
                .filter(|_| name == NAME)
                .ok_or(ToolRejection::Unavailable)?;
            let input: Input =
                serde_json::from_value(input).map_err(|_| ToolRejection::InvalidInput {
                    message: "Invalid WorkHub task input".into(),
                })?;
            // Actual ToolCallContext supplies the source, never model JSON.
            hub.coordinator_source(&context.invocation)
                .await
                .map_err(|_| ToolRejection::Unavailable)?;
            Ok(PreparedEffect::new(move |cancel| {
                Box::pin(async move {
                    hub.tasks_input(input, context, cancel)
                        .await
                        .map(Into::into)
                        .map_err(|error| {
                            if hub.tasks.cleanup_failed() {
                                ToolError::CleanupUnconfirmed(error)
                            } else {
                                ToolError::OutcomeUnknown(error)
                            }
                        })
                })
            }))
        })
    }
}
impl Hub {
    async fn coordinator_source(
        &self,
        invocation: &Invocation,
    ) -> Result<maka_event_log::turns::TurnBoundary, String> {
        let intent = self.intent().await?.ok_or("workhub_coordinator_missing")?;
        if self.agent.0.operation_id(&intent.operation) != invocation.session_id {
            return Err("workhub_source_invalid".into());
        }
        let boundary = self
            .agent
            .0
            .store
            .invocation_boundary(&invocation.session_id, &invocation.invocation_id)
            .await
            .map_err(|_| "workhub_source_unavailable")?
            .ok_or("workhub_source_unavailable")?
            .0;
        if boundary.invocation != *invocation {
            return Err("workhub_source_invalid".into());
        }
        Ok(boundary)
    }
    fn task_key(&self, id: &str) -> Result<String, String> {
        if id.len() != 67
            || !id.starts_with("wt_")
            || !id[3..]
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("workhub_task_invalid".into());
        }
        Ok(format!("{}tasks/{id}", self.agent.0.prefix))
    }
    pub(super) async fn task_list(&self, after: Option<String>) -> Result<Value, String> {
        let prefix = format!("{}tasks/", self.agent.0.prefix);
        let page = self
            .agent
            .0
            .store
            .plugin_data_scan(
                &namespace(),
                maka_plugins::storage::Scan {
                    prefix: prefix.clone(),
                    after: after.as_deref().map(|id| self.task_key(id)).transpose()?,
                },
            )
            .await
            .map_err(|_| "workhub_store_unavailable")?;
        let more = page.entries.len() > 16 || page.next_after.is_some();
        let mut entries = Vec::new();
        let mut last = None;
        for entry in page.entries.into_iter().take(16) {
            let id = entry
                .key
                .strip_prefix(&prefix)
                .ok_or("workhub_task_invalid")?
                .to_owned();
            self.task_key(&id)?;
            last = Some(id.clone());
            if let Data::Present(value) = entry.record.data {
                let assignment: Assignment =
                    serde_json::from_value(value).map_err(|_| "workhub_task_invalid")?;
                entries.push(serde_json::json!({"taskId":id,"worker":assignment.worker}));
            }
        }
        Ok(serde_json::json!({"entries":entries,"after":if more {last} else {None}}))
    }
    async fn assignment(&self, id: &str) -> Result<Assignment, String> {
        let record = self
            .agent
            .0
            .store
            .plugin_data(&namespace(), &self.task_key(id)?)
            .await
            .map_err(|_| "workhub_store_unavailable")?
            .ok_or("workhub_task_missing")?;
        let Data::Present(value) = record.data else {
            return Err("workhub_task_removed".into());
        };
        let record: Assignment =
            serde_json::from_value(value).map_err(|_| "workhub_task_invalid")?;
        if !self.agent.0.owns(&record.source.session_id)
            || task_id(&record.source, &record.tool_operation) != id
            || record.operation != child_id(id)
        {
            return Err("workhub_task_invalid".into());
        }
        Ok(record)
    }
    async fn tasks_input(
        &self,
        input: Input,
        context: ToolCallContext,
        cancellation: CancellationToken,
    ) -> Result<Value, String> {
        if self.tasks.cleanup_failed() || cancellation.is_cancelled() {
            return Err("workhub_closed".into());
        }
        match input {
            Input::Control { task_id, action } => {
                self.control_start(&task_id, action, context, cancellation)
                    .await
            }
            Input::Controls { after } => self.controls(after).await,
            Input::InspectControl { control_id } => {
                self.control(&control_id, false, cancellation).await
            }
            Input::AdvanceControl { control_id } => {
                self.control(&control_id, true, cancellation).await
            }
            Input::Assignments { after } => self.task_list(after).await,
            Input::Workers => Ok(
                serde_json::json!({"workers":self.tasks.workers.iter().map(|(name,binding)| serde_json::json!({"name":name,"available":binding.is_some()})).collect::<Vec<_>>()}),
            ),
            Input::Directories { worker, location } => {
                self.tasks
                    .workers
                    .get(&worker)
                    .and_then(Option::as_ref)
                    .ok_or("workhub_worker_unavailable")?;
                let request = self.tasks.services.request(
                    Identity::Agent {
                        invocation: context.invocation,
                        operation_id: Some(context.operation_id),
                    },
                    cancellation,
                )?;
                self.observe_request(&request, async {
                    let page = request
                        .call(
                            &worker,
                            0,
                            "directories".into(),
                            serde_json::to_value(location)
                                .map_err(|_| "workhub_directory_invalid")?,
                            Attachments::default(),
                        )?
                        .await?;
                    preparation::directory_page(page)
                })
                .await
            }
            Input::Sessions { worker, after } => {
                self.tasks
                    .workers
                    .get(&worker)
                    .and_then(Option::as_ref)
                    .ok_or("workhub_worker_unavailable")?;
                let request = self.tasks.services.request(
                    Identity::Agent {
                        invocation: context.invocation,
                        operation_id: Some(context.operation_id),
                    },
                    cancellation,
                )?;
                self.observe_request(&request, async {
                    // Each model turn changes the global catalog revision. Refresh at the
                    // original anchor explicitly; do not reuse or silently relax a fence.
                    let position = after.as_ref().map_or_else(||serde_json::json!({"kind":"start"}),|cursor|serde_json::json!({"kind":"at","cursor":cursor}));
                    let mut page = request.call(&worker, 0, "list".into(), serde_json::json!({"filter":"all","position":position}), Attachments::default())?.await?;
                    if let Some(anchor) = after.as_deref() {
                        let rows = page["sessions"].as_array().ok_or("workhub_sessions_invalid")?;
                        if !rows.iter().any(|row|row["id"].as_str().is_some_and(|id|id > anchor)) && !page["after"].is_null() {
                            // A maximum-size summary can fill a native page on its own.
                            // Continue that newly read page with its actual revision.
                            page = request.call(&worker, 0, "list".into(), serde_json::json!({"filter":"all","position":{"kind":"after","cursor":page["after"],"revision":page["revision"]}}),Attachments::default())?.await?;
                        }
                    }
                    session_page(page, after.as_deref())

                }).await
            }
            Input::Route {
                worker,
                target,
                note,
                model,
                thinking_level,
            } => {
                let id = task_id(&context.invocation, &context.operation_id);
                // A same-tool replay reads its original decision before today's capabilities.
                if self
                    .agent
                    .0
                    .store
                    .plugin_data(&namespace(), &self.task_key(&id)?)
                    .await
                    .map_err(|_| "workhub_store_unavailable")?
                    .is_none()
                {
                    let binding = self
                        .tasks
                        .workers
                        .get(&worker)
                        .and_then(Option::as_ref)
                        .ok_or("workhub_worker_unavailable")?;
                    let original = self.coordinator_source(&context.invocation).await?;
                    let maka_runtime::input::InvocationInput::Message {
                        content,
                        source_messages,
                        ..
                    } = original.root_input()
                    else {
                        return Err("workhub_source_unavailable".into());
                    };
                    target.validate()?;
                    if note.trim().is_empty() || note.len() > 16 * 1024 {
                        return Err("workhub_route_invalid".into());
                    }
                    let attachments = self
                        .transfers(
                            &context.invocation,
                            content.attachments.as_deref().unwrap_or_default(),
                        )
                        .await?;
                    let original = if source_messages.len() == 1 {
                        &source_messages[0].unprepared_content.text
                    } else {
                        &content.text
                    };
                    let message = format!(
                        "Original user request:\n{original}\n\nCoordinator instructions:\n{note}"
                    );
                    if message.len() > 64 * 1024 {
                        return Ok(
                            serde_json::json!({"kind":"rejected","reason":"input_too_large"}),
                        );
                    }
                    let assignment = Assignment {
                        source: context.invocation,
                        tool_operation: context.operation_id,
                        worker,
                        binding: binding.execution_basis().into(),
                        operation: child_id(&id),
                        input: serde_json::json!({"message":message,"model":model,"thinkingLevel":thinking_level}),
                        target,
                        attachments,
                    };
                    if cancellation.is_cancelled() {
                        return Err("workhub_cancelled".into());
                    }
                    match self
                        .agent
                        .0
                        .store
                        .plugin_data_batch(
                            &namespace(),
                            vec![Mutation {
                                key: self.task_key(&id)?,
                                expected_revision: None,
                                data: Data::Present(
                                    serde_json::to_value(assignment)
                                        .map_err(|_| "workhub_encoding_failed")?,
                                ),
                            }],
                        )
                        .await
                    {
                        Ok(_) | Err(maka_event_log::StoreError::RevisionConflict { .. }) => {}
                        Err(_) => return Err("workhub_route_commit_unknown".into()),
                    }
                }
                self.task(&id, true, None, cancellation).await
            }
            Input::Wait { task_id } => self.wait_task(&task_id, context, cancellation).await,
            Input::Inspect { task_id, cursor } => {
                self.task(&task_id, false, cursor, cancellation).await
            }
            Input::Advance { task_id } => self.task(&task_id, true, None, cancellation).await,
        }
    }
    async fn wait_task(
        &self,
        id: &str,
        context: ToolCallContext,
        cancellation: CancellationToken,
    ) -> Result<Value, String> {
        let current = self.task(id, false, None, cancellation.clone()).await?;
        if !waiting(&current) {
            return Ok(current);
        }
        let assignment = self.assignment(id).await?;
        let delivery = self
            .delivery(id, &assignment)
            .await?
            .ok_or("workhub_delivery_missing")?;
        let request = self.tasks.services.request(
            Identity::Agent {
                invocation: context.invocation,
                operation_id: Some(context.operation_id),
            },
            cancellation.clone(),
        )?;
        self.observe_request(&request, async {
            let input = serde_json::json!({"sessionId":delivery.session_id});
            let observed = request.call(&assignment.worker, 0, "observe".into(), input, Attachments::default())?.await?;
            let through = observed["through"].as_u64().ok_or("workhub_observation_invalid")?;
            let mut subscription = request.subscribe(&assignment.worker, 0, "watch".into(), serde_json::json!({"sessionId":delivery.session_id,"after":through}), Attachments::default())?.await?;
            loop {
                // Re-read after admission to cover completion between observe and watch.
                let current = self.task(id, false, None, cancellation.clone()).await?;
                if !waiting(&current) { return Ok(current); }
                tokio::select! {
                    biased;
                    () = cancellation.cancelled() => return Err("workhub_cancelled".into()),
                    value = subscription.recv() => if value.is_none() {return Err("workhub_observation_closed".into());},
                }
            }
            // Dropping demand cancels only watch. The original request finish joins
            // the actual subscription executor, never the worker's independent Run.
        }).await
    }
    async fn observe_request<T>(
        &self,
        request: &maka_runtime_host::application::NativeRequest,
        observation: impl std::future::Future<Output = Result<T, String>>,
    ) -> Result<T, String> {
        let mut business = Box::pin(AssertUnwindSafe(observation).catch_unwind());
        let result = tokio::select! {
            biased;
            result = &mut business => result,
            _ = request.scope().cancellation.cancelled() => {
                // Reach the original cleanup budget while the Host retains the actual
                // admitted callback. Keep its original observer after confirmed cleanup.
                self.finish(request).await?;
                business.await
            }
        }
        .unwrap_or_else(|_| Err("workhub_observation_unknown".into()));
        self.finish(request).await?;
        result
    }
    async fn finish(
        &self,
        request: &maka_runtime_host::application::NativeRequest,
    ) -> Result<(), String> {
        if request.finish().await.is_err() {
            self.tasks.failed.store(true, Ordering::Release);
            return Err("workhub_cleanup_unconfirmed".into());
        }
        Ok(())
    }
    async fn dispatch(
        &self,
        request: &maka_runtime_host::application::NativeRequest,
        call: Call<'_>,
        advance: bool,
    ) -> Result<DispatchObservation, String> {
        let same_binding = self
            .tasks
            .workers
            .get(call.worker)
            .and_then(Option::as_ref)
            .is_some_and(|binding| binding.execution_basis() == call.binding);
        let recorded = self
            .agent
            .0
            .store
            .application_intent(
                self.tasks.source.deployment,
                self.agent.0.data_source.namespace.clone(),
                call.operation,
            )
            .await
            .map_err(|_| "workhub_original_operation_unavailable")?;
        if let Some(recorded) = recorded {
            let reference = recorded.reference;
            let observed = request.recover(reference.clone())?.await?;
            if observed.is_some() || !advance || !same_binding || call.attachment.is_some() {
                return Ok(DispatchObservation::Reply(Box::new(OperationReply {
                    reference,
                    receipt: observed.map_or(Receipt::NotRecorded, |observation| {
                        Receipt::Recorded {
                            outcome: observation.outcome,
                            cleanup: observation.cleanup,
                        }
                    }),
                })));
            }
        } else if !advance {
            return Ok(DispatchObservation::Pending);
        }
        if !same_binding {
            return Ok(DispatchObservation::Unavailable);
        }
        let attachments = if let Some(transfer) = call.attachment {
            self.transfer_resource(request, transfer).await?
        } else {
            maka_runtime_host::application::CallResources::default()
        };
        Ok(DispatchObservation::Reply(Box::new(
            request
                .mutate(
                    call.worker,
                    0,
                    call.method.into(),
                    call.input.clone(),
                    call.operation,
                    attachments,
                )?
                .await?,
        )))
    }
    pub(super) async fn task(
        &self,
        id: &str,
        advance: bool,
        cursor: Option<crate::contract::AnswerCursor>,
        cancellation: CancellationToken,
    ) -> Result<Value, String> {
        if self.tasks.cleanup_failed() {
            return Err("workhub_closed".into());
        }
        let assignment = self.assignment(id).await?;
        let same_binding = self
            .tasks
            .workers
            .get(&assignment.worker)
            .and_then(Option::as_ref)
            .is_some_and(|binding| binding.execution_basis() == assignment.binding);
        let request = self.tasks.services.request(
            Identity::Agent {
                invocation: assignment.source.clone(),
                operation_id: Some(assignment.tool_operation.clone()),
            },
            cancellation,
        )?;
        self.observe_request(&request, async {
            let delivery = match self.prepare_delivery(id, &assignment, &request, advance).await? {
                Ok(delivery) => delivery,
                Err(status) => return encode_task(id, &assignment, status),
            };
            let mut input = assignment.input.clone();
            input["sessionId"] = serde_json::json!(delivery.session_id);
            input["attachments"] = serde_json::json!(delivery.attachments);
            let reply = match self.dispatch(&request, Call {
                worker: &assignment.worker, binding: &assignment.binding,
                operation: assignment.operation, method: "send", input: &input, attachment: None,
            }, advance).await? {
                DispatchObservation::Pending => None,
                DispatchObservation::Unavailable => return encode_task(id, &assignment, TaskStatus::Unavailable),
                DispatchObservation::Reply(reply) => Some(*reply),
            };
            let status = match reply {
                None => TaskStatus::Pending,
                Some(OperationReply {
                    receipt:
                        Receipt::Recorded {
                            outcome: OperationOutcome::Returned { value },
                            cleanup: OperationCleanup::Confirmed,
                        },
                    ..
                }) => {
                    if value["sessionId"] != delivery.session_id {
                        return Err("workhub_delivery_invalid".into());
                    }
                    let session = delivery.session_id;
                    match value["kind"].as_str() {
                        Some("started") => TaskStatus::Started {
                            progress: if same_binding {
                                async {
                                    request.call(
                                        &assignment.worker, 0, "work".into(),
                                        serde_json::json!({"sessionId":session,"target":{"kind":"invocation","invocationId":value["invocationId"]},"cursor":cursor}),
                                        Attachments::default(),
                                    )?.await
                                }.await.ok().and_then(|value| serde_json::from_value(value).ok())
                            } else { None },
                            session_id: session,
                            invocation_id: value["invocationId"]
                                .as_str()
                                .ok_or("workhub_delivery_invalid")?
                                .into(),
                        },
                        Some("queued") => TaskStatus::Queued {
                            progress: if same_binding {
                                async {
                                    request.call(
                                        &assignment.worker, 0, "work".into(),
                                        serde_json::json!({"sessionId":session,"target":{"kind":"message","messageId":value["messageId"]},"cursor":cursor}),
                                        Attachments::default(),
                                    )?.await
                                }.await.ok().and_then(|value| serde_json::from_value(value).ok())
                            } else { None },
                            session_id: session,
                            message_id: value["messageId"]
                                .as_str()
                                .ok_or("workhub_delivery_invalid")?
                                .into(),
                        },
                        Some("rejected") => TaskStatus::Rejected {
                            reason: "worker_rejected_input".into(),
                        },
                        _ => return Err("workhub_delivery_invalid".into()),
                    }
                }
                Some(_) => TaskStatus::Uncertain,
            };
            encode_task(id, &assignment, status)
        }).await
    }
}
fn encode_task(id: &str, assignment: &Assignment, status: TaskStatus) -> Result<Value, String> {
    serde_json::to_value(Task {
        task_id: id.into(),
        worker: assignment.worker.clone(),
        operation: assignment.operation,
        status,
    })
    .map_err(|_| "workhub_encoding_failed".into())
}
fn task_id(invocation: &Invocation, operation: &str) -> String {
    format!(
        "wt_{:x}",
        Sha256::digest(serde_json::to_vec(&(invocation, operation)).expect("actual tool identity"))
    )
}
fn child_id(id: &str) -> Uuid {
    let digest = Sha256::digest(id.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn session_page(page: Value, after: Option<&str>) -> Result<Value, String> {
    let source = page["sessions"]
        .as_array()
        .ok_or("workhub_sessions_invalid")?;
    let source = source
        .iter()
        .filter(|row| after.is_none_or(|cursor| row["id"].as_str().is_some_and(|id| id > cursor)))
        .collect::<Vec<_>>();
    let mut sessions = Vec::new();
    for session in source.iter().take(4) {
        sessions.push(serde_json::json!({
            "id":session["id"], "titleExcerpt":excerpt(session["title"].as_str().ok_or("workhub_sessions_invalid")?,64),
            "workspaceExcerpt":excerpt(session["workspace"]["path"].as_str().ok_or("workhub_sessions_invalid")?,96),
            "activity":session["activity"], "archived":session["archived"],
        }));
    }
    let after = if source.len() > sessions.len() || !page["after"].is_null() {
        sessions.last().map(|last| last["id"].clone())
    } else {
        None
    };
    Ok(serde_json::json!({"sessions":sessions,"after":after}))
}
fn excerpt(text: &str, bytes: usize) -> String {
    let end = text.floor_char_boundary(text.len().min(bytes));
    if end == text.len() {
        text.into()
    } else {
        format!("{}…", &text[..end])
    }
}

fn waiting(task: &Value) -> bool {
    let progress = &task["status"]["progress"];
    progress["kind"] == "pending"
        || (progress["kind"] == "delivered"
            && progress["pendingInteractions"] == 0
            && matches!(
                progress["execution"]["state"].as_str(),
                Some("admitted" | "running")
            ))
}
