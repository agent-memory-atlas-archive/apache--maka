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
use maka_event_log::sessions::PluginSession;
use maka_plugins::storage::Namespace;
use maka_runtime::import::{Content, Record, Source};
use maka_runtime::scope::Scope as StorageScope;

#[tokio::test]
async fn history_search_preserves_empty_batches_and_exact_cursors_with_original_authority() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let fixture = Fixture::new("http://127.0.0.1:9/v1", 1, None).await;
        let seed = fixture.create("Search").await;
        let configuration = fixture.owner.session(&seed).await.unwrap().configuration;
        let session = format!("{}{}", fixture.owner.prefix, Uuid::new_v4().simple());
        let plugin = PluginSession {
            session_id: session.clone(),
            creator: Namespace::new("search.fixture", StorageScope::Profile).unwrap(),
            fingerprint: "search-import".into(),
            managed: false,
            authority_session_id: None,
        };
        fixture.store.begin_session_import(&plugin, &Source {
            adapter: "fixture".into(), session_id: "source".into(),
        }, &configuration, 1).await.unwrap();
        for start in (0..112).step_by(8) {
            let records = (start..start + 8).map(|i| Record {
                source_message_id: format!("m{i}"),
                source_turn_id: format!("turn-{i}"),
                timestamp: Some(i + 1),
                content: Content::User { text: if i < 72 { format!("Earlier {i}") } else {
                    format!("中😀 match {i} {}", "界".repeat(300))
                } },
            }).collect();
            fixture.store.append_session_import(&session, start, records).await.unwrap();
        }
        fixture.store.publish_session_import(&session, 112).await.unwrap();
        let through = *fixture.store.subscribe_commits().borrow();
        let request = json!({"sessionId":session,"through":through,"after":0,"query":"中😀","includeInternal":false});
        let mut input = request.clone();
        let mut matches = Vec::new();
        let mut empty_continuation = false;
        let mut preparing = false;
        let mut ended = false;
        for _ in 0..32 {
            let page = fixture.query("search", input.clone()).await.unwrap();
            assert_eq!(page["through"], through);
            assert!(serde_json::to_vec(&page).unwrap().len() <= 16 * 1024);
            if page["kind"] == "preparing" { preparing = true; continue; }
            let found = page["matches"].as_array().unwrap();
            assert!(found.len() <= 16);
            for item in found {
                assert!(item["cursor"]["sequence"].as_u64().unwrap() > input["after"].as_u64().unwrap());
                assert_eq!(item["cursor"]["offset"], 0);
                let preview = item["preview"].as_str().unwrap();
                assert!(preview.len() <= 384 && preview.contains("中😀"));
                matches.push(item.clone());
            }
            let Some(after) = page["nextAfter"].as_u64() else { ended = true; break; };
            empty_continuation |= found.is_empty();
            assert!(after > input["after"].as_u64().unwrap());
            assert!(after <= maka_presentation::watermark(through).unwrap());
            input["after"] = json!(after);
        }
        assert!(preparing && empty_continuation && ended);
        assert_eq!(matches.len(), 40);
        assert!(matches.windows(2).all(|pair| pair[0]["cursor"]["sequence"].as_u64() < pair[1]["cursor"]["sequence"].as_u64()));
        let located = fixture.query("history", json!({"sessionId":session,"through":through,
            "position":{"kind":"after","cursor":matches[17]["cursor"]}})).await.unwrap();
        assert_eq!(located["chunks"][0]["sequence"], matches[17]["cursor"]["sequence"]);
        assert!(located["chunks"][0]["text"].as_str().unwrap().starts_with("中😀 match 89 "));
        for (key, value) in [
            ("sessionId", json!("foreign_session")),
            ("through", json!(through + 1)),
            ("after", json!(maka_presentation::watermark(through).unwrap() + 1)),
            ("query", json!("")),
            ("query", json!("界".repeat(171))),
        ] {
            let mut invalid = request.clone(); invalid[key] = value;
            assert!(fixture.query("search", invalid).await.is_err(), "{key}");
        }
        let denied = fixture.app.client().start(
            fixture.binding(2).await, "search".into(), request,
            scope(&fixture.issuer, false), Attachments::new().into(),
        ).await;
        assert!(denied.is_err());
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn search_hits_read_the_exact_field_when_history_body_is_absent_or_different() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let fixture = Fixture::new("http://127.0.0.1:9/v1", 1, None).await;
        let seed = fixture.create("Fields").await;
        let configuration = fixture.owner.session(&seed).await.unwrap().configuration;
        let session = format!("{}{}", fixture.owner.prefix, Uuid::new_v4().simple());
        let owner = PluginSession {
            session_id: session.clone(),
            creator: Namespace::new("source.fixture", StorageScope::Profile).unwrap(),
            fingerprint: "search-source-fields".into(), managed: false, authority_session_id: None,
        };
        fixture.store.begin_session_import(&owner, &Source {
            adapter: "fixture".into(), session_id: "original".into(),
        }, &configuration, 1).await.unwrap();
        let body = format!("user-body-hit {}", "中😀\0\n".repeat(1400));
        let thought = format!("thinking-hit {}", "中😀\0\n".repeat(1400));
        let args = json!({"argument-key-hit": "原始😀\"\n".repeat(1400)});
        let result = json!({"result-key-hit": "原始输出😀".repeat(1400)});
        let records = [
            Content::User { text: body.clone() },
            Content::Assistant { text: String::new(), model: None, thinking: Some(thought.clone()) },
            Content::Assistant { text: "body-hit-after-empty-thought".into(), model: None, thinking: Some("other-thinking-hit".into()) },
            Content::ToolCall { call_id: "call".into(), name: "tool-name-hit".into(), input: Some(args.clone()) },
            Content::ToolResult { call_id: "call".into(), output: result.clone(), is_error: false },
        ].into_iter().enumerate().map(|(i, content)| Record {
            source_message_id: format!("record-{i}"), source_turn_id: "turn".into(), timestamp: Some(1), content,
        }).collect();
        fixture.store.append_session_import(&session, 0, records).await.unwrap();
        fixture.store.publish_session_import(&session, 5).await.unwrap();
        let through = *fixture.store.subscribe_commits().borrow();
        for include in [false, true] {
            let mut position = json!({"kind":"start"});
            let mut thinking = String::new();
            let mut finished = false;
            for _ in 0..64 {
                let page = fixture.query("history", json!({"sessionId":session,"through":through,"position":position,"includeThinking":include})).await.unwrap();
                assert!(serde_json::to_vec(&page).unwrap().len() <= 64 * 1024);
                if page["kind"] == "preparing" { continue; }
                for chunk in page["chunks"].as_array().unwrap() {
                    if chunk["field"] == "thinking" {
                        assert!(include);
                        assert_eq!(chunk["role"], "assistant");
                        thinking.push_str(chunk["text"].as_str().unwrap());
                    } else { assert_eq!(chunk["field"], "body"); }
                }
                if page["after"].is_null() { finished = true; break; }
                position = json!({"kind":"after","cursor":page["after"]});
            }
            assert!(finished);
            assert_eq!(thinking, if include { thought.clone() + "other-thinking-hit" } else { String::new() });
        }
        for (query, field, expected, is_tool) in [
            ("user-body-hit", "body", body, false),
            ("thinking-hit 中", "thinking", thought, false),
            ("body-hit-after", "body", "body-hit-after-empty-thought".to_owned(), false),
            ("other-thinking-hit", "thinking", "other-thinking-hit".to_owned(), false),
            ("tool-name-hit", "tool_name", "tool-name-hit".to_owned(), true),
            ("argument-key-hit", "arguments", serde_json::to_string(&args).unwrap(), true),
            ("result-key-hit", "result", serde_json::to_string(&result).unwrap(), true),
        ] {
            let mut hit = None;
            for _ in 0..8 {
                let page = fixture.query("search", json!({"sessionId":session,"through":through,"after":0,"query":query,"includeInternal":false})).await.unwrap();
                if page["kind"] == "preparing" { continue; }
                assert_eq!(page["matches"].as_array().unwrap().len(), 1, "{query}");
                hit = Some(page["matches"][0].clone()); break;
            }
            let hit = hit.unwrap();
            assert_eq!(hit["field"], field);
            let sequence = hit["cursor"]["sequence"].as_u64().unwrap();
            for reverse in [false, true] {
                let mut position = json!({"kind":if reverse {"tail"} else {"start"}});
                let mut text = String::new();
                let mut finished = false;
                for _ in 0..16 {
                    let page = fixture.query("source", json!({"sessionId":session,"through":through,
                        "source":{"kind":"sequence","sequence":sequence},"field":field,"position":position})).await.unwrap();
                    assert_eq!(page["kind"], "ready");
                    assert_eq!(page["through"], through);
                    let source = &page["source"];
                    let role = if query == "user-body-hit" { "user" } else if is_tool {
                        if field == "result" { "tool_result" } else { "tool_call" }
                    } else { "assistant" };
                    let fields = match role {
                        "user" => json!(["body"]),
                        "assistant" => json!(["thinking", "body"]),
                        "tool_call" => json!(["tool_name", "body", "arguments"]),
                        _ => json!(["body", "result"]),
                    };
                    assert_eq!(source["role"], role);
                    assert_eq!(source["timestamp"], 1);
                    assert_eq!(source["fields"], fields);
                    if !reverse && source["offset"] == 0 {
                        for other in fields.as_array().unwrap() {
                            let related = fixture.query("source", json!({"sessionId":session,"through":through,
                                "source":{"kind":"sequence","sequence":sequence},"field":other,"position":{"kind":"start"}})).await.unwrap();
                            assert_eq!(related["source"]["sequence"], sequence);
                            assert_eq!(related["source"]["role"], role);
                            assert_eq!(related["source"]["fields"], fields);
                            assert_eq!(related["source"]["timestamp"], source["timestamp"]);
                            if query == "thinking-hit 中" && other == "body" { assert_eq!(related["source"]["text"], ""); }
                        }
                    }
                    assert_eq!(source["sequence"], sequence);
                    assert_eq!(source["field"], field);
                    assert_eq!(source["totalBytes"], expected.len());
                    assert_eq!(source["tool"].is_object(), is_tool);
                    let part = source["text"].as_str().unwrap();
                    assert!(part.len() <= 8192 && serde_json::to_vec(&page).unwrap().len() <= 64 * 1024);
                    if reverse { text.insert_str(0, part); } else { text.push_str(part); }
                    let offset = source["offset"].as_u64().unwrap() + if reverse {0} else {part.len() as u64};
                    if offset == if reverse {0} else {expected.len() as u64} { finished = true; break; }
                    position = json!({"kind":if reverse {"before"} else {"after"},"offset":offset});
                }
                assert!(finished);
                assert_eq!(text, expected);
            }
            if is_tool {
                let body = fixture.query("source", json!({"sessionId":session,"through":through,
                    "source":{"kind":"sequence","sequence":sequence},"field":"body","position":{"kind":"start"}})).await.unwrap();
                let page = fixture.query("history", json!({"sessionId":session,"through":through,
                    "position":{"kind":"after","cursor":hit["cursor"]}})).await.unwrap();
                assert_eq!(body["source"]["field"], "body");
                assert_eq!(body["source"]["format"], "text");
                assert_eq!(body["source"]["sequence"], sequence);
                if body["source"]["totalBytes"] == 0 {
                    assert_eq!(body["source"]["text"], "");
                    assert_eq!(body["source"]["tool"]["call"]["sequence"], body["source"]["sequence"]);
                    assert_eq!(body["source"]["tool"]["call"]["name"], "tool-name-hit");
                    // A zero offset is already the end of an empty projected body.
                    // Exact source must retain the row even when history advances.
                    assert_ne!(page["chunks"][0]["sequence"], sequence);
                } else {
                    for field in ["sequence", "timestamp", "offset", "totalBytes", "text", "tool"] {
                        assert_eq!(body["source"][field], page["chunks"][0][field], "tool body {field}");
                    }
                }
            }
            if query == "thinking-hit 中" {
                let page = fixture.query("history", json!({"sessionId":session,"through":through,"position":{"kind":"after","cursor":hit["cursor"]}})).await.unwrap();
                assert_ne!(page["chunks"][0]["sequence"], sequence, "body projection must not be mistaken for the search source");
                for (key, value) in [("sessionId", json!("foreign")), ("through", json!(0)), ("field", json!("arguments"))] {
                    let mut input = json!({"sessionId":session,"through":through,"source":{"kind":"sequence","sequence":sequence},"field":field,"position":{"kind":"start"}});
                    input[key] = value;
                    assert!(fixture.query("source", input).await.is_err(), "{key}");
                }
                let denied = fixture.app.client().start(fixture.binding(2).await, "source".into(),
                    json!({"sessionId":session,"through":through,"source":{"kind":"sequence","sequence":sequence},"field":field,"position":{"kind":"start"}}),
                    scope(&fixture.issuer, false), Attachments::new().into()).await;
                assert!(denied.is_err());
            }
        }
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}
