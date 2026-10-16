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
use maka_agent_facts::archive::projection_digest;
use sha2::{Digest, Sha256};

#[tokio::test]
async fn effective_digest_covers_archives_beyond_the_first_page() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("digest.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    log.create_session("session", "create", &json!({}), 1)
        .await
        .unwrap();
    open(&log, "history", false).await;
    let mut archives = Vec::new();
    for index in 0..129 {
        let result = tool(&log, "history", &format!("step-{index}"), "x".repeat(8192)).await;
        archives.push(archive("writer", &result));
    }
    end(&log, "history").await;
    open(&log, "writer", false).await;
    for archived in &archives {
        log.append(archived).await.unwrap();
    }
    let after = log
        .read_model_context("session", Some("writer"))
        .await
        .unwrap();
    let archived: Vec<_> = after
        .tail
        .iter()
        .filter_map(|entry| match entry {
            ContextEvent::Archived(entry) => Some(entry),
            _ => None,
        })
        .collect();
    assert_eq!(archived.len(), 129);
    // Check the durable digest contract against the exact append inputs and
    // exposed replacements, including the record past the old page boundary.
    let mut hash = Sha256::new();
    hash.update(b"maka.effective-context.v1\0");
    hash.update(after.source_evidence.digest.as_bytes());
    for (actual, written) in archived.iter().zip(&archives) {
        let Fact::ToolResultArchived { placeholder } = &written.event().fact else {
            panic!()
        };
        assert_eq!(actual.event_id, placeholder.identity.runtime_event_id);
        let encoded = serde_json::to_vec(&(
            actual.sequence,
            &actual.event_id,
            placeholder,
            projection_digest(&actual.replacement).unwrap(),
        ))
        .unwrap();
        hash.update((encoded.len() as u64).to_le_bytes());
        hash.update(encoded);
    }
    assert_eq!(
        after.effective_source_digest,
        format!("sha256:{:x}", hash.finalize())
    );
    let database = rusqlite::Connection::open(&path).unwrap();
    end(&log, "writer").await;
    let revision = log
        .get_session::<serde_json::Value>("session")
        .await
        .unwrap()
        .unwrap()
        .revision;
    assert!(matches!(
        log.copy_session(
            maka_event_log::sessions::SessionCopy {
                source_session_id: "session".into(),
                target_session_id: "copy".into(),
                expected_source_revision: revision,
                purpose: maka_agent_facts::session::CopyPurpose::Branch {
                    turn_id: Some("history".into()),
                    side_conversation: false,
                },
            },
            &json!({}),
            2,
        )
        .await
        .unwrap(),
        maka_event_log::sessions::SessionCopyResult::Committed(_)
    ));
    let writer = event(
        "copy-writer",
        Fact::InvocationOpened {
            configuration: None,
            input: InvocationInput::Message {
                source_messages: Vec::new(),
                content: "question".into(),
                request_fingerprint: None,
            },
        },
    );
    let mut writer = writer.event().clone();
    writer.invocation.session_id = "copy".into();
    log.append(&EventWrite::plain(writer.clone()).unwrap())
        .await
        .unwrap();
    log.read_model_context("copy", Some("copy-writer"))
        .await
        .unwrap();
    // A copied target may match its pinned archive and an owner archive. Put
    // the duplicate exactly across the 128/129 boundary without dropping the
    // per-owner unique index or introducing an invalid archive writer.
    assert_eq!(
        database
            .execute(
                "INSERT INTO event_log(event_id,invocation_id,kind,operation_id,event_json)
         SELECT 'duplicate-archive','copy-writer',kind,operation_id,
           json_set(event_json,'$.id','duplicate-archive','$.invocation',json(?2))
         FROM event_log WHERE event_id=?1",
                [
                    &archives[127].event().id,
                    &serde_json::to_string(&writer.invocation).unwrap()
                ],
            )
            .unwrap(),
        1
    );
    assert!(matches!(
        log.read_model_context("copy", Some("copy-writer")).await,
        Err(StoreError::Archive(
            maka_event_log::archive::ArchiveError::Corrupt
        ))
    ));
    assert_eq!(database.execute(
        "UPDATE event_log SET event_json=json_set(event_json,'$.fact.placeholder.page.content','corrupted archive') WHERE event_id=?",
        [&archives[128].event().id],
    ).unwrap(), 1);
    assert!(matches!(
        log.read_model_context("session", None).await,
        Err(StoreError::Archive(
            maka_event_log::archive::ArchiveError::Corrupt
        ))
    ));
    log.close().await.unwrap();
}

#[tokio::test]
async fn copied_archive_rejects_target_with_wrong_event_kind() {
    use maka_event_log::sessions::{SessionCopy, SessionCopyResult};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("copy-digest.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    log.create_session("session", "create", &json!({}), 1)
        .await
        .unwrap();
    open(&log, "history", false).await;
    let result = tool(&log, "history", "step", "x".repeat(8192)).await;
    end(&log, "history").await;
    open(&log, "writer", false).await;
    log.append(&archive("writer", &result)).await.unwrap();
    end(&log, "writer").await;
    let revision = log
        .get_session::<serde_json::Value>("session")
        .await
        .unwrap()
        .unwrap()
        .revision;
    assert!(matches!(
        log.copy_session(
            SessionCopy {
                source_session_id: "session".into(),
                target_session_id: "copy".into(),
                expected_source_revision: revision,
                purpose: maka_agent_facts::session::CopyPurpose::Branch {
                    turn_id: Some("history".into()),
                    side_conversation: false,
                },
            },
            &json!({}),
            2
        )
        .await
        .unwrap(),
        SessionCopyResult::Committed(_)
    ));
    let context = log.read_model_context("copy", None).await.unwrap();
    let archived: Vec<_> = context
        .tail
        .iter()
        .filter_map(|entry| match entry {
            ContextEvent::Archived(entry) => Some(entry),
            _ => None,
        })
        .collect();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].event_id, result.event().id);
    let database = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        database
            .execute(
                "UPDATE event_log SET kind='model_observed' WHERE event_id=?",
                [&result.event().id],
            )
            .unwrap(),
        1
    );
    assert!(matches!(
        log.read_model_context("copy", None).await,
        Err(StoreError::Archive(
            maka_event_log::archive::ArchiveError::Corrupt
        ))
    ));
    log.close().await.unwrap();
}
