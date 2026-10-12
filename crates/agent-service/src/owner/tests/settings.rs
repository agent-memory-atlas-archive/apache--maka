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
use maka_plugin_core::{contract::ResourceType, operation::RecoveryRef};
struct Text(String);
impl maka_plugins::call::ReadResource for Text {
    fn resource_type(&self) -> ResourceType {
        ResourceType {
            name: "maka.text.utf8".into(),
            version: 1,
        }
    }
    fn bytes(&self) -> usize {
        self.0.len()
    }
    fn metadata(&self) -> Value {
        json!({"private":"private-resource-metadata-marker"})
    }
    fn read(&self, range: std::ops::Range<usize>) -> Result<Vec<u8>, String> {
        Ok(self.0.get(range).ok_or("range")?.as_bytes().to_vec())
    }
}
async fn binding(fixture: &Fixture) -> Binding {
    let client = fixture.app.client();
    client
        .bind(client.activation(&id(2)).unwrap(), "settings".into())
        .await
        .unwrap()
        .remove(0)
}
async fn query(fixture: &Fixture, method: &str, input: Value) -> Value {
    fixture
        .app
        .client()
        .start(
            binding(fixture).await,
            method.into(),
            input,
            scope(&fixture.issuer, true),
            Attachments::new().into(),
        )
        .await
        .unwrap()
        .result()
        .await
        .unwrap()
}
async fn mutate(
    fixture: &Fixture,
    method: &str,
    input: Value,
    secret: Option<&str>,
) -> (Value, RecoveryRef) {
    let reply = mutate_reply(fixture, method, input, secret).await;
    let Receipt::Recorded {
        outcome: OperationOutcome::Returned { value },
        ..
    } = reply.receipt
    else {
        panic!("unexpected operation receipt")
    };
    (value, reply.reference)
}
async fn mutate_reply(
    fixture: &Fixture,
    method: &str,
    input: Value,
    secret: Option<&str>,
) -> maka_plugin_core::operation::OperationReply {
    let attachments = secret
        .map(|value| {
            BTreeMap::from([(
                "secret".into(),
                Arc::new(Text(value.into())) as Arc<dyn maka_plugins::call::ReadResource>,
            )])
        })
        .unwrap_or_default();
    let client = fixture.app.client();
    let prepared = client
        .prepare_mutation(
            binding(fixture).await,
            method.into(),
            input,
            scope(&fixture.issuer, true),
            Uuid::new_v4(),
            attachments.into(),
        )
        .await
        .unwrap();
    let reference = prepared.reference().clone();
    let recorded = fixture
        .store
        .record_application_intent(
            reference.source.clone(),
            prepared.consumer().instance().clone(),
            reference.clone(),
            serde_json::Value::Null,
        )
        .await
        .unwrap();
    client
        .mutate(prepared, recorded)
        .await
        .unwrap()
        .result()
        .await
}

#[tokio::test]
async fn settings_confirmed_validation_rejections_do_not_downgrade_sql_uncertainty() {
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    let original = fixture.config.catalog().await.unwrap();
    let row = &original.connections[0];
    let basis = json!({"connectionId":row.connection_id,"revision":row.revision});
    for (changes, reason) in [
        (json!({"name":"x".repeat(257)}), "invalid_input"),
        (
            json!({"configuration":{"baseUrl":17}}),
            "invalid_configuration",
        ),
        // These individually valid limits are rejected after the original SQL
        // CAS and merge. The transaction must finish rollback before replying.
        (
            json!({"name":"Not committed","modelOverrides":{"fixture-model":{"contextWindow":10,"inputLimit":11}}}),
            "invalid_input",
        ),
    ] {
        let (result, reference) = mutate(
            &fixture,
            "update",
            json!({"expected":basis,"changes":changes}),
            None,
        )
        .await;
        assert_eq!(result, json!({"kind":"rejected","reason":reason}));
        let observation = fixture
            .store
            .operation_observation(&reference)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            observation.outcome,
            OperationOutcome::Returned { value: result }
        );
        assert_eq!(fixture.config.catalog().await.unwrap(), original);
    }
    let detail = query(&fixture, "connection", basis.clone()).await;
    let locator = json!({"scope":"connection","connectionId":row.connection_id,"kind":"provider"});
    let status = query(&fixture, "credentialStatus", json!({"locator":locator})).await;
    let status: CredentialStatus = serde_json::from_value(status["status"].clone()).unwrap();
    for secret in ["", "key-with\ncontrol"] {
        let (result, _) = mutate(
            &fixture,
            "saveCredential",
            json!({"expected":detail["target"],"expectedCredential":status.basis()}),
            Some(secret),
        )
        .await;
        assert_eq!(result, json!({"kind":"rejected","reason":"invalid_secret"}));
        assert_eq!(fixture.config.catalog().await.unwrap(), original);
    }
    let (result, _) = mutate(&fixture,"create",json!({"expectedCatalogRevision":original.revision,"connection":{"slug":"unknown","name":"Unknown provider","provider":{"packageId":"maka.providers","entryId":"maka.providers","scope":"profile","name":"not-compiled"},"configuration":{},"enabled":true,"enabledModelIds":[]}}),None).await;
    assert_eq!(
        result,
        json!({"kind":"rejected","reason":"provider_unavailable"})
    );
    assert_eq!(fixture.config.catalog().await.unwrap(), original);

    // An actual SQLite failure is never interpreted as a validation rejection.
    // Recovery observes the same original Unknown rather than retrying a write.
    let db = rusqlite::Connection::open(fixture.root.join("configuration-rust.sqlite")).unwrap();
    db.execute_batch("CREATE TRIGGER settings_write_fault BEFORE UPDATE ON connections BEGIN SELECT RAISE(ABORT, 'synthetic SQL fault'); END;").unwrap();
    let reply = mutate_reply(
        &fixture,
        "update",
        json!({"expected":basis,"changes":{"name":"SQL fault"}}),
        None,
    )
    .await;
    assert!(matches!(
        reply.receipt,
        Receipt::Recorded {
            outcome: OperationOutcome::Unknown { .. },
            ..
        }
    ));
    let observation = fixture
        .store
        .operation_observation(&reply.reference)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        observation.outcome,
        OperationOutcome::Unknown { .. }
    ));
    assert_eq!(fixture.config.catalog().await.unwrap(), original);
    db.execute_batch("DROP TRIGGER settings_write_fault;")
        .unwrap();
    drop(db);
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}
#[tokio::test]
async fn root_settings_create_partial_edit_save_capture_and_stale_vault_basis_use_original_journals()
 {
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    let page = query(&fixture, "connections", json!({"kind":"start"})).await;
    assert_eq!(page["connections"].as_array().unwrap().len(), 1);
    let original = page["connections"][0]["basis"].clone();
    let detail = query(&fixture, "connection", original).await;
    assert_eq!(detail["kind"], "ready");
    let locator = json!({"scope":"connection","connectionId":detail["target"]["connectionId"],"kind":"provider"});
    let status = query(&fixture, "credentialStatus", json!({"locator":locator})).await;
    let expected: CredentialStatus = serde_json::from_value(status["status"].clone()).unwrap();
    let save = json!({"expected":detail["target"],"expectedCredential":expected.basis()});
    const KEY: &str = "settings-native-fixture-key-7f35";
    let (saved, reference) = mutate(&fixture, "saveCredential", save.clone(), Some(KEY)).await;
    assert_eq!(saved["kind"], "saved");
    let locator: CredentialLocator = serde_json::from_value(locator).unwrap();
    let envelope: maka_runtime::provider::Credential = serde_json::from_str(
        &fixture
            .config
            .credential_secret(&locator, None)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(envelope.secret, KEY);
    let (stale, _) = mutate(&fixture, "saveCredential", save, Some("old-stale-capture")).await;
    assert_eq!(
        stale,
        json!({"kind":"rejected","reason":"connection_changed"})
    );
    let page = query(&fixture, "connections", json!({"kind":"start"})).await;
    let detail = query(
        &fixture,
        "connection",
        page["connections"][0]["basis"].clone(),
    )
    .await;
    let status = query(&fixture, "credentialStatus", json!({"locator":locator})).await;
    let current: CredentialStatus = serde_json::from_value(status["status"].clone()).unwrap();
    let target: ConnectionCredentialTarget =
        serde_json::from_value(detail["target"].clone()).unwrap();
    let missing_input = json!({"expected":target});
    let resource: Arc<dyn maka_plugins::call::ReadResource> = Arc::new(Text(KEY.into()));
    assert!(
        fixture
            .app
            .client()
            .prepare_mutation(
                binding(&fixture).await,
                "saveCredential".into(),
                missing_input,
                scope(&fixture.issuer, true),
                Uuid::new_v4(),
                BTreeMap::from([("secret".into(), resource)]).into()
            )
            .await
            .is_err()
    );
    let delete = DeleteCredentialInput {
        expected: current.basis().unwrap(),
        expected_connection: Some(target.clone()),
    };
    assert!(matches!(
        fixture.config.delete_credential(delete).await.unwrap(),
        CredentialMutationResult::Committed { .. }
    ));
    assert_eq!(
        fixture.config.catalog().await.unwrap().connections[0].revision,
        target.revision
    );
    let (stale, _) = mutate(
        &fixture,
        "saveCredential",
        json!({"expected":target,"expectedCredential":current.basis()}),
        Some("stale-generation-key"),
    )
    .await;
    assert_eq!(
        stale,
        json!({"kind":"rejected","reason":"credential_changed"})
    );
    let renamed = mutate(&fixture,"update",json!({"expected":page["connections"][0]["basis"],"changes":{"name":"Renamed from plugin"}}),None).await.0;
    assert_eq!(renamed["kind"], "committed");
    let row = fixture
        .config
        .catalog()
        .await
        .unwrap()
        .connections
        .remove(0);
    assert_eq!(row.name, "Renamed from plugin");
    assert_eq!(row.enabled_model_ids, vec!["fixture-model"]);
    let basis = json!({"connectionId":row.connection_id,"revision":row.revision});
    for changes in [
        json!({"name":null}),
        json!({"enabled":null}),
        json!({"enabledModelIds":null}),
        json!({"configuration":null}),
        json!({"modelOverrides":{"fixture-model":{"vision":null}}}),
        json!({"modelOverrides":{"fixture-model":{"capabilities":{"chat":null}}}}),
        json!({"requestBodyOverlay":[]}),
    ] {
        assert!(
            fixture
                .app
                .client()
                .prepare_mutation(
                    binding(&fixture).await,
                    "update".into(),
                    json!({"expected":basis,"changes":changes}),
                    scope(&fixture.issuer, true),
                    Uuid::new_v4(),
                    Attachments::new().into()
                )
                .await
                .is_err()
        );
    }
    let cleared = mutate(
        &fixture,
        "update",
        json!({"expected":basis,"changes":{"modelOverrides":null,"requestBodyOverlay":null}}),
        None,
    )
    .await
    .0;
    assert_eq!(cleared["kind"], "committed");
    let providers = query(&fixture, "providers", json!({"offset":0})).await;
    assert!(!providers["providers"].as_array().unwrap().is_empty());
    assert!(serde_json::to_vec(&providers).unwrap().len() <= 128 * 1024);
    let revision = fixture.config.catalog().await.unwrap().revision;
    assert!(fixture.app.client().prepare_mutation(binding(&fixture).await,"create".into(),json!({"expectedCatalogRevision":revision,"connection":{"slug":"invalid","name":"Invalid null","provider":row.provider,"configuration":{},"enabled":true,"enabledModelIds":[],"modelOverrides":null}}),scope(&fixture.issuer,true),Uuid::new_v4(),Attachments::new().into()).await.is_err());
    let (created,_) = mutate(&fixture,"create",json!({"expectedCatalogRevision":revision,"connection":{"slug":"created","name":"Created through settings","provider":{"packageId":"maka.providers","entryId":"maka.providers","scope":"profile","name":"openai"},"configuration":{},"enabled":true,"enabledModelIds":["manual-model"]}}),None).await;
    assert_eq!(created["kind"], "committed");
    let catalog = fixture.config.catalog().await.unwrap();
    let created_row = catalog
        .connections
        .iter()
        .find(|row| row.slug == "created")
        .unwrap();
    assert!(
        created_row.configuration["baseUrl"]
            .as_str()
            .unwrap()
            .starts_with("https://")
    );
    let removed = mutate(
        &fixture,
        "remove",
        json!({"expected":created["connection"]}),
        None,
    )
    .await
    .0;
    assert_eq!(removed["kind"], "committed");
    // Read the actual persisted operation/caller/enrollment rows, excluding the
    // vault table where storing the credential envelope is the intended effect.
    for (database, sql) in [
        (
            "runtime-rust.sqlite",
            "SELECT request_json || outcome_json || cleanup_json FROM application_operations",
        ),
        (
            "runtime-rust.sqlite",
            "SELECT reference_json FROM application_intents",
        ),
        (
            "configuration-rust.sqlite",
            "SELECT target || request_fingerprint || phase FROM oauth_login_receipts",
        ),
    ] {
        let db = rusqlite::Connection::open_with_flags(
            fixture.root.join(database),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut statement = db.prepare(sql).unwrap();
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap();
        let mut count = 0;
        for row in rows {
            count += 1;
            let row = row.unwrap();
            for secret in [
                KEY,
                "old-stale-capture",
                "stale-generation-key",
                "private-resource-metadata-marker",
            ] {
                assert!(
                    !row.contains(secret),
                    "private payload entered persisted journal"
                );
            }
        }
        assert!(count > 0, "actual journal must contain persisted facts");
    }
    assert!(
        fixture
            .store
            .operation_observation(&reference)
            .await
            .unwrap()
            .is_some()
    );
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}

#[tokio::test]
async fn assistant_preferences_and_capability_guidance_freeze_at_message_acceptance() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fixture = Fixture::configured(
            &format!("http://{}/v1", listener.local_addr().unwrap()),
            1, None, None, None,
            |settings| settings.assistant = Some(AssistantConfiguration {
                home: Some(settings.workspace_roots[0].path.parent().unwrap().to_str().unwrap().into()),
            }),
        ).await;
        let global = fixture._dir.path().join(".maka");
        std::fs::create_dir(&global).unwrap();
        std::fs::write(global.join("AGENTS.md"), "GLOBAL FIRST").unwrap();
        std::fs::write(fixture.workspace.join("AGENTS.md"), "PROJECT FIRST").unwrap();
        // A guidance filename is not authority to escape the selected directory.
        #[cfg(unix)] {
            let outside = fixture._dir.path().join("outside-guidance");
            std::fs::write(&outside, "OUTSIDE SECRET").unwrap();
            std::os::unix::fs::symlink(&outside, fixture.workspace.join("CLAUDE.md")).unwrap();
            std::os::unix::fs::symlink(&outside, global.join("CLAUDE.md")).unwrap();
        }
        let initial = query(&fixture, "assistantPreferences", json!({})).await;
        let (saved, reference) = mutate(&fixture, "updateAssistantPreferences", json!({
            "expectedRevision":initial["revision"],
            "mutation":{"kind":"personalization","displayName":"M4n5ter","assistantTone":"FIRST TONE"}
        }), None).await;
        assert_eq!(saved["kind"], "committed");
        let observation = fixture.store.operation_observation(&reference).await.unwrap().unwrap();
        assert_eq!(observation.outcome, OperationOutcome::Returned { value: saved.clone() });
        let (stale, _) = mutate(&fixture, "updateAssistantPreferences", json!({
            "expectedRevision":initial["revision"],
            "mutation":{"kind":"workspace_instructions","enabled":false}
        }), None).await;
        assert_eq!(stale["kind"], "revision_conflict");
        let session = fixture.create("Assistant policy").await;
        assert_eq!(fixture.send(&session).await.unwrap()["kind"], "started");
        let (mut first, _) = listener.accept().await.unwrap();
        let system = |sent: &Value| sent["messages"].as_array().unwrap().iter()
            .filter(|message| message["role"] == "system")
            .map(|message| message["content"].as_str().unwrap()).collect::<Vec<_>>().join("\n");
        let original = system(&request(&mut first).await);
        for value in ["You are Maka", "M4n5ter", "FIRST TONE", "GLOBAL FIRST", "PROJECT FIRST"] {
            assert!(original.contains(value), "missing {value}: {original}");
        }
        assert!(!original.contains("OUTSIDE SECRET"));
        assert_eq!(fixture.send(&session).await.unwrap()["kind"], "queued");
        std::fs::write(global.join("AGENTS.md"), "GLOBAL REPLACED").unwrap();
        std::fs::write(fixture.workspace.join("AGENTS.md"), "PROJECT REPLACED").unwrap();
        let (disabled, _) = mutate(&fixture, "updateAssistantPreferences", json!({
            "expectedRevision":saved["revision"],
            "mutation":{"kind":"workspace_instructions","enabled":false}
        }), None).await;
        assert_eq!(disabled["kind"], "committed");
        response(&mut first).await;
        let (mut second, _) = listener.accept().await.unwrap();
        assert_eq!(system(&request(&mut second).await), original, "queued work uses its persisted acceptance basis");
        response(&mut second).await;
        fixture.drained().await;
        assert_eq!(fixture.send(&session).await.unwrap()["kind"], "started");
        let (mut third, _) = listener.accept().await.unwrap();
        let current = system(&request(&mut third).await);
        assert!(current.contains("FIRST TONE"));
        assert!(!current.contains("GLOBAL") && !current.contains("PROJECT"));
        response(&mut third).await;
        fixture.drained().await;
        let current = query(&fixture, "assistantPreferences", json!({})).await;
        assert_eq!(current["incognitoActive"], false);
        let (privacy, reference) = mutate(&fixture, "updateAssistantPreferences", json!({
            "expectedRevision":current["revision"],
            "mutation":{"kind":"privacy","incognitoActive":true}
        }), None).await;
        assert_eq!(privacy["kind"], "committed");
        assert_eq!(query(&fixture, "assistantPreferences", json!({})).await["incognitoActive"], true);
        assert!(fixture.config.runtime_policy().await.unwrap().policy.privacy.incognito_active);
        assert_eq!(fixture.store.operation_observation(&reference).await.unwrap().unwrap().outcome,
            OperationOutcome::Returned { value: privacy });
        let (stale, _) = mutate(&fixture, "updateAssistantPreferences", json!({
            "expectedRevision":current["revision"],
            "mutation":{"kind":"privacy","incognitoActive":false}
        }), None).await;
        assert_eq!(stale["kind"], "revision_conflict");
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn native_usage_keeps_request_quotes_and_settlement_fences_across_price_changes() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fixture = Fixture::new(&format!("http://{}/v1", listener.local_addr().unwrap()), 1, None).await;
        let page = query(&fixture, "prices", json!({"kind":"start"})).await;
        let price = |revision: Value, input: f64| json!({
            "expectedRevision": revision,
            "mutation":{"kind":"upsert","pricing":{"modelKey":"openai-compatible:fixture-model","inputUsdPer1M":input,"outputUsdPer1M":20.0}}
        });
        let (saved, reference) = mutate(&fixture, "updatePrice", price(page["revision"].clone(), 10.0), None).await;
        assert_eq!(saved["result"]["kind"], "committed");
        assert_eq!(fixture.store.operation_observation(&reference).await.unwrap().unwrap().outcome,
            OperationOutcome::Returned { value: saved.clone() });
        let session = fixture.create("Physical usage").await;
        fixture.send(&session).await.unwrap();
        let (mut first, _) = listener.accept().await.unwrap();
        request(&mut first).await;
        // Change prices while the first provider request is still in flight.
        let (changed, _) = mutate(&fixture, "updatePrice", price(saved["result"]["revision"].clone(), 100.0), None).await;
        assert_eq!(changed["result"]["kind"], "committed");
        let (stale, _) = mutate(&fixture, "updatePrice", price(saved["result"]["revision"].clone(), 1000.0), None).await;
        assert_eq!(stale["result"]["kind"], "revision_conflict");
        response(&mut first).await;
        fixture.drained().await;
        let range = json!({"from":0,"to":9_007_199_254_740_991u64,"sessionId":session,"through":null});
        let original = query(&fixture, "usageSummary", json!({"range":range,"group":"byProvider","offset":0})).await;
        assert_eq!(original["summary"]["models"]["calls"], 1);
        let cost = original["summary"]["models"]["cost"]["knownUsd"].as_f64().unwrap();
        assert!((cost - 0.00005).abs() < 1e-12, "first request must retain original quote: {original}");
        let mut frozen = range.clone();
        frozen["through"] = original["through"].clone();
        fixture.send(&session).await.unwrap();
        let (mut second, _) = listener.accept().await.unwrap();
        request(&mut second).await;
        response(&mut second).await;
        fixture.drained().await;
        assert_eq!(query(&fixture, "usageSummary", json!({"range":frozen,"group":"byProvider","offset":0})).await, original);
        let activity = query(&fixture, "usage", json!({"range":frozen,"selection":{"kind":null,"status":null,"search":""},"offset":0})).await;
        assert_eq!(activity["total"], 1);
        assert_eq!(activity["attempts"][0]["attempt"]["quote"]["pricing"]["inputUsdPer1M"], 10.0);
        let latest = query(&fixture, "usageSummary", json!({"range":range,"group":"byProvider","offset":0})).await;
        assert_eq!(latest["summary"]["models"]["calls"], 2);
        assert_eq!(latest["summary"]["models"]["input"]["known"], 2);
        assert_eq!(latest["summary"]["models"]["output"]["known"], 4);
        assert_eq!(latest["summary"]["models"]["cost"]["unvalued"], 0);
        assert!((latest["summary"]["models"]["cost"]["knownUsd"].as_f64().unwrap() - 0.00019).abs() < 1e-12);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn host_proxy_settings_route_model_requests_and_keep_atomic_credential_receipts() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let proxy = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fixture = Fixture::new("http://proxy-target.invalid/v1", 1, None).await;
        let original = query(&fixture, "network", json!({})).await;
        assert_eq!(original["rootPath"], fixture.root.canonicalize().unwrap().to_str().unwrap());
        assert_eq!(original["rootId"], fixture.store.owned_root().unwrap().root_id());
        let mut policy = original["proxy"].clone();
        policy["enabled"] = json!(true);
        policy["protocol"] = json!("http");
        policy["host"] = json!("127.0.0.1");
        policy["port"] = json!(proxy.local_addr().unwrap().port());
        policy["authEnabled"] = json!(true);
        policy["username"] = json!("proxy-user");
        policy["bypassList"] = json!([]);
        policy["autoBypassDomains"] = json!([]);
        let update = json!({"expectedPolicyRevision":original["revision"],"expectedCredential":null,"networkProxy":policy,"credential":"replace"});
        let (saved, reference) = mutate(&fixture, "updateNetwork", update.clone(), Some("private-proxy-password")).await;
        assert_eq!(saved["result"]["kind"], "committed");
        let current = query(&fixture, "network", json!({})).await;
        assert_eq!(current["proxy"], policy);
        assert_eq!(current["credential"]["configured"], true);
        assert!(!current.to_string().contains("private-proxy-password"));
        assert_eq!(fixture.config.network_configuration().await.unwrap().password.as_deref(), Some("private-proxy-password"));
        assert_eq!(fixture.store.operation_observation(&reference).await.unwrap().unwrap().outcome,
            OperationOutcome::Returned { value: saved.clone() });
        let (stale, _) = mutate(&fixture, "updateNetwork", update, Some("different-password")).await;
        assert_eq!(stale["result"]["kind"], "revision_conflict");
        let session = fixture.create("Proxy route").await;
        fixture.send(&session).await.unwrap();
        // This hostname is deliberately unresolvable. Receiving the request at
        // the local proxy proves the actual native model transport uses settings.
        let (mut stream, _) = proxy.accept().await.unwrap();
        let sent = request(&mut stream).await;
        assert_eq!(sent["model"], "fixture-model");
        response(&mut stream).await;
        fixture.drained().await;
        let mut disabled = policy;
        disabled["enabled"] = json!(false);
        disabled["authEnabled"] = json!(false);
        let delete = json!({"expectedPolicyRevision":current["revision"],"expectedCredential":{
            "locator":current["credential"]["locator"],"credentialId":current["credential"]["credentialId"],"revision":current["credential"]["revision"]
        },"networkProxy":disabled,"credential":"delete"});
        let (removed, _) = mutate(&fixture, "updateNetwork", delete.clone(), None).await;
        assert_eq!(removed["result"]["kind"], "committed");
        let final_state = fixture.config.network_configuration().await.unwrap();
        assert!(!final_state.proxy.enabled && !final_state.proxy.auth_enabled && final_state.password.is_none());
        let mut stale_credential = delete;
        stale_credential["expectedPolicyRevision"] = removed["result"]["revision"].clone();
        let (stale, _) = mutate(&fixture, "updateNetwork", stale_credential, None).await;
        assert_eq!(stale["result"]["kind"], "credential_stale");
        let observation = fixture.store.operation_observation(&reference).await.unwrap().unwrap();
        assert_eq!(observation.outcome, OperationOutcome::Returned { value: saved });
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}
