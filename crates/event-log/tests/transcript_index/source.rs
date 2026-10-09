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
use maka_plugins::{
    composition::Scope,
    session::history::{Page, Position, SourceField, SourceFormat, SourcePosition, SourceRole},
    storage::Namespace,
};
use maka_runtime::import::{Content, Record, Source};

#[tokio::test]
async fn literal_tool_fields_reach_every_byte_without_summarizing_json_or_crossing_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let log = EventLog::open(&dir.path().join("source.sqlite"))
        .await
        .unwrap();
    let value = serde_json::json!({"number":42,"flag":false,"nothing":null,"array":[1,true,"中😀\0\r\n".repeat(700)]});
    let owner = maka_event_log::sessions::PluginSession {
        session_id: "source".into(),
        creator: Namespace::new("source.fixture", Scope::Profile).unwrap(),
        fingerprint: "source-import".into(),
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
            text: "Inspect tools".into(),
        },
        Content::ToolCall {
            call_id: "call".into(),
            name: "Count".into(),
            input: Some(value.clone()),
        },
        Content::ToolResult {
            call_id: "call".into(),
            output: value.clone(),
            is_error: false,
        },
        Content::ToolCall {
            call_id: "null".into(),
            name: "Null".into(),
            input: Some(Value::Null),
        },
        Content::ToolResult {
            call_id: "null".into(),
            output: Value::Null,
            is_error: false,
        },
        Content::ToolCall {
            call_id: "budget".into(),
            name: "name".repeat(1024),
            input: None,
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(i, content)| Record {
        source_message_id: format!("m{i}"),
        source_turn_id: "turn".into(),
        timestamp: Some(1),
        content,
    })
    .collect::<Vec<_>>();
    log.append_session_import("source", 0, records)
        .await
        .unwrap();
    log.publish_session_import("source", 6).await.unwrap();
    let through = *log.subscribe_commits().borrow();
    while !log.prepare_transcript("source", through, 32).await.unwrap() {}
    let Page::Ready { chunks, .. } = log
        .history_text(
            "source",
            through,
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
    // Long projected JSON strings can split one row into several text chunks.
    let sequences = Connection::open(dir.path().join("source.sqlite"))
        .unwrap()
        .prepare("SELECT sequence FROM transcript_rows WHERE session_id='source' AND json_extract(payload, '$.type') IN ('tool_call','tool_result') ORDER BY sequence")
        .unwrap()
        .query_map([], |row| {
            row.get::<_, i64>(0).map(|sequence| sequence as u64)
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(sequences.len(), 5);
    assert!(!chunks.is_empty());
    let expected = serde_json::to_string(&value).unwrap();
    for (sequence, field) in [
        (sequences[0], SourceField::Arguments),
        (sequences[1], SourceField::Result),
    ] {
        for budget in [4, 13, 8192] {
            for reverse in [false, true] {
                let mut position = if reverse {
                    SourcePosition::Tail
                } else {
                    SourcePosition::Start
                };
                let mut text = String::new();
                loop {
                    let source = log
                        .history_source(
                            "source",
                            through,
                            sequence,
                            field,
                            position,
                            budget,
                            64 * 1024,
                        )
                        .await
                        .unwrap();
                    assert_eq!(source.format, SourceFormat::Json);
                    assert_eq!(
                        source.role,
                        if field == SourceField::Arguments {
                            SourceRole::ToolCall
                        } else {
                            SourceRole::ToolResult
                        }
                    );
                    assert_eq!(
                        source.fields,
                        if field == SourceField::Arguments {
                            vec![
                                SourceField::ToolName,
                                SourceField::Body,
                                SourceField::Arguments,
                            ]
                        } else {
                            vec![SourceField::Body, SourceField::Result]
                        }
                    );
                    assert_eq!(source.total_bytes, expected.len() as u64);
                    assert!(source.text.len() <= budget);
                    assert!(!source.text.is_empty());
                    if reverse {
                        text.insert_str(0, &source.text);
                    } else {
                        text.push_str(&source.text);
                    }
                    let offset = if reverse {
                        source.offset
                    } else {
                        source.offset + source.text.len() as u64
                    };
                    if (reverse && offset == 0) || (!reverse && offset == source.total_bytes) {
                        break;
                    }
                    position = if reverse {
                        SourcePosition::Before { offset }
                    } else {
                        SourcePosition::After { offset }
                    };
                }
                assert_eq!(text, expected);
                assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), value);
            }
        }
        assert!(
            log.history_source(
                "source",
                0,
                sequence,
                field,
                SourcePosition::Start,
                8192,
                64 * 1024
            )
            .await
            .is_err()
        );
        assert!(
            log.history_source(
                "other",
                through,
                sequence,
                field,
                SourcePosition::Start,
                8192,
                64 * 1024
            )
            .await
            .is_err()
        );
        let wrong = if field == SourceField::Arguments {
            SourceField::Result
        } else {
            SourceField::Arguments
        };
        assert!(
            log.history_source(
                "source",
                through,
                sequence,
                wrong,
                SourcePosition::Start,
                8192,
                64 * 1024
            )
            .await
            .is_err()
        );
        let middle = expected
            .as_bytes()
            .iter()
            .position(|byte| *byte == 0xf0)
            .unwrap() as u64
            + 1;
        assert!(
            log.history_source(
                "source",
                through,
                sequence,
                field,
                SourcePosition::After { offset: middle },
                8192,
                64 * 1024
            )
            .await
            .is_err()
        );
        assert!(
            log.history_source(
                "source",
                through,
                sequence,
                field,
                SourcePosition::After {
                    offset: expected.len() as u64 + 1
                },
                8192,
                64 * 1024
            )
            .await
            .is_err()
        );
    }
    for (sequence, field) in [
        (sequences[2], SourceField::Arguments),
        (sequences[3], SourceField::Result),
    ] {
        let source = log
            .history_source(
                "source",
                through,
                sequence,
                field,
                SourcePosition::Start,
                8192,
                64 * 1024,
            )
            .await
            .unwrap();
        // Import's optional input treats JSON null as absent; the canonical
        // call therefore contains {}, while its result retains JSON null.
        let expected = if field == SourceField::Arguments {
            "{}"
        } else {
            "null"
        };
        assert_eq!(source.text, expected);
        assert_eq!(source.total_bytes, expected.len() as u64);
        let eof = log
            .history_source(
                "source",
                through,
                sequence,
                field,
                SourcePosition::After {
                    offset: expected.len() as u64,
                },
                8192,
                64 * 1024,
            )
            .await
            .unwrap();
        assert_eq!(eof.offset, expected.len() as u64);
        assert!(eof.text.is_empty());
    }
    assert!(
        log.history_source(
            "source",
            through,
            sequences[4],
            SourceField::Arguments,
            SourcePosition::Start,
            8192,
            1024
        )
        .await
        .is_err()
    );
    let full = log
        .history_source(
            "source",
            through,
            sequences[4],
            SourceField::Arguments,
            SourcePosition::Start,
            8192,
            64 * 1024,
        )
        .await
        .unwrap();
    assert_eq!(full.text, "{}");
    log.close().await.unwrap();
}

#[tokio::test]
async fn failure_search_reads_original_diagnostic_fields_without_a_body_row() {
    let dir = tempfile::tempdir().unwrap();
    let log = EventLog::open(&dir.path().join("failure.sqlite"))
        .await
        .unwrap();
    log.create_session("failed", "create-failed", &serde_json::json!({}), 1)
        .await
        .unwrap();
    log.append(&event("failed", opening())).await.unwrap();
    let message = format!("failure-hit {}", "中😀\n".repeat(2000));
    let through = end(
        &log,
        "failed",
        InvocationOutcome::Failed {
            class: "failure-class-hit".into(),
            message: Some(message.clone()),
        },
    )
    .await;
    assert!(log.prepare_transcript("failed", through, 32).await.unwrap());
    for (query, field, expected) in [
        // The original transcript intentionally limits displayed failure messages
        // to 256 UTF-8 bytes; source reads preserve that same searchable field.
        (
            "failure-hit",
            SourceField::FailureMessage,
            message[..message.floor_char_boundary(256)].to_owned(),
        ),
        (
            "failure-class-hit",
            SourceField::ErrorClass,
            "failure-class-hit".into(),
        ),
    ] {
        let result = log
            .search_transcript(
                "failed",
                maka_event_log::transcript::search::TranscriptSearch {
                    through: maka_presentation::watermark(through).unwrap(),
                    after: 0,
                    query: query.into(),
                    include_internal: false,
                    max_matches: 16,
                },
            )
            .await
            .unwrap();
        assert_eq!(result.matches.len(), 1);
        let hit = &result.matches[0];
        assert_eq!(hit.field, field);
        let mut collected = String::new();
        let mut position = SourcePosition::Start;
        for _ in 0..16 {
            let page = log
                .history_source(
                    "failed",
                    through,
                    hit.sequence,
                    field,
                    position,
                    31,
                    64 * 1024,
                )
                .await
                .unwrap();
            assert_eq!(page.sequence, hit.sequence);
            assert_eq!(page.field, field);
            assert_eq!(page.role, SourceRole::Failure);
            assert_eq!(
                page.fields,
                [SourceField::ErrorClass, SourceField::FailureMessage]
            );
            assert_eq!(page.format, SourceFormat::Text);
            assert!(page.tool.is_none());
            collected.push_str(&page.text);
            let offset = page.offset + page.text.len() as u64;
            if offset == page.total_bytes {
                break;
            }
            position = SourcePosition::After { offset };
        }
        assert_eq!(collected, expected);
        assert!(
            log.history_source(
                "failed",
                through,
                hit.sequence,
                SourceField::Body,
                SourcePosition::Start,
                8192,
                64 * 1024
            )
            .await
            .is_err()
        );
        assert!(
            log.history_source(
                "failed",
                through - 1,
                hit.sequence,
                field,
                SourcePosition::Start,
                8192,
                64 * 1024
            )
            .await
            .is_err()
        );
    }
    log.shutdown().await.unwrap();
}
