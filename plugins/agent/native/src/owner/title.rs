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
use auxiliary::{failed, persistence};
use futures_util::FutureExt;
use maka_agent_facts::{
    event::{Fact, InvocationInput},
    model::ModelFinishReason,
    tools::ToolError,
};
use maka_agent_ports::session::title::Policy;
use maka_event_log::usage::AuxiliarySource;
use maka_model::{ModelRequest, prompt::Message};

impl Owner {
    // Called under the original run admission gate; its TaskTracker owns the work through cleanup.
    pub(super) fn title_after_message(
        self: &Arc<Self>,
        invocation: Invocation,
        model: crate::model::AcceptedModel,
    ) {
        if !self.settings.automatic_titles || self.stop.is_cancelled() {
            return;
        }
        let Ok(mut ticket) = self.auxiliary.resources.reserve() else {
            return;
        };
        let owner = self.clone();
        self.workers.spawn(async move {
            ticket.start();
            let result = std::panic::AssertUnwindSafe(owner.generate_title(invocation, model))
                .catch_unwind()
                .await
                .unwrap_or_else(|_| {
                    Err(ToolError::CleanupUnconfirmed(
                        "Auxiliary title worker disappeared".into(),
                    ))
                });
            let cleanup = match &result {
                Err(ToolError::CleanupUnconfirmed(error)) => Err(error.clone()),
                _ => Ok(()),
            };
            ticket.complete(cleanup);
            if matches!(
                result,
                Err(ToolError::Persistence(_) | ToolError::CleanupUnconfirmed(_))
            ) {
                owner.runs.lock().await.closing = true;
                owner.stop.cancel();
            }
        });
    }
    async fn generate_title(
        &self,
        invocation: Invocation,
        model: crate::model::AcceptedModel,
    ) -> Result<(), ToolError> {
        let Some(session) = self
            .store
            .get_session::<SessionConfiguration>(&invocation.session_id)
            .await
            .map_err(persistence)?
        else {
            return Ok(());
        };
        if session.configuration.title_is_manual
            || session.archived
            || self
                .store
                .session_manager(&invocation.session_id)
                .await
                .map_err(persistence)?
                .is_some()
        {
            return Ok(());
        }
        let Some(opening) = self
            .store
            .first_message_opening(&invocation.session_id)
            .await
            .map_err(|error| match error {
                maka_event_log::StoreError::PrefixTooLarge => failed(error),
                other => persistence(other),
            })?
        else {
            return Ok(());
        };
        if opening.event.invocation != invocation {
            return Ok(());
        }
        let Fact::InvocationOpened {
            input:
                InvocationInput::Message {
                    content,
                    source_messages,
                    ..
                },
            configuration: Some(configuration),
            ..
        } = opening.event.fact
        else {
            return Ok(());
        };
        if configuration.model.as_ref() != Some(&model.binding) {
            return Ok(());
        }
        let text = source_messages
            .first()
            .map_or(content.text.as_str(), |source| {
                source.unprepared_content.text.as_str()
            });
        let Some(input) = maka_session_title::Title.prepare(text) else {
            return Ok(());
        };
        input.validate().map_err(failed)?;
        let cancellation = self.stop.child_token();
        let _cancel = cancellation.clone().drop_guard();
        let operation = async {
            let source = crate::model::Source::restore(
                self.configuration.clone(),
                self.credentials.clone(),
                model,
                invocation.session_id.clone(),
                self.revision.clone(),
            )
            .map_err(failed)?;
            let prepared = source.capture_auxiliary().await.map_err(failed)?;
            let requested = input.max_output_tokens.unwrap_or(2048).min(2048);
            let quote = self
                .configuration
                .quote_model(prepared.provider_id, source.accepted.binding.model.clone())
                .await
                .map_err(persistence)?;
            let mut prompt = Vec::new();
            if let Some(content) = input.system {
                prompt.push(Message::System {
                    content,
                    provider_options: None,
                });
            }
            prompt.push(Message::user(input.prompt));
            self.auxiliary
                .generate(
                    ModelRequest {
                        provider: prepared.provider,
                        prompt,
                        tools: Vec::new(),
                        provider_options: prepared.options,
                        max_output_tokens: Some(
                            prepared
                                .main_output_limit
                                .map_or(requested, |limit| limit.min(requested)),
                        ),
                    },
                    cancellation.clone(),
                    &self.store,
                    auxiliary::Admission::Source(AuxiliarySource::SessionTitle {
                        invocation: invocation.clone(),
                    }),
                    quote,
                )
                .await
        };
        tokio::pin!(operation);
        let generated = tokio::select! {
            biased;
            _ = tokio::time::sleep(Duration::from_secs(30)) => { cancellation.cancel(); operation.await },
            result = &mut operation => result,
        }?;
        if cancellation.is_cancelled() || generated.finish_reason != ModelFinishReason::Stop {
            return Ok(());
        }
        let Some(title) = maka_session_title::normalize(&generated.text) else {
            return Ok(());
        };
        // Main execution and read markers also move the public revision. A manual rename always wins.
        let runs = self.runs.lock().await;
        if runs.closing {
            return Ok(());
        }
        let Some(current) = self
            .store
            .get_session::<SessionConfiguration>(&invocation.session_id)
            .await
            .map_err(persistence)?
        else {
            return Ok(());
        };
        if current.archived
            || current.configuration.title_is_manual
            || current.configuration.title != session.configuration.title
            || self
                .store
                .session_manager(&invocation.session_id)
                .await
                .map_err(persistence)?
                .is_some()
        {
            return Ok(());
        }
        match self
            .store
            .update_session_metadata(
                &invocation.session_id,
                current.revision,
                move |configuration: &mut SessionConfiguration| {
                    if !configuration.title_is_manual {
                        configuration.title = title;
                    }
                    Ok(())
                },
            )
            .await
        {
            Ok(_)
            | Err(
                maka_event_log::StoreError::SessionNotFound
                | maka_event_log::StoreError::SessionRetired
                | maka_event_log::StoreError::SessionBusy,
            ) => Ok(()),
            Err(error) => Err(persistence(error)),
        }
    }
}
