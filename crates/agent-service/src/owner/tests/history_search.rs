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
use maka_plugins::{composition::Scope as StorageScope, storage::Namespace};
use maka_runtime::import::{Content, Record, Source};

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
