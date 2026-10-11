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
use crate::skills::contract::{Preparation, PreparationRecord, Prepare, SkillToolRequest};
use futures_util::FutureExt;
use maka_plugins::call::{Attachments, Scope};
use maka_runtime::input::InputReceipt;
use maka_runtime_host::application::NativeServices;
use std::{collections::BTreeSet, panic::AssertUnwindSafe};

const SOURCE: &str = "maka.agent.skills";
#[derive(Serialize)]
struct Selected {
    import: String,
    binding: String,
}
pub(super) struct Library {
    selected: Selected,
    services: NativeServices,
    source: SourceRevision,
    load: Option<String>,
    search: Option<String>,
}
pub(super) enum Input {
    Ready {
        message: MessageInput,
        required_tools: BTreeSet<String>,
    },
    Rejected {
        reason: SkillInputRejection,
        skills: Option<maka_skills::SkillInvocationResult>,
    },
}
impl Library {
    pub fn select(
        context: &NativeContext,
        settings: &Configuration,
        mut source: SourceRevision,
    ) -> Result<Option<Self>, String> {
        let Some(import) = &settings.skill_library else {
            return Ok(None);
        };
        let services = context.services();
        let bindings = services
            .imports()
            .find(|(name, _)| *name == import)
            .map(|(_, bindings)| bindings)
            .ok_or("agent_skill_import_unavailable")?;
        let [binding] = bindings else {
            return Err("agent_skill_import_ambiguous".into());
        };
        if binding.contract() != &crate::skills::contract() {
            return Err("agent_skill_contract_invalid".into());
        }
        let name = |method: &str| {
            settings
                .tools
                .iter()
                .find(|tool| {
                    tool.input == ToolInput::Skill
                        && tool.method == method
                        && services.imports().any(|(name, targets)| {
                            name == tool.import
                                && targets.len() == 1
                                && targets[0].execution_basis() == binding.execution_basis()
                        })
                })
                .map(|tool| tool.name.clone())
        };
        let load = name("load");
        let search = name("search");
        let selected = Selected {
            import: import.clone(),
            binding: binding.execution_basis().into(),
        };
        source.kind = SourceKind::Input;
        source.name = SOURCE.into();
        Ok(Some(Self {
            selected,
            services,
            source,
            load,
            search,
        }))
    }
}
impl Owner {
    pub(super) async fn prepare_skills(
        &self,
        mut message: MessageInput,
        workspace: Option<crate::contract::Workspace>,
        parent: &Scope,
        tools: Vec<String>,
    ) -> Result<Input, String> {
        if parent.cancellation.is_cancelled() {
            return Err("agent_skill_call_cancelled".into());
        }
        let reject = |reason, skills| Ok(Input::Rejected { reason, skills });
        let Some(library) = &self.skills else {
            let discovery = Default::default();
            let preferences = maka_skills::Preferences::Available(Default::default());
            let host = Default::default();
            let catalog = maka_skills::Catalog {
                discovery: &discovery,
                preferences: &preferences,
                host: &host,
            };
            return match catalog.prepare_invocation(&message.text, &[]) {
                maka_skills::PreparedInvocation::Blocked(invocation) => {
                    reject(SkillInputRejection::SkillsUnavailable, Some(invocation))
                }
                maka_skills::PreparedInvocation::Passthrough => Ok(Input::Ready {
                    message,
                    required_tools: Default::default(),
                }),
                maka_skills::PreparedInvocation::Ready { .. } => {
                    Err("agent_skill_preparation_invalid".into())
                }
            };
        };
        let input = serde_json::to_value(SkillToolRequest {
            workspace,
            tools: tools.clone(),
            input: Prepare {
                message: message.text.clone(),
                load_tool: library
                    .load
                    .as_ref()
                    .filter(|name| tools.contains(name))
                    .cloned(),
                search_tool: library
                    .search
                    .as_ref()
                    .filter(|name| tools.contains(name))
                    .cloned(),
            },
        })
        .map_err(|_| "agent_skill_preparation_invalid")?;
        let import = library.selected.import.clone();
        let mut ticket = parent
            .resources
            .reserve()
            .map_err(|_| "agent_skill_call_closed")?;
        let request = library
            .services
            .request(parent.identity.clone(), parent.cancellation.child_token())?;
        let result = self
            .workers
            .spawn(async move {
                ticket.start();
                let result = AssertUnwindSafe(async {
                    request
                        .call(&import, 0, "prepare".into(), input, Attachments::default())?
                        .await
                })
                .catch_unwind()
                .await;
                let cleanup = request.finish().await;
                ticket.complete(cleanup.clone());
                cleanup?;
                result.map_err(|_| "agent_skill_preparation_panicked".to_owned())
            })
            .await
            .map_err(|_| "agent_skill_preparation_lost")??;
        if parent.cancellation.is_cancelled() {
            return Err("agent_skill_call_cancelled".into());
        }
        let result = match result {
            Ok(value) => serde_json::from_value::<Preparation>(value)
                .map_err(|_| "agent_skill_preparation_invalid")?,
            Err(_) => return reject(SkillInputRejection::SkillsUnavailable, None),
        };
        let (text, record, required_tools) = match result {
            Preparation::Ready {
                text,
                record,
                required_tools,
            } => (text, record, required_tools),
            Preparation::Blocked { invocation } => {
                return reject(SkillInputRejection::SkillsUnavailable, Some(invocation));
            }
            Preparation::InputTooLarge => return reject(SkillInputRejection::InputTooLarge, None),
        };
        record
            .invocation
            .validate()
            .map_err(|_| "agent_skill_preparation_invalid")?;
        if record.metadata_prompt.len() > 18_000
            || !maka_runtime::archive::valid_projection_digest(&record.revision)
            || required_tools.iter().any(|tool| !tools.contains(tool))
        {
            return Err("agent_skill_preparation_invalid".into());
        }
        if let Some(text) = text {
            message.display_text = Some(message.text.clone());
            let references =
                maka_skills::inline_references(&record.invocation.receipts, &message.text);
            message.inline_references = (!references.is_empty()).then_some(references);
            message.text = text;
            if message.text_bytes() > 64 * 1024
                || serde_json::to_vec(&message)
                    .map_err(|_| "agent_skill_preparation_invalid")?
                    .len()
                    > 64 * 1024
            {
                return reject(SkillInputRejection::InputTooLarge, None);
            }
        }
        let mut source = library.source.clone();
        source.revision = maka_runtime::artifact::content_digest(
            &serde_json::to_vec(&(&library.selected, &record))
                .map_err(|_| "agent_skill_preparation_invalid")?,
        );
        message.preparation.push(InputReceipt {
            source,
            receipt: serde_json::to_value(record).map_err(|_| "agent_skill_preparation_invalid")?,
        });
        maka_runtime::input::validate_receipts(&message.preparation)
            .map_err(|_| "agent_skill_preparation_invalid")?;
        Ok(Input::Ready {
            message,
            required_tools,
        })
    }
}
pub(super) fn prompt(message: &MessageInput) -> Result<Option<SystemPrompt>, String> {
    let mut receipts = message
        .preparation
        .iter()
        .filter(|receipt| receipt.source.name == SOURCE);
    let Some(receipt) = receipts.next() else {
        return Ok(None);
    };
    if receipts.next().is_some() {
        return Err("agent_skill_preparation_invalid".into());
    }
    let record: PreparationRecord = serde_json::from_value(receipt.receipt.clone())
        .map_err(|_| "agent_skill_preparation_invalid")?;
    if record.metadata_prompt.is_empty() {
        return Ok(None);
    }
    let prompt = SystemPrompt {
        text: record.metadata_prompt,
        policy_revision: 0,
        sources: vec![receipt.source.clone()],
    };
    prompt
        .validate()
        .map_err(|_| "agent_skill_preparation_invalid")?;
    Ok(Some(prompt))
}
