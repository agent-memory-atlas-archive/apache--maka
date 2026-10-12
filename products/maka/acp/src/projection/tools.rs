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

impl Projection {
    pub(super) fn tool_chunk(
        &mut self,
        chunk: &HistoryChunk,
        text: &str,
    ) -> Result<Vec<acp::SessionUpdate>, crate::Error> {
        let source = chunk
            .tool
            .as_ref()
            .ok_or("Tool history omitted its canonical reference")?;
        let tool = self.ensure_tool(&chunk.turn_id, &source.tool_use_id)?;
        if let Some(call) = &source.call {
            tool.card.name = MaybeUndefined::Value(call.name.clone());
            tool.card.title = MaybeUndefined::Value(bounded(
                call.title
                    .as_ref()
                    .map_or(call.name.as_str(), |title| title.fallback.as_str()),
                4096,
            ));
            tool.meta("presentation", json!(call.presentation));
        }
        if let Some(result) = &source.result {
            tool.authoritative = true;
            tool.card.status = MaybeUndefined::Value(
                if result.is_error
                    || matches!(
                        result.settlement,
                        Some(
                            maka_agent_facts::tool_call::ToolSettlement::Unknown
                                | maka_agent_facts::tool_call::ToolSettlement::Rejected
                                | maka_agent_facts::tool_call::ToolSettlement::Failed
                        )
                    )
                {
                    acp::ToolCallStatus::Failed
                } else {
                    acp::ToolCallStatus::Completed
                },
            );
            tool.meta("settlement", json!(result.settlement));
            tool.meta("image", json!(result.image));
        }
        if chunk.role == Role::ToolResult {
            tool.card.content = MaybeUndefined::Value(text_content(&bounded(text, TOOL_BYTES)));
        } else {
            tool.card.raw_input =
                MaybeUndefined::Value(json!({"preview":bounded(text, TOOL_BYTES)}));
        }
        self.publish(&source.tool_use_id)
    }

    /// Call after durable transcript catchup and authoritative turn settlement.
    pub fn finish(
        &mut self,
        turn: &str,
        completed: bool,
    ) -> Result<Vec<acp::SessionUpdate>, crate::Error> {
        if completed
            && self
                .tools
                .values()
                .any(|tool| tool.turn == turn && !tool.authoritative)
        {
            return Err("Completed turn is missing an announced durable tool result".into());
        }
        let ids: Vec<_> = self
            .tools
            .iter()
            .filter(|(_, tool)| tool.turn == turn && !tool.authoritative)
            .map(|(id, _)| id.clone())
            .collect();
        let mut updates = vec![];
        for id in ids {
            let tool = self.tools.get_mut(&id).expect("existing tool");
            tool.card.status = MaybeUndefined::Value(acp::ToolCallStatus::Failed);
            tool.meta("hostStatus", json!("interrupted"));
            tool.meta("resultPending", json!(false));
            updates.extend(self.publish(&id)?);
        }
        Ok(updates)
    }

    fn ensure_tool(&mut self, turn: &str, id: &str) -> Result<&mut Tool, crate::Error> {
        if !self.tools.contains_key(id) {
            if self.tools.len() + self.streams.len() >= IDENTITIES {
                return Err("ACP presentation identity limit exceeded".into());
            }
            self.tools.insert(
                id.to_owned(),
                Tool {
                    turn: turn.to_owned(),
                    published: None,
                    published_bytes: 0,
                    authoritative: false,
                    card: acp::ToolCallUpdate::new(id.to_owned())
                        .title(id.to_owned())
                        .kind(acp::ToolKind::Other)
                        .status(acp::ToolCallStatus::InProgress)
                        .content(Vec::<acp::ToolCallContent>::new())
                        .meta(acp::Meta::from_iter([(
                            "maka".into(),
                            json!({"turnId":turn}),
                        )])),
                },
            );
        }
        let tool = self.tools.get_mut(id).expect("inserted tool");
        if tool.turn != turn {
            return Err("Tool identity changed its turn".into());
        }
        Ok(tool)
    }

    fn publish(&mut self, id: &str) -> Result<Vec<acp::SessionUpdate>, crate::Error> {
        let tool = self.tools.get_mut(id).expect("existing tool");
        if tool.published.as_ref() == Some(&tool.card) {
            return Ok(vec![]);
        }
        let size = serde_json::to_vec(&tool.card)?.len();
        if self.bytes - tool.published_bytes + size > TOTAL_BYTES {
            return Err("ACP tool presentation limit exceeded".into());
        }
        self.bytes = self.bytes - tool.published_bytes + size;
        tool.published_bytes = size;
        tool.published = Some(tool.card.clone());
        Ok(vec![acp::SessionUpdate::ToolCallUpdate(tool.card.clone())])
    }
}

impl Tool {
    fn meta(&mut self, key: &str, value: Value) {
        if let MaybeUndefined::Value(meta) = &mut self.card.meta {
            meta["maka"][key] = value;
        }
    }
}

fn text_content(text: &str) -> Vec<acp::ToolCallContent> {
    vec![acp::ToolCallContent::Content(Box::new(acp::Content::new(
        text,
    )))]
}
