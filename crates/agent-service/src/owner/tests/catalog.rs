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
use maka_runtime::interaction::{ClosureReason, InteractionOutcome};
use maka_runtime_host::application::Subscription;

async fn next(stream: &mut Subscription) -> Value {
    tokio::time::timeout(Duration::from_secs(5), stream.recv())
        .await
        .expect("catalog update")
        .expect("live subscription")
}

#[tokio::test]
async fn catalog_observes_metadata_pending_and_removal_at_its_original_location() {
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 2, None).await;
    let mut bootstrap = fixture
        .app
        .client()
        .subscribe(
            fixture.binding(2).await,
            "catalog".into(),
            json!({"filter":"all","position":{"kind":"start"}}),
            scope(&fixture.issuer, true),
            Attachments::new().into(),
        )
        .await
        .unwrap();
    assert_eq!(next(&mut bootstrap).await["sessions"], json!([]));
    let mut ids = Vec::new();
    for title in ["First", "Second", "Third"] {
        ids.push(fixture.create(title).await);
        assert!(
            !next(&mut bootstrap).await["sessions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    bootstrap.completion().await;
    ids.sort();
    let mut stream = fixture
        .app
        .client()
        .subscribe(
            fixture.binding(2).await,
            "catalog".into(),
            json!({"filter":"all","position":{"kind":"at","cursor":ids[1]}}),
            scope(&fixture.issuer, true),
            Attachments::new().into(),
        )
        .await
        .unwrap();
    let first = next(&mut stream).await;
    assert_eq!(first["sessions"][0]["id"], ids[1]);
    assert_eq!(first["sessions"].as_array().unwrap().len(), 2);
    assert_eq!(first["needsYouCount"], 0);
    assert_eq!(first["before"], ids[1]);

    let changed = fixture
        .mutation(
            2,
            "metadata",
            json!({
                "sessionId":ids[1],"expectedRevision":first["sessions"][0]["revision"],
                "title":"Updated while reading","labels":[],
            }),
            Uuid::new_v4(),
        )
        .await
        .unwrap();
    assert_eq!(changed["kind"], "committed");
    let renamed = next(&mut stream).await;
    assert_eq!(renamed["sessions"][0]["title"], "Updated while reading");
    fixture
        .store
        .set_session_archived::<SessionConfiguration>(&ids[1], true, 3)
        .await
        .unwrap();
    let archived = next(&mut stream).await;
    assert_eq!(archived["sessions"][0]["archived"], true);

    // Background pending sessions outside this page contribute to the exact count.
    fixture
        .store
        .establish_interaction(&catalog_interaction(&ids[0], "background"))
        .await
        .unwrap();
    let pending = next(&mut stream).await;
    assert_eq!(pending["needsYouCount"], 1);
    assert_eq!(pending["sessions"][0]["id"], ids[1]);
    fixture
        .store
        .establish_interaction(&catalog_interaction(&ids[0], "background-second"))
        .await
        .unwrap();
    assert_eq!(
        next(&mut stream).await["needsYouCount"],
        1,
        "count sessions, not requests"
    );

    fixture
        .store
        .create_session(
            "foreign",
            "foreign",
            &json!({"invalidPrivateConfiguration":true}),
            1,
        )
        .await
        .unwrap();
    assert_eq!(next(&mut stream).await["sessions"], pending["sessions"]);
    fixture
        .store
        .establish_interaction(&catalog_interaction("foreign", "foreign-request"))
        .await
        .unwrap();
    let private = next(&mut stream).await;
    assert_eq!(private["needsYouCount"], 1);
    assert_eq!(private["sessions"], pending["sessions"]);

    for request in ["background", "background-second"] {
        fixture
            .store
            .commit_interaction_outcome(
                request,
                InteractionOutcome::Closure {
                    reason: ClosureReason::ProducerCancelled,
                    committed_at: 20,
                },
            )
            .await
            .unwrap();
        let updated = next(&mut stream).await;
        assert_eq!(
            updated["needsYouCount"],
            if request == "background" { 1 } else { 0 }
        );
    }
    for id in &ids[1..] {
        let record = fixture
            .store
            .get_session::<SessionConfiguration>(id)
            .await
            .unwrap()
            .unwrap();
        fixture
            .store
            .begin_session_removal(id, record.revision)
            .await
            .unwrap();
        let updated = next(&mut stream).await;
        assert!(
            updated["sessions"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["id"] != *id)
        );
        if id == &ids[2] {
            assert_eq!(
                updated["sessions"][0]["id"], ids[0],
                "empty tail falls back in the same owner scope"
            );
            assert!(updated["before"].is_null());
            assert!(updated["after"].is_null());
            assert_eq!(
                updated["needsYouCount"], 0,
                "foreign pending session stays excluded"
            );
        }
    }
    stream.cancel();
    tokio::time::timeout(Duration::from_secs(5), stream.completion())
        .await
        .unwrap();
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}
