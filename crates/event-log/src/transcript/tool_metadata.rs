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

use super::read::sql_number;
use crate::StoreError;
use maka_plugins::session::history::Tool;
use sqlx::SqliteConnection;

const METADATA_SQL: &str = "WITH owner AS (
            SELECT session_id, turn_id, sequence, message_id,
                json_extract(payload, '$.type') AS kind,
                CASE json_extract(payload, '$.type') WHEN 'tool_call' THEN message_id
                    ELSE json_extract(payload, '$.toolUseId') END AS tool_id,
                json_extract(payload, '$.toolName') AS name,
                json_extract(payload, '$.isError') AS error,
                json_extract(payload, '$.content.kind') IS 'image' AS image
            FROM transcript_rows WHERE session_id = ?1 AND sequence = ?2 AND sequence <= ?3
        ), call AS (
            SELECT row.sequence, json_extract(row.payload, '$.toolName') AS name,
                json_extract(row.payload, '$.origin') AS origin,
                json_extract(row.payload, '$.parentToolCallId') AS parent_call,
                json_extract(row.payload, '$.parentOperationId') AS parent_operation
            FROM transcript_rows row JOIN owner ON row.session_id = owner.session_id
                AND row.message_id = owner.tool_id AND row.turn_id = owner.turn_id
            WHERE row.sequence <= ?3 AND json_extract(row.payload, '$.type') = 'tool_call'
                AND json_type(row.payload, '$.args') IS NOT NULL
        ), results AS MATERIALIZED (
            SELECT row.sequence, json_extract(row.payload, '$.origin') AS origin,
                json_extract(row.payload, '$.parentToolCallId') AS parent_call,
                json_extract(row.payload, '$.parentOperationId') AS parent_operation,
                json_type(row.payload, '$.isError') IN ('true', 'false') AND json_type(row.payload, '$.content') = 'object' AS valid,
                json_extract(row.payload, '$.isError') AS error,
                json_extract(row.payload, '$.content.kind') IS 'image' AS image
            FROM transcript_rows row JOIN owner ON row.session_id = owner.session_id
                AND row.turn_id = owner.turn_id AND json_extract(row.payload, '$.toolUseId') = owner.tool_id
            WHERE row.sequence <= ?3 AND json_extract(row.payload, '$.type') = 'tool_result'
            ORDER BY row.sequence LIMIT 2
        ), pair AS (
            SELECT json_object('sequence', call.sequence, 'name', call.name) AS call_ref,
                json_object('sequence', results.sequence,
                    'isError', json(CASE results.error WHEN 1 THEN 'true' ELSE 'false' END),
                    'image', json(CASE results.image WHEN 1 THEN 'true' ELSE 'false' END)) AS result_ref
            FROM call, results WHERE (SELECT count(*) FROM results) = 1
                AND results.valid
                AND call.sequence < results.sequence
                AND call.origin IN ('imported', 'provider', 'host_sdk', 'code_mode')
                AND call.origin = results.origin
                AND call.parent_call IS results.parent_call
                AND call.parent_operation IS results.parent_operation
        ), metadata AS (
            SELECT json_object('toolUseId', tool_id,
                'call', json(CASE kind WHEN 'tool_call' THEN json_object('sequence', sequence, 'name', name)
                    ELSE (SELECT call_ref FROM pair) END),
                'result', json(CASE kind WHEN 'tool_result' THEN json_object('sequence', sequence,
                    'isError', json(CASE error WHEN 1 THEN 'true' ELSE 'false' END),
                    'image', json(CASE image WHEN 1 THEN 'true' ELSE 'false' END))
                    ELSE (SELECT result_ref FROM pair) END)) AS value FROM owner
        ) SELECT length(CAST(value AS BLOB)), CASE WHEN length(CAST(value AS BLOB)) <= ?4 THEN value END FROM metadata";

/// Read scalar metadata only. The unique call key and the bounded result index
/// lookup avoid loading payloads or scanning an entire Turn for each fragment.
/// Pairing is evaluated at the supplied fence, never against future outcomes.
pub(super) async fn read(
    tx: &mut SqliteConnection,
    session: &str,
    sequence: u64,
    through: i64,
    budget: usize,
) -> Result<Option<Tool>, StoreError> {
    let (bytes, encoded): (i64, Option<String>) = sqlx::query_as(METADATA_SQL)
        .bind(session)
        .bind(sql_number(sequence)?)
        .bind(through)
        .bind(budget as i64)
        .fetch_one(tx)
        .await?;
    if bytes < 0 {
        return Err(StoreError::InvalidTransition(
            "invalid history tool metadata size".into(),
        ));
    }
    if bytes as usize > budget {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&encoded.ok_or_else(|| {
        StoreError::InvalidTransition("missing history tool metadata".into())
    })?)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use sqlx::{Connection, Row};

    async fn database() -> SqliteConnection {
        let mut db = SqliteConnection::connect(":memory:").await.unwrap();
        sqlx::query("CREATE TABLE transcript_rows (sequence INTEGER, session_id TEXT, turn_id TEXT, message_id TEXT, payload BLOB, PRIMARY KEY(session_id, sequence), UNIQUE(session_id, message_id))").execute(&mut db).await.unwrap();
        sqlx::raw_sql(include_str!(
            "../../migrations/0011_tool_result_references.sql"
        ))
        .execute(&mut db)
        .await
        .unwrap();
        db
    }
    fn call() -> Value {
        json!({"type":"tool_call","toolName":"Read","args":{"path":"example.rs"},"origin":"provider"})
    }
    fn result() -> Value {
        json!({"type":"tool_result","toolUseId":"call","origin":"provider","isError":false,"content":{"kind":"text","text":"body"}})
    }
    async fn insert(
        db: &mut SqliteConnection,
        session: &str,
        turn: &str,
        sequence: i64,
        id: &str,
        value: Value,
    ) {
        sqlx::query("INSERT INTO transcript_rows VALUES (?, ?, ?, ?, ?)")
            .bind(sequence)
            .bind(session)
            .bind(turn)
            .bind(id)
            .bind(serde_json::to_vec(&value).unwrap())
            .execute(db)
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn paired_references_are_unique_ordered_scoped_and_fenced() {
        let mut db = database().await;
        insert(&mut db, "a", "turn", 1, "call", call()).await;
        insert(&mut db, "a", "turn", 2, "result", result()).await;
        insert(&mut db, "other", "turn", 1, "call", call()).await;
        insert(&mut db, "other", "turn", 2, "result", result()).await;
        let before = read(&mut db, "a", 1, 1, 65536).await.unwrap().unwrap();
        assert!(before.call.is_some() && before.result.is_none());
        let paired = read(&mut db, "a", 1, 2, 65536).await.unwrap().unwrap();
        assert_eq!(paired.call.as_ref().unwrap().sequence, 1);
        assert_eq!(paired.result.as_ref().unwrap().sequence, 2);
        assert_eq!(
            paired,
            read(&mut db, "a", 2, 2, 65536).await.unwrap().unwrap()
        );
        insert(&mut db, "a", "foreign-turn", 3, "foreign-result", result()).await;
        assert!(
            read(&mut db, "a", 3, 3, 65536)
                .await
                .unwrap()
                .unwrap()
                .call
                .is_none()
        );
        assert_eq!(
            paired,
            read(&mut db, "a", 1, 3, 65536).await.unwrap().unwrap()
        );
        insert(&mut db, "a", "turn", 4, "duplicate", result()).await;
        assert_eq!(
            paired,
            read(&mut db, "a", 1, 3, 65536).await.unwrap().unwrap()
        );
        assert!(
            read(&mut db, "a", 1, 4, 65536)
                .await
                .unwrap()
                .unwrap()
                .result
                .is_none()
        );
        for sequence in [2, 4] {
            let orphan = read(&mut db, "a", sequence, 4, 65536)
                .await
                .unwrap()
                .unwrap();
            assert!(orphan.call.is_none());
            assert_eq!(orphan.result.unwrap().sequence, sequence);
        }
        let encoded = serde_json::to_vec(&paired).unwrap();
        assert_eq!(
            Some(paired),
            read(&mut db, "a", 1, 2, encoded.len()).await.unwrap()
        );
        assert!(
            read(&mut db, "a", 1, 2, encoded.len() - 1)
                .await
                .unwrap()
                .is_none()
        );
        let plan = format!("EXPLAIN QUERY PLAN {METADATA_SQL}");
        let rows = sqlx::query(sqlx::AssertSqlSafe(plan.as_str()))
            .bind("a")
            .bind(1i64)
            .bind(4i64)
            .bind(65536i64)
            .fetch_all(&mut db)
            .await
            .unwrap();
        let plan = rows
            .iter()
            .map(|row| row.get::<String, _>(3))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            plan.contains("USING INDEX transcript_tool_results"),
            "{plan}"
        );
    }
    #[tokio::test]
    async fn mismatched_parentage_origin_future_schema_and_reversed_rows_remain_unpaired() {
        for case in 0..7 {
            let mut db = database().await;
            let mut call = call();
            let mut result = result();
            match case {
                0 => result["origin"] = json!("code_mode"),
                1 => result["parentToolCallId"] = json!("different"),
                2 => result["parentOperationId"] = json!("different"),
                3 => {
                    call["origin"] = json!("future");
                    result["origin"] = json!("future");
                }
                4 => result["isError"] = json!("false"),
                5 => result["content"] = json!("malformed"),
                _ => (),
            }
            let (call_sequence, result_sequence) = if case == 6 { (2, 1) } else { (1, 2) };
            insert(&mut db, "a", "turn", call_sequence, "call", call).await;
            insert(&mut db, "a", "turn", result_sequence, "result", result).await;
            let call = read(&mut db, "a", call_sequence as u64, 2, 65536)
                .await
                .unwrap()
                .unwrap();
            let result = read(&mut db, "a", result_sequence as u64, 2, 65536)
                .await
                .unwrap()
                .unwrap();
            assert!(call.result.is_none(), "case {case}");
            assert!(result.call.is_none(), "case {case}");
            assert_eq!(call.call.unwrap().sequence, call_sequence as u64);
            assert_eq!(result.result.unwrap().sequence, result_sequence as u64);
        }
    }
}
