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
use maka_agent::ModelSource;

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
        name: row.name,
        configuration: row.configuration,
        enabled: row.enabled,
        enabled_model_ids: row.enabled_model_ids,
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
            Attachments::new(),
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
            changes.enabled_model_ids.push("second-model".into())
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
    let (source, _) = crate::model::Source::select(
        fixture.config.clone(),
        fixture.selection().await,
        session,
        fixture.owner.revision.clone(),
    )
    .await
    .unwrap();
    edit(&fixture, |changes| {
        changes.name = "A different label".into();
        changes.enabled_model_ids.push("other-model".into());
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
        .observe_model(source.binding.clone())
        .await
        .unwrap()
        .unwrap()
        .credential
        .unwrap();
    assert!(
        credential
            .commit_refresh(
                maka_runtime::provider::Credential {
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
        fixture.selection().await,
        "current".into(),
        fixture.owner.revision.clone(),
    )
    .await
    .unwrap();
    edit(&fixture, |changes| {
        changes.configuration = json!({"baseUrl":"http://127.0.0.1:2/v1"})
    })
    .await;
    assert!(current.capture().await.is_err());
    let (current, _) = crate::model::Source::select(
        fixture.config.clone(),
        fixture.selection().await,
        "current".into(),
        fixture.owner.revision.clone(),
    )
    .await
    .unwrap();
    edit(&fixture, |changes| {
        changes.enabled_model_ids = vec!["other-model".into()]
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
        changes.enabled_model_ids = (0..20).map(|at| format!("model-{at:02}")).collect()
    })
    .await;
    let page = models(&first, json!({"kind":"start"})).await.unwrap();
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
    let input = json!({"expectedCatalogRevision":page["revision"],"target":{"connectionId":page["choices"][0]["selection"]["connectionId"],"modelId":"model-00"}});
    let client = first.app.client();
    assert!(
        client
            .prepare_mutation(
                binding.clone(),
                "setDefault".into(),
                input.clone(),
                scope(&first.issuer, false),
                operation,
                Attachments::new()
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
                Attachments::new()
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
            Attachments::new(),
        )
        .await
        .unwrap();
    let recorded = first
        .store
        .record_application_intent(
            prepared.reference().source.clone(),
            prepared.consumer().instance().clone(),
            prepared.reference().clone(),
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
    assert_eq!(new["choices"][0]["isDefault"], true);
    assert!(models(&first, json!({"kind":"continue","revision":new["revision"],"cursor":{"connection":0,"model":4000}})).await.is_err());
    first.app.shutdown().await.unwrap();
    second.app.shutdown().await.unwrap();
    first.config.shutdown().await.unwrap();
    second.config.shutdown().await.unwrap();
}
