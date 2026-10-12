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

use super::read::{prepared, sql_number};
use crate::{EventLog, StoreError, sequence_number, sessions};
use maka_agent_ports::session::history::SourceField;
use serde_json::Value;
use sqlx::{Connection, Row, SqliteConnection};

pub struct TranscriptSearch {
    pub through: u64,
    /// Last scanned message sequence, exclusive. Zero starts at the beginning.
    pub after: u64,
    pub query: String,
    pub include_internal: bool,
    pub max_matches: usize,
}
pub struct TranscriptMatch {
    pub sequence: u64,
    pub field: SourceField,
    pub preview: String,
}
pub struct TranscriptSearchBatch {
    pub matches: Vec<TranscriptMatch>,
    pub next_after: Option<u64>,
}

impl EventLog {
    /// Read the disposable index without hydrating canonical events or emitting commits.
    /// At most 64 rows / 16 MiB of indexed payload are scanned per call.
    pub async fn search_transcript(
        &self,
        session: &str,
        input: TranscriptSearch,
    ) -> Result<TranscriptSearchBatch, StoreError> {
        self.validate_root()?;
        sessions::validate_id(session)?;
        let through = sql_number(input.through)?;
        let after = sql_number(input.after)?;
        if input.query.is_empty()
            || input.query.len() > 512
            || !(1..=64).contains(&input.max_matches)
            || after > through
        {
            return Err(StoreError::InvalidTransition(
                "invalid transcript search".into(),
            ));
        }
        let session = session.to_owned();
        self.connection
            .run(move |connection| {
                Box::pin(async move {
                    let mut tx = connection.begin().await?;
                    prepared(&mut tx, &session, input.through).await?;
                    let headers = sqlx::query(
                        "SELECT sequence, total_bytes FROM transcript_rows
                 WHERE session_id = ?1 AND sequence > ?2 AND sequence <= ?3
                 ORDER BY sequence LIMIT 65",
                    )
                    .bind(&session)
                    .bind(after)
                    .bind(through)
                    .fetch_all(&mut *tx)
                    .await?;
                    let mut result = TranscriptSearchBatch {
                        matches: vec![],
                        next_after: None,
                    };
                    let mut bytes = 0;
                    let mut position = input.after;
                    for (index, header) in headers.iter().enumerate() {
                        let size = sequence_number(header.try_get(1)?)?;
                        if size > maka_presentation::MAX_TOOL_ROW_BYTES as u64 {
                            return Err(StoreError::PrefixTooLarge);
                        }
                        if index == 64
                            || (index > 0 && bytes + size > 16 * 1024 * 1024)
                            || result.matches.len() == input.max_matches
                        {
                            result.next_after = Some(position);
                            break;
                        }
                        bytes += size;
                        position = sequence_number(header.try_get(0)?)?;
                        let payload: Vec<u8> = sqlx::query_scalar(
                    "SELECT payload FROM transcript_rows WHERE session_id = ? AND sequence = ?"
                ).bind(&session).bind(position as i64).fetch_one(&mut *tx).await?;
                        let row: Value = serde_json::from_slice(&payload)?;
                        if !input.include_internal
                            && internal(&mut tx, &session, through, position, &row).await?
                        {
                            continue;
                        }
                        if let Some((field, preview)) = matching_text(&row, &input.query) {
                            result.matches.push(TranscriptMatch {
                                sequence: position,
                                field,
                                preview,
                            });
                        }
                    }
                    Ok(result)
                })
            })
            .await
    }
}

async fn internal(
    tx: &mut SqliteConnection,
    session: &str,
    through: i64,
    sequence: u64,
    row: &Value,
) -> Result<bool, StoreError> {
    // Errors remain discoverable even when normal orchestration is hidden.
    if row["isError"] == true {
        return Ok(false);
    }
    let owned;
    let call = if row["type"] == "tool_result" {
        owned = super::tool_metadata::read(tx, session, sequence, through, 64 * 1024).await?;
        let Some(call) = owned.as_ref().and_then(|tool| tool.call.as_ref()) else {
            return Ok(false);
        };
        (row["origin"].as_str(), Some(call.name.as_str()))
    } else if row["type"] == "tool_call" {
        (row["origin"].as_str(), row["toolName"].as_str())
    } else {
        return Ok(false);
    };
    Ok(matches!(
        call,
        (Some("provider"), Some("exec" | "wait" | "tool_search"))
            | (Some("code_mode"), Some("code_cell" | "tool_search"))
    ))
}

fn matching_text(row: &Value, query: &str) -> Option<(SourceField, String)> {
    let find = |field, value: &str| text(value, query).map(|preview| (field, preview));
    match row["type"].as_str()? {
        "user" => find(
            SourceField::Body,
            row["displayText"]
                .as_str()
                .or_else(|| row["text"].as_str())?,
        ),
        "assistant" => find(SourceField::Body, row["text"].as_str().unwrap_or("")).or_else(|| {
            find(
                SourceField::Thinking,
                row["thinking"]["text"].as_str().unwrap_or(""),
            )
        }),
        "tool_call" => find(
            SourceField::ToolName,
            row["toolName"].as_str().unwrap_or(""),
        )
        .or_else(|| {
            json_text(&row["args"], query).map(|preview| (SourceField::Arguments, preview))
        }),
        "tool_result" => match row["content"]["kind"].as_str()? {
            "text" => find(SourceField::Result, row["content"]["text"].as_str()?),
            "json" => json_text(&row["content"]["value"], query)
                .map(|preview| (SourceField::Result, preview)),
            "operation" => json_text(
                &row["content"]["reply"]["receipt"]["outcome"]["value"],
                query,
            )
            .map(|preview| (SourceField::Result, preview)),
            _ => None, // No searching image bytes or storage locators.
        },
        "turn_state" if row["status"] == "failed" => find(
            SourceField::FailureMessage,
            row["failureMessage"].as_str().unwrap_or(""),
        )
        .or_else(|| {
            find(
                SourceField::ErrorClass,
                row["errorClass"].as_str().unwrap_or(""),
            )
        }),
        _ => None,
    }
}
fn json_text(value: &Value, query: &str) -> Option<String> {
    match value {
        Value::String(value) => text(value, query),
        Value::Array(values) => values.iter().find_map(|value| json_text(value, query)),
        Value::Object(values) => {
            if matches!(value["type"].as_str(), Some("image" | "audio")) || value["kind"] == "image"
            {
                return None;
            }
            values
                .iter()
                .find_map(|(key, value)| text(key, query).or_else(|| json_text(value, query)))
        }
        Value::Null => None,
        value => text(&value.to_string(), query),
    }
}
fn text(value: &str, query: &str) -> Option<String> {
    let found = value.find(query)?;
    let start = value.floor_char_boundary(found.saturating_sub(64));
    let end = value.floor_char_boundary((start + 378).min(value.len()));
    let mut preview = String::new();
    if start > 0 {
        preview.push('…');
    }
    preview.extend(
        value[start..end]
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c }),
    );
    if end < value.len() {
        preview.push('…');
    }
    Some(preview)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn search_is_literal_content_only_and_previews_are_bounded_unicode() {
        let row = json!({"type":"user","id":"private-id","text":"raw hidden","displayText":"中文🦀 needle"});
        assert_eq!(
            matching_text(&row, "中文")
                .map(|(_, preview)| preview)
                .as_deref(),
            Some("中文🦀 needle")
        );
        assert!(matching_text(&row, "private-id").is_none());
        assert!(matching_text(&row, "hidden").is_none());
        assert!(matching_text(&row, "NEEDLE").is_none());
        let operation = json!({"type":"tool_result","content":{"kind":"operation","reply":{
            "reference":{"method":"private-reference"},"receipt":{"outcome":{"value":{"text":"business 中😀", "number":42}}}
        }}});
        assert_eq!(
            matching_text(&operation, "中😀"),
            Some((SourceField::Result, "business 中😀".into()))
        );
        assert!(matching_text(&operation, "private-reference").is_none());
        // Shape alone does not change ordinary JSON's content semantics.
        let ordinary = json!({"type":"tool_result","content":{"kind":"json","value":operation["content"]["reply"]}});
        assert!(matching_text(&ordinary, "private-reference").is_some());
        assert!(
            matching_text(
                &json!({"type":"tool_result","content":{"kind":"image","ref":"needle"}}),
                "needle"
            )
            .is_none()
        );
        assert!(
            json_text(
                &json!({"content":[{"type":"image","data":"needle"}]}),
                "needle"
            )
            .is_none()
        );
        assert!(
            json_text(
                &json!({"content":[{"type":"text","text":"needle"}]}),
                "needle"
            )
            .is_some()
        );
        let preview = text(
            &format!("{}needle\n{}", "界".repeat(100), "界".repeat(200)),
            "needle",
        )
        .unwrap();
        assert!(
            preview.len() <= 384
                && preview.contains("needle ")
                && preview.starts_with('…')
                && preview.ends_with('…')
        );
        assert_eq!(text("a .* b", ".*").as_deref(), Some("a .* b"));
    }
}
