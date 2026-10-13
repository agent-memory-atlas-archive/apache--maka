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
use maka_agent_engine::ModelSource;

pub(super) async fn edit(
    fixture: &Fixture,
    change: impl FnOnce(&mut ConnectionCatalogEntryUpdate),
) {
    let row = fixture
        .config
        .catalog()
        .await
        .unwrap()
        .connections
        .remove(0);
    let mut changes = ConnectionCatalogEntryUpdate {
        name: Some(row.name),
        configuration: Some(row.configuration),
        enabled: Some(row.enabled),
        enabled_model_ids: Some(row.enabled_model_ids),
        model_overrides: Patch::Keep,
        request_body_overlay: Patch::Keep,
    };
    change(&mut changes);
    assert!(matches!(
        fixture
            .config
            .update_connection(UpdateCatalogConnectionInput {
                expected: ConnectionVersionBasis {
                    connection_id: row.connection_id,
                    revision: row.revision
                },
                changes,
            })
            .await
            .unwrap(),
        CatalogMutationResult::Committed { .. }
    ));
}
async fn settings(fixture: &Fixture) -> Binding {
    let client = fixture.app.client();
    client
        .bind(client.activation(&id(2)).unwrap(), "settings".into())
        .await
        .unwrap()
        .remove(0)
}
async fn models(fixture: &Fixture, input: Value) -> Result<Value, String> {
    fixture
        .app
        .client()
        .start(
            settings(fixture).await,
            "models".into(),
            input,
            scope(&fixture.issuer, true),
            Attachments::new().into(),
        )
        .await?
        .result()
        .await
}

#[tokio::test]
async fn explicit_models_switch_in_one_session_and_stale_selection_has_no_admission() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for expected in ["fixture-model", "second-model"] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let body = request(&mut socket).await;
                assert_eq!(body["model"], expected);
                response(&mut socket).await;
            }
        });
        let fixture = Fixture::new(&base, 1, None).await;
        let session = fixture.create("Per-message model choice").await;
        let stale = fixture.selection().await;
        edit(&fixture, |changes| {
            changes
                .enabled_model_ids
                .as_mut()
                .unwrap()
                .push("second-model".into())
        })
        .await;
        assert!(
            fixture
                .mutation(
                    2,
                    "send",
                    json!({"sessionId":session,"message":"Stale","model":stale}),
                    Uuid::new_v4()
                )
                .await
                .is_err()
        );
        let observed = fixture
            .query("observe", json!({"sessionId":session}))
            .await
            .unwrap();
        assert!(observed["run"].is_null());
        let page = fixture
            .query(
                "history",
                json!({"sessionId":session,"position":{"kind":"start"}}),
            )
            .await
            .unwrap();
        assert!(page["chunks"].as_array().unwrap().is_empty());
        for model in ["fixture-model", "second-model"] {
            let mut selected = fixture.selection().await;
            selected.model_id = model.into();
            let started = fixture
                .mutation(
                    2,
                    "send",
                    json!({"sessionId":session,"message":"Chosen model","model":selected}),
                    Uuid::new_v4(),
                )
                .await
                .unwrap();
            assert_eq!(started["sessionId"], session);
            fixture.drained().await;
        }
        server.await.unwrap();
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn captured_execution_ignores_catalog_labels_and_other_models_but_rejects_execution_changes()
{
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    let session = fixture.create("Original execution basis").await;
    let (mut source, _) = crate::model::Source::select(
        fixture.config.clone(),
        fixture.owner.credentials.clone(),
        fixture.selection().await,
        None,
        session.clone(),
        fixture.owner.revision.clone(),
    )
    .await
    .unwrap();
    let persisted = serde_json::to_string(&source.accepted).unwrap();
    assert!(!persisted.contains("private-fixture-key"));
    source = crate::model::Source::restore(
        fixture.config.clone(),
        fixture.owner.credentials.clone(),
        serde_json::from_str(&persisted).unwrap(),
        session,
        fixture.owner.revision.clone(),
    )
    .unwrap();
    edit(&fixture, |changes| {
        changes.name = Some("A different label".into());
        changes
            .enabled_model_ids
            .as_mut()
            .unwrap()
            .push("other-model".into());
        changes.model_overrides = Patch::Set(BTreeMap::from([
            (
                "fixture-model".into(),
                ModelOverride {
                    display_name: Some("Visible name".into()),
                    description: Some("Updated description".into()),
                    knowledge_cutoff: Some("2026".into()),
                    ..Default::default()
                },
            ),
            (
                "other-model".into(),
                ModelOverride {
                    max_output_tokens: Some(900),
                    ..Default::default()
                },
            ),
        ]));
    })
    .await;
    let next = source.capture().await.unwrap();
    assert_eq!(next.provider.model, "fixture-model");
    assert_eq!(next.provider.base_url, "http://127.0.0.1:1/v1");
    let credential = fixture
        .config
        .observe_model(source.accepted.binding.clone())
        .await
        .unwrap()
        .unwrap()
        .credential
        .unwrap();
    assert!(
        credential
            .commit_refresh(
                maka_agent_facts::provider::Credential {
                    secret: "refreshed-fixture-key".into(),
                    refresh_at: None
                },
                2
            )
            .await
            .unwrap()
            .is_some()
    );
    let refreshed = source.capture().await.unwrap();
    assert!(
        matches!(refreshed.provider.auth, maka_model::ProviderAuth::ApiKey(ref key) if key == "refreshed-fixture-key")
    );
    edit(&fixture, |changes| {
        changes.model_overrides = Patch::Set(BTreeMap::from([(
            "fixture-model".into(),
            ModelOverride {
                max_output_tokens: Some(64),
                ..Default::default()
            },
        )]))
    })
    .await;
    assert!(source.capture().await.is_err());
    let (current, _) = crate::model::Source::select(
        fixture.config.clone(),
        fixture.owner.credentials.clone(),
        fixture.selection().await,
        None,
        "current".into(),
        fixture.owner.revision.clone(),
    )
    .await
    .unwrap();
    edit(&fixture, |changes| {
        changes.configuration = Some(json!({"baseUrl":"http://127.0.0.1:2/v1"}))
    })
    .await;
    assert!(current.capture().await.is_err());
    let (current, _) = crate::model::Source::select(
        fixture.config.clone(),
        fixture.owner.credentials.clone(),
        fixture.selection().await,
        None,
        "current".into(),
        fixture.owner.revision.clone(),
    )
    .await
    .unwrap();
    edit(&fixture, |changes| {
        changes.enabled_model_ids = Some(vec!["other-model".into()])
    })
    .await;
    assert!(current.capture().await.is_err());
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}

#[tokio::test]
async fn ordinary_settings_pages_keep_root_revision_and_exact_default_cas_without_secrets() {
    let first = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    let second = Fixture::new("http://127.0.0.1:2/v1", 1, None).await;
    edit(&first, |changes| {
        changes.enabled_model_ids = Some((0..20).map(|at| format!("model-{at:02}")).collect())
    })
    .await;
    let page = models(&first, json!({"kind":"start"})).await.unwrap();
    assert_eq!(page["defaultModel"], json!({"kind":"none"}));
    assert_eq!(page["choices"].as_array().unwrap().len(), 16);
    assert!(serde_json::to_vec(&page).unwrap().len() < 48 * 1024);
    assert!(!page.to_string().contains("private-fixture-key"));
    assert!(!page.to_string().contains("baseUrl"));
    let continuation = json!({"kind":"continue","revision":page["revision"],"cursor":page["next"]});
    let last = models(&first, continuation.clone()).await.unwrap();
    assert_eq!(last["choices"].as_array().unwrap().len(), 4);
    assert!(last["next"].is_null());
    let other = models(&second, json!({"kind":"start"})).await.unwrap();
    assert_eq!(other["choices"].as_array().unwrap().len(), 1);
    assert_ne!(
        other["choices"][0]["selection"]["connectionId"],
        page["choices"][0]["selection"]["connectionId"]
    );
    let binding = settings(&first).await;
    let operation = Uuid::new_v4();
    let input = json!({"expectedCatalogRevision":page["revision"],"target":{"connectionId":page["choices"][0]["selection"]["connectionId"],"modelId":"model-19"}});
    let client = first.app.client();
    assert!(
        client
            .prepare_mutation(
                binding.clone(),
                "setDefault".into(),
                input.clone(),
                scope(&first.issuer, false),
                operation,
                Attachments::new().into()
            )
            .await
            .is_err()
    );
    assert!(
        client
            .prepare_mutation(
                binding.clone(),
                "setDefault".into(),
                json!({"expectedCatalogRevision":page["revision"]}),
                scope(&first.issuer, true),
                operation,
                Attachments::new().into()
            )
            .await
            .is_err()
    );
    let prepared = client
        .prepare_mutation(
            binding,
            "setDefault".into(),
            input,
            scope(&first.issuer, true),
            operation,
            Attachments::new().into(),
        )
        .await
        .unwrap();
    let recorded = first
        .store
        .record_application_intent(
            prepared.reference().source.clone(),
            prepared.consumer().instance().clone(),
            prepared.reference().clone(),
            serde_json::Value::Null,
        )
        .await
        .unwrap();
    let reply = client
        .mutate(prepared, recorded)
        .await
        .unwrap()
        .result()
        .await;
    assert!(
        matches!(reply.receipt, Receipt::Recorded { outcome: OperationOutcome::Returned { ref value }, .. } if value["kind"] == "committed")
    );
    let changed = models(&first, continuation).await.unwrap();
    assert_eq!(changed["kind"], "changed");
    let new = models(&first, json!({"kind":"start"})).await.unwrap();
    assert_eq!(new["defaultModel"]["kind"], "ready");
    assert_eq!(
        new["defaultModel"]["choice"]["selection"]["modelId"],
        "model-19"
    );
    assert_eq!(
        new["defaultModel"]["choice"]["selection"]["revision"],
        new["choices"][0]["selection"]["revision"]
    );
    assert!(
        !new["choices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|choice| choice["selection"]["modelId"] == "model-19")
    );
    assert!(new["choices"][0].get("isDefault").is_none());
    assert!(models(&first, json!({"kind":"continue","revision":new["revision"],"cursor":{"connection":0,"model":4000}})).await.is_err());
    // A valid Root default can belong to a provider outside this capability's
    // selectable catalog. It must remain visible as unavailable, not disappear
    // or silently become the first available choice.
    let CatalogMutationResult::Committed {
        catalog_revision,
        connection: Some(unsupported),
    } = first.config.create_connection(serde_json::from_value(json!({
        "expectedCatalogRevision":new["revision"],
        "connection":{
            "slug":"other-provider","name":"Other provider",
            "provider":{"packageId":"example.models","entryId":"models","scope":"profile","name":"custom"},
            "configuration":{},"enabled":true,"enabledModelIds":["other-model"]
        }
    })).unwrap()).await.unwrap() else { panic!("other provider commit"); };
    let target = ConnectionTarget {
        connection_id: unsupported.connection_id,
        model_id: "other-model".into(),
    };
    assert!(matches!(
        first
            .config
            .set_default_target(SetDefaultConnectionTargetInput {
                expected_catalog_revision: catalog_revision,
                target: Some(target.clone()),
            })
            .await
            .unwrap(),
        CatalogMutationResult::Committed {
            connection: None,
            ..
        }
    ));
    let unavailable = models(&first, json!({"kind":"start"})).await.unwrap();
    assert_eq!(
        unavailable["defaultModel"],
        json!({"kind":"unavailable","target":target})
    );
    assert_eq!(unavailable["choices"].as_array().unwrap().len(), 16);
    assert_eq!(
        first.config.catalog().await.unwrap().default_target,
        Some(target)
    );
    first.app.shutdown().await.unwrap();
    second.app.shutdown().await.unwrap();
    first.config.shutdown().await.unwrap();
    second.config.shutdown().await.unwrap();
}

#[tokio::test]
async fn model_search_continues_its_query_across_bounded_scan_windows_and_catalog_fences() {
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    edit(&fixture, |changes| {
        changes.name = Some("Research Account".into());
        changes.enabled_model_ids = Some((0..300).map(|at| format!("model-{at:03}")).collect());
    })
    .await;
    let browsing = models(&fixture, json!({"kind":"start"})).await.unwrap();
    assert_eq!(browsing["choices"].as_array().unwrap().len(), 16);
    assert_eq!(browsing["next"]["query"], "");
    assert!(
        models(&fixture, json!({"kind":"start","query":null}))
            .await
            .is_err()
    );
    assert!(
        models(&fixture, json!({"kind":"start","query":"界".repeat(257)}))
            .await
            .is_err()
    );
    let first = models(
        &fixture,
        json!({"kind":"start","query":"RESEARCH model-299"}),
    )
    .await
    .unwrap();
    assert!(first["choices"].as_array().unwrap().is_empty());
    assert_eq!(first["next"]["query"], "RESEARCH model-299");
    assert_eq!(first["next"]["model"], 256);
    let continuation =
        json!({"kind":"continue","revision":first["revision"],"cursor":first["next"]});
    let found = models(&fixture, continuation.clone()).await.unwrap();
    assert_eq!(found["choices"].as_array().unwrap().len(), 1);
    assert_eq!(found["choices"][0]["selection"]["modelId"], "model-299");
    assert!(found["next"].is_null());
    assert!(!found.to_string().contains("private-fixture-key"));
    assert!(!found.to_string().contains("baseUrl"));
    let matches = models(&fixture, json!({"kind":"start","query":"MoDeL-29"}))
        .await
        .unwrap();
    assert!(matches["choices"].as_array().unwrap().is_empty());
    let matches = models(
        &fixture,
        json!({"kind":"continue","revision":matches["revision"],"cursor":matches["next"]}),
    )
    .await
    .unwrap();
    assert_eq!(matches["choices"].as_array().unwrap().len(), 10);
    let none = models(
        &fixture,
        json!({"kind":"start","query":"research missing-token"}),
    )
    .await
    .unwrap();
    let none = models(
        &fixture,
        json!({"kind":"continue","revision":none["revision"],"cursor":none["next"]}),
    )
    .await
    .unwrap();
    assert!(none["choices"].as_array().unwrap().is_empty());
    assert!(none["next"].is_null());
    edit(&fixture, |changes| {
        changes.name = Some("Renamed Account".into())
    })
    .await;
    let changed = models(&fixture, continuation).await.unwrap();
    assert_eq!(changed["kind"], "changed");
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}

#[tokio::test]
async fn queued_model_route_change_settles_original_message_without_retargeting() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let fixture = Fixture::new(&base, 1, None).await;
        let session = fixture.create("Queued original model").await;
        fixture.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        let queued = fixture.send(&session).await.unwrap();
        assert_eq!(queued["kind"], "queued");
        edit(&fixture, |changes| changes.configuration = Some(json!({"baseUrl":"http://127.0.0.1:1/v1"}))).await;
        response(&mut socket).await;
        fixture.drained().await;
        assert!(fixture.store.pending_messages(&session).await.unwrap().is_empty());
        let facts = fixture.store.scoped_prefix(maka_agent_facts::event::LogScope::Session { id: session }, 256, 1024 * 1024).await.unwrap();
        assert!(facts.events.iter().any(|event| matches!(&event.event.fact, maka_agent_facts::event::Fact::InvocationEnded { outcome: maka_agent_facts::event::InvocationOutcome::Failed { class, .. } } if class == "message_preparation_failed")));
        assert_eq!(facts.events.iter().filter(|event| matches!(event.event.fact, maka_agent_facts::event::Fact::InvocationOpened { .. })).count(), 2);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn queue_promotion_rejects_a_different_accepted_model_execution() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let fixture = Fixture::new(&base, 1, None).await;
        let session = fixture.create("Different queued model").await;
        let started = fixture.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        edit(&fixture, |changes| changes.configuration = Some(json!({"baseUrl":"http://127.0.0.1:1/v1"}))).await;
        let queued = fixture.send(&session).await.unwrap();
        let result = fixture.mutation(2, "editQueue", json!({"sessionId":session,"expectedRevision":queued["revision"],"edit":{"kind":"promote","messageId":queued["messageId"]}}), Uuid::new_v4()).await.unwrap();
        assert_eq!(result, json!({"kind":"rejected","reason":"execution_changed"}));
        assert_eq!(fixture.query("queue", json!({"sessionId":session})).await.unwrap()["entries"][0]["kind"], "followup");
        fixture.mutation(2, "stop", json!({"sessionId":session,"invocationId":started["invocationId"]}), Uuid::new_v4()).await.unwrap();
        fixture.drained().await;
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

pub(super) async fn thinking(
    fixture: &Fixture,
    levels: Vec<maka_agent_facts::execution::ThinkingLevel>,
    default: maka_agent_facts::execution::ThinkingLevel,
) {
    edit(fixture, |changes| {
        changes.model_overrides = Patch::Set(BTreeMap::from([(
            "fixture-model".into(),
            ModelOverride {
                thinking_levels: Some(levels),
                default_thinking_level: Some(default),
                ..Default::default()
            },
        )]));
    })
    .await;
}

#[tokio::test]
async fn thinking_choices_reach_requests_and_queued_policy_cannot_steer_another_run() {
    use maka_agent_facts::execution::ThinkingLevel::High;
    use maka_agent_facts::execution::ThinkingLevel::Low;
    use maka_agent_facts::execution::ThinkingLevel::Off;
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let fixture = Fixture::new(&base, 1, None).await;
        thinking(&fixture, vec![Off, Low, High], High).await;
        let page = models(&fixture, json!({"kind":"start"})).await.unwrap();
        assert_eq!(page["choices"][0]["thinkingLevels"], json!(["off", "low", "high"]));
        assert_eq!(page["choices"][0]["defaultThinkingLevel"], "high");
        let session = fixture.create("Per-message thinking").await;
        let selection = fixture.selection().await;
        // Omission selects the configured default; explicit Off is a different policy.
        fixture.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        assert_eq!(request(&mut socket).await["reasoning_effort"], "high");
        for level in [Off, Low] {
            let result = fixture.mutation(2, "send", json!({"sessionId":session,"message":"chosen thinking","model":selection,"thinkingLevel":level}), Uuid::new_v4()).await.unwrap();
            assert_eq!(result["kind"], "queued");
        }
        assert!(fixture.mutation(2, "send", json!({"sessionId":session,"message":"unsupported","model":selection,"thinkingLevel":"ultra"}), Uuid::new_v4()).await.is_err());
        let pending = fixture.store.pending_messages(&session).await.unwrap();
        assert_eq!(pending.len(), 2);
        for (entry, level) in pending.iter().zip([Off, Low]) {
            let basis: MessageBasis = serde_json::from_value(entry.execution.basis.clone()).unwrap();
            assert_eq!(basis.model.unwrap().thinking_level, Some(level));
        }
        let queue = fixture.query("queue", json!({"sessionId":session})).await.unwrap();
        let rejected = fixture.mutation(2, "editQueue", json!({"sessionId":session,"expectedRevision":queue["revision"],"edit":{"kind":"promote","messageId":pending[0].source.message.message_id}}), Uuid::new_v4()).await.unwrap();
        assert_eq!(rejected, json!({"kind":"rejected","reason":"execution_changed"}));
        response(&mut socket).await;
        for expected in ["none", "low"] {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert_eq!(request(&mut socket).await["reasoning_effort"], expected);
            response(&mut socket).await;
        }
        fixture.drained().await;
        assert!(fixture.store.pending_messages(&session).await.unwrap().is_empty());
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn restored_thinking_preserves_explicit_choice_and_refuses_changed_default_or_support() {
    use maka_agent_facts::execution::ThinkingLevel::High;
    use maka_agent_facts::execution::ThinkingLevel::Low;
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    thinking(&fixture, vec![Low, High], High).await;
    let session = fixture.create("Restored thinking policy").await;
    let mut sources = Vec::new();
    for level in [None, Some(High)] {
        let (source, prepared) = crate::model::Source::select(
            fixture.config.clone(),
            fixture.owner.credentials.clone(),
            fixture.selection().await,
            level,
            session.clone(),
            fixture.owner.revision.clone(),
        )
        .await
        .unwrap();
        let encoded = serde_json::to_vec(&source.accepted).unwrap();
        let restored = crate::model::Source::restore(
            fixture.config.clone(),
            fixture.owner.credentials.clone(),
            serde_json::from_slice(&encoded).unwrap(),
            session.clone(),
            fixture.owner.revision.clone(),
        )
        .unwrap();
        assert_eq!(restored.capture().await.unwrap().options, prepared.options);
        sources.push((restored, prepared.options));
    }
    thinking(&fixture, vec![Low, High], Low).await;
    assert!(sources[0].0.capture().await.is_err());
    assert_eq!(sources[1].0.capture().await.unwrap().options, sources[1].1);
    thinking(&fixture, vec![Low], Low).await;
    assert!(sources[1].0.capture().await.is_err());
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_thinking_default_remains_unavailable_without_poisoning_other_models() {
    use maka_agent_facts::execution::ThinkingLevel::High;
    use maka_agent_facts::execution::ThinkingLevel::Low;
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    thinking(&fixture, vec![Low], High).await;
    edit(&fixture, |changes| {
        changes
            .enabled_model_ids
            .as_mut()
            .unwrap()
            .extend((0..260).map(|at| format!("other-{at}")));
    })
    .await;
    let catalog = fixture.config.catalog().await.unwrap();
    let target = ConnectionTarget {
        connection_id: catalog.connections[0].connection_id.clone(),
        model_id: "fixture-model".into(),
    };
    fixture
        .config
        .set_default_target(SetDefaultConnectionTargetInput {
            expected_catalog_revision: catalog.revision,
            target: Some(target.clone()),
        })
        .await
        .unwrap();
    let page = models(&fixture, json!({"kind":"start"})).await.unwrap();
    assert_eq!(
        page["defaultModel"],
        json!({"kind":"unavailable","target":target})
    );
    assert_eq!(page["choices"].as_array().unwrap().len(), 16);
    assert!(
        page["choices"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["selection"]["modelId"] != "fixture-model")
    );
    // Search still advances over the omitted model and non-matching rows.
    let page = models(&fixture, json!({"kind":"start","query":"fixture-model"}))
        .await
        .unwrap();
    assert!(page["choices"].as_array().unwrap().is_empty());
    assert_eq!(page["next"]["model"], 256);
    let last = models(
        &fixture,
        json!({"kind":"continue","revision":page["revision"],"cursor":page["next"]}),
    )
    .await
    .unwrap();
    assert!(last["choices"].as_array().unwrap().is_empty());
    assert!(last["next"].is_null());
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}

#[tokio::test]
async fn accepted_code_mode_survives_settings_changes_and_legacy_messages_remain_direct() {
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    let session = fixture.create("Frozen tool policy").await;
    edit(&fixture, |changes| {
        changes.model_overrides = Patch::Set(BTreeMap::from([(
            "fixture-model".into(),
            ModelOverride {
                code_mode: Some(true),
                apply_patch: Some(false),
                context_window: Some(1_000_000),
                input_limit: Some(900_000),
                compaction_threshold: Some(700_000),
                ..Default::default()
            },
        )]));
    })
    .await;
    let (source, prepared) = crate::model::Source::select(
        fixture.config.clone(),
        fixture.owner.credentials.clone(),
        fixture.selection().await,
        None,
        session.clone(),
        fixture.owner.revision.clone(),
    )
    .await
    .unwrap();
    assert_eq!(source.accepted.tool_mode, ToolMode::CodeMode);
    assert_eq!(
        source.accepted.editing_tools,
        Some(maka_agent_facts::execution::EditingTools::Structured)
    );
    let context = prepared.context.unwrap();
    assert_eq!(context.model_context_window, Some(1_000_000));
    assert_eq!(context.context_window, Some(900_000));
    assert_eq!(context.declared_window, Some(700_000));
    let stored = serde_json::to_value(&source.accepted).unwrap();
    edit(&fixture, |changes| {
        changes.model_overrides = Patch::Set(BTreeMap::from([(
            "fixture-model".into(),
            ModelOverride {
                code_mode: Some(false),
                apply_patch: Some(true),
                context_window: Some(128_000),
                input_limit: Some(96_000),
                ..Default::default()
            },
        )]));
    })
    .await;
    let restored = crate::model::Source::restore(
        fixture.config.clone(),
        fixture.owner.credentials.clone(),
        serde_json::from_value(stored.clone()).unwrap(),
        session.clone(),
        fixture.owner.revision.clone(),
    )
    .unwrap();
    assert_eq!(restored.capture().await.unwrap().context, Some(context));
    assert_eq!(restored.accepted.tool_mode, ToolMode::CodeMode);
    assert_eq!(
        restored.accepted.editing_tools,
        Some(maka_agent_facts::execution::EditingTools::Structured)
    );
    let (next, prepared) = crate::model::Source::select(
        fixture.config.clone(),
        fixture.owner.credentials.clone(),
        fixture.selection().await,
        None,
        session.clone(),
        fixture.owner.revision.clone(),
    )
    .await
    .unwrap();
    let context = prepared.context.unwrap();
    assert_eq!(context.model_context_window, Some(128_000));
    assert_eq!(context.context_window, Some(96_000));
    assert_eq!(context.declared_window, Some(81_600));
    assert_eq!(next.accepted.tool_mode, ToolMode::Direct);
    assert_eq!(
        next.accepted.editing_tools,
        Some(maka_agent_facts::execution::EditingTools::ApplyPatch)
    );
    let mut legacy = stored;
    legacy.as_object_mut().unwrap().remove("tool_mode");
    legacy.as_object_mut().unwrap().remove("editing_tools");
    let accepted: crate::model::AcceptedModel = serde_json::from_value(legacy).unwrap();
    assert_eq!(accepted.tool_mode, ToolMode::Direct);
    assert_eq!(accepted.editing_tools, None);
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}
