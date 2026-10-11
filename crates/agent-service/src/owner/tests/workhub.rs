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
use maka_runtime::event::Fact;
use maka_runtime_host::application::{Handler, NativeOwner};

async fn activate(fixture: &mut Fixture, install: bool) {
    activate_workers(fixture, install, false, false).await
}
async fn activate_workers(fixture: &mut Fixture, install: bool, workers: bool, granted: bool) {
    activate_worker_mode(fixture, install, workers, granted, WorkerBehavior::Returned).await
}
async fn activate_worker_mode(
    fixture: &mut Fixture,
    install: bool,
    workers: bool,
    granted: bool,
    behavior: WorkerBehavior,
) {
    let source = fixture.app.source().unwrap().clone();
    fixture.app.shutdown().await.unwrap();
    if install {
        let (mut composition, mut packages) = input(&fixture.owner.settings);
        let worker_package = if workers {
            let base = package(true);
            let mut manifest = base.manifest().clone();
            manifest.id = "worker".into();
            if !matches!(behavior, WorkerBehavior::Returned) {
                manifest
                    .application
                    .as_mut()
                    .unwrap()
                    .entries
                    .get_mut(&EntryTarget::Host)
                    .unwrap()
                    .implementation = Implementation::Native {
                    implementation: "fixture.lost-agent".into(),
                };
            }
            Some(
                Package::new(BTreeMap::from([(
                    MANIFEST_FILE.into(),
                    serde_json::to_vec(&manifest).unwrap(),
                )]))
                .unwrap(),
            )
        } else {
            None
        };
        if let Some(worker) = &worker_package {
            let mut node = composition.instances["provider"].clone();
            node.key = Uuid::from_u128(21);
            node.package = worker.application_ref().into();
            composition.instances.insert("worker".into(), node);
        }
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
                entry.configuration = hub::configuration_schema();
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
            if workers {
                entry.imports.insert(
                    "worker".into(),
                    Import {
                        contract: contract(),
                        cardinality: Cardinality::One,
                    },
                );
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
                if manifest.id == "agent" {
                    node.configuration = NodeConfiguration {
                        values: BTreeMap::from([
                            ("agent".into(), {
                                let mut settings =
                                    serde_json::to_value(&fixture.owner.settings).unwrap();
                                if workers {
                                    settings["maxSteps"] = json!(2);
                                }
                                settings
                            }),
                            (
                                "workers".into(),
                                if workers {
                                    json!(["worker"])
                                } else {
                                    json!([])
                                },
                            ),
                        ]),
                        parameters: BTreeMap::new(),
                    };
                }
                if workers {
                    node.bindings.insert(
                        "worker".into(),
                        vec![Target::Instance {
                            path: vec!["worker".into()],
                            export: EXPORT.into(),
                        }],
                    );
                }
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
        if let Some(worker) = worker_package {
            packages.push(worker);
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
    if !matches!(behavior, WorkerBehavior::Returned) {
        native
            .insert(
                "fixture.lost-agent".into(),
                Arc::new(move |context, settings| {
                    let behavior = behavior.clone();
                    Box::pin(async move {
                        let owner = Arc::new(Worker(
                            Agent(Owner::open(context, settings).await.unwrap()),
                            behavior,
                        ));
                        NativeActivation {
                            owner: owner.clone(),
                            exports: Ok(BTreeMap::from([(
                                EXPORT.into(),
                                maka_plugin_core::runtime::Published {
                                    contract: contract(),
                                    value: owner as Arc<dyn Handler>,
                                },
                            )])),
                        }
                    })
                }),
            )
            .unwrap();
    }
    maka_settings_service::provide(&mut native).unwrap();
    fixture.app = Application::start_from_store(
        fixture.store.clone(),
        source.deployment,
        native,
        fixture.issuer.clone(),
        Arc::new(Grant(
            fixture.issuer.clone(),
            Some(Arc::new(
                maka_application_bridge::transport::PeerPolicy::with_native_grants(if granted {
                    vec![maka_application_bridge::transport::NativeGrant {
                        origin: id(1),
                        consumer: id(1),
                        provider: id(21),
                        export: EXPORT.into(),
                        methods: ["list", "send", "execution", "messageExecution"]
                            .into_iter()
                            .map(str::to_owned)
                            .collect(),
                        recovery: true,
                    }]
                } else {
                    vec![]
                })
                .unwrap(),
            )),
        )),
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

async fn imported(fixture: &Fixture, import: &str) -> Binding {
    let client = fixture.app.client();
    client
        .bind(client.activation(&id(2)).unwrap(), import.into())
        .await
        .unwrap()
        .remove(0)
}
async fn query_import(fixture: &Fixture, import: &str, method: &str, input: Value) -> Value {
    fixture
        .app
        .client()
        .start(
            imported(fixture, import).await,
            method.into(),
            input,
            scope(&fixture.issuer, true),
            Attachments::default().into(),
        )
        .await
        .unwrap()
        .result()
        .await
        .unwrap()
}
async fn worker_mutation(fixture: &Fixture, method: &str, input: Value) -> Value {
    let client = fixture.app.client();
    let prepared = client
        .prepare_mutation(
            imported(fixture, "worker").await,
            method.into(),
            input,
            scope(&fixture.issuer, true),
            Uuid::new_v4(),
            Attachments::default().into(),
        )
        .await
        .unwrap();
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
    let reply = client
        .mutate(prepared, recorded)
        .await
        .unwrap()
        .result()
        .await;
    let Receipt::Recorded {
        outcome: OperationOutcome::Returned { value },
        ..
    } = reply.receipt
    else {
        panic!("{reply:?}")
    };
    value
}
async fn worker_session(fixture: &Fixture) -> String {
    let roots = query_import(fixture, "worker", "directories", json!({"kind":"roots"})).await;
    worker_mutation(fixture,"create",json!({"title":"Worker","labels":[],"workspace":{"kind":"directory","selection":roots["roots"][0]["selection"]}})).await["session"]["id"].as_str().unwrap().into()
}
async fn task_response(socket: &mut TcpStream, input: Value) {
    let delta = json!({"id":"task-model","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"route-call","type":"function","function":{"name":"workhub_tasks","arguments":input.to_string()}}]},"finish_reason":null}]});
    let body = format!(
        "data: {delta}\n\ndata: {{\"id\":\"task-model\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"tool_calls\"}}]}}\n\ndata: [DONE]\n\n"
    );
    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
}
async fn first_task(fixture: &Fixture) -> String {
    loop {
        let tasks = query_import(fixture, hub::EXPORT, "tasks", json!({})).await;
        if let Some(id) = tasks["entries"][0]["taskId"].as_str() {
            return id.into();
        }
        tokio::task::yield_now().await;
    }
}
async fn idle(fixture: &Fixture, import: &str, session: &str) {
    loop {
        let observation =
            query_import(fixture, import, "observe", json!({"sessionId":session})).await;
        if observation["run"].is_null() {
            return;
        }
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn workhub_real_model_routes_original_input_and_recovers_child_receipt_without_redispatch() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut fixture=Fixture::new(&format!("http://{}/v1",listener.local_addr().unwrap()),1,None).await;
        activate_workers(&mut fixture,true,true,true).await;
        let worker=worker_session(&fixture).await;
        let ready=resolve(&fixture,selection(&fixture).await,Uuid::new_v4()).await;
        let coordinator=ready["session"]["id"].as_str().unwrap().to_owned();
        fixture.send(&coordinator).await.unwrap();
        let (mut socket,_)=listener.accept().await.unwrap();
        let initial=request(&mut socket).await;
        assert!(initial["tools"].to_string().contains("workhub_tasks"));
        task_response(&mut socket,json!({"operation":"route","worker":"worker","sessionId":worker,"note":"Process this assigned work","model":fixture.selection().await})).await;
        let mut worker_calls=0;
        for _ in 0..2 {
            let (mut socket,_)=listener.accept().await.unwrap();
            let input=request(&mut socket).await;
            if input["messages"].to_string().contains("Original user request:") {
                worker_calls+=1;
                assert!(input["messages"].to_string().contains("Hello"));
                assert!(input["messages"].to_string().contains("Process this assigned work"));
            } else {assert!(input["messages"].to_string().contains("wt_"));}
            response(&mut socket).await;
        }
        assert_eq!(worker_calls,1);
        idle(&fixture,EXPORT,&coordinator).await;
        idle(&fixture,"worker",&worker).await;
        let task_id=first_task(&fixture).await;
        let task=query_import(&fixture,hub::EXPORT,"task",json!({"taskId":task_id})).await;
        assert_eq!(task["status"]["kind"],"started");
        assert_eq!(task["status"]["progress"]["execution"]["state"],"completed");
        let originals=fixture.store.application_intents(fixture.app.source().unwrap().deployment,id(1),0,128).await.unwrap();
        assert_eq!(originals.len(),1);
        assert_eq!(originals[0].reference.namespace,id(21));
        assert_eq!(originals[0].reference.method,"send");
        assert_eq!(task["operation"], originals[0].reference.operation.to_string());
        activate_workers(&mut fixture,false,true,true).await;
        let restored=query_import(&fixture,hub::EXPORT,"task",json!({"taskId":task_id})).await;
        assert_eq!(restored["status"]["invocationId"],task["status"]["invocationId"]);
        assert_eq!(restored["operation"],task["operation"]);
        // Opaque replacement bindings cannot reinterpret the original worker receipt.
        assert!(restored["status"]["progress"].is_null());
        assert!(tokio::time::timeout(Duration::from_millis(30),listener.accept()).await.is_err());
        fixture.app.shutdown().await.unwrap();fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn workhub_routing_to_busy_worker_follows_exact_queued_message_after_later_work() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut fixture=Fixture::new(&format!("http://{}/v1",listener.local_addr().unwrap()),1,None).await;
        activate_workers(&mut fixture,true,true,true).await;
        let worker=worker_session(&fixture).await;
        let busy=worker_mutation(&fixture,"send",json!({"sessionId":worker,"message":"Unrelated running work","model":fixture.selection().await})).await;
        let (mut running,_)=listener.accept().await.unwrap();request(&mut running).await;
        let ready=resolve(&fixture,selection(&fixture).await,Uuid::new_v4()).await;
        let coordinator=ready["session"]["id"].as_str().unwrap().to_owned();
        fixture.send(&coordinator).await.unwrap();
        let (mut socket,_)=listener.accept().await.unwrap();request(&mut socket).await;
        task_response(&mut socket,json!({"operation":"route","worker":"worker","sessionId":worker,"note":"Queued assignment","model":fixture.selection().await})).await;
        let (mut socket,_)=listener.accept().await.unwrap();request(&mut socket).await;response(&mut socket).await;
        let task_id=first_task(&fixture).await;
        let queued=query_import(&fixture,hub::EXPORT,"task",json!({"taskId":task_id})).await;
        assert_eq!(queued["status"]["kind"],"queued");
        assert_eq!(queued["status"]["progress"]["kind"],"pending");
        response(&mut running).await;
        let (mut socket,_)=listener.accept().await.unwrap();
        let input=request(&mut socket).await;assert!(input["messages"].to_string().contains("Queued assignment"));response(&mut socket).await;
        idle(&fixture,"worker",&worker).await;
        let delivered=query_import(&fixture,hub::EXPORT,"task",json!({"taskId":task_id})).await;
        assert_eq!(delivered["status"]["progress"]["kind"],"delivered");
        assert_eq!(delivered["status"]["progress"]["shared"],false);
        assert_ne!(delivered["status"]["progress"]["execution"]["invocationId"],busy["invocationId"]);
        worker_mutation(&fixture,"send",json!({"sessionId":worker,"message":"Later separate work","model":fixture.selection().await})).await;
        let (mut socket,_)=listener.accept().await.unwrap();request(&mut socket).await;response(&mut socket).await;
        idle(&fixture,"worker",&worker).await;
        let original=query_import(&fixture,hub::EXPORT,"task",json!({"taskId":task_id})).await;
        assert_eq!(original,delivered,"task follows its message, never the Session tip");
        fixture.app.shutdown().await.unwrap();fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn workhub_selected_worker_requires_independent_native_dispatch_grant() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut fixture=Fixture::new(&format!("http://{}/v1",listener.local_addr().unwrap()),1,None).await;
        activate_workers(&mut fixture,true,true,false).await;
        let worker=worker_session(&fixture).await;
        let ready=resolve(&fixture,selection(&fixture).await,Uuid::new_v4()).await;
        let coordinator=ready["session"]["id"].as_str().unwrap().to_owned();
        fixture.send(&coordinator).await.unwrap();
        let (mut socket,_)=listener.accept().await.unwrap();request(&mut socket).await;
        task_response(&mut socket,json!({"operation":"route","worker":"worker","sessionId":worker,"note":"Denied assignment","model":fixture.selection().await})).await;
        let (mut socket,_)=listener.accept().await.unwrap();
        let next=request(&mut socket).await;assert!(!next["messages"].to_string().contains("Original user request:"));response(&mut socket).await;
        idle(&fixture,EXPORT,&coordinator).await;
        let task_id=first_task(&fixture).await;
        let task=query_import(&fixture,hub::EXPORT,"task",json!({"taskId":task_id})).await;
        assert_eq!(task["status"]["kind"],"pending");
        let worker_view=query_import(&fixture,"worker","observe",json!({"sessionId":worker})).await;
        assert!(worker_view["run"].is_null());
        assert!(fixture.store.application_intents(fixture.app.source().unwrap().deployment,id(1),0,128).await.unwrap().is_empty());
        assert!(tokio::time::timeout(Duration::from_millis(30),listener.accept()).await.is_err());
        // The durable assignment precedes dispatch. New grants and activation cannot
        // convert that old intent into a fresh call on a replacement binding.
        activate_workers(&mut fixture,false,true,true).await;
        fixture.send(&coordinator).await.unwrap();
        let (mut socket,_)=listener.accept().await.unwrap();request(&mut socket).await;
        task_response(&mut socket,json!({"operation":"advance","taskId":task_id})).await;
        let (mut socket,_)=listener.accept().await.unwrap();let input=request(&mut socket).await;
        assert!(input["messages"].to_string().contains("unavailable"));
        assert!(!input["messages"].to_string().contains("Original user request:"));response(&mut socket).await;
        idle(&fixture,EXPORT,&coordinator).await;
        assert!(fixture.store.application_intents(fixture.app.source().unwrap().deployment,id(1),0,128).await.unwrap().is_empty());
        fixture.app.shutdown().await.unwrap();fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[derive(Clone)]
enum WorkerBehavior {
    Returned,
    Lost,
    Blocked(Arc<Blocked>),
}
struct Blocked {
    method: &'static str,
    entered: tokio::sync::Semaphore,
    cancelled: tokio::sync::Semaphore,
    release: tokio::sync::Semaphore,
}
struct Worker(Agent, WorkerBehavior);
impl Handler for Worker {
    fn call(
        &self,
        method: String,
        input: Value,
        context: maka_runtime_host::application::Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let owner = self.0.0.clone();
        let behavior = self.1.clone();
        Box::pin(async move {
            if let WorkerBehavior::Blocked(blocked) = &behavior
                && method == blocked.method
            {
                blocked.entered.add_permits(1);
                context.scope().cancellation.cancelled().await;
                blocked.cancelled.add_permits(1);
                blocked.release.acquire().await.unwrap().forget();
            }
            let value = Agent(owner).call(method.clone(), input, context).await?;
            if matches!(behavior, WorkerBehavior::Lost) && method == "send" {
                Err("fixture lost reply after real worker admission".into())
            } else {
                Ok(value)
            }
        })
    }
}
impl NativeOwner for Worker {
    fn activated(&self) {
        self.0.0.activated();
    }
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        self.0.0.close()
    }
}
#[tokio::test]
async fn workhub_unknown_worker_admission_keeps_original_receipt_across_source_update_and_advance()
{
    tokio::time::timeout(Duration::from_secs(60), async {
        let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut fixture=Fixture::new(&format!("http://{}/v1",listener.local_addr().unwrap()),1,None).await;
        activate_worker_mode(&mut fixture,true,true,true,WorkerBehavior::Lost).await;
        let worker=worker_session(&fixture).await;
        let ready=resolve(&fixture,selection(&fixture).await,Uuid::new_v4()).await;
        let coordinator=ready["session"]["id"].as_str().unwrap().to_owned();
        fixture.send(&coordinator).await.unwrap();
        let (mut socket,_)=listener.accept().await.unwrap();request(&mut socket).await;
        task_response(&mut socket,json!({"operation":"route","worker":"worker","sessionId":worker,"note":"Executed despite lost reply","model":fixture.selection().await})).await;
        let mut effects=0;
        for _ in 0..2 {
            let (mut socket,_)=listener.accept().await.unwrap();let input=request(&mut socket).await;
            if input["messages"].to_string().contains("Original user request:") {effects+=1;} else {assert!(input["messages"].to_string().contains("uncertain"));}
            response(&mut socket).await;
        }
        idle(&fixture,EXPORT,&coordinator).await;idle(&fixture,"worker",&worker).await;
        assert_eq!(effects,1);
        let task_id=first_task(&fixture).await;
        let original=query_import(&fixture,hub::EXPORT,"task",json!({"taskId":task_id})).await;
        assert_eq!(original["status"]["kind"],"uncertain");
        let intents=fixture.store.application_intents(fixture.app.source().unwrap().deployment,id(1),0,128).await.unwrap();
        assert_eq!(intents.len(),1);
        let reference=intents[0].reference.clone();
        activate_workers(&mut fixture,true,true,true).await;
        assert!(fixture.app.source().unwrap().revision > reference.source.revision);
        assert_eq!(query_import(&fixture,hub::EXPORT,"task",json!({"taskId":task_id})).await,original);
        fixture.send(&coordinator).await.unwrap();
        let (mut socket,_)=listener.accept().await.unwrap();request(&mut socket).await;
        task_response(&mut socket,json!({"operation":"advance","taskId":task_id})).await;
        let (mut socket,_)=listener.accept().await.unwrap();let input=request(&mut socket).await;
        assert!(!input["messages"].to_string().contains("Original user request:"));
        assert!(input["messages"].to_string().contains("uncertain"));response(&mut socket).await;
        idle(&fixture,EXPORT,&coordinator).await;
        assert_eq!(query_import(&fixture,hub::EXPORT,"task",json!({"taskId":task_id})).await,original);
        let receipt=fixture.store.operation_observation(&reference).await.unwrap().unwrap();
        assert!(matches!(receipt.outcome,OperationOutcome::Unknown {..}));
        assert_eq!(fixture.store.application_intents(fixture.app.source().unwrap().deployment,id(1),0,128).await.unwrap().len(),1);
        assert!(tokio::time::timeout(Duration::from_millis(30),listener.accept()).await.is_err());
        fixture.app.shutdown().await.unwrap();fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn workhub_cancelled_worker_queries_and_mutations_reach_cleanup_budget() {
    for method in ["list", "send"] {
        tokio::time::timeout(Duration::from_secs(30), async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let mut fixture = Fixture::new(&format!("http://{}/v1",listener.local_addr().unwrap()),1,None).await;
            let blocked = Arc::new(Blocked { method, entered: tokio::sync::Semaphore::new(0), cancelled: tokio::sync::Semaphore::new(0), release: tokio::sync::Semaphore::new(0) });
            activate_worker_mode(&mut fixture,true,true,true,WorkerBehavior::Blocked(blocked.clone())).await;
            let worker = worker_session(&fixture).await;
            let ready = resolve(&fixture,selection(&fixture).await,Uuid::new_v4()).await;
            let coordinator = ready["session"]["id"].as_str().unwrap().to_owned();
            let sent = fixture.send(&coordinator).await.unwrap();
            let (mut socket,_) = listener.accept().await.unwrap();request(&mut socket).await;
            task_response(&mut socket, if method == "list" { json!({"operation":"sessions","worker":"worker"}) } else { json!({"operation":"route","worker":"worker","sessionId":worker,"note":"Blocked admission","model":fixture.selection().await}) }).await;
            blocked.entered.acquire().await.unwrap().forget();
            fixture.mutation(2,"stop",json!({"sessionId":coordinator,"invocationId":sent["invocationId"]}),Uuid::new_v4()).await.unwrap();
            blocked.cancelled.acquire().await.unwrap().forget();
            // Physical callback stays owned beyond the original five-second budget.
            tokio::time::timeout(Duration::from_secs(8),idle(&fixture,EXPORT,&coordinator)).await.unwrap();
            assert!(fixture.app.client().snapshot().instances.iter().any(|instance| instance.activation.instance() == &id(21) && instance.calls > 0));
            let facts = fixture.store.scoped_prefix(maka_runtime::event::LogScope::Session { id: coordinator.clone() },256,4*1024*1024).await.unwrap();
            assert!(!facts.events.iter().any(|event| matches!(event.event.fact,Fact::ToolSettled { .. })));
            blocked.release.add_permits(1);
            assert!(fixture.app.shutdown().await.is_err());
            assert!(!fixture.app.client().snapshot().closed);
            fixture.config.shutdown().await.unwrap();
            eprintln!("Retained WorkHub cleanup-failure Root: {}",fixture._dir.keep().display());
        }).await.unwrap();
    }
}

#[tokio::test]
async fn workhub_queued_input_cannot_adopt_reactivated_worker_bindings() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut fixture=Fixture::new(&format!("http://{}/v1",listener.local_addr().unwrap()),1,None).await;
        activate_workers(&mut fixture,true,true,true).await;
        let ready=resolve(&fixture,selection(&fixture).await,Uuid::new_v4()).await;
        let session=ready["session"]["id"].as_str().unwrap().to_owned();
        fixture.send(&session).await.unwrap();
        let (mut socket,_)=listener.accept().await.unwrap();request(&mut socket).await;
        let queued=fixture.mutation(2,"send",json!({"sessionId":session,"message":"Keep original workers","model":fixture.selection().await}),Uuid::new_v4()).await.unwrap();
        assert_eq!(queued["kind"],"queued");
        activate_workers(&mut fixture,false,true,true).await;drop(socket);
        let resolved=loop {
            let value=fixture.query("messageExecution",json!({"sessionId":session,"messageId":queued["messageId"]})).await.unwrap();
            if value["kind"] == "delivered" {break value;}
            tokio::task::yield_now().await;
        };
        assert_eq!(resolved["execution"]["state"],"failed");
        let facts=fixture.store.scoped_prefix(maka_runtime::event::LogScope::Session {id:session},256,4*1024*1024).await.unwrap();
        assert!(facts.events.iter().any(|event| matches!(&event.event.fact,Fact::InvocationEnded {outcome:maka_runtime::event::InvocationOutcome::Failed {message:Some(message),..}} if message == "agent_tools_changed")));
        assert!(tokio::time::timeout(Duration::from_millis(30),listener.accept()).await.is_err());
        fixture.app.shutdown().await.unwrap();fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}
