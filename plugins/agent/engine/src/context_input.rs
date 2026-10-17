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

//! Fit only a provider's summary input. Canonical history is never shortened.
use crate::RunInput;
use maka_agent_facts::context::ModelPurpose;
use maka_agent_facts::model::ToolInput;
use maka_model::prompt::{AssistantPart, ContentPart, Message, ToolOutput};

/// A preflight estimate, never measured provider usage. Attachment encodings
/// are not text tokens; provider overflow is handled by fitting a smaller request.
pub(super) fn estimate(prompt: &[Message]) -> u64 {
    prompt
        .iter()
        .map(|message| {
            16 + match message {
                Message::System { content, .. } => text_tokens(content),
                Message::User { content, .. } => content.iter().map(content_tokens).sum(),
                Message::Assistant { content, .. } => content
                    .iter()
                    .map(|part| match part {
                        AssistantPart::Text { text, .. }
                        | AssistantPart::Reasoning { text, .. } => text_tokens(text),
                        AssistantPart::ToolCall {
                            tool_name, input, ..
                        } => {
                            16 + text_tokens(tool_name)
                                + match input {
                                    ToolInput::Json(value) => json_tokens(value),
                                    ToolInput::InvalidJson { raw } => text_tokens(raw),
                                }
                        }
                        AssistantPart::ToolResult {
                            tool_name, output, ..
                        } => 16 + text_tokens(tool_name) + output_tokens(output),
                    })
                    .sum(),
                Message::Tool { content, .. } => content
                    .iter()
                    .map(|part| 16 + text_tokens(&part.tool_name) + output_tokens(&part.output))
                    .sum(),
            }
        })
        .sum()
}
fn text_tokens(text: &str) -> u64 {
    (text.len() as u64).div_ceil(3)
}
fn content_tokens(part: &ContentPart) -> u64 {
    match part {
        ContentPart::Text { text, .. } => text_tokens(text),
        ContentPart::File { .. } => 1024,
    }
}
fn output_tokens(output: &ToolOutput) -> u64 {
    match output {
        ToolOutput::Text(text) | ToolOutput::ErrorText(text) => text_tokens(text),
        ToolOutput::Json(value) | ToolOutput::ErrorJson(value) => json_tokens(value),
        ToolOutput::Content(parts) => parts.iter().map(content_tokens).sum(),
    }
}
pub(super) fn json_tokens(value: &(impl serde::Serialize + ?Sized)) -> u64 {
    struct Count(u64);
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len() as u64);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    serde_json::to_writer(&mut count, value).expect("model prompt is serializable");
    count.0.div_ceil(3)
}

/// Keep 4% of total context capacity (200k if unknown) available for summary
/// output, even when the full prefix must first be sent to the provider.
fn summary_output_reserve<W>(input: &RunInput<W>) -> u64 {
    input
        .context
        .as_ref()
        .and_then(|context| context.model_context_window)
        .unwrap_or(200_000)
        .saturating_mul(4)
        .div_ceil(100)
        .max(1)
}

/// The desired summary output may exceed the reserve; the final request caps
/// it to available capacity. This is not a minimum summary length.
pub(super) fn summary_output_budget<W>(input: &RunInput<W>) -> u64 {
    summary_output_reserve(input).max(input.main_output_limit.unwrap_or(8000))
}

pub(super) fn summary_budget<W>(input: &RunInput<W>) -> u64 {
    let context = input.context.as_ref();
    let total = context
        .and_then(|context| context.model_context_window)
        .unwrap_or(200_000);
    let window = context
        .and_then(|context| context.context_window)
        .unwrap_or(total);
    window.min(total.saturating_sub(summary_output_reserve(input)))
}

pub(super) fn needs_compaction<W>(input: &RunInput<W>, prompt: &[Message]) -> bool {
    estimate(prompt) > summary_budget(input)
}

/// Keep complete recent turns when possible. A single enormous turn becomes a
/// labelled head/tail excerpt, not an invalid partial tool-call/result sequence.
/// This is a summary-only rendering; the exact request digest records this input.
pub(super) fn fit_summary(prompt: &[Message], tokens: u64) -> Vec<Message> {
    if estimate(prompt) <= tokens {
        return prompt.to_vec();
    }
    // Preserve instruction roles and keep the conversational anchor independent
    // of the System prefix. If the System alone cannot fit, provider overflow
    // will fail safely rather than silently truncating its instructions.
    let system_end = prompt
        .iter()
        .take_while(|message| matches!(message, Message::System { .. }))
        .count();
    let (system, prompt) = prompt.split_at(system_end);
    let tokens = tokens.saturating_sub(estimate(system));
    let bytes = usize::try_from(tokens.saturating_mul(3)).unwrap_or(usize::MAX);
    const NOTICE: &str = "Conversation excerpts for summarization: some older text is omitted to fit this model. The complete original history remains stored locally. Preserve known goals and recent progress; do not infer facts from omitted text.\n\n";
    let available = bytes.saturating_sub(NOTICE.len() + 100);
    // Preserve the original task or previous checkpoint as well as the recent tail.
    let head = prompt
        .first()
        .map(|message| serde_json::to_string(message).expect("model prompt"))
        .unwrap_or_default();
    let head = excerpt(&head, available / 4);
    let remaining = available.saturating_sub(head.len() + 32);
    let mut starts = vec![0];
    for (index, message) in prompt.iter().enumerate().skip(1) {
        if matches!(message, Message::User { .. }) {
            starts.push(index);
        }
    }
    starts.push(prompt.len());
    let mut first = prompt.len();
    let mut used = 0;
    for unit in starts.windows(2).rev() {
        let size = estimate(&prompt[unit[0]..unit[1]]).saturating_mul(3);
        if size > remaining.saturating_sub(used) as u64 {
            break;
        }
        used += size as usize;
        first = unit[0];
    }
    if first == prompt.len() {
        first = *starts.iter().rev().nth(1).unwrap_or(&0);
    }
    let tail = serde_json::to_string(&prompt[first..]).expect("model prompt");
    let text = format!(
        "{NOTICE}Earlier task or summary:\n{head}\n\nRecent conversation:\n{}",
        excerpt(&tail, remaining)
    );
    let mut fitted = system.to_vec();
    fitted.push(Message::user(text));
    fitted
}

fn excerpt(text: &str, bytes: usize) -> String {
    if text.len() <= bytes {
        return text.into();
    }
    const GAP: &str = "\n[… omitted from summary input …]\n";
    if bytes < GAP.len() {
        return String::new();
    }
    let remaining = bytes - GAP.len();
    let start = text.floor_char_boundary(remaining / 2);
    let end = text.ceil_char_boundary(text.len() - (remaining - remaining / 2));
    format!("{}{GAP}{}", &text[..start], &text[end..])
}

/// The final request owns its output reserve, after all prompt/tool projections.
/// This cap never changes the configured model limit or the persisted history.
pub(super) fn output_limit<W>(
    input: &RunInput<W>,
    prompt: &[Message],
    tools: &[maka_model::ToolDefinition],
    requested: u64,
    purpose: ModelPurpose,
) -> u64 {
    let minimum = match purpose {
        ModelPurpose::Main => 1,
        ModelPurpose::Summary => summary_output_reserve(input),
    };
    input
        .context
        .as_ref()
        .and_then(|context| context.model_context_window)
        .map_or(requested, |window| {
            requested.min(
                window
                    .saturating_sub(estimate(prompt).saturating_add(json_tokens(tools)))
                    .max(minimum),
            )
        })
}
