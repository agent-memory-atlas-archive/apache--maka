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
use crate::{EventLog, StoreError, sequence_number};
use maka_plugins::session::history::{Chunk, Cursor, HistoryField, Page, Position, Role, Tool};
use sqlx::{Connection, Row, SqliteConnection};

// One body projection for conversation windows and exact source reads.
pub(super) const BODY: &str = "COALESCE(CASE json_extract(payload, '$.type')
                    WHEN 'user' THEN (SELECT group_concat(line, char(10)) FROM (
                        SELECT COALESCE(json_extract(payload, '$.displayText'), json_extract(payload, '$.text'), '') AS line
                            WHERE COALESCE(json_extract(payload, '$.displayText'), json_extract(payload, '$.text'), '') != ''
                        UNION ALL
                        SELECT '↳ ' || json_extract(value, '$.name') || CASE
                            WHEN json_type(value, '$.bytes') IS NOT 'integer' THEN ''
                            WHEN json_extract(value, '$.bytes') < 1024 THEN ' · ' || json_extract(value, '$.bytes') || ' B'
                            WHEN json_extract(value, '$.bytes') < 1048576 THEN printf(' · %.1f KiB', json_extract(value, '$.bytes') / 1024.0)
                            ELSE printf(' · %.1f MiB', json_extract(value, '$.bytes') / 1048576.0) END
                            FROM json_each(payload, '$.attachments') WHERE json_type(value, '$.name') = 'text'
                        UNION ALL
                        SELECT '▱ ' || json_extract(value, '$.path')
                            FROM json_each(payload, '$.directoryReferences') WHERE json_type(value, '$.path') = 'text'))
                    WHEN 'assistant' THEN json_extract(payload, '$.text')
                    WHEN 'tool_call' THEN COALESCE(json_extract(payload, '$.intent'), json_extract(payload, '$.args.intent'), '')
                    WHEN 'tool_result' THEN CASE json_extract(payload, '$.content.kind')
                        WHEN 'text' THEN json_extract(payload, '$.content.text')
                        WHEN 'json' THEN (SELECT group_concat(atom, char(10)) FROM json_tree(payload, '$.content.value') WHERE type = 'text')
                        WHEN 'operation' THEN (SELECT group_concat(atom, char(10)) FROM json_tree(payload, '$.content.reply.receipt.outcome.value') WHERE type = 'text')
                        ELSE '' END
                    ELSE '' END, '')";

impl EventLog {
    /// Exact ascending UTF-8 windows over immutable derived text. The byte and
    /// fragment budgets apply to the whole window, never to a silently clipped
    /// message. Before reads a prefix ending exclusively at its cursor; After
    /// starts inclusively. `max_json_bytes` bounds the complete serialized page,
    /// including metadata and empty projection boundaries. An indivisible first
    /// fragment that exceeds it returns an error. Cursor positions grant no access.
    #[allow(clippy::too_many_arguments)]
    pub async fn history_text(
        &self,
        session: &str,
        through: u64,
        position: Position,
        include_thinking: bool,
        max_bytes: usize,
        max_chunks: usize,
        max_json_bytes: usize,
    ) -> Result<Page, StoreError> {
        self.validate_root()?;
        if !(4..=128 * 1024).contains(&max_bytes)
            || !(1..=8).contains(&max_chunks)
            || !(1024..=1024 * 1024).contains(&max_json_bytes)
        {
            return Err(invalid("invalid history window limits"));
        }
        crate::sessions::validate_id(session)?;
        let fence = maka_presentation::watermark(through)?;
        let through_sql = sql_number(fence)?;
        let session = session.to_owned();
        self.connection.run(move |connection| Box::pin(async move {
            let mut tx = connection.begin().await?;
            prepared(&mut tx, &session, fence).await?;
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM session_control WHERE id = ?)"
            ).bind(&session).fetch_one(&mut *tx).await?;
            if !exists { return Err(invalid("Session no longer exists")); }
            let anchor = match position {
                Position::Before { cursor } | Position::After { cursor } => {
                    if cursor.sequence == 0 || cursor.sequence > fence || (!include_thinking && cursor.field == HistoryField::Thinking) {
                        return Err(invalid("history cursor exceeds its fence"));
                    }
                    sql_number(cursor.offset)?;
                    materialize(&mut tx, &session, cursor.sequence).await?;
                    let row = header(&mut tx, &session, cursor.sequence, cursor.field).await?;
                    if cursor.offset > row.total {
                        return Err(invalid("history offset exceeds message"));
                    }
                    let byte: Vec<u8> = sqlx::query_scalar(
                        "SELECT substr(body, ?3, 1) FROM transcript_text WHERE session_id = ?1 AND sequence = ?2 AND field = ?4"
                    ).bind(&session).bind(sql_number(cursor.sequence)?).bind(sql_number(cursor.offset)? + 1).bind(field_name(cursor.field))
                        .fetch_one(&mut *tx).await?;
                    if byte.first().is_some_and(|byte| byte & 0xc0 == 0x80) {
                        return Err(invalid("history offset splits UTF-8"));
                    }
                    Some((cursor, row))
                }
                _ => None,
            };
            let (reverse, boundary, inclusive) = match position {
                Position::Start => (false, 0, true),
                Position::Tail => (true, through_sql, true),
                Position::Before { cursor } => (true, sql_number(cursor.sequence)?, cursor.offset > 0 || (include_thinking && cursor.field == HistoryField::Body)),
                Position::After { cursor } => (false, sql_number(cursor.sequence)?, cursor.offset < anchor.as_ref().unwrap().1.total || cursor.field == HistoryField::Thinking),
                Position::Event { sequence } => {
                    if sequence == 0 || sequence > through { return Err(invalid("history event exceeds its fence")); }
                    (false, sql_number(maka_presentation::watermark(sequence - 1)? + 1)?, true)
                }
            };
            let query = if reverse {
                "SELECT sequence FROM transcript_rows WHERE session_id = ?1 AND sequence <= ?2
                    AND sequence <= ?3 AND (?4 OR sequence < ?3) ORDER BY sequence DESC LIMIT ?5"
            } else {
                "SELECT sequence FROM transcript_rows WHERE session_id = ?1 AND sequence <= ?2
                    AND sequence >= ?3 AND (?4 OR sequence > ?3) ORDER BY sequence ASC LIMIT ?5"
            };
            let rows: Vec<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(query))
                .bind(&session).bind(through_sql).bind(boundary).bind(inclusive)
                .bind((max_chunks + 1) as i64).fetch_all(&mut *tx).await?;
            let mut chunks = Vec::new();
            let mut bytes = 0;
            let mut projected = 0;
            let mut left = None;
            let mut right = None;
            'rows: for sequence in rows {
                if chunks.len() == max_chunks || bytes == max_bytes || (projected >= 1024 * 1024 && left.is_some()) {
                    break;
                }
                let sequence = sequence_number(sequence)?;
                // Materialize only actually visited rows, never a long message's
                // following rows. One oversized first row still progresses.
                projected += materialize(&mut tx, &session, sequence).await?;
                let has_thinking: bool = include_thinking && sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM transcript_text WHERE session_id = ?1 AND sequence = ?2 AND field = 'thinking')"
                ).bind(&session).bind(sql_number(sequence)?).fetch_one(&mut *tx).await?;
                let mut fields = vec![HistoryField::Body];
                if include_thinking && has_thinking { fields.insert(0, HistoryField::Thinking); }
                if reverse { fields.reverse(); }
                for field in fields {
                    if chunks.len() == max_chunks || bytes == max_bytes { break 'rows; }
                    let row = header(&mut tx, &session, sequence, field).await?;
                    if let Some((cursor, _)) = &anchor && cursor.sequence == sequence {
                        let before = field == HistoryField::Thinking && cursor.field == HistoryField::Body;
                        let after = field == HistoryField::Body && cursor.field == HistoryField::Thinking;
                        if (reverse && after) || (!reverse && before) { continue; }
                        if field == cursor.field && ((reverse && cursor.offset == 0) || (!reverse && cursor.offset == row.total)) { continue; }
                    }
                    let cursor_offset = anchor.as_ref().filter(|(cursor, _)| cursor.sequence == sequence && cursor.field == field)
                        .map(|(cursor, _)| cursor.offset);
                    let (mut start, end) = if reverse {
                        let end = cursor_offset.unwrap_or(row.total);
                        (end.saturating_sub((max_bytes - bytes) as u64), end)
                    } else {
                        let start = cursor_offset.unwrap_or(0);
                        (start, row.total.min(start + (max_bytes - bytes) as u64))
                    };
                    let fragment: Vec<u8> = sqlx::query_scalar(
                        "SELECT substr(body, ?3, ?4) FROM transcript_text WHERE session_id = ?1 AND sequence = ?2 AND field = ?5"
                    ).bind(&session).bind(sql_number(sequence)?).bind(sql_number(start)? + 1)
                        .bind(sql_number(end - start)?).bind(field_name(field)).fetch_one(&mut *tx).await?;
                    let text = if reverse {
                        let skip = fragment.iter().take_while(|byte| **byte & 0xc0 == 0x80).count();
                        start += skip as u64;
                        std::str::from_utf8(&fragment[skip..]).map_err(|_| invalid("invalid history UTF-8"))?
                    } else {
                        match std::str::from_utf8(&fragment) {
                            Ok(text) => text,
                            Err(error) if error.error_len().is_none() =>
                                std::str::from_utf8(&fragment[..error.valid_up_to()]).map_err(|_| invalid("invalid history UTF-8"))?,
                            Err(_) => return Err(invalid("invalid history UTF-8")),
                        }
                    };
                    if text.is_empty() && row.total > 0 { break 'rows; }
                    let end = start + text.len() as u64;
                    let beginning = Cursor { field, sequence, offset: start };
                    let ending = (Cursor { field, sequence, offset: end }, row.total);
                    let candidate_left = if reverse { Some(beginning) } else { left.or(Some(beginning)) };
                    let candidate_right = if reverse { right.or(Some(ending)) } else { Some(ending) };
                    let emit = row.total != 0 || matches!(row.role.as_str(), "tool_call" | "tool_result");
                    if emit {
                    let role = match row.role.as_str() {
                        "user" => Role::User, "assistant" => Role::Assistant,
                        "tool_call" => Role::ToolCall, "tool_result" => Role::ToolResult,
                        _ => return Err(invalid("invalid history text role")),
                    };
                    let attachments = if start == 0 && role == Role::User {
                        let encoded: Option<String> = sqlx::query_scalar(
                            "SELECT json_extract(payload, '$.attachments') FROM transcript_rows WHERE session_id = ?1 AND sequence = ?2"
                        ).bind(&session).bind(sql_number(sequence)?).fetch_one(&mut *tx).await?;
                        encoded.map(|value| serde_json::from_str(&value)).transpose()?.unwrap_or_default()
                    } else { Vec::new() };
                    let tool = if matches!(role, Role::ToolCall | Role::ToolResult) {
                        let Some(tool) = tool_metadata(&mut tx, &session, sequence, through_sql, max_json_bytes).await? else {
                            if left.is_none() { return Err(invalid("history fragment exceeds JSON window budget")); }
                            break 'rows;
                        };
                        Some(tool)
                    } else { None };
                    chunks.push(Chunk { field,
                        message_id: row.message, turn_id: row.turn, timestamp: row.timestamp, role,
                        sequence, offset: start, total_bytes: row.total, text: text.to_owned(), attachments, tool,
                    });
                    }
                    // Bound the complete canonical delivery, including attachment
                    // descriptors and JSON escaping. A rejected candidate does not
                    // advance the source boundary or silently discard any metadata.
                    let (before, after) = boundaries(&mut tx, &session, through_sql, include_thinking, candidate_left, candidate_right).await?;
                    let candidate = Page::Ready { through, chunks: std::mem::take(&mut chunks), before, after };
                    let fits = serde_json::to_vec(&candidate)?.len() <= max_json_bytes;
                    let Page::Ready { chunks: packed, .. } = candidate else { unreachable!() };
                    chunks = packed;
                    if !fits {
                        if emit { chunks.pop(); }
                        if left.is_none() { return Err(invalid("history fragment exceeds JSON window budget")); }
                        break 'rows;
                    }
                    bytes += text.len();
                    left = candidate_left;
                    right = candidate_right;
                    if (!reverse && end < row.total) || (reverse && start > 0) { break 'rows; }
                }
            }
            if reverse { chunks.reverse(); }
            if left.is_none() {
                if let Some((cursor, row)) = anchor {
                    left = Some(cursor); right = Some((cursor, row.total));
                } else if matches!(position, Position::Event { .. }) {
                    let last: Option<i64> = sqlx::query_scalar(
                        "SELECT MAX(sequence) FROM transcript_rows WHERE session_id = ?1 AND sequence <= ?2"
                    ).bind(&session).bind(through_sql).fetch_one(&mut *tx).await?;
                    if let Some(sequence) = last {
                        let sequence = sequence_number(sequence)?;
                        materialize(&mut tx, &session, sequence).await?;
                        let row = header(&mut tx, &session, sequence, HistoryField::Body).await?;
                        let cursor = Cursor { field: HistoryField::Body, sequence, offset: row.total };
                        left = Some(cursor); right = Some((cursor, row.total));
                    }
                }
            }
            let (before, after) = boundaries(&mut tx, &session, through_sql, include_thinking, left, right).await?;
            tx.commit().await?;
            Ok(Page::Ready { through, chunks, before, after })
        })).await
    }
}

pub(super) async fn tool_metadata(
    tx: &mut SqliteConnection,
    session: &str,
    sequence: u64,
    through: i64,
    budget: usize,
) -> Result<Option<Tool>, StoreError> {
    // Select scalar canonical facts only. A result's call must belong to this
    // Session and be visible at this exact fence, including copied histories.
    // Reject oversized metadata before transporting/allocating its JSON string.
    let (bytes, encoded): (i64, Option<String>) = sqlx::query_as(
        "WITH metadata AS (SELECT CASE json_extract(source.payload, '$.type')
            WHEN 'tool_call' THEN json_object('kind', 'call',
                'toolUseId', source.message_id, 'toolName', json_extract(source.payload, '$.toolName'))
            WHEN 'tool_result' THEN json_object('kind', 'result',
                'toolUseId', json_extract(source.payload, '$.toolUseId'),
                'toolName', (SELECT json_extract(call.payload, '$.toolName') FROM transcript_rows call
                    WHERE call.session_id = source.session_id AND call.sequence <= ?3 AND call.sequence < source.sequence
                      AND call.message_id = json_extract(source.payload, '$.toolUseId')
                      AND json_extract(call.payload, '$.type') = 'tool_call' LIMIT 1),
                'image', json(CASE json_extract(source.payload, '$.content.kind') WHEN 'image' THEN 'true' ELSE 'false' END),
                'isError', json(CASE json_extract(source.payload, '$.isError') WHEN 1 THEN 'true' ELSE 'false' END))
            END AS value FROM transcript_rows source WHERE source.session_id = ?1 AND source.sequence = ?2)
         SELECT length(CAST(value AS BLOB)), CASE WHEN length(CAST(value AS BLOB)) <= ?4 THEN value END FROM metadata",
    )
    .bind(session)
    .bind(sql_number(sequence)?)
    .bind(through)
    .bind(budget as i64)
    .fetch_one(tx)
    .await?;
    if bytes < 0 {
        return Err(invalid("invalid history tool metadata size"));
    }
    if bytes as usize > budget {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&encoded.ok_or_else(|| {
        invalid("missing history tool metadata")
    })?)?))
}

async fn boundaries(
    tx: &mut SqliteConnection,
    session: &str,
    through: i64,
    include_thinking: bool,
    left: Option<Cursor>,
    right: Option<(Cursor, u64)>,
) -> Result<(Option<Cursor>, Option<Cursor>), StoreError> {
    let before = match left {
        Some(cursor)
            if cursor.offset > 0
                || neighbor(tx, session, through, cursor, include_thinking, true).await? =>
        {
            Some(cursor)
        }
        _ => None,
    };
    let after = match right {
        Some((cursor, total))
            if cursor.offset < total
                || neighbor(tx, session, through, cursor, include_thinking, false).await? =>
        {
            Some(cursor)
        }
        _ => None,
    };
    Ok((before, after))
}

struct Header {
    message: String,
    turn: String,
    timestamp: u64,
    role: String,
    total: u64,
}
async fn header(
    tx: &mut SqliteConnection,
    session: &str,
    sequence: u64,
    field: HistoryField,
) -> Result<Header, StoreError> {
    let row = sqlx::query(
        "SELECT source.message_id, source.turn_id, cached.timestamp, cached.role, length(cached.body)
            FROM transcript_rows source JOIN transcript_text cached USING(session_id, sequence)
            WHERE source.session_id = ?1 AND source.sequence = ?2 AND cached.field = ?3"
    ).bind(session).bind(sql_number(sequence)?).bind(field_name(field)).fetch_one(tx).await?;
    Ok(Header {
        message: row.try_get(0)?,
        turn: row.try_get(1)?,
        timestamp: sequence_number(row.try_get(2)?)?,
        role: row.try_get(3)?,
        total: sequence_number(row.try_get(4)?)?,
    })
}
async fn neighbor(
    tx: &mut SqliteConnection,
    session: &str,
    through: i64,
    cursor: Cursor,
    include_thinking: bool,
    before: bool,
) -> Result<bool, StoreError> {
    if include_thinking {
        if !before && cursor.field == HistoryField::Thinking {
            return Ok(true);
        }
        if before && cursor.field == HistoryField::Body {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM transcript_text WHERE session_id = ?1 AND sequence = ?2 AND field = 'thinking')"
            ).bind(session).bind(sql_number(cursor.sequence)?).fetch_one(&mut *tx).await?;
            if exists {
                return Ok(true);
            }
        }
    }
    let query = if before {
        "SELECT EXISTS(SELECT 1 FROM transcript_rows WHERE session_id = ?1 AND sequence <= ?2 AND sequence < ?3)"
    } else {
        "SELECT EXISTS(SELECT 1 FROM transcript_rows WHERE session_id = ?1 AND sequence <= ?2 AND sequence > ?3)"
    };
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(query))
        .bind(session)
        .bind(through)
        .bind(sql_number(cursor.sequence)?)
        .fetch_one(tx)
        .await?)
}
async fn materialize(
    tx: &mut SqliteConnection,
    session: &str,
    sequence: u64,
) -> Result<usize, StoreError> {
    let source: Option<(i64, bool)> = sqlx::query_as(
        "SELECT total_bytes, EXISTS(SELECT 1 FROM transcript_text cached WHERE cached.session_id = source.session_id AND cached.sequence = source.sequence AND cached.field = 'body')
            FROM transcript_rows source WHERE session_id = ?1 AND sequence = ?2"
    ).bind(session).bind(sql_number(sequence)?).fetch_optional(&mut *tx).await?;
    let Some((bytes, cached)) = source else {
        return Err(invalid("history row was not found"));
    };
    if cached {
        return Ok(0);
    }
    let query = format!(
        "INSERT INTO transcript_text
            SELECT session_id, sequence, 'body', json_extract(payload, '$.ts'), json_extract(payload, '$.type'),
                CAST({BODY} AS BLOB)
            FROM transcript_rows WHERE session_id = ?1 AND sequence = ?2
            UNION ALL
            SELECT session_id, sequence, 'thinking', json_extract(payload, '$.ts'), 'assistant',
                CAST(json_extract(payload, '$.thinking.text') AS BLOB)
            FROM transcript_rows WHERE session_id = ?1 AND sequence = ?2
                AND json_extract(payload, '$.type') = 'assistant' AND json_type(payload, '$.thinking.text') = 'text'"
    );
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(session)
        .bind(sql_number(sequence)?)
        .execute(tx)
        .await?;
    usize::try_from(sequence_number(bytes)?)
        .map_err(|_| invalid("history source exceeds address space"))
}
fn invalid(message: &str) -> StoreError {
    StoreError::InvalidTransition(message.into())
}

fn field_name(field: HistoryField) -> &'static str {
    match field {
        HistoryField::Body => "body",
        HistoryField::Thinking => "thinking",
    }
}
