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
use maka_plugin_core::{
    contract::{Contract, Interface, Method, MethodMode, Transport},
    runtime::Published,
};
use maka_runtime::{
    archive::outcome_projection,
    event::{Fact, ToolOutcome},
    tool_output::DurableToolProjection,
};
use maka_runtime_host::application::{Context, Handler, NativeOwner};
use std::sync::atomic::{AtomicUsize, Ordering};

fn counter_contract() -> Contract {
    let write = Method {
        mode: MethodMode::Mutation,
        input: json!({"type":"object","properties":{"amount":{"type":"integer","minimum":1}},"required":["amount"],"additionalProperties":false}),
        output: json!({"type":"object","properties":{"effects":{"type":"integer"},"payload":{"type":"string"}},"required":["effects"],"additionalProperties":false}),
        resources: BTreeMap::new(),
    };
    let read = Method {
        mode: MethodMode::Query,
        ..write.clone()
    };
    Contract {
        name: "fixture.Counter".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([("write".into(), write), ("read".into(), read)]),
        },
    }
}
#[derive(Clone, Copy)]
enum Behavior {
    Returned,
    Unknown,
    CleanupFailed,
    LargeCleanupFailed,
    Blocked,
}
struct Counter {
    behavior: Behavior,
    effects: AtomicUsize,
    entered: Arc<Semaphore>,
    cancelled: Arc<Semaphore>,
    release: Arc<Semaphore>,
}
impl Counter {
    fn new(behavior: Behavior) -> Arc<Self> {
        Arc::new(Self {
            behavior,
            effects: AtomicUsize::new(0),
            entered: Arc::new(Semaphore::new(0)),
            cancelled: Arc::new(Semaphore::new(0)),
            release: Arc::new(Semaphore::new(0)),
        })
    }
}
impl Handler for Counter {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let count = if method == "read" {
            assert!(context.operation().is_none());
            self.effects.load(Ordering::SeqCst)
        } else {
            assert!(context.operation().is_some());
            self.effects
                .fetch_add(input["amount"].as_u64().unwrap() as usize, Ordering::SeqCst)
                + 1
        };
        let behavior = self.behavior;
        let entered = self.entered.clone();
        let cancelled = self.cancelled.clone();
        let release = self.release.clone();
        Box::pin(async move {
            entered.add_permits(1);
            match behavior {
                Behavior::Returned => Ok(json!({"effects":count})),
                Behavior::Unknown => Err("lost reply after original effect".into()),
                Behavior::CleanupFailed | Behavior::LargeCleanupFailed => {
                    let mut ticket = context.scope().resources.reserve().unwrap();
                    ticket.start();
                    ticket.complete(Err("counter cleanup failed".into()));
                    if matches!(behavior, Behavior::LargeCleanupFailed) {
                        let value = json!({"effects":count,"payload":"x".repeat(1024*1024-128)});
                        assert!(serde_json::to_vec(&value).unwrap().len() <= 1024 * 1024);
                        Ok(value)
                    } else {
                        Ok(json!({"effects":count}))
                    }
                }
                Behavior::Blocked => {
                    context.scope().cancellation.cancelled().await;
                    cancelled.add_permits(1);
                    release.acquire().await.unwrap().forget();
                    Ok(json!({"effects":count}))
                }
            }
        })
    }
}
impl NativeOwner for Counter {
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}
pub(super) struct Setup {
    counter: Option<Arc<Counter>>,
    granted: bool,
    query: bool,
}
impl Setup {
    fn javascript() -> Self {
        Self {
            counter: None,
            granted: true,
            query: false,
        }
    }
    fn native(counter: Arc<Counter>, granted: bool) -> Self {
        Self {
            counter: Some(counter),
            granted,
            query: false,
        }
    }
    pub(super) fn method(&self) -> &str {
        if self.query { "read" } else { "write" }
    }
    pub(super) fn install(&self, input: &mut Input, packages: &mut Vec<Package>) {
        let mut manifest = packages[0].manifest().clone();
        manifest
            .application
            .as_mut()
            .unwrap()
            .entries
            .get_mut(&EntryTarget::Host)
            .unwrap()
            .imports
            .insert(
                "counter".into(),
                Import {
                    contract: counter_contract(),
                    cardinality: Cardinality::One,
                },
            );
        let provider = Package::new(BTreeMap::from([(
            MANIFEST_FILE.into(),
            serde_json::to_vec(&manifest).unwrap(),
        )]))
        .unwrap();
        input.instances.get_mut("provider").unwrap().package = provider.application_ref().into();
        input
            .instances
            .get_mut("provider")
            .unwrap()
            .bindings
            .insert(
                "counter".into(),
                vec![Target::Instance {
                    path: vec!["counter".into()],
                    export: "Counter".into(),
                }],
            );
        packages[0] = provider;
        let implementation = if self.counter.is_some() {
            json!({"kind":"native","implementation":"fixture.counter"})
        } else {
            json!({"kind":"javascript","module":"counter.mjs","realm":"shared"})
        };
        let mut files = BTreeMap::from([(MANIFEST_FILE.into(), serde_json::to_vec(&json!({"schemaVersion":2,"id":"fixture.counter","application":{"entries":{"host":{
            "implementation":implementation,"configuration":{"type":"object","additionalProperties":false},"imports":{},
            "exports":{"Counter":{"contract":counter_contract(),"availability":"required"}}
        }}}})).unwrap())]);
        if self.counter.is_none() {
            files.insert("counter.mjs".into(), format!(r#"let effects=0; export function activate(){{return {{Counter:{{contract:{},service:['counter']}}}};}} export const counter={{write({{amount}}){{return {{effects:effects+=amount}};}},read(){{return {{effects}};}}}};"#, serde_json::to_string(&counter_contract()).unwrap()).into_bytes());
        }
        let package = Package::new(files).unwrap();
        input.instances.insert(
            "counter".into(),
            Node {
                key: Uuid::from_u128(21),
                package: package.application_ref().into(),
                use_package: PackageUse::Entry,
                configuration: Default::default(),
                parameters: Default::default(),
                disabled: false,
                overrides: vec![],
                bindings: BTreeMap::new(),
            },
        );
        packages.push(package);
    }
    pub(super) fn provide(&self, registry: &mut NativeRegistry) {
        if let Some(counter) = &self.counter {
            let counter = counter.clone();
            registry
                .insert(
                    "fixture.counter".into(),
                    Arc::new(move |_, _| {
                        let counter = counter.clone();
                        Box::pin(async move {
                            NativeActivation {
                                exports: Ok(BTreeMap::from([(
                                    "Counter".into(),
                                    Published {
                                        contract: counter_contract(),
                                        value: counter.clone() as Arc<dyn Handler>,
                                    },
                                )])),
                                owner: counter,
                            }
                        })
                    }),
                )
                .unwrap();
        }
    }
    pub(super) fn policy(&self) -> Arc<maka_application_bridge::transport::PeerPolicy> {
        use maka_application_bridge::transport::{NativeGrant, PeerPolicy};
        Arc::new(
            PeerPolicy::with_native_grants(if self.granted {
                vec![NativeGrant {
                    origin: id(1),
                    consumer: id(1),
                    provider: id(21),
                    export: "Counter".into(),
                    methods: [self.method().into()].into(),
                    recovery: true,
                }]
            } else {
                vec![]
            })
            .unwrap(),
        )
    }
}
async fn tool_response(socket: &mut TcpStream) {
    let delta = json!({"id":"tool-model","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"counter-call","type":"function","function":{"name":"Counter","arguments":"{\"amount\":1}"}}]},"finish_reason":null}]});
    let body = format!(
        "data: {delta}\n\ndata: {{\"id\":\"tool-model\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"tool_calls\"}}]}}\n\ndata: [DONE]\n\n"
    );
    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
}
async fn facts(fixture: &Fixture, session: &str) -> Vec<Fact> {
    fixture
        .store
        .scoped_prefix(
            maka_runtime::event::LogScope::Session { id: session.into() },
            256,
            4 * 1024 * 1024,
        )
        .await
        .unwrap()
        .events
        .into_iter()
        .map(|event| event.event.fact)
        .collect()
}
#[tokio::test]
async fn installed_javascript_service_is_a_real_model_tool_with_business_projection_and_original_receipt()
 {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let first = request(&mut socket).await;
            assert_eq!(first["tools"].as_array().unwrap().len(), 1);
            assert_eq!(first["tools"][0]["function"]["name"], "Counter");
            assert_eq!(
                first["tools"][0]["function"]["parameters"]["required"],
                json!(["amount"])
            );
            tool_response(&mut socket).await;
            let (mut socket, _) = listener.accept().await.unwrap();
            let next = request(&mut socket).await;
            let result = next["messages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|message| message["role"] == "tool")
                .unwrap();
            assert_eq!(result["tool_call_id"], "counter-call");
            assert_eq!(
                serde_json::from_str::<Value>(result["content"].as_str().unwrap()).unwrap(),
                json!({"effects":1})
            );
            assert!(!next.to_string().contains("stateDomain"));
            response(&mut socket).await;
        });
        let fixture = Fixture::with_services(&base, 1, None, None, Some(Setup::javascript())).await;
        let session = fixture.create("Actual service tool").await;
        fixture.send(&session).await.unwrap();
        fixture.drained().await;
        let observed = facts(&fixture, &session).await;
        server.await.unwrap();
        let facts = observed;
        assert_eq!(
            facts
                .iter()
                .filter(|fact| matches!(fact, Fact::ToolDispatched { .. }))
                .count(),
            1
        );
        let outcome = facts
            .iter()
            .find_map(|fact| {
                if let Fact::ToolSettled { outcome, .. } = fact {
                    Some(outcome)
                } else {
                    None
                }
            })
            .unwrap();
        let ToolOutcome::Succeeded {
            model_projection, ..
        } = outcome
        else {
            panic!("{outcome:?}")
        };
        assert_eq!(
            model_projection,
            &DurableToolProjection::Json {
                value: json!({"effects":1})
            }
        );
        let prefix = fixture
            .store
            .scoped_prefix(
                maka_runtime::event::LogScope::Session {
                    id: session.clone(),
                },
                256,
                4 * 1024 * 1024,
            )
            .await
            .unwrap();
        let event = prefix
            .events
            .iter()
            .find(|event| matches!(event.event.fact, Fact::ToolSettled { .. }))
            .unwrap();
        let raw = fixture
            .store
            .resolve_tool_result(&session, &event.event.id)
            .await
            .unwrap();
        let text = serde_json::to_string(&raw).unwrap();
        assert!(text.contains("journal_v1"));
        assert!(text.contains("00000000-0000-0000-0000-000000000015"));
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn unknown_service_outcome_has_typed_original_evidence_and_never_enters_model_diagnostic() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            tool_response(&mut socket).await;
            let (mut socket, _) = listener.accept().await.unwrap();
            let next = request(&mut socket).await;
            let text = next.to_string();
            assert!(text.contains("result is unknown"));
            assert!(!text.contains("journal_v1"));
            assert!(!text.contains("fixture.Counter"));
            response(&mut socket).await;
        });
        let counter = Counter::new(Behavior::Unknown);
        let fixture = Fixture::with_services(
            &base,
            1,
            None,
            None,
            Some(Setup::native(counter.clone(), true)),
        )
        .await;
        let session = fixture.create("Unknown original effect").await;
        fixture.send(&session).await.unwrap();
        fixture.drained().await;
        let observed = facts(&fixture, &session).await;
        server.await.unwrap();
        let facts = observed;
        let outcome = facts
            .iter()
            .find_map(|fact| {
                if let Fact::ToolSettled { outcome, .. } = fact {
                    Some(outcome)
                } else {
                    None
                }
            })
            .unwrap();
        let ToolOutcome::Unknown {
            operation: Some(operation),
            message,
        } = outcome
        else {
            panic!("{outcome:?}")
        };
        assert_eq!(counter.effects.load(Ordering::SeqCst), 1);
        assert_eq!(operation.reference.namespace, id(21));
        assert_eq!(operation.reference.method, "write");
        assert!(matches!(
            operation.receipt,
            Receipt::Recorded {
                outcome: OperationOutcome::Unknown { .. },
                cleanup: maka_plugin_core::operation::OperationCleanup::Confirmed
            }
        ));
        assert_eq!(
            outcome_projection(outcome).as_ref(),
            &DurableToolProjection::Text {
                text: message.clone()
            }
        );
        let recovered = fixture
            .store
            .operation_observation(&operation.reference)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            recovered.outcome,
            OperationOutcome::Unknown { .. }
        ));
        assert_eq!(fixture.owner.permits.available_permits(), 1);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn unlisted_native_tool_edge_never_executes_the_selected_provider() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            tool_response(&mut socket).await;
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            response(&mut socket).await;
        });
        let counter = Counter::new(Behavior::Returned);
        let fixture = Fixture::with_services(
            &base,
            1,
            None,
            None,
            Some(Setup::native(counter.clone(), false)),
        )
        .await;
        let session = fixture.create("No implicit grant").await;
        fixture.send(&session).await.unwrap();
        fixture.drained().await;
        server.await.unwrap();
        assert_eq!(counter.effects.load(Ordering::SeqCst), 0);
        let actual = facts(&fixture, &session).await;
        assert!(actual.iter().any(|fact| matches!(
            fact,
            Fact::ToolSettled {
                outcome: ToolOutcome::Failed { .. },
                ..
            }
        )));
        assert!(!actual.iter().any(|fact| matches!(
            fact,
            Fact::ToolSettled {
                outcome: ToolOutcome::Unknown { .. },
                ..
            }
        )));
        assert!(!facts(&fixture, &session).await.iter().any(|fact| matches!(
            fact,
            Fact::ToolSettled {
                outcome: ToolOutcome::Succeeded { .. },
                ..
            }
        )));
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn stopping_real_agent_run_cancels_tool_but_retains_capacity_until_physical_cleanup() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            tool_response(&mut socket).await;
        });
        let counter = Counter::new(Behavior::Blocked);
        let fixture = Fixture::with_services(
            &base,
            1,
            None,
            None,
            Some(Setup::native(counter.clone(), true)),
        )
        .await;
        let session = fixture.create("Stop original tool").await;
        let started = fixture.send(&session).await.unwrap();
        counter.entered.acquire().await.unwrap().forget();
        fixture
            .mutation(
                2,
                "stop",
                json!({"sessionId":session,"invocationId":started["invocationId"]}),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        counter.cancelled.acquire().await.unwrap().forget();
        assert_eq!(fixture.owner.permits.available_permits(), 0);
        assert_eq!(fixture.owner.workers.len(), 1);
        let other = fixture.create("Not available yet").await;
        assert!(
            fixture
                .send(&other)
                .await
                .unwrap_err()
                .contains("agent_capacity_exhausted")
        );
        counter.release.add_permits(1);
        fixture.drained().await;
        server.await.unwrap();
        assert_eq!(counter.effects.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.owner.permits.available_permits(), 1);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn returned_effect_with_failed_cleanup_preserves_receipt_without_settling_or_reusing_capacity()
 {
    tokio::time::timeout(Duration::from_secs(30), async {
        for behavior in [Behavior::CleanupFailed, Behavior::LargeCleanupFailed] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}/v1", listener.local_addr().unwrap());
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                request(&mut socket).await;
                tool_response(&mut socket).await;
            });
            let counter = Counter::new(behavior);
            let fixture = Fixture::with_services(
                &base,
                1,
                None,
                None,
                Some(Setup::native(counter.clone(), true)),
            )
            .await;
            let session = fixture.create("Failed original cleanup").await;
            fixture.send(&session).await.unwrap();
            fixture.drained().await;
            let observed = facts(&fixture, &session).await;
            server.await.unwrap();
            let facts = observed;
            assert!(
                !facts
                    .iter()
                    .any(|fact| matches!(fact, Fact::ToolSettled { .. }))
            );
            let observation = facts
                .iter()
                .find_map(|fact| {
                    if let Fact::ToolOperationObserved { observation, .. } = fact {
                        Some(observation)
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(observation.reference.namespace, id(21));
            if matches!(behavior, Behavior::LargeCleanupFailed) {
                assert!(serde_json::to_vec(observation).unwrap().len() > 1024 * 1024);
            }
            assert!(matches!(
                observation.receipt,
                Receipt::Recorded {
                    outcome: OperationOutcome::Returned { .. },
                    cleanup: maka_plugin_core::operation::OperationCleanup::Failed { .. }
                }
            ));
            assert_eq!(fixture.owner.permits.available_permits(), 0);
            assert_eq!(fixture.owner.runs.lock().await.quarantined.len(), 1);
            assert!(fixture.app.shutdown().await.is_err());
            assert!(!fixture.app.client().snapshot().closed);
            fixture.config.shutdown().await.unwrap();
            // Failed cleanup does not authorize destruction of the original Root.
            eprintln!(
                "Retained failed-cleanup Root: {}",
                fixture._dir.keep().display()
            );
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn query_tool_uses_original_query_gate_and_projects_plain_business_json() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            tool_response(&mut socket).await;
            let (mut socket, _) = listener.accept().await.unwrap();
            let next = request(&mut socket).await;
            let result = next["messages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|message| message["role"] == "tool")
                .unwrap();
            assert_eq!(
                serde_json::from_str::<Value>(result["content"].as_str().unwrap()).unwrap(),
                json!({"effects":0})
            );
            response(&mut socket).await;
        });
        let counter = Counter::new(Behavior::Returned);
        let mut setup = Setup::native(counter.clone(), true);
        setup.query = true;
        let fixture = Fixture::with_services(&base, 1, None, None, Some(setup)).await;
        let session = fixture.create("Actual query tool").await;
        fixture.send(&session).await.unwrap();
        fixture.drained().await;
        server.await.unwrap();
        let prefix = fixture
            .store
            .scoped_prefix(
                maka_runtime::event::LogScope::Session {
                    id: session.clone(),
                },
                256,
                4 * 1024 * 1024,
            )
            .await
            .unwrap();
        let event = prefix
            .events
            .iter()
            .find(|event| {
                matches!(
                    event.event.fact,
                    Fact::ToolSettled {
                        outcome: ToolOutcome::Succeeded { .. },
                        ..
                    }
                )
            })
            .unwrap();
        assert_eq!(
            fixture
                .store
                .resolve_tool_result(&session, &event.event.id)
                .await
                .unwrap(),
            maka_runtime::tool_output::ToolOutput::Json(json!({"effects":0}))
        );
        assert_eq!(counter.effects.load(Ordering::SeqCst), 0);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn unresponsive_native_tool_reaches_original_cleanup_budget_and_keeps_host_work_owned() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            tool_response(&mut socket).await;
        });
        let counter = Counter::new(Behavior::Blocked);
        let fixture = Fixture::with_services(
            &base,
            1,
            None,
            None,
            Some(Setup::native(counter.clone(), true)),
        )
        .await;
        let session = fixture.create("Unresponsive original tool").await;
        let started = fixture.send(&session).await.unwrap();
        counter.entered.acquire().await.unwrap().forget();
        fixture
            .mutation(
                2,
                "stop",
                json!({"sessionId":session,"invocationId":started["invocationId"]}),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        counter.cancelled.acquire().await.unwrap().forget();
        // Keep the physical original callback blocked beyond the real 5s budget.
        // Old observer-first code never drained here, even after that budget.
        tokio::time::timeout(Duration::from_secs(8), fixture.drained())
            .await
            .unwrap();
        assert_eq!(fixture.owner.permits.available_permits(), 0);
        assert_eq!(fixture.owner.runs.lock().await.quarantined.len(), 1);
        let snapshot = fixture.app.client().snapshot();
        assert!(
            snapshot
                .instances
                .iter()
                .any(|status| status.activation.instance() == &id(21) && status.calls > 0)
        );
        assert!(
            !facts(&fixture, &session)
                .await
                .iter()
                .any(|fact| matches!(fact, Fact::ToolSettled { .. }))
        );
        assert_eq!(counter.effects.load(Ordering::SeqCst), 1);
        counter.release.add_permits(1);
        server.await.unwrap();
        assert!(fixture.app.shutdown().await.is_err());
        assert!(!fixture.app.client().snapshot().closed);
        fixture.config.shutdown().await.unwrap();
        eprintln!(
            "Retained unresponsive-tool Root: {}",
            fixture._dir.keep().display()
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn admitted_message_keeps_original_model_workspace_and_dispatcher_when_opening_fails() {
    let fixture = Fixture::with_services(
        "http://127.0.0.1:1/v1",
        1,
        None,
        None,
        Some(Setup::javascript()),
    )
    .await;
    let session = fixture.create("Pending original basis").await;
    let raw =
        rusqlite::Connection::open(fixture.root.join(maka_event_log::root::ROOT_DATABASE)).unwrap();
    raw.execute_batch("CREATE TRIGGER hold_opening BEFORE INSERT ON event_log WHEN NEW.kind='invocation_opened' BEGIN SELECT RAISE(ABORT, 'fixture opening failure'); END;").unwrap();
    let operation = Uuid::new_v4();
    assert!(fixture.mutation(2, "send", json!({
        "sessionId":session,"message":"durable original", "model":fixture.selection().await
    }), operation).await.is_err());
    fixture.drained().await;
    let queued = fixture.store.pending_messages(&session).await.unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].execution.owner, fixture.owner.prefix);
    let encoded = serde_json::to_string(&queued[0].execution.basis).unwrap();
    assert!(!encoded.contains("private-fixture-key"));
    let basis: MessageBasis = serde_json::from_str(&encoded).unwrap();
    assert_eq!(basis.model.binding.model, "fixture-model");
    assert_eq!(
        basis.workspace,
        fixture
            .owner
            .session(&session)
            .await
            .unwrap()
            .configuration
            .workspace
    );
    assert!(!basis.fingerprint.is_empty());
    assert_eq!(basis.tools.len(), 1);
    assert!(basis.tools == fixture.owner.tool_basis);
    assert_eq!(basis.max_steps, fixture.owner.settings.max_steps);
    assert_eq!(
        fixture
            .store
            .pending_message_sessions(&fixture.owner.prefix, None)
            .await
            .unwrap(),
        [session]
    );
    assert!(
        fixture
            .store
            .pending_message_sessions("maka-runtime-host", None)
            .await
            .unwrap()
            .is_empty()
    );
    raw.execute_batch("DROP TRIGGER hold_opening").unwrap();
    drop(raw);
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}
