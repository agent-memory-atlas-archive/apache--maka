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

#[path = "tests/selection.rs"]
mod selection;

#[path = "tests/settings.rs"]
mod settings;

#[path = "tests/product.rs"]
mod product;

use super::*;
use crate::{EXPORT, ModelSelection, contract, service::Agent};
use maka_event_log::root::{RootNamespaces, RootOwner};
use maka_plugin_core::{
    composition::{
        Configuration as NodeConfiguration, Environment, ExecutionLocation, Input, InstanceId,
        Node, PackageUse, Target,
    },
    contract::{Availability, Cardinality, Export, Import},
    operation::{OperationOutcome, Receipt},
    package::{Declaration, EntryDeclaration, EntryTarget, Implementation, Realm},
};
use maka_plugins::{
    call::{Attachments, Identity, Issuer, Scope},
    package::{MANIFEST_FILE, Package},
};
use maka_runtime::configuration::*;
use maka_runtime_host::application::{
    Application, Binding, Invocation as HostInvocation, NativeActivation, NativeRegistry, Policy,
};
use serde_json::json;
use std::sync::Mutex as StdMutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

struct Grant(Issuer);
impl Policy for Grant {
    fn recover(
        &self,
        invocation: maka_runtime_host::application::RecoveryInvocation<'_>,
    ) -> Result<(), String> {
        if self.0.evidence::<bool>(invocation.scope) == Some(&true) {
            Ok(())
        } else {
            Err("denied".into())
        }
    }
    fn admit(&self, invocation: HostInvocation<'_>) -> Result<(), String> {
        if self.0.evidence::<bool>(invocation.scope) == Some(&true) {
            Ok(())
        } else {
            Err("denied".into())
        }
    }
}
fn scope(issuer: &Issuer, grant: bool) -> Scope {
    issuer
        .issue_with(
            Identity::Remote {
                request_id: Uuid::new_v4(),
            },
            grant,
            CancellationToken::new(),
        )
        .unwrap()
}
fn id(value: u128) -> InstanceId {
    InstanceId(vec![Uuid::from_u128(value)])
}
fn package(provider: bool) -> Package {
    let entry = EntryDeclaration {
        scene: None,
        implementation: if provider {
            Implementation::Native {
                implementation: IMPLEMENTATION.into(),
            }
        } else {
            Implementation::JavaScript {
                module: "host.mjs".into(),
                realm: Realm::Shared,
            }
        },
        configuration: if provider {
            configuration_schema()
        } else {
            json!({"type":"object","additionalProperties":false})
        },
        imports: if provider {
            BTreeMap::new()
        } else {
            BTreeMap::from([(
                EXPORT.into(),
                Import {
                    contract: contract(),
                    cardinality: Cardinality::One,
                },
            )])
        },
        exports: if provider {
            BTreeMap::from([(
                EXPORT.into(),
                Export {
                    contract: contract(),
                    availability: Availability::Required,
                },
            )])
        } else {
            BTreeMap::new()
        },
    };
    let declaration = Declaration {
        entries: BTreeMap::from([(EntryTarget::Host, entry)]),
        composition: None,
    };
    let mut files = BTreeMap::from([(MANIFEST_FILE.into(), serde_json::to_vec(&json!({"schemaVersion":2,"id":if provider {"agent"} else {"consumer"},"application":declaration})).unwrap())]);
    if !provider {
        files.insert(
            "host.mjs".into(),
            b"export function activate() { return {}; }".to_vec(),
        );
    }
    Package::new(files).unwrap()
}
fn input(settings: &Configuration) -> (Input, Vec<Package>) {
    let provider = package(true);
    let consumer = package(false);
    let node = |key: u128, provider_node: bool| Node {
        key: Uuid::from_u128(key),
        package: if provider_node {
            provider.application_ref()
        } else {
            consumer.application_ref()
        }
        .into(),
        use_package: PackageUse::Entry,
        configuration: if provider_node {
            NodeConfiguration {
                values: serde_json::to_value(settings)
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                parameters: BTreeMap::new(),
            }
        } else {
            NodeConfiguration::default()
        },
        parameters: NodeConfiguration::default(),
        disabled: false,
        overrides: vec![],
        bindings: if provider_node {
            BTreeMap::new()
        } else {
            BTreeMap::from([(
                EXPORT.into(),
                vec![Target::Instance {
                    path: vec!["provider".into()],
                    export: EXPORT.into(),
                }],
            )])
        },
    };
    let receipt_contract = maka_plugin_core::operation::receipt_reader_contract();
    let receipts = Package::new(BTreeMap::from([(
        MANIFEST_FILE.into(),
        serde_json::to_vec(&json!({
            "schemaVersion":2,"id":"receipts","application":{"entries":{"host":{
                "implementation":{"kind":"native","implementation":"maka.operations.receipts"},
                "configuration":{"type":"object","additionalProperties":false},"imports":{},
                "exports":{"Receipts":{"contract":receipt_contract,"availability":"required"}}
            }}}
        }))
        .unwrap(),
    )]))
    .unwrap();
    let receipt_node = Node {
        key: Uuid::from_u128(4),
        package: receipts.application_ref().into(),
        use_package: PackageUse::Entry,
        configuration: Default::default(),
        parameters: Default::default(),
        bindings: BTreeMap::new(),
        disabled: false,
        overrides: vec![],
    };
    let mut input = Input {
        environment: Environment {
            location: ExecutionLocation::Host,
            presentation: None,
        },
        instances: BTreeMap::from([
            ("provider".into(), node(1, true)),
            ("a".into(), node(2, false)),
            ("b".into(), node(3, false)),
        ]),
        launch: None,
    };
    input.instances.insert("receipts".into(), receipt_node);
    for name in ["a", "b"] {
        input.instances.get_mut(name).unwrap().bindings.insert(
            "receipts".into(),
            vec![Target::Instance {
                path: vec!["receipts".into()],
                export: "Receipts".into(),
            }],
        );
    }
    // Both consumers declare recovery independently of their execution import.
    let mut files = consumer.files().clone();
    let mut manifest = consumer.manifest().clone();
    manifest
        .application
        .as_mut()
        .unwrap()
        .entries
        .get_mut(&EntryTarget::Host)
        .unwrap()
        .imports
        .insert(
            "receipts".into(),
            Import {
                contract: receipt_contract,
                cardinality: Cardinality::One,
            },
        );
    files.insert(MANIFEST_FILE.into(), serde_json::to_vec(&manifest).unwrap());
    let consumer = Package::new(files).unwrap();
    for name in ["a", "b"] {
        input.instances.get_mut(name).unwrap().package = consumer.application_ref().into();
    }
    let settings = {
        let mut manifest = provider.manifest().clone();
        manifest.id = "settings".into();
        let entry = manifest
            .application
            .as_mut()
            .unwrap()
            .entries
            .get_mut(&EntryTarget::Host)
            .unwrap();
        entry.implementation = Implementation::Native {
            implementation: maka_settings_service::IMPLEMENTATION.into(),
        };
        entry.configuration = json!({"type":"object","properties":{},"additionalProperties":false});
        entry.exports = BTreeMap::from([(
            maka_settings_service::EXPORT.into(),
            Export {
                contract: maka_settings_service::contract::contract(),
                availability: Availability::Required,
            },
        )]);
        Package::new(BTreeMap::from([(
            MANIFEST_FILE.into(),
            serde_json::to_vec(&manifest).unwrap(),
        )]))
        .unwrap()
    };
    let mut files = consumer.files().clone();
    let mut manifest = consumer.manifest().clone();
    manifest
        .application
        .as_mut()
        .unwrap()
        .entries
        .get_mut(&EntryTarget::Host)
        .unwrap()
        .imports
        .insert(
            "settings".into(),
            Import {
                contract: maka_settings_service::contract::contract(),
                cardinality: Cardinality::One,
            },
        );
    files.insert(MANIFEST_FILE.into(), serde_json::to_vec(&manifest).unwrap());
    let consumer = Package::new(files).unwrap();
    input.instances.insert(
        "settings".into(),
        Node {
            key: Uuid::from_u128(5),
            package: settings.application_ref().into(),
            use_package: PackageUse::Entry,
            configuration: NodeConfiguration::default(),
            parameters: NodeConfiguration::default(),
            disabled: false,
            overrides: vec![],
            bindings: BTreeMap::new(),
        },
    );
    for name in ["a", "b"] {
        let node = input.instances.get_mut(name).unwrap();
        node.package = consumer.application_ref().into();
        node.bindings.insert(
            "settings".into(),
            vec![Target::Instance {
                path: vec!["settings".into()],
                export: maka_settings_service::EXPORT.into(),
            }],
        );
    }
    (input, vec![provider, consumer, receipts, settings])
}
struct Fixture {
    _dir: tempfile::TempDir,
    namespaces: RootNamespaces,
    root: std::path::PathBuf,
    store: Arc<EventLog>,
    config: Arc<ConfigurationStore>,
    app: Application,
    issuer: Issuer,
    owner: Arc<Owner>,
}
impl Fixture {
    async fn new(base: &str, cap: usize, model: Option<ModelExecutor>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let namespaces = RootNamespaces {
            ownership: dir.path().join("owners"),
            control: dir.path().join("control"),
        };
        let root_owner = Arc::new(RootOwner::create(&root, &namespaces).unwrap());
        let store = Arc::new(EventLog::for_root(root_owner.clone()).await.unwrap());
        let config = Arc::new(ConfigurationStore::for_root(root_owner).await.unwrap());
        let provider = json!({"packageId":maka_providers::api::ID,"entryId":maka_providers::api::ID,"scope":"profile","name":"openai-compatible"});
        let create = serde_json::from_value(json!({"expectedCatalogRevision":0,"connection":{"slug":"fixture","name":"Fixture","provider":provider,"configuration":{"baseUrl":base},"enabled":true,"enabledModelIds":["fixture-model"]}})).unwrap();
        let CatalogMutationResult::Committed {
            connection: Some(basis),
            ..
        } = config.create_connection(create).await.unwrap()
        else {
            panic!("connection commit");
        };
        let maka_config::oauth::enrollment::LoginPreparation::Ready(ticket) = config
            .prepare_oauth_login(maka_runtime::oauth::LoginStart {
                expected_credential: None,
                attempt_id: Uuid::new_v4().to_string(),
                target: maka_runtime::oauth::Target::Existing {
                    expected: ConnectionCredentialTarget {
                        connection_id: basis.connection_id.clone(),
                        revision: basis.revision,
                        provider: serde_json::from_value(provider).unwrap(),
                        slug: "fixture".into(),
                        configuration: json!({"baseUrl":base}),
                    },
                    configuration: json!({"baseUrl":base}),
                },
                authentication: maka_runtime::provider::AuthenticationInput {
                    method: "api-key".into(),
                    input: json!({"apiKey":"private-fixture-key"}),
                },
            })
            .await
            .unwrap()
        else {
            panic!("actual login preparation");
        };
        assert!(ticket.claim().await.unwrap());
        assert!(matches!(
            ticket
                .complete(
                    maka_runtime::provider::Credential {
                        secret: "private-fixture-key".into(),
                        refresh_at: None
                    },
                    1
                )
                .await
                .unwrap(),
            maka_config::oauth::enrollment::LoginCompletion::Committed(_)
        ));
        let settings = Configuration {
            max_active_runs: cap,
            max_steps: 1,
        };
        let (input, packages) = input(&settings);
        let deployment = Uuid::new_v4();
        store
            .commit_application(deployment, 0, input, vec![], packages)
            .await
            .unwrap();
        let captured = Arc::new(StdMutex::new(None));
        let capture = captured.clone();
        let replacement = Arc::new(StdMutex::new(model));
        let mut native = NativeRegistry::default();
        maka_settings_service::provide(&mut native).unwrap();
        // Actual Host NativeContext and original NativeOwner; only the model
        // implementation is controlled in failure tests, never Root/admission.
        native
            .insert(
                IMPLEMENTATION.into(),
                Arc::new(move |context, configuration| {
                    let capture = capture.clone();
                    let replacement = replacement.lock().unwrap().take();
                    Box::pin(async move {
                        let mut owner = Owner::open(context, configuration).await.unwrap();
                        if let Some(model) = replacement {
                            let store = owner.store.clone();
                            Arc::get_mut(&mut owner).unwrap().engine = Engine::new(
                                store,
                                model,
                                maka_js_runtime::CodeExecutor::new(1, Default::default()).unwrap(),
                            );
                        }
                        *capture.lock().unwrap() = Some(owner.clone());
                        let service = Arc::new(Agent(owner));
                        NativeActivation {
                            owner: service.clone(),
                            exports: Ok(BTreeMap::from([(
                                EXPORT.into(),
                                maka_plugin_core::runtime::Published {
                                    contract: contract(),
                                    value: service
                                        as Arc<dyn maka_runtime_host::application::Handler>,
                                },
                            )])),
                        }
                    })
                }),
            )
            .unwrap();
        let issuer = Issuer::default();
        let app = Application::start_from_store(
            store.clone(),
            deployment,
            native,
            issuer.clone(),
            Arc::new(Grant(issuer.clone())),
            Default::default(),
        )
        .await
        .unwrap();
        app.ready().await.unwrap();
        let owner = captured.lock().unwrap().take().unwrap();
        Self {
            _dir: dir,
            namespaces,
            root,
            store,
            config,
            app,
            issuer,
            owner,
        }
    }
    async fn binding(&self, consumer: u128) -> Binding {
        let client = self.app.client();
        client
            .bind(client.activation(&id(consumer)).unwrap(), EXPORT.into())
            .await
            .unwrap()
            .remove(0)
    }
    async fn mutation(
        &self,
        consumer: u128,
        method: &str,
        input: Value,
        operation: Uuid,
    ) -> Result<Value, String> {
        let client = self.app.client();
        let prepared = client
            .prepare_mutation(
                self.binding(consumer).await,
                method.into(),
                input,
                scope(&self.issuer, true),
                operation,
                Attachments::new(),
            )
            .await?;
        let recorded = self
            .store
            .record_application_intent(
                prepared.reference().source.clone(),
                prepared.consumer().instance().clone(),
                prepared.reference().clone(),
            )
            .await
            .unwrap();
        let reply = client.mutate(prepared, recorded).await?.result().await;
        match reply.receipt {
            Receipt::Recorded {
                outcome: OperationOutcome::Returned { value },
                ..
            } => Ok(value),
            Receipt::Recorded {
                outcome: OperationOutcome::Unknown { diagnostic },
                ..
            } => Err(diagnostic),
            Receipt::Unavailable { diagnostic } => Err(diagnostic),
            other => panic!("unexpected receipt {other:?}"),
        }
    }
    async fn create(&self, title: &str) -> String {
        self.mutation(2, "create", json!({"title":title}), Uuid::new_v4())
            .await
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .into()
    }
    async fn selection(&self) -> ModelSelection {
        let row = self.config.catalog().await.unwrap().connections.remove(0);
        ModelSelection {
            connection_id: row.connection_id,
            revision: row.revision,
            model_id: "fixture-model".into(),
        }
    }
    async fn send(&self, session: &str) -> Result<Value, String> {
        self.mutation(
            2,
            "send",
            json!({"sessionId":session,"message":"Hello","model":self.selection().await}),
            Uuid::new_v4(),
        )
        .await
    }
    async fn query(&self, method: &str, input: Value) -> Result<Value, String> {
        self.app
            .client()
            .start(
                self.binding(2).await,
                method.into(),
                input,
                scope(&self.issuer, true),
                Attachments::new(),
            )
            .await?
            .result()
            .await
    }
    async fn drained(&self) {
        while !self.owner.workers.is_empty() {
            tokio::task::yield_now().await;
        }
    }
}
async fn request(socket: &mut TcpStream) -> Value {
    request_with_key(socket, "private-fixture-key").await
}
async fn request_with_key(socket: &mut TcpStream, key: &str) -> Value {
    let mut bytes = Vec::new();
    let boundary = loop {
        let mut buffer = [0; 4096];
        let count = socket.read(&mut buffer).await.unwrap();
        assert_ne!(count, 0);
        bytes.extend_from_slice(&buffer[..count]);
        assert!(bytes.len() < 512 * 1024);
        if let Some(at) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break at + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..boundary]);
    assert!(
        headers
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {key}"))
    );
    let length: usize = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().unwrap())
        })
        .unwrap();
    while bytes.len() < boundary + length {
        let mut buffer = [0; 4096];
        let count = socket.read(&mut buffer).await.unwrap();
        assert_ne!(count, 0);
        bytes.extend_from_slice(&buffer[..count]);
    }
    serde_json::from_slice(&bytes[boundary..boundary + length]).unwrap()
}
async fn response(socket: &mut TcpStream) {
    let body = "data: {\"id\":\"fixture\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Elegant response\"},\"finish_reason\":null}]}\n\ndata: {\"id\":\"fixture\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"total_tokens\":3}}\n\ndata: [DONE]\n\n";
    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
}

#[tokio::test]
async fn canonical_history_is_bounded_utf8_and_keeps_its_original_fence() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let reply = "中文👩🏽‍💻\u{1}".repeat(700);
        let response_text = reply.clone();
        let server = tokio::spawn(async move {
            for turn in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                request(&mut socket).await;
                let text = if turn == 0 { &response_text } else { "Later response" };
                let delta = json!({"id":"history","choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]});
                let body = format!("data: {delta}\n\ndata: {{\"id\":\"history\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}]}}\n\ndata: [DONE]\n\n");
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let fixture = Fixture::new(&base, 1, None).await;
        let session = fixture.create("History").await;
        let message = format!("{}中文", "\u{0}".repeat(9000));
        fixture.mutation(2, "send", json!({"sessionId":session,"message":message,"model":fixture.selection().await}), Uuid::new_v4()).await.unwrap();
        fixture.drained().await;
        let mut input = json!({"sessionId":session,"position":{"kind":"start"}});
        let mut user = String::new();
        let mut assistant = String::new();
        let mut fence = None;
        let mut later = false;
        let mut finished = false;
        for _ in 0..128 {
            let page = fixture.query("history", input.clone()).await.unwrap();
            assert!(serde_json::to_vec(&page).unwrap().len() < 64 * 1024);
            assert!(!page.to_string().contains("private-fixture-key"));
            let through = page["through"].as_u64().unwrap();
            assert_eq!(*fence.get_or_insert(through), through);
            input["through"] = json!(through);
            if page["kind"] == "preparing" { continue; }
            let chunks = page["chunks"].as_array().unwrap();
            assert!(chunks.len() <= 2);
            for chunk in chunks {
                let text = chunk["text"].as_str().unwrap();
                assert!(text.len() <= 4096);
                match chunk["role"].as_str().unwrap() {
                    "user" => user.push_str(text),
                    "assistant" => assistant.push_str(text),
                    role => panic!("unexpected conversation role {role}"),
                }
                assert!(chunk.get("attachments").is_none());
            }
            if !later {
                fixture.send(&session).await.unwrap();
                fixture.drained().await;
                later = true;
            }
            if page["next"].is_null() { finished = true; break; }
            input["position"] = json!({"kind":"cursor","cursor":page["next"]});
        }
        assert!(finished);
        assert_eq!(user, message);
        assert_eq!(assistant, reply);
        let event_fence = fence.unwrap();
        let mut input = json!({"sessionId":session,"position":{"kind":"event","sequence":event_fence + 1}});
        let mut newer_fence = None;
        let mut newer_user = String::new();
        let mut newer_assistant = String::new();
        let mut finished = false;
        for _ in 0..128 {
            let page = fixture.query("history", input.clone()).await.unwrap();
            let through = page["through"].as_u64().unwrap();
            assert_eq!(*newer_fence.get_or_insert(through), through);
            input["through"] = json!(through);
            if page["kind"] == "preparing" { continue; }
            for chunk in page["chunks"].as_array().unwrap() {
                assert!(chunk["sequence"].as_u64().unwrap() > maka_presentation::watermark(event_fence).unwrap());
                match chunk["role"].as_str().unwrap() {
                    "user" => newer_user.push_str(chunk["text"].as_str().unwrap()),
                    "assistant" => newer_assistant.push_str(chunk["text"].as_str().unwrap()),
                    role => panic!("unexpected conversation role {role}"),
                }
            }
            if page["next"].is_null() { finished = true; break; }
            input["position"] = json!({"kind":"cursor","cursor":page["next"]});
        }
        assert!(finished);
        assert_eq!(newer_user, "Hello");
        assert_eq!(newer_assistant, "Later response");
        assert!(fixture.query("history", json!({"sessionId":session,"position":{"kind":"cursor","cursor":{"sequence":0,"offset":0}}})).await.is_err());
        assert!(fixture.query("history", json!({"sessionId":"foreign_session","position":{"kind":"start"}})).await.is_err());
        server.await.unwrap();
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}
#[tokio::test]
async fn actual_http_runs_are_bounded_and_observer_loss_does_not_release_capacity() {
    tokio::time::timeout(Duration::from_secs(30),async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1",listener.local_addr().unwrap());
        let entered = Arc::new(Semaphore::new(0)); let release = Arc::new(Semaphore::new(0));
        let worker_entered = entered.clone(); let worker_release = release.clone();
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket,_) = listener.accept().await.unwrap();
                let body = request(&mut socket).await; assert_eq!(body["model"],"fixture-model");
                worker_entered.add_permits(1); worker_release.acquire().await.unwrap().forget(); response(&mut socket).await;
            }
        });
        let fixture = Fixture::new(&base,2,None).await;
        let first = fixture.create("First").await; let second = fixture.create("Second").await; let third = fixture.create("Third").await;
        let started = fixture.send(&first).await.unwrap(); entered.acquire().await.unwrap().forget();
        let active = fixture.query("observe", json!({"sessionId":first})).await.unwrap();
        assert_eq!(active["session"]["id"], first);
        assert_eq!(active["run"]["invocationId"], started["invocationId"]);
        assert_eq!(active["run"]["state"], "running");
        assert!(!active.to_string().contains("private-fixture-key"));
        let mut watch = fixture.app.client().subscribe(fixture.binding(2).await,"watch".into(),json!({"sessionId":first,"after":0}),scope(&fixture.issuer,true),Attachments::new()).await.unwrap();
        let page = watch.recv().await.unwrap(); assert!(!page.to_string().contains("private-fixture-key"));
        drop(watch);
        fixture.send(&second).await.unwrap();
        assert!(fixture.send(&third).await.unwrap_err().contains("agent_capacity_exhausted"));
        let unopened = fixture.store.session_projection::<Value>(&third).await.unwrap().unwrap(); assert!(unopened.root_turn.is_none());
        assert_eq!(fixture.owner.runs.lock().await.controls.len(),2);
        release.add_permits(2); server.await.unwrap(); fixture.drained().await;
        assert!(fixture.owner.runs.lock().await.controls.is_empty()); assert_eq!(fixture.owner.permits.available_permits(),2);
        let completed = fixture.query("observe", json!({"sessionId":first})).await.unwrap();
        assert!(completed["run"].is_null());
        assert_eq!(completed["parts"], json!([]));
        assert!(completed["through"].as_u64().unwrap() >= active["through"].as_u64().unwrap());
        let observation = fixture.store.session_projection::<Value>(&first).await.unwrap().unwrap();
        let events = fixture.store.session_stream_events(&first,0,observation.through_sequence,32,64*1024).await.unwrap();
        assert!(events.events.iter().any(|event| matches!(&event.fact,maka_event_log::observation::StreamFact::PartDelta{text,..} if text=="Elegant response")));
        fixture.app.shutdown().await.unwrap(); fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}
#[tokio::test]
async fn original_operation_conflicts_and_private_namespace_never_cross_sessions() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let fixture = Fixture::new("http://127.0.0.1:1/v1", 2, None).await;
        let operation = Uuid::new_v4();
        let first = fixture
            .mutation(2, "create", json!({"title":"A"}), operation)
            .await
            .unwrap();
        let conflict = fixture
            .mutation(3, "create", json!({"title":"B"}), operation)
            .await
            .unwrap_err();
        assert!(conflict.contains("event identity conflicts"));
        let second = fixture
            .mutation(3, "create", json!({"title":"B"}), Uuid::new_v4())
            .await
            .unwrap();
        assert_ne!(first["id"], second["id"]);
        fixture
            .store
            .create_session("foreign_session", "foreign", &json!({"foreign":true}), 1)
            .await
            .unwrap();
        let listed = fixture
            .app
            .client()
            .start(
                fixture.binding(2).await,
                "list".into(),
                json!({}),
                scope(&fixture.issuer, true),
                Attachments::new(),
            )
            .await
            .unwrap()
            .result()
            .await
            .unwrap();
        assert_eq!(listed["sessions"].as_array().unwrap().len(), 2);
        assert!(!listed.to_string().contains("foreign_session"));
        assert!(
            fixture
                .send("foreign_session")
                .await
                .unwrap_err()
                .contains("agent_session_unavailable")
        );
        assert!(
            fixture
                .app
                .client()
                .prepare_mutation(
                    fixture.binding(2).await,
                    "send".into(),
                    json!({"sessionId":first["id"],"message":"Unauthorized","model":fixture.selection().await}),
                    scope(&fixture.issuer, false),
                    Uuid::new_v4(),
                    Attachments::new(),
                )
                .await
                .is_err()
        );
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn exact_stop_cannot_cancel_a_successor_and_close_waits_for_original_workers() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let entered = Arc::new(Semaphore::new(0));
        let release = Arc::new(Semaphore::new(0));
        let worker_entered = entered.clone();
        let worker_release = release.clone();
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                request(&mut socket).await;
                worker_entered.add_permits(1);
                worker_release.acquire().await.unwrap().forget();
                response(&mut socket).await;
            }
        });
        let fixture = Fixture::new(&base, 2, None).await;
        let session = fixture.create("Conversation").await;
        let first = fixture.send(&session).await.unwrap();
        entered.acquire().await.unwrap().forget();
        release.add_permits(1);
        fixture.drained().await;
        let second = fixture.send(&session).await.unwrap();
        entered.acquire().await.unwrap().forget();
        let stale = fixture
            .mutation(
                2,
                "stop",
                json!({"sessionId":session,"invocationId":first["invocationId"]}),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        assert_eq!(stale["requested"], false);
        let runs = fixture.owner.runs.lock().await;
        let control = runs.controls.get(&session).unwrap();
        assert_eq!(control.invocation, second["invocationId"].as_str().unwrap());
        assert!(!control.cancellation.is_cancelled());
        drop(runs);
        release.add_permits(1);
        server.await.unwrap();
        fixture.drained().await;
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[derive(Clone)]
struct Fault {
    panic: bool,
    entered: Arc<Semaphore>,
    release: Arc<Semaphore>,
    store: Arc<StdMutex<Option<Arc<EventLog>>>>,
}
impl maka_plugins::model::ProviderAdapter for Fault {
    fn open(
        &self,
        _: maka_plugins::model::Lifetime,
        _: CancellationToken,
    ) -> BoxFuture<'_, Result<Arc<dyn maka_plugins::model::Session>, maka_model::ModelError>> {
        let session = self.clone();
        Box::pin(async { Ok(Arc::new(session) as Arc<dyn maka_plugins::model::Session>) })
    }
}
impl maka_plugins::model::Session for Fault {
    fn stream(
        &self,
        _: maka_plugins::model::Request,
        _: maka_plugins::model::Context,
    ) -> BoxFuture<'static, Result<(), maka_model::ModelError>> {
        let fault = self.clone();
        Box::pin(async move {
            fault.entered.add_permits(1);
            fault.release.acquire().await.unwrap().forget();
            // Real EventLog SQL owner shuts down after durable start. Later
            // model-interruption and terminal writes must fail independently.
            let store = fault.store.lock().unwrap().as_ref().unwrap().clone();
            store.shutdown().await.unwrap();
            assert!(!fault.panic, "controlled original model worker panic");
            Err(maka_model::ModelError::Adapter(
                "ordinary model failure".into(),
            ))
        })
    }
}
fn controlled_model(
    panic: bool,
) -> (
    ModelExecutor,
    Fault,
    maka_plugins::Registration,
    maka_plugins::fiber::Fiber,
) {
    let fault = Fault {
        panic,
        entered: Arc::new(Semaphore::new(0)),
        release: Arc::new(Semaphore::new(0)),
        store: Arc::new(StdMutex::new(None)),
    };
    let catalog = maka_plugins::contributions::Catalog::default();
    let fiber = maka_plugins::fiber::Fiber::new(
        "fixture.adapter",
        "fixture.adapter",
        maka_runtime::scope::Scope::Profile,
    )
    .unwrap();
    fiber.begin_loading().unwrap();
    fiber.ready().unwrap();
    fiber.publish().unwrap();
    let mut staged = maka_plugins::contributions::Staged::default();
    staged
        .insert(
            "chat-completions",
            maka_plugins::model::Adapter {
                provider: Arc::new(fault.clone()),
            },
        )
        .unwrap();
    let registration = catalog.register(&fiber.context(), staged).unwrap();
    (
        ModelExecutor::new(1, Duration::from_secs(30))
            .unwrap()
            .with_catalog(catalog),
        fault,
        registration,
        fiber,
    )
}
async fn cleanup_case(panic: bool) {
    let (model, fault, registration, fiber) = controlled_model(panic);
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, Some(model)).await;
    *fault.store.lock().unwrap() = Some(fixture.store.clone());
    let session = fixture.create("Physical lifetime").await;
    fixture.send(&session).await.unwrap();
    fault.entered.acquire().await.unwrap().forget();
    fault.release.add_permits(1);
    fixture.drained().await;
    assert_eq!(
        fixture.owner.runs.lock().await.quarantined.len(),
        usize::from(panic)
    );
    assert_eq!(
        fixture.owner.permits.available_permits(),
        usize::from(!panic)
    );
    let close = fixture.app.shutdown().await;
    assert_eq!(close.is_err(), panic);
    fixture.config.shutdown().await.unwrap();
    fault.store.lock().unwrap().take();
    let Fixture {
        _dir,
        namespaces,
        root,
        store,
        config,
        app,
        issuer,
        owner,
    } = fixture;
    drop(owner);
    drop(issuer);
    drop(app);
    drop(config);
    drop(store);
    drop(fault);
    drop(registration);
    drop(fiber);
    if panic {
        assert_eq!(
            RootOwner::open(&root, &namespaces).err().unwrap().kind(),
            std::io::ErrorKind::WouldBlock
        );
    } else {
        drop(RootOwner::open(&root, &namespaces).unwrap());
    }
}
#[tokio::test]
async fn model_panic_and_terminal_sql_failure_quarantine_capacity_and_keep_root() {
    tokio::time::timeout(Duration::from_secs(20), cleanup_case(true))
        .await
        .unwrap();
}
#[tokio::test]
async fn ordinary_model_and_terminal_sql_failures_preserve_confirmed_physical_cleanup() {
    tokio::time::timeout(Duration::from_secs(20), cleanup_case(false))
        .await
        .unwrap();
}

#[tokio::test]
async fn public_availability_resumes_stable_sessions_and_separates_deployments() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
        let session = fixture.create("Stable conversation").await;
        let deployment = fixture.app.source().unwrap().deployment;
        fixture.app.shutdown().await.unwrap();
        let mut native = NativeRegistry::default();
        maka_settings_service::provide(&mut native).unwrap();
        crate::provide(&mut native).unwrap();
        let resumed = Application::start_from_store(
            fixture.store.clone(),
            deployment,
            native,
            fixture.issuer.clone(),
            Arc::new(Grant(fixture.issuer.clone())),
            Default::default(),
        )
        .await
        .unwrap();
        resumed.ready().await.unwrap();
        let client = resumed.client();
        let binding = client
            .bind(client.activation(&id(2)).unwrap(), EXPORT.into())
            .await
            .unwrap()
            .remove(0);
        let listed = client
            .start(
                binding.clone(),
                "list".into(),
                json!({}),
                scope(&fixture.issuer, true),
                Attachments::new(),
            )
            .await
            .unwrap()
            .result()
            .await
            .unwrap();
        assert_eq!(listed["sessions"][0]["id"], session);
        let other_deployment = Uuid::new_v4();
        let (input, packages) = input(&fixture.owner.settings);
        fixture
            .store
            .commit_application(other_deployment, 0, input, vec![], packages)
            .await
            .unwrap();
        let mut native = NativeRegistry::default();
        maka_settings_service::provide(&mut native).unwrap();
        crate::provide(&mut native).unwrap();
        let other = Application::start_from_store(
            fixture.store.clone(),
            other_deployment,
            native,
            fixture.issuer.clone(),
            Arc::new(Grant(fixture.issuer.clone())),
            Default::default(),
        )
        .await
        .unwrap();
        other.ready().await.unwrap();
        let client = other.client();
        let binding = client
            .bind(client.activation(&id(2)).unwrap(), EXPORT.into())
            .await
            .unwrap()
            .remove(0);
        let listed = client
            .start(
                binding.clone(),
                "list".into(),
                json!({}),
                scope(&fixture.issuer, true),
                Attachments::new(),
            )
            .await
            .unwrap()
            .result()
            .await
            .unwrap();
        assert!(listed["sessions"].as_array().unwrap().is_empty());
        let mut foreign = client
            .subscribe(
                binding,
                "watch".into(),
                json!({"sessionId":session,"after":0}),
                scope(&fixture.issuer, true),
                Attachments::new(),
            )
            .await
            .unwrap();
        assert!(foreign.recv().await.is_none());
        assert!(
            foreign
                .completion()
                .await
                .result
                .unwrap_err()
                .contains("agent_session_unavailable")
        );
        other.shutdown().await.unwrap();
        resumed.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

async fn disabled_connection() {
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    let session = fixture.create("Bound model").await;
    let catalog = fixture.config.catalog().await.unwrap();
    let row = &catalog.connections[0];
    let expected: ConnectionVersionBasis =
        serde_json::from_value(json!({"connectionId":row.connection_id,"revision":row.revision}))
            .unwrap();
    let updated = fixture
        .config
        .update_connection(UpdateCatalogConnectionInput {
            expected,
            changes: ConnectionCatalogEntryUpdate {
                name: Some(row.name.clone()),
                configuration: Some(row.configuration.clone()),
                enabled: Some(false),
                enabled_model_ids: Some(row.enabled_model_ids.clone()),
                model_overrides: Patch::Keep,
                request_body_overlay: Patch::Keep,
            },
        })
        .await
        .unwrap();
    assert!(matches!(updated, CatalogMutationResult::Committed { .. }));
    let revision = fixture.config.catalog().await.unwrap().revision;
    let alternative = serde_json::from_value(json!({"expectedCatalogRevision":revision,"connection":{"slug":"available","name":"Available","provider":row.provider,"configuration":row.configuration,"enabled":true,"enabledModelIds":["fixture-model"]}})).unwrap();
    assert!(matches!(
        fixture.config.create_connection(alternative).await.unwrap(),
        CatalogMutationResult::Committed { .. }
    ));
    let alternative_row = fixture
        .config
        .catalog()
        .await
        .unwrap()
        .connections
        .into_iter()
        .find(|row| row.slug == "available")
        .unwrap();
    let maka_config::oauth::enrollment::LoginPreparation::Ready(ticket) = fixture
        .config
        .prepare_oauth_login(maka_runtime::oauth::LoginStart {
            expected_credential: None,
            attempt_id: Uuid::new_v4().to_string(),
            target: maka_runtime::oauth::Target::Existing {
                expected: ConnectionCredentialTarget {
                    connection_id: alternative_row.connection_id.clone(),
                    revision: alternative_row.revision,
                    slug: alternative_row.slug,
                    provider: alternative_row.provider,
                    configuration: alternative_row.configuration.clone(),
                },
                configuration: alternative_row.configuration,
            },
            authentication: maka_runtime::provider::AuthenticationInput {
                method: "api-key".into(),
                input: json!({"apiKey":"private-alternative-key"}),
            },
        })
        .await
        .unwrap()
    else {
        panic!("alternative login preparation");
    };
    assert!(ticket.claim().await.unwrap());
    assert!(matches!(
        ticket
            .complete(
                maka_runtime::provider::Credential {
                    secret: "private-alternative-key".into(),
                    refresh_at: None
                },
                2
            )
            .await
            .unwrap(),
        maka_config::oauth::enrollment::LoginCompletion::Committed(_)
    ));
    let result = fixture
        .config
        .set_default_target(SetDefaultConnectionTargetInput {
            expected_catalog_revision: fixture.config.catalog().await.unwrap().revision,
            target: Some(ConnectionTarget {
                connection_id: alternative_row.connection_id,
                model_id: "fixture-model".into(),
            }),
        })
        .await
        .unwrap();
    assert!(matches!(result, CatalogMutationResult::Committed { .. }));
    let error = fixture.send(&session).await.unwrap_err();
    assert!(error.contains("agent_execution_unavailable"));
    assert!(!error.contains("private-fixture-key") && !error.contains("private-alternative-key"));
    let observation = fixture
        .store
        .session_projection::<Value>(&session)
        .await
        .unwrap()
        .unwrap();
    assert!(observation.root_turn.is_none());
    assert_eq!(fixture.owner.permits.available_permits(), 1);
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}
#[tokio::test]
async fn disabled_selected_connection_does_not_use_an_authenticated_default_or_open_a_run() {
    tokio::time::timeout(Duration::from_secs(20), disabled_connection())
        .await
        .unwrap();
}
