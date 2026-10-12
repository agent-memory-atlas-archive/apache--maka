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
    pub(super) fn text(
        &mut self,
        turn: &str,
        id: &str,
        kind: MessageKind,
        next: &str,
        replace: bool,
        meta: acp::Meta,
    ) -> Result<Vec<acp::SessionUpdate>, crate::Error> {
        let key = (turn.to_owned(), id.to_owned(), kind);
        let previous = self.streams.get(&key);
        if previous.is_some_and(|p| p.text == next && p.meta == meta)
            || (previous.is_none() && next.is_empty() && kind != MessageKind::User)
        {
            return Ok(vec![]);
        }
        if previous.is_none() && self.streams.len() + self.tools.len() >= IDENTITIES {
            return Err("ACP presentation identity limit exceeded".into());
        }
        let size = next.len() + serde_json::to_vec(&meta)?.len();
        let old_size = previous.map_or(0, |p| p.bytes);
        if self.bytes - old_size + size > TOTAL_BYTES {
            return Err("ACP text presentation limit exceeded".into());
        }
        let current = previous.map_or("", |p| p.text.as_str());
        let message_id = match kind {
            MessageKind::Thinking => format!("thinking:{id}"),
            _ => id.to_owned(),
        };
        let suffix = if replace {
            None
        } else {
            next.strip_prefix(current)
        };
        let update = match (kind, suffix) {
            (MessageKind::Text, Some(suffix)) => acp::SessionUpdate::AgentMessageChunk(
                acp::ContentChunk::new(suffix.into(), message_id).meta(meta.clone()),
            ),
            (MessageKind::Thinking, Some(suffix)) => acp::SessionUpdate::AgentThoughtChunk(
                acp::ContentChunk::new(suffix.into(), message_id).meta(meta.clone()),
            ),
            (kind, _) => {
                let content = MaybeUndefined::Value(vec![acp::ContentBlock::from(next)]);
                let metadata = MaybeUndefined::Value(meta.clone());
                match kind {
                    MessageKind::User => acp::SessionUpdate::UserMessage(
                        acp::UserMessage::new(message_id)
                            .content(content)
                            .meta(metadata),
                    ),
                    MessageKind::Text => acp::SessionUpdate::AgentMessage(
                        acp::AgentMessage::new(message_id)
                            .content(content)
                            .meta(metadata),
                    ),
                    MessageKind::Thinking => acp::SessionUpdate::AgentThought(
                        acp::AgentThought::new(message_id)
                            .content(content)
                            .meta(metadata),
                    ),
                }
            }
        };
        self.bytes = self.bytes - old_size + size;
        self.streams.insert(
            key,
            Message {
                text: next.to_owned(),
                meta,
                bytes: size,
            },
        );
        Ok(vec![update])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_stream_and_canonical_history_keep_the_same_message() {
        let mut projection = Projection::default();
        projection
            .append("t", "m", &ContentKind::Text, "🦀中")
            .unwrap();
        let update = projection
            .append("t", "m", &ContentKind::Text, "!")
            .unwrap();
        assert_eq!(
            serde_json::to_value(&update[0]).unwrap()["content"]["text"],
            "!"
        );
        let mut chunk = HistoryChunk {
            attachments: Vec::new(),
            timing: None,
            field: HistoryField::Body,
            message_id: "m".into(),
            turn_id: "t".into(),
            timestamp: 0,
            role: Role::Assistant,
            sequence: 1,
            offset: 0,
            total_bytes: 8,
            text: "🦀中".into(),
            tool: None,
        };
        assert!(projection.chunk(&chunk).unwrap().is_empty());
        chunk.offset = 7;
        chunk.text = "!".into();
        assert!(projection.chunk(&chunk).unwrap().is_empty());
        chunk.offset = 0;
        chunk.text = "revised".into();
        chunk.total_bytes = 7;
        let update = serde_json::to_value(&projection.chunk(&chunk).unwrap()[0]).unwrap();
        assert_eq!(update["sessionUpdate"], "agent_message");
        assert_eq!(update["messageId"], "m");
        assert_eq!(update["content"][0]["text"], "revised");
    }
    #[test]
    fn missing_history_fragment_is_rejected() {
        let mut projection = Projection::default();
        let chunk = HistoryChunk {
            attachments: Vec::new(),
            timing: None,
            field: HistoryField::Body,
            message_id: "m".into(),
            turn_id: "t".into(),
            timestamp: 0,
            role: Role::Assistant,
            sequence: 1,
            offset: 2,
            total_bytes: 3,
            text: "a".into(),
            tool: None,
        };
        assert!(projection.chunk(&chunk).is_err());
    }
    #[test]
    fn attachment_only_user_message_keeps_safe_descriptions_during_replay() {
        let mut projection = Projection::default();
        let chunk = HistoryChunk {
            attachments: vec![maka_agent_api::contract::HistoryAttachment {
                name: "photo.png".into(),
                mime_type: "image/png".into(),
                bytes: 30,
                kind: maka_agent_facts::attachment::AttachmentKind::Image,
            }],
            timing: None,
            field: HistoryField::Body,
            message_id: "u".into(),
            turn_id: "t".into(),
            timestamp: 0,
            role: Role::User,
            sequence: 1,
            offset: 0,
            total_bytes: 0,
            text: String::new(),
            tool: None,
        };
        let update = serde_json::to_value(&projection.chunk(&chunk).unwrap()[0]).unwrap();
        assert_eq!(update["sessionUpdate"], "user_message");
        assert_eq!(
            update["_meta"]["maka"]["attachments"][0]["name"],
            "photo.png"
        );
        assert!(
            update["_meta"]["maka"]["attachments"][0]
                .get("ref")
                .is_none()
        );
        assert!(projection.chunk(&chunk).unwrap().is_empty());
    }
}
