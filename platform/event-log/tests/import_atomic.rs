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

use maka_agent_facts::event::{Fact, LogScope};
use maka_agent_facts::import::Content;
use maka_agent_facts::import::Record;
use maka_agent_facts::import::Source;
use maka_capabilities::storage::Namespace;
use maka_event_log::{EventLog, StoreError, sessions::PluginSession};
use maka_plugin_core::scope::Scope;
use serde_json::{Value, json};

fn owner(id: &str) -> PluginSession {
    PluginSession {
        session_id: id.into(),
        creator: Namespace::new("import.fixture", Scope::Profile).unwrap(),
        fingerprint: format!("original-{id}"),
        managed: false,
        authority_session_id: None,
    }
}
fn record(id: usize, text: String) -> Record {
    Record {
        source_message_id: format!("source-{id}"),
        source_turn_id: "foreign-turn".into(),
        timestamp: Some(17),
        content: Content::User { text },
    }
}
#[tokio::test]
async fn atomic_import_publishes_once_in_its_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("history.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    let source = Source {
        adapter: "fixture".into(),
        session_id: "foreign".into(),
    };
    let claim = owner("owned-one");
    let records = vec![
        record(0, "Hello imported history".into()),
        Record {
            source_message_id: "call".into(),
            source_turn_id: "foreign-turn".into(),
            timestamp: None,
            content: Content::ToolCall {
                call_id: "foreign-call".into(),
                name: "Shell".into(),
                input: Some(json!({"command":"must not execute"})),
            },
        },
    ];
    let first = log
        .import_session(
            &claim,
            &source,
            &json!({"name":"copy"}),
            records.clone(),
            20,
        )
        .await
        .unwrap();
    assert_eq!(
        log.get_session::<Value>(&claim.session_id)
            .await
            .unwrap()
            .as_ref(),
        Some(&first)
    );
    assert_eq!(
        log.session_creator(&claim.session_id).await.unwrap(),
        Some(claim.creator.clone())
    );
    let published = log
        .scoped_prefix(
            LogScope::Session {
                id: claim.session_id.clone(),
            },
            16,
            64 * 1024,
        )
        .await
        .unwrap();
    assert_eq!(published.events.len(), 2);
    let imported: Vec<_> = published
        .events
        .iter()
        .map(|event| match &event.event.fact {
            Fact::MessageImported { record, .. } => record.as_ref(),
            _ => panic!("publication must preserve canonical imported records"),
        })
        .collect();
    assert_eq!(imported, records.iter().collect::<Vec<_>>());
    let original_digest = published.digest;
    assert!(log.unfinished_invocations(16).await.unwrap().is_empty());
    let through = *log.subscribe_commits().borrow();
    let retry = log
        .import_session(
            &claim,
            &source,
            &json!({"name":"copy"}),
            records.clone(),
            99,
        )
        .await
        .unwrap();
    assert_eq!(first, retry);
    assert_eq!(*log.subscribe_commits().borrow(), through);
    assert!(matches!(
        log.import_session(
            &PluginSession {
                fingerprint: "different-origin".into(),
                ..claim.clone()
            },
            &source,
            &json!({"name":"copy"}),
            records.clone(),
            20
        )
        .await,
        Err(StoreError::SessionConflict)
    ));
    let mut changed = records.clone();
    changed[0] = record(0, "Replaced source".into());
    assert!(matches!(
        log.import_session(&claim, &source, &json!({"name":"copy"}), changed, 20)
            .await,
        Err(StoreError::EventConflict)
    ));
    log.import_session(
        &owner("foreign-one"),
        &source,
        &json!(["different configuration"]),
        records.clone(),
        21,
    )
    .await
    .unwrap();
    log.import_session(
        &owner("owned-two"),
        &source,
        &json!({"name":"second"}),
        records,
        22,
    )
    .await
    .unwrap();
    let (page, next) = log
        .imported_sessions::<Value>("owned-", None, 1)
        .await
        .unwrap();
    assert_eq!(page[0].id, "owned-two");
    let (page, next) = log
        .imported_sessions::<Value>("owned-", next.as_deref(), 1)
        .await
        .unwrap();
    assert_eq!(page[0].id, "owned-one");
    assert!(next.is_none());
    assert!(
        log.imported_sessions::<Value>("owned-", Some("foreign-one"), 1)
            .await
            .is_err()
    );
    log.close().await.unwrap();
    let reopened = EventLog::open(&path).await.unwrap();
    assert_eq!(
        reopened
            .imported_sessions::<Value>("owned-", None, 16)
            .await
            .unwrap()
            .0
            .len(),
        2
    );
    assert_eq!(*reopened.subscribe_commits().borrow(), through + 4);
    assert_eq!(
        reopened
            .scoped_prefix(
                LogScope::Session {
                    id: claim.session_id.clone()
                },
                16,
                64 * 1024
            )
            .await
            .unwrap()
            .digest,
        original_digest
    );
    reopened.close().await.unwrap();
}

#[tokio::test]
async fn second_record_sql_failure_rolls_back_the_complete_import_and_allows_retry() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("rollback.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    let inspect = rusqlite::Connection::open(&path).unwrap();
    inspect
        .execute_batch(
            "CREATE TRIGGER fail_second_import_record BEFORE INSERT ON event_log
        WHEN NEW.kind='message_imported'
          AND json_extract(NEW.event_json,'$.fact.record.sourceMessageId')='source-1'
          AND EXISTS(SELECT 1 FROM event_log WHERE kind='message_imported'
            AND json_extract(event_json,'$.fact.record.sourceMessageId')='source-0')
        BEGIN SELECT RAISE(ABORT,'atomic second record fault'); END;",
        )
        .unwrap();
    let claim = owner("rollback-owned");
    let source = Source {
        adapter: "fixture".into(),
        session_id: "foreign".into(),
    };
    let records = vec![
        record(0, "first".into()),
        record(1, "second".into()),
        record(2, "third".into()),
    ];
    let notices = log.subscribe_commits();
    let through = *notices.borrow();
    let result = log
        .import_session(
            &claim,
            &source,
            &json!({"title":"Atomic"}),
            records.clone(),
            20,
        )
        .await;
    assert!(
        matches!(result,Err(StoreError::Sqlx(error)) if error.as_database_error().and_then(|error|error.code()).as_deref()==Some("1811"))
    );
    assert_eq!(*notices.borrow(), through);
    assert!(!notices.has_changed().unwrap());
    assert!(
        log.get_session::<Value>(&claim.session_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        log.session_creator(&claim.session_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        log.imported_sessions::<Value>("rollback-", None, 16)
            .await
            .unwrap()
            .0
            .is_empty()
    );
    assert!(log.prefix(16, 64 * 1024).await.unwrap().events.is_empty());
    inspect
        .execute_batch("DROP TRIGGER fail_second_import_record")
        .unwrap();
    drop(inspect);
    let accepted = log
        .import_session(
            &claim,
            &source,
            &json!({"title":"Atomic"}),
            records.clone(),
            20,
        )
        .await
        .unwrap();
    assert_eq!(
        log.imported_sessions::<Value>("rollback-", None, 16)
            .await
            .unwrap()
            .0,
        vec![accepted.clone()]
    );
    let published = log.prefix(16, 64 * 1024).await.unwrap();
    let imported: Vec<_> = published
        .events
        .iter()
        .map(|event| match &event.event.fact {
            Fact::MessageImported { record, .. } => record.as_ref(),
            _ => panic!("retry must publish only the complete ordered import"),
        })
        .collect();
    assert_eq!(imported, records.iter().collect::<Vec<_>>());
    let digest = published.digest;
    log.close().await.unwrap();
    let log = EventLog::open(&path).await.unwrap();
    let notices = log.subscribe_commits();
    assert_eq!(
        log.import_session(&claim, &source, &json!({"title":"Atomic"}), records, 99)
            .await
            .unwrap(),
        accepted
    );
    assert!(!notices.has_changed().unwrap());
    assert_eq!(log.prefix(16, 64 * 1024).await.unwrap().digest, digest);
    log.close().await.unwrap();
}
