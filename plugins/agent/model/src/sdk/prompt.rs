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

//! Borrowed SDK projection; canonical prompt serialization remains unchanged.

use crate::prompt::{AssistantPart, Message};
use maka_agent_facts::model::ToolInput;
use serde::{Serialize, Serializer};

pub(super) fn serialize<S: Serializer>(
    messages: &[Message],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(messages.iter().map(WireMessage))
}

struct WireMessage<'a>(&'a Message);
impl Serialize for WireMessage<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self.0 {
            Message::Assistant {
                content,
                provider_options,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("role", "assistant")?;
                map.serialize_entry("content", &WireParts(content))?;
                if let Some(options) = provider_options {
                    map.serialize_entry("providerOptions", options)?;
                }
                map.end()
            }
            message => message.serialize(serializer),
        }
    }
}
struct WireParts<'a>(&'a [AssistantPart]);
impl Serialize for WireParts<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(WirePart))
    }
}
struct WirePart<'a>(&'a AssistantPart);
impl Serialize for WirePart<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self.0 {
            AssistantPart::ToolCall {
                tool_call_id,
                tool_name,
                input,
                provider_executed,
                provider_options,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "tool-call")?;
                map.serialize_entry("toolCallId", tool_call_id)?;
                map.serialize_entry("toolName", tool_name)?;
                map.serialize_entry("input", &WireInput(input))?;
                if let Some(executed) = provider_executed {
                    map.serialize_entry("providerExecuted", executed)?;
                }
                if let Some(options) = provider_options {
                    map.serialize_entry("providerOptions", options)?;
                }
                map.end()
            }
            part => part.serialize(serializer),
        }
    }
}
struct WireInput<'a>(&'a ToolInput);
impl Serialize for WireInput<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            ToolInput::Json(value) => value.serialize(serializer),
            ToolInput::InvalidJson { .. } => {
                use serde::ser::SerializeMap;
                serializer.serialize_map(Some(0))?.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sdk_view_preserves_all_metadata_and_non_call_parts_without_mutating_canonical_input() {
        let messages = vec![Message::Assistant {
            content: vec![
                AssistantPart::Reasoning {
                    text: "reason".into(),
                    provider_options: Some(json!({"vendor":{"opaque":"reason"}})),
                },
                AssistantPart::ToolCall {
                    tool_call_id: "call".into(),
                    tool_name: "echo".into(),
                    input: ToolInput::Json(json!("abc")),
                    provider_executed: Some(true),
                    provider_options: Some(json!({"vendor":{"opaque":"call"}})),
                },
                AssistantPart::ToolCall {
                    tool_call_id: "broken".into(),
                    tool_name: "echo".into(),
                    input: ToolInput::InvalidJson { raw: "{".into() },
                    provider_executed: Some(false),
                    provider_options: None,
                },
            ],
            provider_options: Some(json!({"vendor":{"opaque":"message"}})),
        }];
        #[derive(Serialize)]
        struct Request<'a> {
            #[serde(serialize_with = "serialize")]
            prompt: &'a [Message],
        }
        let before = serde_json::to_value(&messages).unwrap();
        let wire = serde_json::to_value(Request { prompt: &messages }).unwrap();
        assert_eq!(
            wire,
            json!({"prompt":[{"role":"assistant","providerOptions":{"vendor":{"opaque":"message"}},"content":[
                {"type":"reasoning","text":"reason","providerOptions":{"vendor":{"opaque":"reason"}}},
                {"type":"tool-call","toolCallId":"call","toolName":"echo","input":"abc","providerExecuted":true,"providerOptions":{"vendor":{"opaque":"call"}}},
                {"type":"tool-call","toolCallId":"broken","toolName":"echo","input":{},"providerExecuted":false}
            ]}]})
        );
        assert_eq!(serde_json::to_value(&messages).unwrap(), before);
    }
}
