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

use maka_event_log::{
    EventLog, StoreError,
    sessions::{ImportState, PluginSession, SessionCopy, SessionCopyResult},
};
use maka_plugins::storage::Namespace;
use maka_runtime::scope::Scope;
use maka_runtime::{
    event::{
        EventWrite, Fact, Invocation, InvocationInput, InvocationOutcome, LogScope, RuntimeEvent,
    },
    import::{Content, Record, Source},
};
use serde_json::{Value, json};

fn owner(id: &str) -> PluginSession {
    PluginSession {
        session_id: id.into(),
        creator: Namespace::new("external.reader", Scope::Profile).unwrap(),
        fingerprint: format!("request-{id}"),
        managed: false,
        authority_session_id: None,
    }
}
fn source() -> Source {
    Source {
        adapter: "arbitrary-format".into(),
        session_id: "foreign-session".into(),
    }
}
fn record(id: &str, content: Content) -> Record {
    Record {
        source_message_id: id.into(),
        source_turn_id: "foreign-turn".into(),
        timestamp: Some(123),
        content,
    }
}

#[tokio::test]
async fn import_is_retryable_history_not_an_execution_and_survives_copy_and_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("import.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    let owner = owner("imported");
    let first = record(
        "leading",
        Content::Assistant {
            text: "retained, not a prefill".into(),
            model: None,
            thinking: None,
        },
    );
    let records = vec![
        record(
            "user",
            Content::User {
                text: "the original question".into(),
            },
        ),
        record(
            "tool",
            Content::ToolCall {
                call_id: "foreign-call".into(),
                name: "old-command".into(),
                input: Some(json!({"command":"do not run"})),
            },
        ),
        record(
            "answer",
            Content::Assistant {
                text: "the original answer".into(),
                model: Some("foreign-model".into()),
                thinking: Some("original thought".into()),
            },
        ),
        record(
            "result",
            Content::ToolResult {
                call_id: "foreign-call".into(),
                output: json!({"result":"historical"}),
                is_error: false,
            },
        ),
    ];
    log.begin_session_import(&owner, &source(), &json!({"name":"Imported"}), 1)
        .await
        .unwrap();
    log.append_session_import("imported", 0, vec![first.clone()])
        .await
        .unwrap();
    assert!(
        log.get_session::<Value>("imported")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        log.create_session("imported", &owner.fingerprint, &json!({}), 1)
            .await
            .is_err()
    );
    assert!(log.retain_session("imported").await.is_err());
    assert!(
        log.append_session_import("imported", 2, records.clone())
            .await
            .is_err()
    );
    let prefix = log
        .scoped_prefix(
            LogScope::Session {
                id: "imported".into(),
            },
            100,
            1024 * 1024,
        )
        .await
        .unwrap();
    assert!(EventWrite::plain(prefix.events[0].event.clone()).is_err());
    assert!(
        maka_agent::project_model_history(&prefix, "imported")
            .unwrap()
            .is_empty()
    );
    assert!(log.unfinished_invocations(100).await.unwrap().is_empty());
    log.close().await.unwrap();

    let log = EventLog::open(&path).await.unwrap();
    assert_eq!(
        log.begin_session_import(&owner, &source(), &json!({"name":"Imported"}), 99)
            .await
            .unwrap()
            .records,
        1
    );
    assert_eq!(
        log.append_session_import("imported", 0, vec![first.clone()])
            .await
            .unwrap()
            .records,
        1
    );
    let changed = record(
        "leading",
        Content::User {
            text: "changed".into(),
        },
    );
    assert!(matches!(
        log.append_session_import("imported", 0, vec![changed])
            .await,
        Err(StoreError::EventConflict)
    ));
    log.append_session_import("imported", 1, records)
        .await
        .unwrap();
    assert!(log.publish_session_import("imported", 3).await.is_err());
    let receipt = log.publish_session_import("imported", 5).await.unwrap();
    assert_eq!(receipt.state, ImportState::Published);
    assert_eq!(
        log.publish_session_import("imported", 5).await.unwrap(),
        receipt
    );
    assert_eq!(
        log.abandon_session_import("imported").await.unwrap(),
        receipt
    );
    let session = log.get_session::<Value>("imported").await.unwrap().unwrap();
    assert!(session.execution.is_none());
    assert_eq!(
        session.last_message.as_ref().unwrap().preview.as_deref(),
        Some("the original answer")
    );
    let source = log.read_model_context("imported", None).await.unwrap();
    let frozen = source.source_evidence;
    let prefix = log
        .scoped_prefix(
            LogScope::Session {
                id: "imported".into(),
            },
            100,
            1024 * 1024,
        )
        .await
        .unwrap();
    let history = maka_agent::project_model_history(&prefix, "imported").unwrap();
    assert_eq!(history.len(), 2);
    assert!(
        serde_json::to_string(&history)
            .unwrap()
            .contains("the original answer")
    );
    assert!(
        !serde_json::to_string(&history)
            .unwrap()
            .contains("do not run")
    );
    assert!(
        !serde_json::to_string(&history)
            .unwrap()
            .contains("original thought")
    );
    while !log
        .prepare_transcript("imported", prefix.high_water, 32)
        .await
        .unwrap()
    {}
    let read = log
        .history_text(
            "imported",
            prefix.high_water,
            maka_plugins::session::history::Position::Start,
            false,
            16 * 1024,
            8,
            1024 * 1024,
        )
        .await
        .unwrap();
    let text = serde_json::to_string(&read).unwrap();
    assert!(text.contains("retained, not a prefill"));
    assert!(text.contains("the original answer"));

    let rows: Vec<_> = prefix
        .events
        .iter()
        .flat_map(|event| {
            maka_presentation::InvocationView::new(1024 * 1024)
                .unwrap()
                .push(event)
                .unwrap()
        })
        .collect();
    assert_eq!(rows.len(), 5);
    let call = &rows[2].message;
    assert!(matches!(
        rows[3].message.content,
        maka_presentation::Content::Assistant { .. }
    ));
    let maka_presentation::Content::ToolResult { tool_use_id, .. } = &rows[4].message.content
    else {
        panic!("result must retain its position after the intervening answer");
    };
    assert_eq!(tool_use_id, &call.id);

    let copy = log
        .copy_session(
            SessionCopy {
                source_session_id: "imported".into(),
                target_session_id: "copy".into(),
                expected_source_revision: session.revision,
                purpose: maka_runtime::session::CopyPurpose::Branch {
                    turn_id: None,
                    side_conversation: false,
                },
            },
            &json!({}),
            2,
        )
        .await
        .unwrap();
    assert!(matches!(copy, SessionCopyResult::Committed(_)));
    assert_eq!(
        log.read_model_context("copy", None)
            .await
            .unwrap()
            .tail
            .len(),
        5
    );

    let invocation = Invocation {
        session_id: "imported".into(),
        turn_id: "new-turn".into(),
        run_id: "new-run".into(),
        invocation_id: "new-invocation".into(),
    };
    for fact in [
        Fact::InvocationOpened {
            input: InvocationInput::Message {
                content: "continue".into(),
                request_fingerprint: None,
                source_messages: vec![],
            },
            configuration: None,
        },
        Fact::InvocationEnded {
            outcome: InvocationOutcome::Completed,
        },
    ] {
        log.append(&EventWrite::plain(RuntimeEvent::new(invocation.clone(), fact)).unwrap())
            .await
            .unwrap();
    }
    assert_eq!(
        log.read_frozen_model_context(&frozen, 100, 1024 * 1024)
            .await
            .unwrap()
            .tail
            .len(),
        5
    );
    let prefix = log
        .scoped_prefix(
            LogScope::Session {
                id: "imported".into(),
            },
            100,
            1024 * 1024,
        )
        .await
        .unwrap();
    assert_eq!(
        maka_agent::project_model_history(&prefix, "imported")
            .unwrap()
            .len(),
        3
    );
    assert!(!prefix.events.iter().any(|event| matches!(
        event.event.fact,
        Fact::ModelRequested { .. } | Fact::ToolDispatched { .. }
    )));
    log.close().await.unwrap();
}

#[tokio::test]
async fn nonconversation_import_cannot_publish_and_abandon_releases_material() {
    let directory = tempfile::tempdir().unwrap();
    let log = EventLog::open(&directory.path().join("abandon.sqlite"))
        .await
        .unwrap();
    log.create_session("parent", "parent", &json!({}), 1)
        .await
        .unwrap();
    let mut child = owner("dependent");
    child.authority_session_id = Some("parent".into());
    log.begin_session_import(&child, &source(), &json!({}), 1)
        .await
        .unwrap();
    log.append_session_import(
        "dependent",
        0,
        vec![record(
            "user",
            Content::User {
                text: "historical".into(),
            },
        )],
    )
    .await
    .unwrap();
    let parent = log.get_session::<Value>("parent").await.unwrap().unwrap();
    log.begin_session_removal("parent", parent.revision)
        .await
        .unwrap();
    assert!(log.publish_session_import("dependent", 1).await.is_err());
    assert_eq!(
        log.session_import_progress("dependent")
            .await
            .unwrap()
            .state,
        ImportState::Collecting
    );
    log.abandon_session_import("dependent").await.unwrap();
    log.begin_session_import(&owner("notes"), &source(), &json!({}), 1)
        .await
        .unwrap();
    assert!(
        log.append_session_import(
            "notes",
            0,
            vec![record(
                "oversize",
                Content::User {
                    text: "x".repeat(6 * 1024 * 1024),
                }
            )]
        )
        .await
        .is_err()
    );
    assert_eq!(
        log.session_import_progress("notes").await.unwrap().records,
        0
    );
    log.append_session_import(
        "notes",
        0,
        vec![record(
            "note",
            Content::Note {
                text: "not a conversation".into(),
            },
        )],
    )
    .await
    .unwrap();
    assert!(log.publish_session_import("notes", 1).await.is_err());
    assert_eq!(
        log.abandon_session_import("notes").await.unwrap().state,
        ImportState::Abandoned
    );
    assert!(log.publish_session_import("notes", 1).await.is_err());
    while log.collect_session_material(None).await.unwrap()
        != maka_event_log::sessions::MaterialCollection::Done
    {}
    assert!(matches!(
        log.scoped_prefix(LogScope::Session { id: "notes".into() }, 100, 1024 * 1024)
            .await,
        Err(StoreError::MaterialCollected)
    ));
    assert!(log.get_session::<Value>("notes").await.unwrap().is_none());
    log.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn imported_conversation_compacts_before_its_first_native_turn() {
    use crate::support::context::{SUMMARY, engine, input, read_request, respond};
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;
    let directory = tempfile::tempdir().unwrap();
    let log = Arc::new(
        EventLog::open(&directory.path().join("compact.sqlite"))
            .await
            .unwrap(),
    );
    log.begin_session_import(&owner("session"), &source(), &json!({}), 1)
        .await
        .unwrap();
    log.append_session_import(
        "session",
        0,
        vec![
            record(
                "user",
                Content::User {
                    text: "original-import-question".into(),
                },
            ),
            record("blank", Content::User { text: "  ".into() }),
            record(
                "thought",
                Content::Assistant {
                    text: String::new(),
                    model: None,
                    thinking: Some("historical reasoning".into()),
                },
            ),
            record(
                "answer",
                Content::Assistant {
                    text: "original-import-answer".into(),
                    model: None,
                    thinking: None,
                },
            ),
        ],
    )
    .await
    .unwrap();
    log.publish_session_import("session", 4).await.unwrap();
    let prefix = log
        .scoped_prefix(
            LogScope::Session {
                id: "session".into(),
            },
            100,
            128 * 1024,
        )
        .await
        .unwrap();
    assert_eq!(
        maka_agent::project_model_history(&prefix, "session")
            .unwrap()
            .len(),
        2
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for text in [SUMMARY, "continued-answer"] {
            let (mut socket, _) =
                tokio::time::timeout(std::time::Duration::from_secs(10), listener.accept())
                    .await
                    .unwrap()
                    .unwrap();
            requests.push(read_request(&mut socket).await);
            respond(&mut socket, text, "stop").await;
        }
        requests
    });
    let worker = engine(log.clone());
    worker
        .run(input(&base, "compact", true), CancellationToken::new())
        .await
        .unwrap();
    assert!(
        log.read_model_context("session", None)
            .await
            .unwrap()
            .baseline
            .is_some()
    );
    worker
        .run(input(&base, "continue", false), CancellationToken::new())
        .await
        .unwrap();
    worker.drain().await;
    let requests = server.await.unwrap();
    assert!(requests[0].to_string().contains("original-import-answer"));
    assert!(!requests[1].to_string().contains("original-import-answer"));
    assert!(requests[1].to_string().contains("summary-marker"));
    drop(worker);
    Arc::try_unwrap(log).ok().unwrap().close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn large_import_fits_summary_recovers_overflow_and_continues_after_restart() {
    use crate::support::context::{SUMMARY, engine, input, read_request, respond};
    use maka_runtime::context::ModelRequestContext;
    use std::sync::Arc;
    use tokio::io::AsyncWriteExt;
    use tokio_util::sync::CancellationToken;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("large-import.sqlite");
    let log = Arc::new(EventLog::open(&path).await.unwrap());
    let large = format!(
        "original-goal-marker {} original-tail-marker",
        "历史".repeat(1_600_000)
    );
    let large_bytes = large.len();
    let mut records = vec![record("large", Content::User { text: large })];
    records.extend((0..7600).map(|index| {
        record(
            &format!("row-{index}"),
            Content::User {
                text: format!("Historical update {index}: keep this work available."),
            },
        )
    }));
    log.import_session(&owner("session"), &source(), &json!({}), records, 1)
        .await
        .unwrap();
    let progress = log.session_import_progress("session").await.unwrap();
    assert_eq!(progress.records, 7601);
    assert!(progress.bytes > 9 * 1024 * 1024);
    assert!(log.unfinished_invocations(16).await.unwrap().is_empty());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in [
            None,
            Some(SUMMARY),
            Some("continued-answer"),
            Some("reopened-answer"),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            requests.push(read_request(&mut socket).await);
            if let Some(response) = response {
                respond(&mut socket, response, "stop").await;
            } else {
                let body = json!({"error":{"message":"maximum context length exceeded", "type":"invalid_request_error", "code":"context_length_exceeded"}}).to_string();
                socket.write_all(format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        }
        requests
    });
    let run_input = |id| {
        let mut run = input(&base, id, false);
        let window = if id == "reopen" { 4096 } else { 65_536 };
        run.context = Some(ModelRequestContext {
            provider_id: "fixture".into(),
            context_window: Some(window),
            model_context_window: Some(window),
            declared_window: Some(window * 85 / 100),
        });
        run
    };
    let worker = engine(log.clone());
    worker
        .run(run_input("continue"), CancellationToken::new())
        .await
        .unwrap();
    worker.drain().await;
    let checkpoint = log
        .read_model_context("session", None)
        .await
        .unwrap()
        .baseline
        .unwrap()
        .event_id;
    assert_eq!(
        log.session_import_progress("session").await.unwrap(),
        progress
    );
    let original = log
        .scoped_prefix(
            LogScope::Session {
                id: "session".into(),
            },
            8000,
            32 * 1024 * 1024,
        )
        .await
        .unwrap();
    assert!(
        matches!(&original.events[0].event.fact, Fact::MessageImported { record, .. }
        if matches!(&record.content, Content::User { text } if text.len() == large_bytes && text.ends_with("original-tail-marker")))
    );
    drop(original);
    drop(worker);
    Arc::try_unwrap(log).ok().unwrap().close().await.unwrap();

    let log = Arc::new(EventLog::open(&path).await.unwrap());
    let worker = engine(log.clone());
    worker
        .run(run_input("reopen"), CancellationToken::new())
        .await
        .unwrap();
    worker.drain().await;
    let requests = server.await.unwrap();
    assert!(requests[3]["max_tokens"].as_u64().unwrap() < 4096);
    let first = requests[0].to_string();
    let second = requests[1].to_string();
    assert!(first.contains("original-goal-marker") && first.contains("Historical update 7599"));
    assert!(first.len() < 200_000, "{}", first.len());
    assert!(
        second.len() < first.len() * 3 / 4,
        "overflow must shrink the actual request"
    );
    for (index, current) in [(2, "question-continue"), (3, "question-reopen")] {
        let request = requests[index].to_string();
        assert!(request.contains("summary-marker") && request.contains(current));
        assert!(
            !request.contains("Historical update") && !request.contains("original-goal-marker")
        );
    }
    assert_eq!(
        log.read_model_context("session", None)
            .await
            .unwrap()
            .baseline
            .unwrap()
            .event_id,
        checkpoint
    );
    assert_eq!(
        log.session_import_progress("session").await.unwrap(),
        progress
    );
    drop(worker);
    Arc::try_unwrap(log).ok().unwrap().close().await.unwrap();
}
