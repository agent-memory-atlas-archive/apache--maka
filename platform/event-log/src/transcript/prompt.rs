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

use crate::{EventLog, StoreError, sequence_number};
use maka_agent_ports::session::history::Prompt;
use sqlx::{Connection, Row};

const QUERY: &str = "SELECT sequence,
    COALESCE(navigation_preview(COALESCE(json_extract(payload, '$.displayText'),
        json_extract(payload, '$.text'), ''), 4096), '[attachments]')
    FROM transcript_rows
    WHERE session_id = ?1 AND sequence <= ?2 AND json_extract(payload, '$.type') = 'user'
    ORDER BY sequence DESC LIMIT 1";

impl EventLog {
    /// Inclusive reading anchor, fenced to an already prepared transcript.
    /// The partial index skips answer/tool bodies regardless of session size.
    pub async fn history_prompt(
        &self,
        session: &str,
        through: u64,
        sequence: u64,
    ) -> Result<Option<Prompt>, StoreError> {
        self.validate_root()?;
        crate::sessions::validate_id(session)?;
        let fence = maka_presentation::watermark(through)?;
        if sequence == 0 || sequence > fence {
            return Err(StoreError::InvalidTransition(
                "prompt anchor exceeds history fence".into(),
            ));
        }
        let sequence = super::read::sql_number(sequence)?;
        let session = session.to_owned();
        self.connection
            .run(move |connection| {
                Box::pin(async move {
                    let mut tx = connection.begin().await?;
                    super::read::prepared(&mut tx, &session, fence).await?;
                    sqlx::query(QUERY)
                        .bind(session)
                        .bind(sequence)
                        .fetch_optional(&mut *tx)
                        .await?
                        .map(|row| {
                            Ok(Prompt {
                                source: sequence_number(row.try_get(0)?)?,
                                text: row.try_get(1)?,
                            })
                        })
                        .transpose()
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{Connection, params};
    #[test]
    fn preceding_prompt_uses_index_and_does_not_read_other_sessions_or_later_prompts() {
        let db = Connection::open_in_memory().unwrap();
        super::super::navigation::register(&db).unwrap();
        db.execute_batch(
            "CREATE TABLE transcript_rows(session_id TEXT, sequence INTEGER, payload BLOB);",
        )
        .unwrap();
        db.execute_batch(include_str!(
            "../../migrations/0018_transcript_user_prompt.sql"
        ))
        .unwrap();
        for (session, sequence, payload) in [
            (
                "a",
                1,
                serde_json::json!({"type":"user","text":"First question"}),
            ),
            (
                "a",
                2,
                serde_json::json!({"type":"assistant","text":"x".repeat(1024 * 1024)}),
            ),
            (
                "b",
                3,
                serde_json::json!({"type":"user","text":"Other session"}),
            ),
            ("a", 4, serde_json::json!({"type":"user","text":"?"})),
        ] {
            db.execute(
                "INSERT INTO transcript_rows VALUES(?1,?2,?3)",
                params![session, sequence, serde_json::to_vec(&payload).unwrap()],
            )
            .unwrap();
        }
        let plan: String = db
            .prepare(&format!("EXPLAIN QUERY PLAN {QUERY}"))
            .unwrap()
            .query_map(params!["a", 3], |row| row.get::<_, String>(3))
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            plan.contains("USING INDEX transcript_user_prompt"),
            "{plan}"
        );
        for (anchor, expected) in [(3, (1, "First question")), (4, (4, "?"))] {
            let actual: (i64, String) = db
                .query_row(QUERY, params!["a", anchor], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .unwrap();
            assert_eq!(actual, (expected.0, expected.1.into()));
        }
    }
}
