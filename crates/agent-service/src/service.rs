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
    contract::*,
    owner::{Owner, SessionConfiguration, now},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures_util::future::BoxFuture;
use maka_event_log::{
    observation::{StoreStreamEvent, StreamFact},
    sessions::{CatalogScope, SessionRecord},
};
use maka_runtime_host::application::{Context, Handler};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::sync::Arc;

pub(crate) struct Agent(pub Arc<Owner>);
impl Handler for Agent {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let owner = self.0.clone();
        Box::pin(async move {
            match method.as_str() {
                "list" => {
                    let input: List = decode(input)?;
                    use maka_event_log::sessions::SessionPosition;
                    let (revision, position) = match &input {
                        List::Start => (None, SessionPosition::Start),
                        List::After { revision, cursor } => {
                            (Some(revision.as_str()), SessionPosition::After(cursor))
                        }
                        List::Before { revision, cursor } => {
                            (Some(revision.as_str()), SessionPosition::Before(cursor))
                        }
                    };
                    if matches!(position, SessionPosition::After(cursor) | SessionPosition::Before(cursor) if !owner.owns(cursor))
                    {
                        return Err("agent_cursor_invalid".into());
                    }
                    let page = owner
                        .store
                        .scoped_sessions::<SessionConfiguration>(
                            CatalogScope::IdPrefix(owner.prefix.clone()),
                            revision,
                            position,
                            false,
                        )
                        .await
                        .map_err(|_| "agent_catalog_unavailable")?;
                    encode(SessionPage {
                        revision: page.revision,
                        sessions: page.sessions.into_iter().map(summary).collect(),
                        before: page.previous_cursor,
                        after: page.next_cursor,
                    })
                }
                "create" => {
                    let input: Create = decode(input)?;
                    if input.title.trim().is_empty()
                        || input.title.chars().count() > 128
                        || input.title.len() > 512
                        || input.title.chars().any(char::is_control)
                    {
                        return Err("agent_title_invalid".into());
                    }
                    let operation = context.operation().ok_or("agent_mutation_required")?;
                    let id = owner.operation_id(operation);
                    let session = owner
                        .store
                        .create_session(
                            &id,
                            &operation.fingerprint,
                            &SessionConfiguration {
                                schema_version: 1,
                                title: input.title,
                            },
                            now(),
                        )
                        .await
                        .map_err(|_| "agent_creation_failed")?;
                    encode(summary(session))
                }
                "observe" => {
                    use maka_event_log::turns::InvocationState;
                    let input: SessionRead = decode(input)?;
                    owner.session(&input.session_id).await?;
                    let observed = owner
                        .store
                        .observe_session::<SessionConfiguration>(&input.session_id)
                        .await
                        .map_err(|_| "agent_observation_unavailable")?
                        .ok_or("agent_session_unavailable")?;
                    let run = observed.root_turn.and_then(|turn| {
                        let state = match turn.state {
                            InvocationState::Admitted => RunState::Admitted,
                            InvocationState::Running => RunState::Running,
                            InvocationState::WaitingForUser => RunState::WaitingForUser,
                            InvocationState::Ended { .. } => return None,
                        };
                        Some(Run {
                            invocation_id: turn.invocation.invocation_id,
                            state,
                        })
                    });
                    let result = encode(Observation {
                        session: summary(observed.session),
                        through: observed.through_sequence,
                        run,
                        parts: observed
                            .active_streams
                            .into_iter()
                            .map(|part| Part {
                                start_sequence: part.start_sequence,
                                message_id: part.message_id,
                                step_id: part.step_id,
                                part_id: part.part_id,
                                text_kind: part.text_kind.into(),
                            })
                            .collect(),
                    })?;
                    maka_js_runtime::plugin::check_json_delivery(&result, 48 * 1024)
                        .map_err(|_| "agent_observation_unavailable")?;
                    owner.session(&input.session_id).await?;
                    if context.scope().cancellation.is_cancelled() {
                        return Err("agent_observation_cancelled".into());
                    }
                    Ok(result)
                }
                "history" => {
                    use maka_plugins::session::history::{Page, Read};
                    let input: History = decode(input)?;
                    owner.session(&input.session_id).await?;
                    let high_water = *owner.store.subscribe_commits().borrow();
                    let through = input.through.unwrap_or(high_water);
                    if through > high_water {
                        return Err("agent_cursor_invalid".into());
                    }
                    Read {
                        session_id: input.session_id.clone(),
                        through: input.through,
                        position: input.position,
                    }
                    .validate()
                    .map_err(|_| "agent_cursor_invalid")?;
                    let result = if !owner
                        .store
                        .prepare_transcript(&input.session_id, through, 32)
                        .await
                        .map_err(|_| "agent_history_unavailable")?
                    {
                        HistoryPage::Preparing { through }
                    } else {
                        let Page::Ready {
                            through,
                            chunks,
                            before,
                            after,
                        } = owner
                            .store
                            .history_text(
                                &input.session_id,
                                through,
                                input.position,
                                8192,
                                2,
                                64 * 1024,
                            )
                            .await
                            .map_err(|_| "agent_history_unavailable")?
                        else {
                            return Err("agent_history_unavailable".into());
                        };
                        HistoryPage::Ready {
                            through,
                            before,
                            after,
                            chunks: chunks
                                .into_iter()
                                .map(|chunk| HistoryChunk {
                                    message_id: chunk.message_id,
                                    turn_id: chunk.turn_id,
                                    timestamp: chunk.timestamp,
                                    role: chunk.role,
                                    sequence: chunk.sequence,
                                    offset: chunk.offset,
                                    total_bytes: chunk.total_bytes,
                                    text: chunk.text,
                                    tool: chunk.tool,
                                })
                                .collect(),
                        }
                    };
                    owner.session(&input.session_id).await?;
                    if context.scope().cancellation.is_cancelled() {
                        return Err("agent_observation_cancelled".into());
                    }
                    let result = encode(result)?;
                    maka_js_runtime::plugin::check_json_delivery(&result, 64 * 1024)
                        .map_err(|_| "agent_history_unavailable")?;
                    Ok(result)
                }
                "toolSource" => {
                    let input: ToolSourceRead = decode(input)?;
                    owner.session(&input.session_id).await?;
                    if input.through > *owner.store.subscribe_commits().borrow() {
                        return Err("agent_cursor_invalid".into());
                    }
                    let page = if owner
                        .store
                        .prepare_transcript(&input.session_id, input.through, 32)
                        .await
                        .map_err(|_| "agent_history_unavailable")?
                    {
                        ToolSourcePage::Ready {
                            through: input.through,
                            source: owner
                                .store
                                .tool_source(
                                    &input.session_id,
                                    input.through,
                                    input.sequence,
                                    input.field,
                                    input.position,
                                    8192,
                                    64 * 1024,
                                )
                                .await
                                .map_err(|_| "agent_history_unavailable")?,
                        }
                    } else {
                        ToolSourcePage::Preparing {
                            through: input.through,
                        }
                    };
                    owner.session(&input.session_id).await?;
                    if context.scope().cancellation.is_cancelled() {
                        return Err("agent_observation_cancelled".into());
                    }
                    let result = encode(page)?;
                    maka_js_runtime::plugin::check_json_delivery(&result, 64 * 1024)
                        .map_err(|_| "agent_history_unavailable")?;
                    Ok(result)
                }
                "toolImage" => {
                    let input: ToolImageRead = decode(input)?;
                    owner.session(&input.session_id).await?;
                    if input.through > *owner.store.subscribe_commits().borrow() {
                        return Err("agent_cursor_invalid".into());
                    }
                    let page = if owner
                        .store
                        .prepare_transcript(&input.session_id, input.through, 32)
                        .await
                        .map_err(|_| "agent_history_unavailable")?
                    {
                        let image = owner
                            .store
                            .tool_image(
                                &input.session_id,
                                input.through,
                                input.sequence,
                                input.offset,
                                32 * 1024,
                            )
                            .await
                            .map_err(|_| "agent_image_unavailable")?;
                        ToolImagePage::Ready {
                            through: input.through,
                            sequence: input.sequence,
                            offset: input.offset,
                            total_bytes: image.total_bytes,
                            mime_type: image.mime_type,
                            digest: image.digest,
                            data: STANDARD.encode(image.bytes),
                        }
                    } else {
                        ToolImagePage::Preparing {
                            through: input.through,
                        }
                    };
                    owner.session(&input.session_id).await?;
                    if context.scope().cancellation.is_cancelled() {
                        return Err("agent_observation_cancelled".into());
                    }
                    let result = encode(page)?;
                    maka_js_runtime::plugin::check_json_delivery(&result, 64 * 1024)
                        .map_err(|_| "agent_image_unavailable")?;
                    Ok(result)
                }
                "send" => {
                    let input: Send = decode(input)?;
                    let operation = context.operation().ok_or("agent_mutation_required")?;
                    encode(
                        owner
                            .send(input.session_id, input.message, input.model, operation)
                            .await?,
                    )
                }
                "stop" => {
                    let input: Stop = decode(input)?;
                    context.operation().ok_or("agent_mutation_required")?;
                    encode(Stopped {
                        requested: owner
                            .stop_run(&input.session_id, &input.invocation_id)
                            .await?,
                    })
                }
                "watch" => {
                    let input: Watch = decode(input)?;
                    owner.session(&input.session_id).await?;
                    let mut commits = owner.store.subscribe_commits();
                    let mut after = input.after;
                    loop {
                        if context.scope().cancellation.is_cancelled() {
                            return Err("agent_observation_cancelled".into());
                        }
                        let observation = owner
                            .store
                            .session_projection::<SessionConfiguration>(&input.session_id)
                            .await
                            .map_err(|_| "agent_observation_unavailable")?
                            .ok_or("agent_session_unavailable")?;
                        if after > observation.through_sequence {
                            return Err("agent_cursor_invalid".into());
                        }
                        let page = owner
                            .store
                            .session_stream_events(
                                &input.session_id,
                                after,
                                observation.through_sequence,
                                32,
                                48 * 1024,
                            )
                            .await
                            .map_err(|_| "agent_observation_unavailable")?;
                        after = page.next_after.unwrap_or(page.through_sequence);
                        context
                            .emit(encode(DeliveryPage {
                                through: page.through_sequence,
                                next_after: after,
                                events: page.events.into_iter().filter_map(delivery).collect(),
                            })?)
                            .await?;
                        if page.next_after.is_some() {
                            continue;
                        }
                        tokio::select! {
                            biased;
                            _ = context.scope().cancellation.cancelled() => return Err("agent_observation_cancelled".into()),
                            result = commits.changed() => result.map_err(|_| "agent_observation_closed")?,
                        }
                    }
                }
                _ => Err("agent_method_unavailable".into()),
            }
        })
    }
}
fn decode<T: DeserializeOwned>(input: Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|_| "agent_input_invalid".into())
}
fn encode<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|_| "agent_output_unavailable".into())
}
fn summary(session: SessionRecord<SessionConfiguration>) -> Session {
    Session {
        id: session.id,
        revision: session.revision,
        title: session.configuration.title,
        updated_at: session.updated_at,
        archived: session.archived,
    }
}
fn delivery(event: StoreStreamEvent) -> Option<Delivery> {
    let fact = match event.fact {
        StreamFact::InvocationOpened => DeliveryFact::Opened,
        StreamFact::PartStarted {
            step_id,
            part_id,
            text_kind,
        } => DeliveryFact::PartStarted {
            message_id: event.id,
            step_id,
            part_id,
            text_kind: text_kind.into(),
        },
        StreamFact::PartDelta {
            step_id,
            part_id,
            text,
        } => DeliveryFact::Delta {
            step_id,
            part_id,
            text,
        },
        StreamFact::PartFinished { step_id, part_id } => {
            DeliveryFact::PartFinished { step_id, part_id }
        }
        StreamFact::InvocationEnded { failed, .. } => DeliveryFact::Ended { failed },
        StreamFact::StepEnded { step_id, .. } => DeliveryFact::StepEnded { step_id },
        StreamFact::MessageSteered
        | StreamFact::ToolDispatched { .. }
        | StreamFact::ToolRejected { .. }
        | StreamFact::ToolSettled { .. } => DeliveryFact::TranscriptChanged,
        _ => return None,
    };
    Some(Delivery {
        sequence: event.sequence,
        invocation_id: event.invocation.invocation_id,
        fact,
    })
}

impl maka_runtime_host::application::NativeOwner for Agent {
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        self.0.close()
    }
}
