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
use sha2::{Digest, Sha256};

#[tokio::test]
async fn a_rehashed_latest_checkpoint_cannot_hide_a_broken_predecessor_commitment() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("events.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    let observed = closed(&log, "first-main", 128, 1).await;
    log.append_batch(&prepared(&log, "first-compact").await)
        .await
        .unwrap();
    closed(&log, "second-main", 128, 1).await;
    let latest = prepared(&log, "second-compact").await;
    log.append_batch(&latest).await.unwrap();
    assert_eq!(
        log.read_model_context("session", None)
            .await
            .unwrap()
            .baseline
            .unwrap()
            .event_id,
        latest[0].event().id
    );
    log.close().await.unwrap();

    let inspect = rusqlite::Connection::open(&path).unwrap();
    inspect
        .execute(
            "UPDATE event_log SET event_json=' ' || event_json WHERE event_id=?",
            [&observed],
        )
        .unwrap();
    let mut checkpoint_event = latest[0].event().clone();
    let Fact::ContextCheckpointRecorded { checkpoint } = &mut checkpoint_event.fact else {
        panic!()
    };
    // Rebuild only the newest commitment from the documented raw encoding.
    // Its covered bytes include the unchanged old commitment, which is now
    // false. A valid newest digest therefore cannot replace checking every cut.
    let scope = serde_json::to_vec(&LogScope::Session {
        id: "session".into(),
    })
    .unwrap();
    let mut raw = Sha256::new();
    raw.update(b"maka.log-prefix.v1\0");
    raw.update((scope.len() as u64).to_be_bytes());
    raw.update(scope);
    let mut statement = inspect.prepare("SELECT sequence,event_json FROM session_history_events WHERE owner_session_id='session' AND sequence<=? ORDER BY sequence").unwrap();
    for row in statement
        .query_map([checkpoint.covered_through as i64], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
    {
        let (sequence, json) = row.unwrap();
        raw.update(sequence.to_be_bytes());
        raw.update((json.len() as u64).to_be_bytes());
        raw.update(json.as_bytes());
    }
    raw.update(checkpoint.covered_through.to_be_bytes());
    checkpoint.source_digest = format!("sha256:{}", hex::encode(raw.finalize()));
    let mut effective = Sha256::new();
    effective.update(b"maka.effective-context.v1\0");
    effective.update(checkpoint.source_digest.as_bytes());
    let request_json: String = inspect.query_row("SELECT event_json FROM runtime_events WHERE invocation_id='second-compact' AND kind='model_requested'", [], |row| row.get(0)).unwrap();
    let mut request: RuntimeEvent = serde_json::from_str(&request_json).unwrap();
    let Fact::ModelRequested {
        source_digest,
        effective_source_digest,
        ..
    } = &mut request.fact
    else {
        panic!()
    };
    *source_digest = checkpoint.source_digest.clone();
    *effective_source_digest = Some(format!("sha256:{}", hex::encode(effective.finalize())));
    for event in [&request, &checkpoint_event] {
        inspect
            .execute(
                "UPDATE event_log SET event_json=? WHERE event_id=?",
                rusqlite::params![serde_json::to_string(event).unwrap(), event.id],
            )
            .unwrap();
    }
    drop(statement);
    drop(inspect);

    let log = EventLog::open(&path).await.unwrap();
    let error = log.read_model_context("session", None).await.unwrap_err();
    assert!(
        error.to_string().contains("source digest changed"),
        "{error}"
    );
    log.close().await.unwrap();
}
