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
    owner::{Owner, SessionConfiguration},
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
                "list" => encode(catalog_page(&owner, &decode(input)?).await?),
                "catalog" => {
                    let mut input: List = decode(input)?;
                    let mut changes = owner.store.subscribe_catalog_changes();
                    changes.borrow_and_update();
                    let page = catalog_page(&owner, &input).await?;
                    let mut revision = page.revision.clone();
                    let mut anchor = page.sessions.first().map(|session| session.id.clone());
                    context.emit(encode(page)?).await?;
                    loop {
                        tokio::select! {
                            biased;
                            _ = context.scope().cancellation.cancelled() => return Err("agent_observation_cancelled".into()),
                            result = changes.changed() => result.map_err(|_| "agent_observation_closed")?,
                        }
                        changes.borrow_and_update();
                        let current = owner
                            .store
                            .catalog_revision(
                                CatalogScope::IdPrefix(owner.prefix.clone()),
                                catalog_filter(&input),
                            )
                            .await
                            .map_err(|_| "agent_catalog_unavailable")?;
                        if current == revision {
                            continue;
                        }
                        input.position = anchor.as_ref().map_or(
                            crate::contract::SessionPosition::Start,
                            |cursor| crate::contract::SessionPosition::At {
                                cursor: cursor.clone(),
                            },
                        );
                        let page = catalog_page(&owner, &input).await?;
                        revision = page.revision.clone();
                        anchor = page.sessions.first().map(|session| session.id.clone());
                        context.emit(encode(page)?).await?;
                    }
                }
                "directories" => encode(
                    crate::workspace::query(owner.workspace_roots.clone(), decode(input)?).await?,
                ),
                "create" => {
                    let input: Create = decode(input)?;
                    let operation = context.operation().ok_or("agent_mutation_required")?;
                    encode(owner.create(input, operation).await?)
                }
                "metadata" => {
                    context.operation().ok_or("agent_mutation_required")?;
                    encode(owner.metadata(decode(input)?).await?)
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
                        include_thinking: input.include_thinking,
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
                                input.include_thinking,
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
                                    field: chunk.field,
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
                "search" => {
                    let input: HistorySearch = decode(input)?;
                    owner.session(&input.session_id).await?;
                    let fence = maka_presentation::watermark(input.through)
                        .map_err(|_| "agent_cursor_invalid")?;
                    if input.through > *owner.store.subscribe_commits().borrow()
                        || input.after > fence
                        || input.query.is_empty()
                        || input.query.len() > 512
                    {
                        return Err("agent_search_invalid".into());
                    }
                    let page = if owner
                        .store
                        .prepare_transcript(&input.session_id, input.through, 32)
                        .await
                        .map_err(|_| "agent_history_unavailable")?
                    {
                        let batch = owner
                            .store
                            .search_transcript(
                                &input.session_id,
                                maka_event_log::transcript::search::TranscriptSearch {
                                    through: fence,
                                    after: input.after,
                                    query: input.query,
                                    include_internal: input.include_internal,
                                    max_matches: 16,
                                },
                            )
                            .await
                            .map_err(|_| "agent_history_unavailable")?;
                        HistorySearchPage::Ready {
                            through: input.through,
                            matches: batch
                                .matches
                                .into_iter()
                                .map(|found| HistoryMatch {
                                    field: found.field,
                                    cursor: maka_plugins::session::history::Cursor {
                                        field: Default::default(),
                                        sequence: found.sequence,
                                        offset: 0,
                                    },
                                    preview: found.preview,
                                })
                                .collect(),
                            next_after: batch.next_after,
                        }
                    } else {
                        HistorySearchPage::Preparing {
                            through: input.through,
                        }
                    };
                    owner.session(&input.session_id).await?;
                    if context.scope().cancellation.is_cancelled() {
                        return Err("agent_observation_cancelled".into());
                    }
                    let result = encode(page)?;
                    maka_js_runtime::plugin::check_json_delivery(&result, 16 * 1024)
                        .map_err(|_| "agent_history_unavailable")?;
                    Ok(result)
                }
                "source" => {
                    let input: HistorySourceRead = decode(input)?;
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
                        let sequence = match input.source {
                            HistorySourceReference::Sequence { sequence } => sequence,
                            HistorySourceReference::ToolUse { tool_use_id } => owner
                                .store
                                .tool_source_sequence(
                                    &input.session_id,
                                    input.through,
                                    &tool_use_id,
                                )
                                .await
                                .map_err(|_| "agent_history_unavailable")?,
                        };
                        HistorySourcePage::Ready {
                            through: input.through,
                            source: Box::new(
                                owner
                                    .store
                                    .history_source(
                                        &input.session_id,
                                        input.through,
                                        sequence,
                                        input.field,
                                        input.path,
                                        input.position,
                                        8192,
                                        64 * 1024,
                                    )
                                    .await
                                    .map_err(|_| "agent_history_unavailable")?,
                            ),
                        }
                    } else {
                        HistorySourcePage::Preparing {
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
                            .send(
                                input.session_id,
                                input.message,
                                input.model,
                                input.thinking_level,
                                operation,
                            )
                            .await?,
                    )
                }
                "stop" => {
                    let input: Stop = decode(input)?;
                    let operation = context.operation().ok_or("agent_mutation_required")?;
                    encode(Stopped {
                        requested: owner
                            .stop_run(&input.session_id, &input.invocation_id, operation)
                            .await?,
                    })
                }
                "queue" => {
                    let input: SessionRead = decode(input)?;
                    encode(owner.queue(&input.session_id).await?)
                }
                "queueMessage" => encode(owner.queue_message(decode(input)?).await?),
                "editQueue" => {
                    let operation = context.operation().ok_or("agent_mutation_required")?;
                    encode(owner.edit_queue(decode(input)?, operation).await?)
                }
                "pendingInteractions" => {
                    let input: SessionRead = decode(input)?;
                    encode(owner.pending_interactions(&input.session_id).await?)
                }
                "interaction" => {
                    let input: InteractionRead = decode(input)?;
                    encode(
                        owner
                            .interaction(&input.session_id, &input.request_id)
                            .await?,
                    )
                }
                "respond" => {
                    context.operation().ok_or("agent_mutation_required")?;
                    encode(owner.respond(decode(input)?).await?)
                }
                "watch" => {
                    let input: Watch = decode(input)?;
                    owner.session(&input.session_id).await?;
                    let mut commits = owner.store.subscribe_commits();
                    let mut after = input.after;
                    let mut queue_revision = None;
                    let mut interactions_revision = None;
                    loop {
                        if context.scope().cancellation.is_cancelled() {
                            return Err("agent_observation_cancelled".into());
                        }
                        // Queue edits need not advance the transcript cursor. Deliver
                        // them fairly even while catching up a growing transcript.
                        let revision = owner
                            .store
                            .message_queue_revision(&input.session_id)
                            .await
                            .map_err(|_| "agent_queue_unavailable")?;
                        if queue_revision != Some(revision) {
                            let queue = owner.queue(&input.session_id).await?;
                            queue_revision = Some(queue.revision);
                            let update = encode(DeliveryPage::Queue { queue })?;
                            maka_js_runtime::plugin::check_json_delivery(&update, 48 * 1024)
                                .map_err(|_| "agent_queue_unavailable")?;
                            context.emit(update).await?;
                        }
                        let pending = owner.pending_interactions(&input.session_id).await?;
                        if interactions_revision != Some(pending.revision) {
                            interactions_revision = Some(pending.revision);
                            context
                                .emit(encode(DeliveryPage::Interactions { pending })?)
                                .await?;
                        }
                        owner.session(&input.session_id).await?;
                        let through = *commits.borrow();
                        if after > through {
                            return Err("agent_cursor_invalid".into());
                        }
                        let page = owner
                            .store
                            .session_stream_events(&input.session_id, after, through, 32, 48 * 1024)
                            .await
                            .map_err(|_| "agent_observation_unavailable")?;
                        after = page.next_after.unwrap_or(page.through_sequence);
                        context
                            .emit(encode(DeliveryPage::Events {
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
fn catalog_filter(input: &List) -> maka_event_log::sessions::CatalogFilter {
    maka_event_log::sessions::CatalogFilter {
        include_archived: true,
        pending_only: matches!(input.filter, SessionFilter::NeedsYou),
    }
}
async fn catalog_page(owner: &Owner, input: &List) -> Result<SessionPage, String> {
    use maka_event_log::sessions::SessionPosition;
    let (revision, position) = match &input.position {
        crate::contract::SessionPosition::Start => (None, SessionPosition::Start),
        crate::contract::SessionPosition::At { cursor } => (None, SessionPosition::At(cursor)),
        crate::contract::SessionPosition::After { revision, cursor } => {
            (Some(revision.as_str()), SessionPosition::After(cursor))
        }
        crate::contract::SessionPosition::Before { revision, cursor } => {
            (Some(revision.as_str()), SessionPosition::Before(cursor))
        }
    };
    if matches!(position, SessionPosition::At(cursor) | SessionPosition::After(cursor) | SessionPosition::Before(cursor) if !owner.owns(cursor))
    {
        return Err("agent_cursor_invalid".into());
    }
    let page = owner
        .store
        .scoped_sessions::<SessionConfiguration>(
            CatalogScope::IdPrefix(owner.prefix.clone()),
            revision,
            position,
            catalog_filter(input),
        )
        .await
        .map_err(|_| "agent_catalog_unavailable")?;
    let reverse = matches!(position, SessionPosition::Before(_));
    let mut result = SessionPage {
        revision: page.revision,
        sessions: page.sessions.into_iter().map(summary).collect(),
        needs_you_count: page.pending_session_count,
        before: page.previous_cursor,
        after: page.next_cursor,
    };
    while !crate::workspace::fits(&result) {
        if result.sessions.len() <= 1 {
            return Err("agent_catalog_unavailable".into());
        }
        if reverse {
            result.sessions.remove(0);
            result.before = result.sessions.first().map(|session| session.id.clone());
        } else {
            result.sessions.pop();
            result.after = result.sessions.last().map(|session| session.id.clone());
        }
    }
    Ok(result)
}
fn decode<T: DeserializeOwned>(input: Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|_| "agent_input_invalid".into())
}
fn encode<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|_| "agent_output_unavailable".into())
}
pub(crate) fn summary(session: SessionRecord<SessionConfiguration>) -> Session {
    use maka_event_log::sessions::SessionExecutionState;
    use maka_runtime::event::TerminalStatus;
    let activity = if session.pending_interaction_since.is_some() {
        SessionActivity::Waiting
    } else if matches!(
        session.execution.map(|execution| execution.state),
        Some(
            SessionExecutionState::Live { .. }
                | SessionExecutionState::Ended {
                    status: TerminalStatus::Paused,
                    ..
                }
        )
    ) {
        SessionActivity::Working
    } else {
        SessionActivity::Idle
    };
    Session {
        permissions: session.configuration.permissions,
        boundary_revision: session.configuration.boundary_revision,
        activity,
        id: session.id,
        revision: session.revision,
        title: session.configuration.title,
        labels: session.configuration.labels,
        workspace: session.configuration.workspace,
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
    fn activated(&self) {
        self.0.activated();
    }
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        self.0.close()
    }
}
