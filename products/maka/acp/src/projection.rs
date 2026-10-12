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

//! Bounded ACP presentation of public observations and durable transcript rows.
use agent_client_protocol::schema::{MaybeUndefined, v2 as acp};
use maka_agent_api::contract::{ContentKind, HistoryChunk};
use maka_agent_ports::session::history::HistoryField;
use maka_agent_ports::session::history::Role;
use serde_json::{Value, json};
use std::collections::HashMap;

mod messages;
mod tools;

const IDENTITIES: usize = 4096;
const TOTAL_BYTES: usize = 16 * 1024 * 1024;
const TOOL_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum MessageKind {
    User,
    Text,
    Thinking,
}

struct Message {
    text: String,
    meta: acp::Meta,
    bytes: usize,
}

#[derive(Default)]
pub struct Projection {
    streams: HashMap<(String, String, MessageKind), Message>,
    tools: HashMap<String, Tool>,
    bytes: usize,
    assembly: Option<(u64, HistoryField, String, acp::Meta)>,
}

struct Tool {
    turn: String,
    card: acp::ToolCallUpdate,
    published: Option<acp::ToolCallUpdate>,
    published_bytes: usize,
    authoritative: bool,
}

impl Projection {
    pub fn chunk(&mut self, chunk: &HistoryChunk) -> Result<Vec<acp::SessionUpdate>, crate::Error> {
        if chunk.offset == 0 {
            let mut meta = message_meta(&chunk.turn_id, &chunk.message_id);
            if !chunk.attachments.is_empty() {
                meta["maka"]["attachments"] = serde_json::to_value(&chunk.attachments)?;
            }
            self.assembly = Some((chunk.sequence, chunk.field, String::new(), meta));
        }
        let (sequence, field, text, _) = self
            .assembly
            .as_mut()
            .ok_or("History field starts with a missing fragment")?;
        if *sequence != chunk.sequence || *field != chunk.field || text.len() as u64 != chunk.offset
        {
            return Err("History field fragments are discontinuous".into());
        }
        if chunk.total_bytes > TOTAL_BYTES as u64
            || text.len() + chunk.text.len() > chunk.total_bytes as usize
        {
            return Err("History field exceeds its declared size".into());
        }
        text.push_str(&chunk.text);
        if text.len() as u64 != chunk.total_bytes {
            return Ok(vec![]);
        }
        let (_, _, text, meta) = self.assembly.take().expect("complete field");
        let kind = match (chunk.role, chunk.field) {
            (Role::User, _) => MessageKind::User,
            (Role::Assistant, HistoryField::Thinking) => MessageKind::Thinking,
            (Role::Assistant, HistoryField::Body) => MessageKind::Text,
            _ => return self.tool_chunk(chunk, &text),
        };
        self.text(&chunk.turn_id, &chunk.message_id, kind, &text, true, meta)
    }
    pub fn append(
        &mut self,
        turn: &str,
        id: &str,
        kind: &ContentKind,
        delta: &str,
    ) -> Result<Vec<acp::SessionUpdate>, crate::Error> {
        let kind = match kind {
            ContentKind::Text => MessageKind::Text,
            ContentKind::Thinking => MessageKind::Thinking,
        };
        let current = self
            .streams
            .get(&(turn.into(), id.into(), kind))
            .map_or("", |message| message.text.as_str());
        if current.len() + delta.len() > TOTAL_BYTES {
            return Err("ACP stream exceeds its text limit".into());
        }
        let next = format!("{current}{delta}");
        self.text(turn, id, kind, &next, false, message_meta(turn, id))
    }
}

fn message_meta(turn: &str, id: &str) -> acp::Meta {
    acp::Meta::from_iter([("maka".into(), json!({"hostMessageId":id,"turnId":turn}))])
}
fn bounded(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    let suffix = "\n[Presentation truncated]";
    let mut end = max - suffix.len();
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{suffix}", &text[..end])
}
