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
                    if input
                        .cursor
                        .as_ref()
                        .is_some_and(|cursor| !owner.owns(cursor))
                    {
                        return Err("agent_cursor_invalid".into());
                    }
                    let page = owner
                        .store
                        .scoped_sessions::<SessionConfiguration>(
                            CatalogScope::IdPrefix(owner.prefix.clone()),
                            input.revision.as_deref(),
                            input.cursor.as_deref(),
                            false,
                        )
                        .await
                        .map_err(|_| "agent_catalog_unavailable")?;
                    encode(SessionPage {
                        revision: page.revision,
                        sessions: page.sessions.into_iter().map(summary).collect(),
                        next_cursor: page.next_cursor,
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
                                model: owner.settings.model.clone(),
                                provider: owner.settings.provider.clone(),
                            },
                            now(),
                        )
                        .await
                        .map_err(|_| "agent_creation_failed")?;
                    encode(summary(session))
                }
                "history" => {
                    use maka_plugins::session::history::{Page, Read};
                    let input: Read = decode(input)?;
                    input.validate().map_err(|_| "agent_cursor_invalid")?;
                    owner.session(&input.session_id).await?;
                    let high_water = *owner.store.subscribe_commits().borrow();
                    let through = input.through.unwrap_or(high_water);
                    if through > high_water {
                        return Err("agent_cursor_invalid".into());
                    }
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
                            next,
                        } = owner
                            .store
                            .history_text(&input.session_id, through, input.cursor, 4096, 2)
                            .await
                            .map_err(|_| "agent_history_unavailable")?
                        else {
                            return Err("agent_history_unavailable".into());
                        };
                        HistoryPage::Ready {
                            through,
                            next,
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
                                })
                                .collect(),
                        }
                    };
                    owner.session(&input.session_id).await?;
                    if context.scope().cancellation.is_cancelled() {
                        return Err("agent_observation_cancelled".into());
                    }
                    encode(result)
                }
                "send" => {
                    let input: Send = decode(input)?;
                    let operation = context.operation().ok_or("agent_mutation_required")?;
                    encode(
                        owner
                            .send(input.session_id, input.message, operation)
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
    let (kind, text, part_id, failed) = match event.fact {
        StreamFact::InvocationOpened => (DeliveryKind::Opened, None, None, None),
        StreamFact::PartStarted { part_id, .. } => {
            (DeliveryKind::PartStarted, None, Some(part_id), None)
        }
        StreamFact::PartDelta { part_id, text, .. } => {
            (DeliveryKind::Delta, Some(text), Some(part_id), None)
        }
        StreamFact::PartFinished { part_id, .. } => {
            (DeliveryKind::PartFinished, None, Some(part_id), None)
        }
        StreamFact::InvocationEnded { failed, .. } => {
            (DeliveryKind::Ended, None, None, Some(failed))
        }
        _ => return None,
    };
    Some(Delivery {
        sequence: event.sequence,
        invocation_id: event.invocation.invocation_id,
        kind,
        text,
        part_id,
        failed,
    })
}

impl maka_runtime_host::application::NativeOwner for Agent {
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        self.0.close()
    }
}
