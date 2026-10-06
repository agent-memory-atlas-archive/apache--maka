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

use super::{
    history::tool_metadata,
    read::{prepared, sql_number},
};
use crate::{EventLog, StoreError, sequence_number};
use maka_plugins::session::history::{SourceFormat, SourcePosition, ToolSource, ToolSourceField};
use sqlx::Connection;

const SOURCE: &str = "WITH source AS (SELECT
    CASE ?3 WHEN 'arguments' THEN CASE WHEN json_extract(payload, '$.type') = 'tool_call' THEN payload -> '$.args' END
    WHEN 'result' THEN CASE WHEN json_extract(payload, '$.type') = 'tool_result' THEN
        CASE json_extract(payload, '$.content.kind') WHEN 'text' THEN json_extract(payload, '$.content.text')
        WHEN 'json' THEN payload -> '$.content.value' WHEN 'image' THEN payload -> '$.content' END END END AS body,
    CASE ?3 WHEN 'arguments' THEN 'json' WHEN 'result' THEN
        CASE json_extract(payload, '$.content.kind') WHEN 'text' THEN 'text' WHEN 'json' THEN 'json'
        WHEN 'image' THEN 'image_descriptor' END END AS format
    FROM transcript_rows WHERE session_id = ?1 AND sequence = ?2)";

impl EventLog {
    /// Reads one selected immutable canonical field; no cache or arbitrary path.
    #[allow(clippy::too_many_arguments)]
    pub async fn tool_source(
        &self,
        session: &str,
        through: u64,
        sequence: u64,
        field: ToolSourceField,
        position: SourcePosition,
        max_bytes: usize,
        max_json_bytes: usize,
    ) -> Result<ToolSource, StoreError> {
        self.validate_root()?;
        crate::sessions::validate_id(session)?;
        if !(4..=128 * 1024).contains(&max_bytes) || !(1024..=1024 * 1024).contains(&max_json_bytes)
        {
            return Err(invalid("invalid tool source limits"));
        }
        let fence = maka_presentation::watermark(through)?;
        if sequence == 0 || sequence > fence {
            return Err(invalid("tool source exceeds its fence"));
        }
        let sequence_sql = sql_number(sequence)?;
        let fence_sql = sql_number(fence)?;
        let session = session.to_owned();
        let selected = match field {
            ToolSourceField::Arguments => "arguments",
            ToolSourceField::Result => "result",
        };
        self.connection
            .run(move |connection| {
                Box::pin(async move {
                    let mut tx = connection.begin().await?;
                    prepared(&mut tx, &session, fence).await?;
                    let exists: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM session_control WHERE id = ?)",
                    )
                    .bind(&session)
                    .fetch_one(&mut *tx)
                    .await?;
                    if !exists {
                        return Err(invalid("Session no longer exists"));
                    }
                    let header =
                        format!("{SOURCE} SELECT format, length(CAST(body AS BLOB)) FROM source");
                    let (format, total): (Option<String>, Option<i64>) =
                        sqlx::query_as(sqlx::AssertSqlSafe(header.as_str()))
                            .bind(&session)
                            .bind(sequence_sql)
                            .bind(selected)
                            .fetch_optional(&mut *tx)
                            .await?
                            .ok_or_else(|| invalid("tool source was not found"))?;
                    let total = sequence_number(
                        total.ok_or_else(|| invalid("field is not a tool source"))?,
                    )?;
                    let format = match format.as_deref() {
                        Some("text") => SourceFormat::Text,
                        Some("json") => SourceFormat::Json,
                        Some("image_descriptor") => SourceFormat::ImageDescriptor,
                        _ => return Err(invalid("invalid tool source format")),
                    };
                    let read =
                        format!("{SOURCE} SELECT substr(CAST(body AS BLOB), ?4, ?5) FROM source");
                    let offset = match position {
                        SourcePosition::Start => 0,
                        SourcePosition::Tail => total,
                        SourcePosition::Before { offset } | SourcePosition::After { offset } => {
                            offset
                        }
                    };
                    sql_number(offset)?;
                    if offset > total {
                        return Err(invalid("tool source offset exceeds field"));
                    }
                    let byte: Vec<u8> = sqlx::query_scalar(sqlx::AssertSqlSafe(read.as_str()))
                        .bind(&session)
                        .bind(sequence_sql)
                        .bind(selected)
                        .bind(sql_number(offset)? + 1)
                        .bind(1i64)
                        .fetch_one(&mut *tx)
                        .await?;
                    if byte.first().is_some_and(|byte| byte & 0xc0 == 0x80) {
                        return Err(invalid("tool source offset splits UTF-8"));
                    }
                    let reverse = matches!(
                        position,
                        SourcePosition::Tail | SourcePosition::Before { .. }
                    );
                    let (mut start, end) = if reverse {
                        (offset.saturating_sub(max_bytes as u64), offset)
                    } else {
                        (offset, total.min(offset + max_bytes as u64))
                    };
                    let bytes: Vec<u8> = sqlx::query_scalar(sqlx::AssertSqlSafe(read.as_str()))
                        .bind(&session)
                        .bind(sequence_sql)
                        .bind(selected)
                        .bind(sql_number(start)? + 1)
                        .bind((end - start) as i64)
                        .fetch_one(&mut *tx)
                        .await?;
                    let text = if reverse {
                        let skipped = bytes
                            .iter()
                            .take_while(|byte| **byte & 0xc0 == 0x80)
                            .count();
                        start += skipped as u64;
                        std::str::from_utf8(&bytes[skipped..])
                            .map_err(|_| invalid("invalid UTF-8 tool source"))?
                    } else {
                        match std::str::from_utf8(&bytes) {
                            Ok(text) => text,
                            Err(error) if error.error_len().is_none() => {
                                std::str::from_utf8(&bytes[..error.valid_up_to()])
                                    .map_err(|_| invalid("invalid UTF-8 tool source"))?
                            }
                            Err(_) => return Err(invalid("invalid UTF-8 tool source")),
                        }
                    };
                    if text.is_empty() && start < total && !(reverse && offset == 0) {
                        return Err(invalid("tool source made no progress"));
                    }
                    let tool =
                        tool_metadata(&mut tx, &session, sequence, fence_sql, max_json_bytes)
                            .await?
                            .ok_or_else(|| invalid("tool source metadata exceeds JSON budget"))?;
                    let source = ToolSource {
                        sequence,
                        field,
                        format,
                        offset: start,
                        total_bytes: total,
                        text: text.to_owned(),
                        tool,
                    };
                    if serde_json::to_vec(&source)?.len() > max_json_bytes {
                        return Err(invalid("tool source exceeds JSON budget"));
                    }
                    tx.commit().await?;
                    Ok(source)
                })
            })
            .await
    }
}
fn invalid(message: &str) -> StoreError {
    StoreError::InvalidTransition(message.into())
}
