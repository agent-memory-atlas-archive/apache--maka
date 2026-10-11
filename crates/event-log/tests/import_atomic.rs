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

use maka_event_log::{EventLog, StoreError, sessions::PluginSession};
use maka_plugins::{composition::Scope, storage::Namespace};
use maka_runtime::import::{Content, ImportState, Record, Source};
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
        log.session_import_progress(&claim.session_id)
            .await
            .unwrap()
            .state,
        ImportState::Published
    );
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
    reopened.close().await.unwrap();
}
