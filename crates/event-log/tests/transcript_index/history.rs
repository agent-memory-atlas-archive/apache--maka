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
use maka_plugins::session::history::{Page, Position, Role};
use std::collections::BTreeMap;

#[tokio::test]
async fn history_pages_preserve_utf8_nul_suffixes_archive_and_frozen_fences() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("log.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    log.create_session("a", "create", &serde_json::json!({}), 1)
        .await
        .unwrap();
    let text = format!("a{}\0suffix-needle", "🦀".repeat(10_000));
    let (_, old) = partial(&log, "a", &text).await;
    interrupt(&log, "a").await;
    log.append(&event("a", request("history"))).await.unwrap();
    let call = maka_runtime::model::ModelToolCall {
        id: "read".into(),
        name: "Read".into(),
        input: serde_json::json!({"path": "source.rs", "intent": "Inspect source"}),
        provider_options: None,
        provider_executed: false,
    };
    log.append(&event(
        "a",
        Fact::ModelObserved {
            step_id: "step-history".into(),
            event: ModelEvent::ToolCall(call.clone()),
        },
    ))
    .await
    .unwrap();
    log.append(&event(
        "a",
        Fact::ModelCompleted {
            step_id: "step-history".into(),
            output: ModelStep {
                parts: vec![ModelPart::ToolCall { call }],
                finish_reason: maka_runtime::model::ModelFinishReason::ToolCalls,
                usage: ModelUsage::default(),
                provider_options: None,
                response_id: None,
                model: None,
                timestamp: None,
            },
        },
    ))
    .await
    .unwrap();
    log.append(&event(
        "a",
        Fact::ToolDispatched {
            title: None,
            operation_id: "step-history:read".into(),
            call: maka_runtime::tool_call::ToolCallIdentity::provider(
                "step-history".into(),
                "read".into(),
            ),
            name: "Read".into(),
            input: serde_json::json!({"path": "source.rs", "intent": "Inspect source"}),
        },
    ))
    .await
    .unwrap();
    let template = event(
        "a",
        Fact::InvocationEnded {
            outcome: InvocationOutcome::Completed,
        },
    );
    let (result, _) = maka_runtime::event::EventWrite::tool_success(
        "read-result".into(),
        template.event().recorded_at,
        template.event().invocation.clone(),
        "step-history:read".into(),
        serde_json::json!({"content": text, "nested": ["second\nline", 42, false]}).into(),
    )
    .unwrap();
    log.append(&result).await.unwrap();
    let through = end(
        &log,
        "a",
        InvocationOutcome::Cancelled {
            source: "user".into(),
        },
    )
    .await;
    assert!(log.prepare_transcript("a", through, 32).await.unwrap());
    log.set_session_archived::<Value>("a", true, 10)
        .await
        .unwrap();
    let archived = log
        .scoped_sessions::<Value>(
            maka_event_log::sessions::CatalogScope::Profile,
            None,
            None.into(),
            maka_event_log::sessions::CatalogFilter {
                include_archived: true,
                pending_only: false,
            },
        )
        .await
        .unwrap();
    assert!(archived.sessions[0].archived);
    let mut position = Position::Start;
    let mut messages: BTreeMap<String, (Role, String)> = BTreeMap::new();
    let mut pages = 0;
    loop {
        let page = log
            .history_text("a", through, position, false, 16 * 1024, 8, 1024 * 1024)
            .await
            .unwrap();
        assert!(serde_json::to_vec(&page).unwrap().len() < 256 * 1024);
        let Page::Ready { chunks, after, .. } = page else {
            panic!("prepared")
        };
        for chunk in chunks {
            match &chunk.tool {
                Some(maka_plugins::session::history::Tool::Call {
                    tool_use_id,
                    tool_name,
                }) => {
                    assert_eq!(tool_use_id, &chunk.message_id);
                    assert_eq!(tool_name, "Read");
                    assert_eq!(chunk.role, Role::ToolCall);
                }
                Some(maka_plugins::session::history::Tool::Result {
                    tool_use_id,
                    tool_name,
                    is_error,
                    image,
                }) => {
                    assert_eq!(tool_name.as_deref(), Some("Read"));
                    assert!(!is_error);
                    assert!(!image);
                    assert!(messages.contains_key(tool_use_id));
                    assert_eq!(chunk.role, Role::ToolResult);
                }
                None => assert!(matches!(chunk.role, Role::User | Role::Assistant)),
            }
            let entry = messages
                .entry(chunk.message_id)
                .or_insert((chunk.role, String::new()));
            assert_eq!(entry.1.len() as u64, chunk.offset);
            entry.1.push_str(&chunk.text);
            assert!(entry.1.len() as u64 <= chunk.total_bytes);
        }
        pages += 1;
        assert!(pages < 20, "continuation must advance");
        let Some(cursor) = after else {
            break;
        };
        position = Position::After { cursor };
    }
    assert!(pages >= 3, "large message must be split, not clipped");
    assert_eq!(
        messages
            .values()
            .find(|(role, _)| *role == Role::Assistant)
            .unwrap()
            .1,
        text
    );
    assert_eq!(
        messages
            .values()
            .find(|(role, _)| *role == Role::ToolResult)
            .unwrap()
            .1,
        format!("{text}\nsecond\nline"),
        "JSON string values stay multiline; property names and non-text leaves are not searchable"
    );
    let Page::Ready { chunks, after, .. } = log
        .history_text("a", old, Position::Start, false, 16 * 1024, 8, 1024 * 1024)
        .await
        .unwrap()
    else {
        panic!("prepared")
    };
    assert!(after.is_none());
    assert!(
        chunks.iter().all(|chunk| chunk.role == Role::User),
        "old fence excludes unfinished answer"
    );
    // Exercise the disposable cache with two owners of identical source positions.
    // A destination's cache must neither satisfy nor duplicate a source lookup.
    log.create_session("b", "second-owner", &serde_json::json!({}), 1)
        .await
        .unwrap();
    let cache = rusqlite::Connection::open(&path).unwrap();
    cache
        .execute(
            "INSERT INTO transcript_rows
         SELECT sequence, 'b', turn_id, message_id, payload, digest, total_bytes
         FROM transcript_rows WHERE session_id = 'a'",
            [],
        )
        .unwrap();
    cache
        .execute(
            "INSERT INTO transcript_progress SELECT 'b', through_sequence
         FROM transcript_progress WHERE session_id = 'a'",
            [],
        )
        .unwrap();
    cache
        .execute(
            "INSERT INTO transcript_text
         SELECT 'b', sequence, field, timestamp, role, CASE WHEN role IN ('user', 'assistant', 'tool_call', 'tool_result') THEN CAST('destination cache' AS BLOB) ELSE body END
         FROM transcript_text WHERE session_id = 'a'",
            [],
        )
        .unwrap();
    let Page::Ready {
        chunks: destination,
        ..
    } = log
        .history_text("b", old, Position::Start, false, 16 * 1024, 8, 1024 * 1024)
        .await
        .unwrap()
    else {
        panic!("prepared destination");
    };
    assert!(!destination.is_empty());
    assert!(
        destination
            .iter()
            .all(|chunk| chunk.text == "destination cache")
    );
    let Page::Ready { chunks: source, .. } = log
        .history_text("a", old, Position::Start, false, 16 * 1024, 8, 1024 * 1024)
        .await
        .unwrap()
    else {
        panic!("prepared source");
    };
    assert_eq!(
        serde_json::to_value(source).unwrap(),
        serde_json::to_value(chunks).unwrap()
    );
    cache
        .execute("DELETE FROM transcript_text WHERE session_id = 'b'", [])
        .unwrap();
    let rebuilt = log
        .history_text("b", old, Position::Start, false, 16 * 1024, 8, 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(rebuilt).unwrap(),
        serde_json::to_value(
            log.history_text("a", old, Position::Start, false, 16 * 1024, 8, 1024 * 1024)
                .await
                .unwrap()
        )
        .unwrap(),
    );
    drop(cache);
    log.close().await.unwrap();
    let log = EventLog::open(&path).await.unwrap();
    let Page::Ready { chunks, .. } = log
        .history_text(
            "a",
            through,
            Position::Start,
            false,
            16 * 1024,
            8,
            1024 * 1024,
        )
        .await
        .unwrap()
    else {
        panic!("prepared")
    };
    assert!(!chunks.is_empty(), "restart retains readable archive");
    log.close().await.unwrap();
}

#[tokio::test]
async fn tool_metadata_preserves_empty_calls_and_non_text_results_through_frozen_windows() {
    use maka_plugins::session::history::Tool;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("tools.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    for session in ["numeric", "unknown", "image"] {
        log.create_session(session, session, &serde_json::json!({}), 1)
            .await
            .unwrap();
        log.append(&event(session, opening())).await.unwrap();
        log.append(&event(session, request(session))).await.unwrap();
        let call = maka_runtime::model::ModelToolCall {
            id: "count".into(),
            name: "Count".into(),
            input: serde_json::json!({"count": 42}),
            provider_options: None,
            provider_executed: false,
        };
        let step = format!("step-{session}");
        log.append(&event(
            session,
            observe(session, ModelEvent::ToolCall(call.clone())),
        ))
        .await
        .unwrap();
        log.append(&event(
            session,
            Fact::ModelCompleted {
                step_id: step.clone(),
                output: ModelStep {
                    parts: vec![ModelPart::ToolCall { call }],
                    finish_reason: maka_runtime::model::ModelFinishReason::ToolCalls,
                    usage: ModelUsage::default(),
                    provider_options: None,
                    response_id: None,
                    model: None,
                    timestamp: None,
                },
            },
        ))
        .await
        .unwrap();
        let operation = format!("{step}:count");
        let call_fence = log
            .append(&event(
                session,
                Fact::ToolDispatched {
                    title: None,
                    operation_id: operation.clone(),
                    call: maka_runtime::tool_call::ToolCallIdentity::provider(step, "count".into()),
                    name: "Count".into(),
                    input: serde_json::json!({"count": 42}),
                },
            ))
            .await
            .unwrap();
        if session != "unknown" {
            let template = event(
                session,
                Fact::InvocationEnded {
                    outcome: InvocationOutcome::Completed,
                },
            );
            let output = if session == "image" {
                maka_runtime::tool_output::ToolOutput::Image(
                    maka_runtime::tool_output::ImageOutput {
                        detail: None,
                        mime_type: "image/png".into(),
                        reference: maka_runtime::attachment::StorageRef::WorkspaceFile {
                            relative_path: "unread-image.png".into(),
                        },
                    },
                )
            } else {
                serde_json::json!({"count": 42, "ready": true}).into()
            };
            let (result, _) = maka_runtime::event::EventWrite::tool_success(
                format!("{session}-result"),
                template.event().recorded_at,
                template.event().invocation.clone(),
                operation,
                output.into(),
            )
            .unwrap();
            log.append(&result).await.unwrap();
        } else {
            log.append(&event(
                session,
                Fact::ToolSettled {
                    operation_id: operation,
                    outcome: maka_runtime::event::ToolOutcome::Unknown {
                        message: "Result unknown; do not repeat".into(),
                        operation: None,
                    },
                },
            ))
            .await
            .unwrap();
        }
        let through = end(&log, session, InvocationOutcome::Completed).await;
        assert!(log.prepare_transcript(session, through, 32).await.unwrap());
        let Page::Ready { chunks, .. } = log
            .history_text(
                session,
                call_fence,
                Position::Start,
                false,
                8192,
                8,
                64 * 1024,
            )
            .await
            .unwrap()
        else {
            panic!("ready")
        };
        assert!(chunks.iter().any(|chunk| chunk.role == Role::ToolCall
            && chunk.text.is_empty()
            && matches!(&chunk.tool, Some(Tool::Call { tool_name, .. }) if tool_name == "Count")));
        assert!(
            chunks.iter().all(|chunk| chunk.role != Role::ToolResult),
            "old fence excludes settled result"
        );
        for reverse in [false, true] {
            let mut position = if reverse {
                Position::Tail
            } else {
                Position::Start
            };
            let mut tools = BTreeMap::new();
            for _ in 0..16 {
                let Page::Ready {
                    chunks,
                    before,
                    after,
                    ..
                } = log
                    .history_text(session, through, position, false, 8192, 1, 64 * 1024)
                    .await
                    .unwrap()
                else {
                    panic!("ready")
                };
                assert!(chunks.len() <= 1);
                for chunk in chunks {
                    if matches!(chunk.role, Role::ToolCall | Role::ToolResult) {
                        use maka_plugins::session::history::{
                            SourceField, SourceFormat, SourcePosition,
                        };
                        let field = if chunk.role == Role::ToolCall {
                            SourceField::Arguments
                        } else {
                            SourceField::Result
                        };
                        let source = log
                            .history_source(
                                session,
                                through,
                                chunk.sequence,
                                field,
                                SourcePosition::Start,
                                8192,
                                64 * 1024,
                            )
                            .await
                            .unwrap();
                        assert_eq!(source.tool.as_ref().unwrap(), chunk.tool.as_ref().unwrap());
                        if chunk.role == Role::ToolResult {
                            assert!(
                                log.history_source(
                                    session,
                                    call_fence,
                                    chunk.sequence,
                                    field,
                                    SourcePosition::Start,
                                    8192,
                                    64 * 1024
                                )
                                .await
                                .is_err()
                            );
                            if session == "numeric" {
                                assert_eq!(source.format, SourceFormat::Json);
                                assert_eq!(
                                    serde_json::from_str::<Value>(&source.text).unwrap(),
                                    serde_json::json!({"count":42,"ready":true})
                                );
                            } else if session == "image" {
                                assert_eq!(source.format, SourceFormat::ImageDescriptor);
                                let descriptor: Value = serde_json::from_str(&source.text).unwrap();
                                assert_eq!(descriptor["kind"], "image");
                                assert_eq!(descriptor["mimeType"], "image/png");
                                assert!(source.text.contains("unread-image.png"));
                                assert!(!temp.path().join("unread-image.png").exists());
                            } else {
                                assert_eq!(source.format, SourceFormat::Text);
                                assert_eq!(source.text, "Result unknown; do not repeat");
                            }
                        }
                    }
                    if let Some(tool) = chunk.tool {
                        tools.insert(chunk.role as u8, (chunk.text, tool));
                    }
                }
                let next = if reverse { before } else { after };
                let Some(cursor) = next else { break };
                position = if reverse {
                    Position::Before { cursor }
                } else {
                    Position::After { cursor }
                };
            }
            assert_eq!(
                tools.len(),
                2,
                "empty tool rows must not disappear or stall paging"
            );
            let (
                text,
                Tool::Result {
                    tool_use_id,
                    tool_name,
                    is_error,
                    image,
                },
            ) = &tools[&(Role::ToolResult as u8)]
            else {
                panic!("result")
            };
            assert_eq!(tool_name.as_deref(), Some("Count"));
            assert_eq!(*is_error, session == "unknown");
            assert_eq!(*image, session == "image");
            assert_eq!(text.is_empty(), session != "unknown");
            let (
                _,
                Tool::Call {
                    tool_use_id: call_id,
                    ..
                },
            ) = &tools[&(Role::ToolCall as u8)]
            else {
                panic!("call")
            };
            assert_eq!(tool_use_id, call_id);
        }
    }
    log.close().await.unwrap();
    let log = EventLog::open(&path).await.unwrap();
    let through = *log.subscribe_commits().borrow();
    assert!(
        log.prepare_transcript("numeric", through, 32)
            .await
            .unwrap()
    );
    let Page::Ready { chunks, .. } = log
        .history_text(
            "numeric",
            through,
            Position::Tail,
            false,
            8192,
            8,
            64 * 1024,
        )
        .await
        .unwrap()
    else {
        panic!("ready")
    };
    assert!(chunks.iter().any(|chunk| matches!(&chunk.tool, Some(Tool::Result { tool_name, is_error: false, .. }) if tool_name.as_deref() == Some("Count"))));
    log.close().await.unwrap();
}

#[tokio::test]
async fn oversized_tool_metadata_returns_accepted_window_without_consuming_its_boundary() {
    use maka_plugins::{composition::Scope, storage::Namespace};
    use maka_runtime::import::{Content, Record, Source};
    let temp = tempfile::tempdir().unwrap();
    let log = EventLog::open(&temp.path().join("tool-budget.sqlite"))
        .await
        .unwrap();
    let owner = maka_event_log::sessions::PluginSession {
        session_id: "budget".into(),
        creator: Namespace::new("history.fixture", Scope::Profile).unwrap(),
        fingerprint: "tool-budget".into(),
        managed: false,
        authority_session_id: None,
    };
    log.begin_session_import(
        &owner,
        &Source {
            adapter: "fixture".into(),
            session_id: "foreign".into(),
        },
        &serde_json::json!({}),
        1,
    )
    .await
    .unwrap();
    let records = [
        Content::User {
            text: "accepted prefix".into(),
        },
        Content::ToolCall {
            call_id: "call".into(),
            name: "name".repeat(1024),
            input: None,
        },
        Content::Assistant {
            text: "accepted suffix".into(),
            model: None,
            thinking: None,
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(index, content)| Record {
        source_message_id: format!("m{index}"),
        source_turn_id: "turn".into(),
        timestamp: Some(1),
        content,
    })
    .collect::<Vec<_>>();
    log.append_session_import("budget", 0, records)
        .await
        .unwrap();
    log.publish_session_import("budget", 3).await.unwrap();
    let through = *log.subscribe_commits().borrow();
    assert!(log.prepare_transcript("budget", through, 32).await.unwrap());
    for reverse in [false, true] {
        let position = if reverse {
            Position::Tail
        } else {
            Position::Start
        };
        let Page::Ready {
            chunks,
            before,
            after,
            ..
        } = log
            .history_text("budget", through, position, false, 8192, 8, 1024)
            .await
            .unwrap()
        else {
            panic!("ready")
        };
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0].text,
            if reverse {
                "accepted suffix"
            } else {
                "accepted prefix"
            }
        );
        let cursor = if reverse { before } else { after }.unwrap();
        let position = if reverse {
            Position::Before { cursor }
        } else {
            Position::After { cursor }
        };
        assert!(
            log.history_text("budget", through, position, false, 8192, 8, 1024)
                .await
                .is_err(),
            "indivisible first tool metadata must fail without being consumed"
        );
        let Page::Ready { chunks, .. } = log
            .history_text("budget", through, position, false, 8192, 1, 64 * 1024)
            .await
            .unwrap()
        else {
            panic!("ready")
        };
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].role, Role::ToolCall);
        assert!(chunks[0].tool.is_some());
        assert!(chunks[0].text.is_empty());
    }
    log.close().await.unwrap();
}

#[tokio::test]
async fn bounded_history_windows_cover_sparse_rows_both_directions_and_validate_real_positions() {
    use maka_plugins::session::history::Cursor;
    use maka_plugins::{composition::Scope, storage::Namespace};
    use maka_runtime::import::{Content, Record, Source};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("windows.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    let owner = maka_event_log::sessions::PluginSession {
        session_id: "window".into(),
        creator: Namespace::new("history.fixture", Scope::Profile).unwrap(),
        fingerprint: "window-import".into(),
        managed: false,
        authority_session_id: None,
    };
    log.begin_session_import(
        &owner,
        &Source {
            adapter: "fixture".into(),
            session_id: "foreign".into(),
        },
        &serde_json::json!({}),
        1,
    )
    .await
    .unwrap();
    let mut records = Vec::new();
    let mut expected = Vec::new();
    for index in 0..24 {
        let content = if index % 4 == 2 {
            let text = format!("{index} a🦀中é\0tail\r\n{}", "🌏".repeat(index % 5 + 1));
            expected.push(text.clone());
            Content::Assistant {
                text,
                model: None,
                thinking: None,
            }
        } else if index % 4 == 1 {
            Content::User {
                text: String::new(),
            }
        } else {
            Content::Note {
                text: "bookkeeping is not conversation text".into(),
            }
        };
        records.push(Record {
            source_message_id: format!("m{index}"),
            source_turn_id: "turn".into(),
            timestamp: Some(1),
            content,
        });
    }
    let count = records.len() as u64;
    for (batch, records) in records.chunks(8).enumerate() {
        log.append_session_import("window", (batch * 8) as u64, records.to_vec())
            .await
            .unwrap();
    }
    log.publish_session_import("window", count).await.unwrap();
    let through = *log.subscribe_commits().borrow();
    while !log.prepare_transcript("window", through, 32).await.unwrap() {}
    let mut canonical = None;
    for budget in [4, 5, 7, 13, 128 * 1024] {
        for reverse in [false, true] {
            let db = Connection::open(&path).unwrap();
            db.execute("DELETE FROM transcript_text", []).unwrap();
            drop(db);
            let mut position = if reverse {
                Position::Tail
            } else {
                Position::Start
            };
            let mut collected: BTreeMap<u64, (u64, String)> = BTreeMap::new();
            let mut previous = None;
            let mut done = false;
            for _ in 0..512 {
                let Page::Ready {
                    chunks,
                    before,
                    after,
                    through: actual,
                } = log
                    .history_text("window", through, position, false, budget, 2, 1024 * 1024)
                    .await
                    .unwrap()
                else {
                    panic!("prepared");
                };
                assert_eq!(actual, through);
                assert!(chunks.len() <= 2);
                assert!(chunks.iter().map(|chunk| chunk.text.len()).sum::<usize>() <= budget);
                assert!(
                    chunks
                        .windows(2)
                        .all(|pair| pair[0].sequence < pair[1].sequence)
                );
                for chunk in chunks {
                    let entry = collected.entry(chunk.sequence).or_insert_with(|| {
                        (if reverse { chunk.total_bytes } else { 0 }, String::new())
                    });
                    if reverse {
                        assert_eq!(chunk.offset + chunk.text.len() as u64, entry.0);
                        entry.1.insert_str(0, &chunk.text);
                        entry.0 = chunk.offset;
                    } else {
                        assert_eq!(chunk.offset, entry.0);
                        entry.1.push_str(&chunk.text);
                        entry.0 += chunk.text.len() as u64;
                    }
                    assert!(
                        chunk.sequence > through,
                        "sparse row positions differ from raw event fence"
                    );
                }
                let cursor = if reverse { before } else { after };
                let Some(cursor) = cursor else {
                    done = true;
                    break;
                };
                assert_ne!(
                    previous,
                    Some(cursor),
                    "empty pages and cache rebuilding must progress"
                );
                previous = Some(cursor);
                position = if reverse {
                    Position::Before { cursor }
                } else {
                    Position::After { cursor }
                };
            }
            assert!(done, "budget {budget}, reverse {reverse}");
            let texts = collected
                .values()
                .map(|(_, text)| text.clone())
                .collect::<Vec<_>>();
            assert_eq!(texts, expected);
            let sequences = collected.keys().copied().collect::<Vec<_>>();
            assert!(sequences.windows(2).all(|pair| pair[1] - pair[0] > 1));
            canonical.get_or_insert(collected);
        }
    }
    let first = *canonical.as_ref().unwrap().keys().next().unwrap();
    // The first message starts with two ASCII digits plus a space and then 'a'.
    // Find every exact byte boundary rather than deriving one from a JS index.
    let text = &canonical.as_ref().unwrap()[&first].1;
    for offset in 0..=text.len() {
        for reverse in [false, true] {
            let cursor = Cursor {
                field: Default::default(),
                sequence: first,
                offset: offset as u64,
            };
            let result = log
                .history_text(
                    "window",
                    through,
                    if reverse {
                        Position::Before { cursor }
                    } else {
                        Position::After { cursor }
                    },
                    false,
                    7,
                    2,
                    1024 * 1024,
                )
                .await;
            assert_eq!(
                result.is_ok(),
                text.is_char_boundary(offset),
                "offset {offset}, reverse {reverse}"
            );
        }
    }
    for cursor in [
        Cursor {
            field: Default::default(),
            sequence: first + 1,
            offset: 0,
        },
        Cursor {
            field: Default::default(),
            sequence: first,
            offset: text.len() as u64 + 1,
        },
        Cursor {
            field: Default::default(),
            sequence: 0,
            offset: 0,
        },
    ] {
        assert!(
            log.history_text(
                "window",
                through,
                Position::After { cursor },
                false,
                7,
                2,
                1024 * 1024
            )
            .await
            .is_err()
        );
    }
    log.create_session("other", "other", &serde_json::json!({}), 1)
        .await
        .unwrap();
    let high = *log.subscribe_commits().borrow();
    assert!(log.prepare_transcript("other", high, 32).await.unwrap());
    assert!(
        log.history_text(
            "other",
            high,
            Position::After {
                cursor: Cursor {
                    field: Default::default(),
                    sequence: first,
                    offset: 0
                }
            },
            false,
            7,
            2,
            1024 * 1024
        )
        .await
        .is_err()
    );
    let Page::Ready { chunks, after, .. } = log
        .history_text(
            "window",
            through,
            Position::Event { sequence: through },
            false,
            7,
            2,
            1024 * 1024,
        )
        .await
        .unwrap()
    else {
        panic!("prepared");
    };
    assert!(chunks.is_empty());
    assert!(after.is_none());
    log.close().await.unwrap();
}

#[tokio::test]
async fn history_text_requires_the_actual_prepared_raw_fence() {
    let temp = tempfile::tempdir().unwrap();
    let log = EventLog::open(&temp.path().join("fence.sqlite"))
        .await
        .unwrap();
    log.create_session("a", "create", &serde_json::json!({}), 1)
        .await
        .unwrap();
    let (_, through) = partial(&log, "a", "answer").await;
    assert!(log.prepare_transcript("a", through, 32).await.unwrap());
    let later = interrupt(&log, "a").await;
    assert!(later > through);
    assert!(
        log.history_text("a", later, Position::Start, false, 16, 2, 1024 * 1024)
            .await
            .is_err(),
        "raw fence must not be compared directly with a prepared row watermark"
    );
    assert!(log.prepare_transcript("a", later, 32).await.unwrap());
    let Page::Ready { chunks, .. } = log
        .history_text("a", later, Position::Tail, false, 16, 2, 1024 * 1024)
        .await
        .unwrap()
    else {
        panic!("prepared");
    };
    assert!(chunks.iter().any(|chunk| chunk.role == Role::Assistant));
    log.close().await.unwrap();
}

#[tokio::test]
async fn empty_projection_boundaries_obey_the_exact_json_window_budget() {
    use maka_runtime::{
        attachment::{AttachmentKind, AttachmentRef, StorageRef},
        input::DeliveredMessage,
        message::{MessageDisposition, Placement, RootSourceMessage},
    };
    let temp = tempfile::tempdir().unwrap();
    let log = EventLog::open(&temp.path().join("wire.sqlite"))
        .await
        .unwrap();
    log.create_session("a", "create", &serde_json::json!({}), 1)
        .await
        .unwrap();
    log.append(&event("a", opening())).await.unwrap();
    let mut target = 0;
    for index in 0..55 {
        let mut content: maka_runtime::input::MessageInput = "".into();
        if index == 37 {
            content.attachments = Some(vec![AttachmentRef {
                kind: AttachmentKind::Other,
                name: "a".into(),
                mime_type: "application/octet-stream".into(),
                bytes: 1,
                storage_ref: StorageRef::WorkspaceFile {
                    relative_path: "a".repeat(1100),
                },
            }]);
        }
        let message = DeliveredMessage {
            message_id: format!("empty-{index}"),
            submitted_content_digest: content.content_digest().unwrap(),
            content: content.clone(),
        };
        let source = RootSourceMessage {
            message: message.clone(),
            unprepared_content: content,
            submitted_placement: Placement::CurrentTurn,
            disposition: MessageDisposition::Steering,
            submitted_intent: None,
        };
        source.validate().unwrap();
        let sequence = log
            .append(&event(
                "a",
                Fact::MessageSteered {
                    message: Box::new(message),
                    source: Some(Box::new(source)),
                },
            ))
            .await
            .unwrap();
        if index == 37 {
            target = sequence;
            // The attachment body now ends at a two-digit byte offset. Jump
            // two sequence digits so consuming the next empty row still grows
            // the wire cursor even when that offset returns to zero. Other
            // Sessions advance the real log without adding rows to this one.
            let gap = (0..400)
                .map(|index| event(&format!("gap-{index}"), opening()))
                .collect::<Vec<_>>();
            log.append_batch(&gap).await.unwrap();
        }
    }
    let through = end(&log, "a", InvocationOutcome::Completed).await;
    while !log.prepare_transcript("a", through, 32).await.unwrap() {}
    let Page::Ready { chunks, .. } = log
        .history_text(
            "a",
            through,
            Position::Event { sequence: target },
            false,
            4,
            1,
            1024 * 1024,
        )
        .await
        .unwrap()
    else {
        panic!("prepared");
    };
    let cursor = maka_plugins::session::history::Cursor {
        field: Default::default(),
        sequence: chunks[0].sequence,
        offset: 0,
    };
    assert!(cursor.sequence < 10000);
    let baseline = log
        .history_text(
            "a",
            through,
            Position::After { cursor },
            false,
            32,
            1,
            1024 * 1024,
        )
        .await
        .unwrap();
    let budget = serde_json::to_vec(&baseline).unwrap().len();
    assert!(budget >= 1024);
    let page = log
        .history_text(
            "a",
            through,
            Position::After { cursor },
            false,
            32,
            8,
            budget,
        )
        .await
        .unwrap();
    assert_eq!(serde_json::to_vec(&page).unwrap().len(), budget);
    let Page::Ready {
        after: Some(cursor),
        chunks,
        ..
    } = page
    else {
        panic!("boundary retained");
    };
    assert_eq!(chunks.len(), 1);
    assert!(
        cursor.sequence < 10000,
        "oversized empty-row boundary was not consumed"
    );
    let mut cursor = cursor;
    let mut ended = false;
    for _ in 0..4 {
        let next = log
            .history_text(
                "a",
                through,
                Position::After { cursor },
                false,
                4,
                8,
                budget,
            )
            .await
            .unwrap();
        assert!(serde_json::to_vec(&next).unwrap().len() <= budget);
        let Page::Ready { chunks, after, .. } = next else {
            panic!("prepared");
        };
        assert!(chunks.is_empty());
        let Some(after) = after else {
            ended = true;
            break;
        };
        assert!(after.sequence > cursor.sequence);
        cursor = after;
    }
    assert!(ended, "empty projection windows reach the actual end");
    log.close().await.unwrap();
}

#[tokio::test]
async fn message_material_labels_and_time_survive_exact_paging_and_text_cache_upgrade() {
    use maka_plugins::session::history::{SourceField, SourcePosition};
    use maka_runtime::{
        attachment::{AttachmentKind, AttachmentRef, StorageRef},
        input::{DirectoryReference, MessageInput},
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("details.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    let attachment = |name: &str, bytes| AttachmentRef {
        kind: AttachmentKind::Other,
        name: name.into(),
        mime_type: "application/octet-stream".into(),
        bytes,
        storage_ref: StorageRef::WorkspaceFile {
            relative_path: "not-opened".into(),
        },
    };
    let directory = DirectoryReference {
        host_id: "original-host".into(),
        path: "src/中文".into(),
    };
    let mut complete: MessageInput = "hidden model text".into();
    complete.display_text = Some("Visible 中\0tail".into());
    complete.attachments = Some(vec![
        attachment("notes.txt", 512),
        attachment("report.txt", 2048),
        attachment("image.png", 1572864),
    ]);
    complete.directory_references = Some(vec![directory.clone()]);
    let mut folder: MessageInput = "hidden model text".into();
    folder.display_text = Some(String::new());
    folder.directory_references = Some(vec![directory]);
    let mut file: MessageInput = "".into();
    file.attachments = Some(vec![attachment("empty.txt", 0)]);
    let cases = [
        (
            "a",
            complete,
            "Visible 中\0tail\n↳ notes.txt · 512 B\n↳ report.txt · 2.0 KiB\n↳ image.png · 1.5 MiB\n▱ src/中文",
        ),
        ("b", folder, "▱ src/中文"),
        ("c", file, "↳ empty.txt · 0 B"),
    ];
    let timestamp = 1_700_000_000_000u64;
    let mut sources = Vec::new();
    for (session, content, expected) in cases {
        log.create_session(session, "create", &serde_json::json!({}), 1)
            .await
            .unwrap();
        let mut original = event(
            session,
            Fact::InvocationOpened {
                configuration: None,
                input: InvocationInput::Message {
                    source_messages: Vec::new(),
                    content,
                    request_fingerprint: None,
                },
            },
        )
        .event()
        .clone();
        original.recorded_at = std::time::UNIX_EPOCH + std::time::Duration::from_millis(timestamp);
        let through = log
            .append(&maka_runtime::event::EventWrite::plain(original).unwrap())
            .await
            .unwrap();
        assert!(log.prepare_transcript(session, through, 32).await.unwrap());
        let Page::Ready { chunks, .. } = log
            .history_text(session, through, Position::Start, false, 8192, 1, 64 * 1024)
            .await
            .unwrap()
        else {
            panic!("ready")
        };
        assert_eq!(chunks.len(), 1, "material-only prompts remain visible");
        assert_eq!(chunks[0].text, expected);
        assert_eq!(chunks[0].timestamp, timestamp);
        let sequence = chunks[0].sequence;
        for budget in [4, 19, 8192] {
            for reverse in [false, true] {
                let mut position = if reverse {
                    Position::Tail
                } else {
                    Position::Start
                };
                let mut actual = String::new();
                let mut pages = 0;
                loop {
                    pages += 1;
                    assert!(pages < expected.len() + 8, "bounded windows must progress");
                    let Page::Ready {
                        chunks,
                        before,
                        after,
                        ..
                    } = log
                        .history_text(session, through, position, false, budget, 1, 64 * 1024)
                        .await
                        .unwrap()
                    else {
                        panic!("ready")
                    };
                    assert!(chunks.len() <= 1);
                    for chunk in &chunks {
                        assert_eq!(chunk.timestamp, timestamp);
                        assert_eq!(chunk.total_bytes, expected.len() as u64);
                        let source = log
                            .history_source(
                                session,
                                through,
                                sequence,
                                SourceField::Body,
                                if reverse {
                                    SourcePosition::Before {
                                        offset: chunk.offset + chunk.text.len() as u64,
                                    }
                                } else {
                                    SourcePosition::After {
                                        offset: chunk.offset,
                                    }
                                },
                                budget,
                                64 * 1024,
                            )
                            .await
                            .unwrap();
                        assert_eq!(source.text, chunk.text);
                        assert_eq!(source.offset, chunk.offset);
                        assert_eq!(source.timestamp, chunk.timestamp);
                        if reverse {
                            actual.insert_str(0, &chunk.text);
                        } else {
                            actual.push_str(&chunk.text);
                        }
                    }
                    let Some(cursor) = (if reverse { before } else { after }) else {
                        break;
                    };
                    let next = if reverse {
                        Position::Before { cursor }
                    } else {
                        Position::After { cursor }
                    };
                    assert_ne!(next, position);
                    position = next;
                }
                assert_eq!(actual, expected);
            }
        }
        sources.push((session, through, sequence, expected));
    }
    assert!(
        !temp.path().join("not-opened").exists(),
        "display paths perform no I/O"
    );
    let before = serde_json::to_vec(&log.prefix(10, 65536).await.unwrap()).unwrap();
    log.close().await.unwrap();
    let db = Connection::open(&path).unwrap();
    let canonical = sources
        .iter()
        .map(|(session, ..)| rows(&db, session))
        .collect::<Vec<_>>();
    db.execute(
        "UPDATE transcript_text SET body = CAST('old derived text' AS BLOB)",
        [],
    )
    .unwrap();
    db.execute("DELETE FROM _sqlx_migrations WHERE version = 9", [])
        .unwrap();
    drop(db);
    let log = EventLog::open(&path).await.unwrap();
    let db = Connection::open(&path).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM transcript_text", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        sources
            .iter()
            .map(|(session, ..)| rows(&db, session))
            .collect::<Vec<_>>(),
        canonical
    );
    assert_eq!(
        serde_json::to_vec(&log.prefix(10, 65536).await.unwrap()).unwrap(),
        before
    );
    for (session, through, sequence, expected) in sources {
        let Page::Ready { chunks, .. } = log
            .history_text(session, through, Position::Start, false, 8192, 1, 64 * 1024)
            .await
            .unwrap()
        else {
            panic!("ready")
        };
        assert_eq!(chunks[0].text, expected);
        assert_eq!(chunks[0].sequence, sequence);
        assert_eq!(chunks[0].timestamp, timestamp);
    }
    drop(db);
    log.close().await.unwrap();
}

#[tokio::test]
async fn thinking_windows_preserve_field_order_bytes_and_explicit_body_only_reads() {
    use maka_plugins::{
        composition::Scope,
        session::history::{Cursor, HistoryField},
        storage::Namespace,
    };
    use maka_runtime::import::{Content, Record, Source};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("thinking.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    let owner = maka_event_log::sessions::PluginSession {
        session_id: "thinking".into(),
        creator: Namespace::new("history.fixture", Scope::Profile).unwrap(),
        fingerprint: "thinking-import".into(),
        managed: false,
        authority_session_id: None,
    };
    log.begin_session_import(
        &owner,
        &Source {
            adapter: "fixture".into(),
            session_id: "foreign".into(),
        },
        &serde_json::json!({}),
        1,
    )
    .await
    .unwrap();
    let thinking = "reason 中😀\0\r\n".repeat(101);
    let body = "answer é🦀\0tail".repeat(13);
    let records = [
        Content::User {
            text: "question".into(),
        },
        Content::Assistant {
            text: body.clone(),
            thinking: Some(thinking.clone()),
            model: None,
        },
        Content::Assistant {
            text: String::new(),
            thinking: Some("thinking only 🦀".into()),
            model: None,
        },
        Content::Assistant {
            text: "body with empty thinking".into(),
            thinking: Some(String::new()),
            model: None,
        },
        Content::Assistant {
            text: "body without thinking".into(),
            thinking: None,
            model: None,
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(index, content)| Record {
        source_message_id: format!("m{index}"),
        source_turn_id: "turn".into(),
        timestamp: Some(123),
        content,
    })
    .collect();
    log.append_session_import("thinking", 0, records)
        .await
        .unwrap();
    log.publish_session_import("thinking", 5).await.unwrap();
    let through = *log.subscribe_commits().borrow();
    while !log
        .prepare_transcript("thinking", through, 32)
        .await
        .unwrap()
    {}
    let expected = vec![
        "question".to_owned(),
        thinking.clone(),
        body.clone(),
        "thinking only 🦀".into(),
        "body with empty thinking".into(),
        "body without thinking".into(),
    ];
    let mut thinking_cursor = None;
    for include in [false, true] {
        for reverse in [false, true] {
            for budget in [4, 13, 8192] {
                let db = Connection::open(&path).unwrap();
                db.execute("DELETE FROM transcript_text", []).unwrap();
                drop(db);
                let mut position = if reverse {
                    Position::Tail
                } else {
                    Position::Start
                };
                let mut last = None;
                let mut collected = BTreeMap::<(u64, u8), (u64, String)>::new();
                let mut done = false;
                for _ in 0..2048 {
                    let page = log
                        .history_text("thinking", through, position, include, budget, 2, 64 * 1024)
                        .await
                        .unwrap();
                    assert!(serde_json::to_vec(&page).unwrap().len() <= 64 * 1024);
                    let Page::Ready {
                        chunks,
                        before,
                        after,
                        ..
                    } = page
                    else {
                        panic!("ready")
                    };
                    assert!(chunks.len() <= 2);
                    assert!(chunks.iter().map(|c| c.text.len()).sum::<usize>() <= budget);
                    let keys = chunks
                        .iter()
                        .map(|c| {
                            (
                                c.sequence,
                                if c.field == HistoryField::Thinking {
                                    0
                                } else {
                                    1
                                },
                            )
                        })
                        .collect::<Vec<_>>();
                    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
                    for (chunk, key) in chunks.into_iter().zip(keys) {
                        if chunk.field == HistoryField::Thinking {
                            assert!(include);
                            assert_eq!(chunk.role, Role::Assistant);
                            assert!(chunk.tool.is_none() && chunk.attachments.is_empty());
                            thinking_cursor.get_or_insert(Cursor {
                                sequence: chunk.sequence,
                                field: chunk.field,
                                offset: 0,
                            });
                        }
                        let entry = collected.entry(key).or_insert((
                            if reverse { chunk.total_bytes } else { 0 },
                            String::new(),
                        ));
                        if reverse {
                            assert_eq!(chunk.offset + chunk.text.len() as u64, entry.0);
                            entry.1.insert_str(0, &chunk.text);
                            entry.0 = chunk.offset;
                        } else {
                            assert_eq!(chunk.offset, entry.0);
                            entry.1.push_str(&chunk.text);
                            entry.0 += chunk.text.len() as u64;
                        }
                    }
                    let Some(cursor) = (if reverse { before } else { after }) else {
                        done = true;
                        break;
                    };
                    assert_ne!(last, Some(cursor), "every field boundary must advance");
                    last = Some(cursor);
                    position = if reverse {
                        Position::Before { cursor }
                    } else {
                        Position::After { cursor }
                    };
                }
                assert!(done);
                let actual = collected
                    .into_values()
                    .map(|(_, text)| text)
                    .collect::<Vec<_>>();
                let wanted = if include {
                    expected.clone()
                } else {
                    vec![
                        expected[0].clone(),
                        body.clone(),
                        expected[4].clone(),
                        expected[5].clone(),
                    ]
                };
                assert_eq!(
                    actual, wanted,
                    "include={include} reverse={reverse} budget={budget}"
                );
            }
        }
    }
    let cursor = thinking_cursor.unwrap();
    assert!(
        log.history_text(
            "thinking",
            through,
            Position::After { cursor },
            false,
            8192,
            2,
            64 * 1024
        )
        .await
        .is_err(),
        "thinking cursor cannot silently change a body-only read"
    );
    let Page::Ready { chunks, .. } = log
        .history_text(
            "thinking",
            through,
            Position::After { cursor },
            true,
            8192,
            2,
            64 * 1024,
        )
        .await
        .unwrap()
    else {
        panic!("ready")
    };
    assert_eq!(chunks[0].text, thinking);
    assert_eq!(chunks[1].text, body);
    let body_cursor = Cursor {
        field: HistoryField::Body,
        ..cursor
    };
    let Page::Ready { chunks, .. } = log
        .history_text(
            "thinking",
            through,
            Position::Before {
                cursor: body_cursor,
            },
            true,
            8192,
            1,
            64 * 1024,
        )
        .await
        .unwrap()
    else {
        panic!("ready")
    };
    assert_eq!(chunks[0].field, HistoryField::Thinking);
    assert_eq!(chunks[0].text, thinking);
    let source = log
        .history_source(
            "thinking",
            through,
            cursor.sequence,
            maka_plugins::session::history::SourceField::Thinking,
            maka_plugins::session::history::SourcePosition::Start,
            8192,
            64 * 1024,
        )
        .await
        .unwrap();
    assert_eq!(source.text, thinking);
    assert!(
        log.history_text(
            "thinking",
            through,
            Position::After {
                cursor: Cursor {
                    offset: 8,
                    ..cursor
                }
            },
            true,
            8192,
            2,
            64 * 1024
        )
        .await
        .is_err(),
        "Thinking offsets must not split UTF-8"
    );
    log.close().await.unwrap();
    // Reopen an actual pre-field cache shape. Migration may rebuild derived
    // text, but must preserve canonical identities, fences and both fields.
    let db = Connection::open(&path).unwrap();
    db.execute_batch("ALTER TABLE transcript_text RENAME TO field_cache;
        CREATE TABLE transcript_text AS SELECT session_id, sequence, timestamp, role, body FROM field_cache WHERE field = 'body';
        DROP TABLE field_cache;
        DELETE FROM _sqlx_migrations WHERE version = 10;").unwrap();
    drop(db);
    let reopened = EventLog::open(&path).await.unwrap();
    let Page::Ready {
        chunks,
        through: restored_fence,
        ..
    } = reopened
        .history_text(
            "thinking",
            through,
            Position::After { cursor },
            true,
            8192,
            2,
            64 * 1024,
        )
        .await
        .unwrap()
    else {
        panic!("ready")
    };
    assert_eq!(restored_fence, through);
    assert_eq!(chunks[0].text, thinking);
    assert_eq!(chunks[1].text, body);
    reopened.close().await.unwrap();
}
