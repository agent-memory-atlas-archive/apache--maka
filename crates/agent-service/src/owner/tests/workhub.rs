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
use crate::workhub as hub;

async fn activate(fixture: &mut Fixture, install: bool) {
    let source = fixture.app.source().unwrap().clone();
    fixture.app.shutdown().await.unwrap();
    if install {
        let (mut composition, mut packages) = input(&fixture.owner.settings);
        for package in &mut packages {
            let previous = package.application_ref();
            let mut manifest = package.manifest().clone();
            let entry = manifest
                .application
                .as_mut()
                .unwrap()
                .entries
                .get_mut(&EntryTarget::Host)
                .unwrap();
            if manifest.id == "agent" {
                entry.implementation = Implementation::Native {
                    implementation: hub::IMPLEMENTATION.into(),
                };
                entry.exports.insert(
                    hub::EXPORT.into(),
                    Export {
                        contract: hub::contract(),
                        availability: Availability::Required,
                    },
                );
            } else if manifest.id == "consumer" {
                entry.imports.insert(
                    hub::EXPORT.into(),
                    Import {
                        contract: hub::contract(),
                        cardinality: Cardinality::One,
                    },
                );
            } else {
                continue;
            }
            let mut files = package.files().clone();
            files.insert(MANIFEST_FILE.into(), serde_json::to_vec(&manifest).unwrap());
            *package = Package::new(files).unwrap();
            for node in composition
                .instances
                .values_mut()
                .filter(|node| node.package == previous.clone().into())
            {
                node.package = package.application_ref().into();
                if manifest.id == "consumer" {
                    node.bindings.insert(
                        hub::EXPORT.into(),
                        vec![Target::Instance {
                            path: vec!["provider".into()],
                            export: hub::EXPORT.into(),
                        }],
                    );
                }
            }
        }
        fixture
            .store
            .commit_application(
                source.deployment,
                source.revision,
                composition,
                vec![],
                packages,
            )
            .await
            .unwrap();
    }
    let mut native = NativeRegistry::default();
    crate::provide(&mut native).unwrap();
    maka_settings_service::provide(&mut native).unwrap();
    fixture.app = Application::start_from_store(
        fixture.store.clone(),
        source.deployment,
        native,
        fixture.issuer.clone(),
        Arc::new(Grant(fixture.issuer.clone(), None)),
        Default::default(),
    )
    .await
    .unwrap();
    fixture.app.ready().await.unwrap();
}
async fn binding(fixture: &Fixture) -> Binding {
    let client = fixture.app.client();
    client
        .bind(client.activation(&id(2)).unwrap(), hub::EXPORT.into())
        .await
        .unwrap()
        .remove(0)
}
async fn coordinator(fixture: &Fixture) -> Value {
    fixture
        .app
        .client()
        .start(
            binding(fixture).await,
            "coordinator".into(),
            json!({}),
            scope(&fixture.issuer, true),
            Attachments::default().into(),
        )
        .await
        .unwrap()
        .result()
        .await
        .unwrap()
}
async fn prepare(
    fixture: &Fixture,
    selection: Value,
    operation: Uuid,
) -> maka_runtime_host::application::PreparedMutation {
    fixture
        .app
        .client()
        .prepare_mutation(
            binding(fixture).await,
            "resolve".into(),
            json!({"workspace":selection}),
            scope(&fixture.issuer, true),
            operation,
            Attachments::default().into(),
        )
        .await
        .unwrap()
}
async fn resolve(fixture: &Fixture, selection: Value, operation: Uuid) -> Value {
    let prepared = prepare(fixture, selection, operation).await;
    let recorded = fixture
        .store
        .record_application_intent(
            prepared.reference().source.clone(),
            prepared.consumer().instance().clone(),
            prepared.reference().clone(),
            Value::Null,
        )
        .await
        .unwrap();
    let reply = fixture
        .app
        .client()
        .mutate(prepared, recorded)
        .await
        .unwrap()
        .result()
        .await;
    match reply.receipt {
        Receipt::Recorded {
            outcome: OperationOutcome::Returned { value },
            cleanup: maka_plugin_core::operation::OperationCleanup::Confirmed,
        } => value,
        other => panic!("unexpected coordinator receipt {other:?}"),
    }
}
async fn selection(fixture: &Fixture) -> Value {
    fixture
        .query("directories", json!({"kind":"roots"}))
        .await
        .unwrap()["roots"][0]["selection"]
        .clone()
}

#[tokio::test]
async fn workhub_concurrent_resolution_reactivation_and_old_agent_namespace() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let mut fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
        let old = fixture.create("ordinary Agent").await;
        activate(&mut fixture, true).await;
        assert_eq!(coordinator(&fixture).await, json!({"kind":"missing"}));
        assert!(
            fixture
                .query("observe", json!({"sessionId":old}))
                .await
                .is_err()
        );
        let selected = selection(&fixture).await;
        let op = Uuid::new_v4();
        let (a, b) = tokio::join!(
            resolve(&fixture, selected.clone(), op),
            resolve(&fixture, selected.clone(), Uuid::new_v4())
        );
        assert_eq!(a, b);
        assert_eq!(a["kind"], "ready");
        assert_ne!(a["session"]["id"], old);
        assert_eq!(resolve(&fixture, selected.clone(), op).await, a);
        assert_eq!(coordinator(&fixture).await, a);
        activate(&mut fixture, false).await;
        // Resolving an accepted original coordinator never revalidates a later path.
        std::fs::rename(&fixture.workspace, fixture.workspace.with_extension("gone")).unwrap();
        assert_eq!(resolve(&fixture, selected, Uuid::new_v4()).await, a);
        assert_eq!(coordinator(&fixture).await, a);
        let page = fixture
            .query("list", json!({"filter":"all","position":{"kind":"start"}}))
            .await
            .unwrap();
        assert_eq!(page["sessions"].as_array().unwrap().len(), 1);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn workhub_invalid_directory_does_not_freeze_an_intent() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let mut fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
        activate(&mut fixture, true).await;
        let selected = selection(&fixture).await;
        let mut invalid = selected.clone();
        invalid["rootId"] = json!("unknown");
        assert_eq!(
            resolve(&fixture, invalid, Uuid::new_v4()).await,
            json!({"kind":"rejected","reason":"workspace_unavailable"})
        );
        assert_eq!(coordinator(&fixture).await, json!({"kind":"missing"}));
        assert_eq!(
            resolve(&fixture, selected, Uuid::new_v4()).await["kind"],
            "ready"
        );
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn workhub_persisted_intent_recovers_original_identity_before_session_creation() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let mut fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
        activate(&mut fixture, true).await;
        let selected = selection(&fixture).await;
        let prepared = prepare(&fixture, selected.clone(), Uuid::new_v4()).await;
        let reference = prepared.reference().clone();
        drop(prepared);
        let prefix = namespace(
            &reference.source.state_domain,
            reference.source.deployment,
            &reference.namespace,
        );
        let prefix = format!("wh_{}", &prefix[3..]);
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&reference).unwrap())
        );
        let expected = format!("{}{}", prefix, &digest[..32]);
        fixture
            .store
            .plugin_data_batch(
                &maka_plugins::storage::Namespace::new(
                    hub::IMPLEMENTATION,
                    maka_runtime::scope::Scope::Profile,
                )
                .unwrap(),
                vec![maka_plugins::storage::Mutation {
                    key: format!("{prefix}coordinator"),
                    expected_revision: None,
                    data: maka_plugins::storage::Data::Present(
                        json!({"operation":reference,"workspace":selected}),
                    ),
                }],
            )
            .await
            .unwrap();
        assert_eq!(coordinator(&fixture).await, json!({"kind":"pending"}));
        activate(&mut fixture, false).await;
        let mut different = selected.clone();
        different["rootId"] = json!("ignored-later-choice");
        let ready = resolve(&fixture, different, Uuid::new_v4()).await;
        assert_eq!(ready["session"]["id"], expected);
        assert_eq!(coordinator(&fixture).await, ready);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn workhub_uses_original_agent_engine_and_freezes_coordinator_prompt() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut fixture = Fixture::new(
            &format!("http://{}/v1", listener.local_addr().unwrap()),
            1,
            None,
        )
        .await;
        activate(&mut fixture, true).await;
        let ready = resolve(&fixture, selection(&fixture).await, Uuid::new_v4()).await;
        let session = ready["session"]["id"].as_str().unwrap();
        let sent = fixture.send(session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = request(&mut socket).await;
        assert!(
            request["messages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|message| message["role"] == "system"
                    && message["content"]
                        .as_str()
                        .unwrap()
                        .contains("WorkHub coordinator"))
        );
        response(&mut socket).await;
        loop {
            let execution = fixture
                .query(
                    "execution",
                    json!({"sessionId":session,"invocationId":sent["invocationId"]}),
                )
                .await
                .unwrap();
            if execution["execution"]["state"] == "completed" {
                break;
            }
            tokio::task::yield_now().await;
        }
        let boundary = fixture
            .store
            .invocation_boundary(session, sent["invocationId"].as_str().unwrap())
            .await
            .unwrap()
            .unwrap()
            .0;
        let config = fixture
            .store
            .invocation_configuration(&boundary.invocation)
            .await
            .unwrap()
            .unwrap();
        let prompt = config.system_prompt.unwrap();
        assert!(prompt.text.contains("WorkHub coordinator"));
        assert_eq!(prompt.sources[0].name, hub::IMPLEMENTATION);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn workhub_queued_message_keeps_original_behavior_across_activation() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut fixture = Fixture::new(&format!("http://{}/v1", listener.local_addr().unwrap()), 1, None).await;
        activate(&mut fixture, true).await;
        let ready = resolve(&fixture, selection(&fixture).await, Uuid::new_v4()).await;
        let session = ready["session"]["id"].as_str().unwrap().to_owned();
        let sent = fixture.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        let queued = fixture.mutation(2, "send", json!({"sessionId":session,"message":"Follow up after shutdown","model":fixture.selection().await}), Uuid::new_v4()).await.unwrap();
        assert_eq!(queued["kind"], "queued");
        let queue = fixture.store.message_queue(&session).await.unwrap();
        let original = queue.entries.iter().find(|entry| entry.source.message.message_id == queued["messageId"]).unwrap().execution.basis["behavior"].clone();
        assert!(original["text"].as_str().unwrap().contains("WorkHub coordinator"));
        activate(&mut fixture, false).await;
        drop(socket);
        let (mut socket, _) = listener.accept().await.unwrap();
        let delivered = request(&mut socket).await;
        assert!(delivered["messages"].to_string().contains("Follow up after shutdown"));
        response(&mut socket).await;
        let invocation = loop {
            let observed = fixture.query("observe", json!({"sessionId":session})).await.unwrap();
            if observed["run"].is_null() {
                let boundary = fixture.store.observe_session::<SessionConfiguration>(&session).await.unwrap().unwrap().root_turn.unwrap();
                if boundary.invocation.invocation_id != sent["invocationId"] {
                    break boundary.invocation;
                }
            }
            tokio::task::yield_now().await;
        };
        let config = fixture.store.invocation_configuration(&invocation).await.unwrap().unwrap();
        assert_eq!(serde_json::to_value(config.system_prompt.unwrap()).unwrap(), original);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}
