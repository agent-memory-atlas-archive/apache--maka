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

#[path = "receipts.rs"]
mod receipts;

use super::*;
use maka_event_log::{
    EventLog,
    applications::intents::RecordedIntent,
    root::{ROOT_DATABASE, RootNamespaces, RootOwner},
};
use maka_plugin_core::{
    composition::{InstanceId, Target},
    contract::{
        Availability, Cardinality, Contract, Export, Import, Interface, Method, MethodMode,
        Transport,
    },
    deployment::{DeploymentRef, RemoteObservation},
    operation::{OperationCleanup, OperationOutcome, OperationReply, Receipt, RecoveryRef},
    package::EntryTarget,
    plan::RemoteCapability,
};
use maka_runtime_host::application::{Binding, Client, Context, Handler, PreparedMutation};
use maka_ui::actions::{
    CleanupFuture, MutationCompletion, MutationFuture, NativeMutation, NativeService,
    ReferenceFuture, RemoteService, ServiceCompletion, ServiceFuture, ServiceRequest,
    ServiceSource,
};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::Semaphore;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    LostReply,
    Inexact,
    Envelope,
    Queued,
    ChangedRef,
    Unknown,
    FailedCleanup,
    RecordUnknown,
    SqlExit,
    PreparePanic,
    FailedClose,
}
#[gpui::test]
fn each_queued_receipt_checks_the_latest_state_at_actual_delivery(cx: &mut TestAppContext) {
    let timer = timer();
    let h = Harness::new(cx, &timer, Mode::Queued);
    for _ in 0..2 {
        let revision = h.scene.app.borrow().frame(&h.scene.root).unwrap().revision;
        h.scene
            .app
            .borrow_mut()
            .dispatch(&h.scene.root, revision, "read", Value::Null)
            .unwrap();
    }
    h.scene
        .app
        .borrow_mut()
        .capture_actions(&h.scene.root, |_| unreachable!("resource-free mutation"))
        .unwrap();
    let mut jobs = Vec::new();
    loop {
        let next = { h.scene.app.borrow_mut().start_task(&h.scene.root).unwrap() };
        let Some(task) = next else {
            break;
        };
        let id = task.id;
        let completion = timer.block_on(task.future);
        let dispatch = h
            .scene
            .app
            .borrow_mut()
            .finish_task(id, completion)
            .unwrap()
            .unwrap();
        jobs.push((id, timer.spawn(dispatch.future)));
    }
    assert_eq!(jobs.len(), 2);
    timer.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            h.state.started.acquire_many(2),
        )
        .await
        .unwrap()
        .unwrap()
        .forget();
    });
    h.state.release.add_permits(2);
    let mut originals = Vec::new();
    for (id, job) in jobs {
        let completion = timer.block_on(job).unwrap();
        let maka_ui::actions::TaskCompletion::Mutation(ref result) = completion else {
            panic!("expected original mutation receipt");
        };
        originals.push(result.reply.reference.clone());
        assert!(
            h.scene
                .app
                .borrow_mut()
                .finish_task(id, completion)
                .unwrap()
                .is_none()
        );
    }
    // Both completions passed against small state. The first callback stores
    // 800 KiB in state; the second must downgrade using that new actual state.
    assert_eq!(
        h.scene
            .app
            .borrow_mut()
            .deliver_action_results(&h.scene.root)
            .unwrap(),
        2
    );
    let reply: OperationReply = serde_json::from_value(h.outcome()["reply"].clone()).unwrap();
    assert_eq!(reply.reference, originals[1]);
    assert!(matches!(reply.receipt, Receipt::Unavailable { .. }));
    for original in originals {
        let saved = timer
            .block_on(h.host_store.operation_observation(&original))
            .unwrap()
            .unwrap();
        assert_eq!(saved.cleanup, OperationCleanup::Confirmed);
        assert!(
            matches!(saved.outcome, OperationOutcome::Returned { value } if value.as_str().unwrap().len() == 800 * 1024)
        );
    }
    assert_eq!(h.state.calls.load(Ordering::SeqCst), 2);
    assert!(!h.scene.app.borrow().is_retiring(&h.scene.root));
    h.finish(cx, &timer);
}

fn contract(name: &str, method: &str) -> Contract {
    Contract {
        name: name.into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([(
                method.into(),
                Method {
                    mode: MethodMode::Mutation,
                    input: json!({}),
                    output: json!({}),
                    resources: BTreeMap::new(),
                },
            )]),
        },
    }
}
struct State {
    mode: Mode,
    caller: Weak<EventLog>,
    source: Mutex<Option<DeploymentRef>>,
    original: Mutex<Option<RecoveryRef>>,
    observed: Mutex<Option<ServiceSource>>,
    session: Mutex<Option<Weak<Session>>>,
    started: Arc<Semaphore>,
    release: Arc<Semaphore>,
    close_started: Arc<Semaphore>,
    close_release: Arc<Semaphore>,
    calls: AtomicUsize,
}
struct Effect(Arc<State>);
impl Handler for Effect {
    fn call(&self, method: String, _: Value, context: Context) -> EffectFuture {
        assert_eq!(method, "commit");
        if self.0.mode == Mode::FailedCleanup {
            let mut ticket = context.scope().resources.reserve().unwrap();
            ticket.start();
            ticket.complete(Err("actual mutation resource cleanup failed".into()));
        }
        let state = self.0.clone();
        Box::pin(async move {
            let original = context.operation().unwrap();
            let caller = state.caller.upgrade().unwrap();
            let source = state.source.lock().unwrap().clone().unwrap();
            let intents = caller
                .application_intents(
                    source.deployment,
                    InstanceId(vec![Uuid::from_u128(2)]),
                    0,
                    128,
                )
                .await
                .unwrap();
            assert!(
                intents
                    .iter()
                    .any(|i| i.caller == source && i.reference == *original),
                "real caller commit must precede the real effect"
            );
            drop(caller);
            state.calls.fetch_add(1, Ordering::SeqCst);
            state.started.add_permits(1);
            state.release.acquire().await.unwrap().forget();
            match state.mode {
                Mode::Inexact => Ok(json!(9_007_199_254_740_993_u64)),
                Mode::Queued => Ok(json!("x".repeat(800 * 1024))),
                Mode::Envelope => {
                    let empty = OperationReply {
                        reference: original.clone(),
                        receipt: Receipt::Recorded {
                            outcome: OperationOutcome::Returned { value: json!("") },
                            cleanup: OperationCleanup::Confirmed,
                        },
                    };
                    let size = serde_json::to_vec(&empty).unwrap().len();
                    Ok(json!("x".repeat(1024 * 1024 - size - 4)))
                }
                Mode::Unknown => Err("effect result is unknown".into()),
                _ => Ok(json!("saved")),
            }
        })
    }
}
type EffectFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send>>;
// Deliberate UI boundary fault injection over the direct Host Client.
// It changes method/contract and delivery behavior; it is not a transport adapter.
struct Bridge {
    client: Client,
    binding: Binding,
    issuer: maka_plugins::call::Issuer,
    state: Arc<State>,
}
struct Session {
    client: Client,
    binding: Binding,
    request: ServiceRequest,
    scope: maka_plugins::call::Scope,
    prepared: Mutex<Option<PreparedMutation>>,
    pending: Mutex<Option<maka_plugins::call::Ticket>>,
    state: Arc<State>,
}
impl NativeService for Bridge {
    fn call(&self, _: ServiceRequest) -> ServiceFuture {
        Box::pin(async {
            ServiceCompletion {
                result: Err("query is not exported".into()),
                cleanup: Ok(()),
            }
        })
    }
    fn mutation(&self, request: ServiceRequest) -> Result<Arc<dyn NativeMutation>, String> {
        assert_eq!(request.method, "save");
        assert!(request.resources.is_empty());
        *self.state.observed.lock().unwrap() = Some(request.source.clone());
        let scope = self
            .issuer
            .issue_with(
                maka_plugins::call::Identity::Remote {
                    request_id: request.source.request_id(),
                },
                UiReadGrant {
                    source: request.source.clone(),
                    approved: true,
                },
                Default::default(),
            )
            .map_err(|e| e.to_string())?;
        let pending = if self.state.mode == Mode::FailedClose {
            let mut ticket = scope.resources.reserve().unwrap();
            ticket.start();
            Some(ticket)
        } else {
            None
        };
        let session = Arc::new(Session {
            client: self.client.clone(),
            binding: self.binding.clone(),
            request,
            scope,
            prepared: Mutex::new(None),
            pending: Mutex::new(pending),
            state: self.state.clone(),
        });
        *self.state.session.lock().unwrap() = Some(Arc::downgrade(&session));
        Ok(session)
    }
}
impl NativeMutation for Session {
    fn prepare(self: Arc<Self>) -> ReferenceFuture {
        Box::pin(async move {
            let prepared = self
                .client
                .prepare_mutation(
                    self.binding.clone(),
                    "commit".into(),
                    self.request.input.clone(),
                    self.scope.clone(),
                    self.request.source.request_id(),
                    self.request.resources.clone().into(),
                )
                .await?;
            let reference = prepared.reference().clone();
            *self.prepared.lock().unwrap() = Some(prepared);
            *self.state.original.lock().unwrap() = Some(reference.clone());
            if self.state.mode == Mode::PreparePanic {
                panic!("actual preparation future panicked");
            }
            Ok(reference)
        })
    }
    fn dispatch(self: Arc<Self>, recorded: RecordedIntent) -> MutationFuture {
        Box::pin(async move {
            assert_eq!(
                recorded.intent().caller,
                self.request.source.deployment().unwrap().clone()
            );
            let prepared = self.prepared.lock().unwrap().take().unwrap();
            let original = prepared.reference().clone();
            let mut completion = match self.client.mutate(prepared, recorded).await {
                Ok(accepted) => {
                    let completed = accepted.completion().await;
                    MutationCompletion {
                        reply: completed.reply,
                        cleanup: completed.cleanup,
                    }
                }
                Err(diagnostic) => MutationCompletion {
                    reply: OperationReply {
                        reference: original,
                        receipt: Receipt::Unavailable { diagnostic },
                    },
                    cleanup: self.scope.finish().await.map_err(|e| e.to_string()),
                },
            };
            if self.state.mode == Mode::LostReply {
                // Deliberately discard only the completed business delivery, not
                // the journal or real executor cleanup. This models reply loss.
                completion.reply.receipt = Receipt::Unavailable {
                    diagnostic: "test bridge discarded the original delivery".into(),
                };
            }
            if self.state.mode == Mode::ChangedRef {
                completion.reply.reference.operation = Uuid::new_v4();
            }
            completion
        })
    }
    fn close(self: Arc<Self>) -> CleanupFuture {
        Box::pin(async move {
            self.state.close_started.add_permits(1);
            self.state.close_release.acquire().await.unwrap().forget();
            self.prepared.lock().unwrap().take();
            self.scope.finish().await.map_err(|e| e.to_string())
        })
    }
}
fn ui_fixture() -> (
    maka_plugin_core::composition::Input,
    Vec<maka_plugins::package::Package>,
    RemoteObservation,
) {
    let (mut input, mut packages) = support::resource_fixture();
    input.instances.remove("reader");
    packages.retain(|p| p.manifest().id != "native.reader");
    let index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(index);
    let mut declaration = old.manifest().application.clone().unwrap();
    let ui_contract = contract("test.Save", "save");
    declaration
        .entries
        .get_mut(&EntryTarget::Desktop)
        .unwrap()
        .imports
        .insert(
            "reader".into(),
            Import {
                contract: ui_contract.clone(),
                cardinality: Cardinality::One,
            },
        );
    let source = std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap().replace(
        "method:'read',input:{start:event.event?.start??0,end:event.event?.end??5},resources:{text:{child:'editor',method:'snapshot',input:event.event?.capture??{kind:'current'}}}",
        "method:'save',input:'draft',resources:{}",
    );
    let package = support::package("dynamic.app", declaration, Some(source));
    let app = input.instances.get_mut("app").unwrap();
    app.package = package.application_ref().into();
    let target = RemoteCapability {
        deployment: "original-host".into(),
        instance: InstanceId(vec![Uuid::from_u128(11)]),
        export: "Reader".into(),
    };
    app.bindings.insert(
        "reader".into(),
        vec![Target::Remote {
            deployment: target.deployment.clone(),
            instance: target.instance.clone(),
            export: target.export.clone(),
        }],
    );
    packages.push(package);
    (
        input,
        packages,
        RemoteObservation {
            capability: target,
            declaration: Export {
                contract: ui_contract,
                availability: Availability::Required,
            },
        },
    )
}
struct Harness {
    scene: Scene,
    host: maka_runtime_host::application::Application,
    state: Arc<State>,
    caller: Weak<EventLog>,
    host_store: Arc<EventLog>,
    caller_dir: tempfile::TempDir,
    caller_ns: RootNamespaces,
    host_dir: tempfile::TempDir,
    host_ns: RootNamespaces,
    source: DeploymentRef,
}
impl Harness {
    fn new(cx: &mut TestAppContext, timer: &tokio::runtime::Runtime, mode: Mode) -> Self {
        cx.executor().allow_parking();
        let (caller_dir, caller_ns, caller) = stored_root(timer);
        let (host_dir, host_ns, host_store) = stored_root(timer);
        let state = Arc::new(State {
            mode,
            caller: Arc::downgrade(&caller),
            source: Mutex::new(None),
            original: Mutex::new(None),
            observed: Mutex::new(None),
            session: Mutex::new(None),
            started: Arc::new(Semaphore::new(0)),
            release: Arc::new(Semaphore::new(0)),
            close_started: Arc::new(Semaphore::new(0)),
            close_release: Arc::new(Semaphore::new(if mode == Mode::SqlExit { 0 } else { 2 })),
            calls: AtomicUsize::new(0),
        });
        let issuer = maka_plugins::call::Issuer::default();
        let host = timer.block_on(host_service_fixture(
            Arc::new(Effect(state.clone())),
            issuer.clone(),
            Arc::new(HostGrant(issuer.clone())),
            host_store.clone(),
            contract("test.Store", "commit"),
        ));
        timer.block_on(host.ready()).unwrap();
        let client = host.client();
        let binding = timer
            .block_on(
                client.bind(
                    client
                        .activation(&InstanceId(vec![Uuid::from_u128(12)]))
                        .unwrap(),
                    "reader".into(),
                ),
            )
            .unwrap()
            .remove(0);
        let bridge = Arc::new(Bridge {
            client,
            binding: binding.clone(),
            issuer,
            state: state.clone(),
        });
        let (input, packages, observation) = ui_fixture();
        let deployment = Uuid::new_v4();
        let source = timer
            .block_on(caller.commit_application(
                deployment,
                0,
                input.clone(),
                vec![observation.clone()],
                packages.clone(),
            ))
            .unwrap();
        *state.source.lock().unwrap() = Some(source.clone());
        let registration = RemoteService {
            target: observation.capability.clone(),
            registration_epoch: binding.registration_epoch(),
            contract: observation.declaration.contract.clone(),
            implementation: bridge,
        };
        let application = timer
            .block_on(Application::start_from_store(
                caller.clone(),
                source.clone(),
                PresentationTarget::Desktop,
                timer.handle().clone(),
                Default::default(),
                widgets::registry(),
                vec![registration],
            ))
            .unwrap();
        // Desired role replacement cannot redirect the already-running source.
        let mut replacement = input;
        let mut replacement_observation = observation;
        replacement_observation.capability.deployment = "replacement-host".into();
        if let Target::Remote { deployment, .. } = &mut replacement
            .instances
            .get_mut("app")
            .unwrap()
            .bindings
            .get_mut("reader")
            .unwrap()[0]
        {
            *deployment = "replacement-host".into();
        }
        let desired = timer
            .block_on(caller.commit_application(
                deployment,
                1,
                replacement,
                vec![replacement_observation],
                packages,
            ))
            .unwrap();
        assert_ne!(desired, source);
        let scene = Scene::from_application(cx, application);
        cx.update(|cx| actions::own(scene.app.clone(), None, None, cx).unwrap());
        scene.draw(cx);
        Self {
            scene,
            host,
            state,
            caller: Arc::downgrade(&caller),
            host_store,
            caller_dir,
            caller_ns,
            host_dir,
            host_ns,
            source,
        }
    }
    fn dispatch(&self, cx: &mut TestAppContext) {
        cx.update_window(self.scene.window, |_, window, cx| {
            let revision = self
                .scene
                .app
                .borrow()
                .frame(&self.scene.root)
                .unwrap()
                .revision;
            self.scene
                .app
                .borrow_mut()
                .dispatch(&self.scene.root, revision, "read", Value::Null)
                .unwrap();
            self.scene
                .surface
                .update(cx, |s, cx| s.sync_native(window, cx).unwrap());
        })
        .unwrap();
    }
    fn reference(&self) -> RecoveryRef {
        self.state.original.lock().unwrap().clone().unwrap()
    }
    fn session(&self) -> Arc<Session> {
        self.state
            .session
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap()
    }
    fn outcome(&self) -> Value {
        self.scene
            .app
            .borrow()
            .frame(&self.scene.root)
            .unwrap()
            .state["outcome"]
            .clone()
    }
    fn blocked(&self) {
        assert_eq!(
            RootOwner::open(&self.caller_dir.path().join("root"), &self.caller_ns)
                .err()
                .unwrap()
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert_eq!(
            RootOwner::open(&self.host_dir.path().join("root"), &self.host_ns)
                .err()
                .unwrap()
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    fn finish(self, cx: &mut TestAppContext, timer: &tokio::runtime::Runtime) {
        assert_eq!(self.scene.app.borrow().source(), Some(&self.source));
        if let Some(caller) = self.caller.upgrade() {
            timer.block_on(caller.shutdown()).unwrap();
        }
        if self.scene.app.borrow().frame(&self.scene.root).is_ok() {
            self.scene.close(cx);
        }
        await_action(cx, &self.scene.app, |a| a.can_shutdown());
        self.scene.app.borrow_mut().shutdown().unwrap();
        timer.block_on(self.host.shutdown()).unwrap();
        assert!(
            self.state
                .session
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .upgrade()
                .is_none(),
            "no completed operation owner may retain the endpoint"
        );
        timer.block_on(self.host_store.shutdown()).unwrap();
        let caller_root = self.caller_dir.path().join("root");
        let host_root = self.host_dir.path().join("root");
        let caller_ns = self.caller_ns;
        let host_ns = self.host_ns;
        drop(self.scene);
        drop(self.host);
        drop(self.caller);
        drop(self.host_store);
        drop(self.state);
        cx.run_until_parked();
        assert!(RootOwner::open(&caller_root, &caller_ns).is_ok());
        assert!(RootOwner::open(&host_root, &host_ns).is_ok());
    }
}

fn returned_scene(cx: &mut TestAppContext, mode: Mode) {
    let timer = timer();
    let h = Harness::new(cx, &timer, mode);
    h.dispatch(cx);
    await_action(cx, &h.scene.app, |_| {
        h.state.started.available_permits() == 1
    });
    let original = h.reference();
    assert_eq!(original.method, "commit");
    let source = h.state.observed.lock().unwrap().clone().unwrap();
    assert_eq!(source.deployment(), Some(&h.source));
    assert_eq!(source.mount(), &h.scene.root);
    assert_eq!(original.operation, source.request_id());
    assert_eq!(h.scene.app.borrow().accepted_action_count(), 1);
    h.blocked();
    h.state.release.add_permits(1);
    await_action(cx, &h.scene.app, |a| {
        a.accepted_action_count() == 0
            && a.frame(&h.scene.root)
                .unwrap()
                .state
                .get("outcome")
                .is_some()
    });
    let outcome = h.outcome();
    assert_eq!(outcome["status"], "operation");
    let reply: OperationReply = serde_json::from_value(outcome["reply"].clone()).unwrap();
    assert_eq!(reply.reference, original);
    match mode {
        Mode::Normal => assert!(
            matches!(reply.receipt, Receipt::Recorded { outcome: OperationOutcome::Returned { value }, .. } if value == json!("saved"))
        ),
        Mode::Unknown => assert!(matches!(
            reply.receipt,
            Receipt::Recorded {
                outcome: OperationOutcome::Unknown { .. },
                cleanup: OperationCleanup::Confirmed
            }
        )),
        _ => assert!(matches!(reply.receipt, Receipt::Unavailable { .. })),
    }
    let saved = timer
        .block_on(h.host_store.operation_observation(&original))
        .unwrap()
        .unwrap();
    assert_eq!(saved.cleanup, OperationCleanup::Confirmed);
    match mode {
        Mode::Unknown => assert!(matches!(saved.outcome, OperationOutcome::Unknown { .. })),
        Mode::Inexact => assert!(
            matches!(saved.outcome, OperationOutcome::Returned { value } if value == json!(9_007_199_254_740_993_u64))
        ),
        Mode::Envelope => {
            let raw = OperationReply {
                reference: original,
                receipt: Receipt::Recorded {
                    outcome: saved.outcome,
                    cleanup: saved.cleanup,
                },
            };
            assert!(serde_json::to_vec(&raw).unwrap().len() < 1024 * 1024);
        }
        _ => assert!(matches!(saved.outcome, OperationOutcome::Returned { .. })),
    }
    assert_eq!(h.state.calls.load(Ordering::SeqCst), 1);
    assert!(!h.scene.app.borrow().is_retiring(&h.scene.root));
    h.finish(cx, &timer);
}
#[gpui::test]
fn actual_ui_mutation_records_original_caller_before_host_effect(cx: &mut TestAppContext) {
    returned_scene(cx, Mode::Normal);
}
#[gpui::test]
fn actual_host_inexact_integer_preserves_journal_and_delivers_original_unavailable(
    cx: &mut TestAppContext,
) {
    returned_scene(cx, Mode::Inexact);
}
#[gpui::test]
fn actual_host_receipt_wrapper_overflow_delivers_original_unavailable(cx: &mut TestAppContext) {
    returned_scene(cx, Mode::Envelope);
}
#[gpui::test]
fn different_native_completion_cannot_replace_original_recovery_reference(cx: &mut TestAppContext) {
    returned_scene(cx, Mode::ChangedRef);
}
#[gpui::test]
fn unknown_host_result_is_delivered_separately_from_successful_cleanup(cx: &mut TestAppContext) {
    returned_scene(cx, Mode::Unknown);
}

#[gpui::test]
fn unconfirmed_actual_caller_commit_closes_session_without_dispatch(cx: &mut TestAppContext) {
    let timer = timer();
    let h = Harness::new(cx, &timer, Mode::RecordUnknown);
    let sql =
        rusqlite::Connection::open(h.caller_dir.path().join("root").join(ROOT_DATABASE)).unwrap();
    sql.execute_batch("CREATE TABLE caller_commit_probe (deployment TEXT, revision INTEGER, FOREIGN KEY(deployment,revision) REFERENCES application_sources(deployment,revision) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER reject_caller_commit AFTER INSERT ON application_intents BEGIN INSERT INTO caller_commit_probe VALUES ('missing',1); END;").unwrap();
    h.dispatch(cx);
    await_action(cx, &h.scene.app, |a| {
        a.frame(&h.scene.root)
            .unwrap()
            .state
            .get("outcome")
            .is_some()
    });
    let original = h.reference();
    let reply: OperationReply = serde_json::from_value(h.outcome()["reply"].clone()).unwrap();
    assert_eq!(reply.reference, original);
    assert!(matches!(reply.receipt, Receipt::Unavailable { .. }));
    assert_eq!(h.state.calls.load(Ordering::SeqCst), 0);
    assert!(
        timer
            .block_on(h.host_store.operation_observation(&original))
            .unwrap()
            .is_none()
    );
    assert!(
        timer
            .block_on(h.caller.upgrade().unwrap().application_intents(
                h.source.deployment,
                InstanceId(vec![Uuid::from_u128(2)]),
                0,
                128
            ))
            .unwrap()
            .is_empty()
    );
    assert!(
        h.state
            .session
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .upgrade()
            .is_none()
    );
    drop(sql);
    h.finish(cx, &timer);
}

#[gpui::test]
fn actual_window_exit_during_caller_sql_runs_close_successor_and_keeps_root(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let h = Harness::new(cx, &timer, Mode::SqlExit);
    let sql =
        rusqlite::Connection::open(h.caller_dir.path().join("root").join(ROOT_DATABASE)).unwrap();
    sql.execute_batch("BEGIN IMMEDIATE;").unwrap();
    h.dispatch(cx);
    await_action(cx, &h.scene.app, |_| {
        h.state.original.lock().unwrap().is_some()
    });
    assert_eq!(
        h.scene.app.borrow().accepted_action_count(),
        0,
        "SQL preparation is not accepted Work"
    );
    h.scene.close(cx);
    assert!(!h.scene.app.borrow().can_shutdown());
    h.blocked();
    sql.execute_batch("COMMIT;").unwrap();
    await_action(cx, &h.scene.app, |_| {
        h.state.close_started.available_permits() == 1
    });
    h.blocked();
    assert!(
        h.state
            .session
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .upgrade()
            .is_some()
    );
    assert_eq!(h.state.calls.load(Ordering::SeqCst), 0);
    let saved = timer
        .block_on(h.caller.upgrade().unwrap().application_intents(
            h.source.deployment,
            InstanceId(vec![Uuid::from_u128(2)]),
            0,
            128,
        ))
        .unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].caller, h.source);
    assert_eq!(saved[0].reference, h.reference());
    assert!(
        timer
            .block_on(h.host_store.operation_observation(&h.reference()))
            .unwrap()
            .is_none()
    );
    h.state.close_release.add_permits(1);
    await_action(cx, &h.scene.app, |a| a.can_shutdown());
    drop(sql);
    h.finish(cx, &timer);
}

#[gpui::test]
fn interrupted_preparation_keeps_native_session_until_actual_cleanup_confirmation(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let h = Harness::new(cx, &timer, Mode::PreparePanic);
    h.dispatch(cx);
    await_action(cx, &h.scene.app, |a| a.failed_action_ids().len() == 1);
    let id = h.scene.app.borrow().failed_action_ids()[0];
    let session = h.session();
    h.scene.close(cx);
    assert!(!h.scene.app.borrow().can_shutdown());
    h.blocked();
    assert_eq!(h.state.calls.load(Ordering::SeqCst), 0);
    let cleanup = timer.block_on(session.clone().close());
    assert!(cleanup.is_ok());
    drop(session);
    h.scene
        .app
        .borrow_mut()
        .complete_action_cleanup(id, cleanup)
        .unwrap();
    cx.update(|cx| actions::quit_when_drained(&h.scene.app, cx));
    assert!(h.scene.app.borrow().can_shutdown());
    h.finish(cx, &timer);
}

#[gpui::test]
fn failed_preparation_close_keeps_original_fence_until_real_resource_finishes(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let h = Harness::new(cx, &timer, Mode::FailedClose);
    let sql =
        rusqlite::Connection::open(h.caller_dir.path().join("root").join(ROOT_DATABASE)).unwrap();
    sql.execute_batch("CREATE TRIGGER reject_caller_intent BEFORE INSERT ON application_intents BEGIN SELECT RAISE(ABORT,'caller record unavailable'); END;").unwrap();
    h.dispatch(cx);
    await_action(cx, &h.scene.app, |_| {
        h.state.close_started.available_permits() == 1
    });
    let session = h.session();
    h.scene.close(cx);
    h.blocked();
    // Scope's actual started Ticket remains outstanding through its timeout.
    timer.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                cx.run_until_parked();
                if !h.scene.app.borrow().failed_action_ids().is_empty() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    });
    assert!(!h.scene.app.borrow().can_shutdown());
    let id = h.scene.app.borrow().failed_action_ids()[0];
    assert!(
        h.scene
            .app
            .borrow_mut()
            .complete_action_cleanup(id, Err("still outstanding".into()))
            .is_err()
    );
    session
        .pending
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .complete(Ok(()));
    let cleanup = timer.block_on(session.clone().close());
    assert!(cleanup.is_ok());
    drop(session);
    h.scene
        .app
        .borrow_mut()
        .complete_action_cleanup(id, cleanup)
        .unwrap();
    cx.update(|cx| actions::quit_when_drained(&h.scene.app, cx));
    assert!(h.scene.app.borrow().can_shutdown());
    assert_eq!(h.state.calls.load(Ordering::SeqCst), 0);
    drop(sql);
    h.finish(cx, &timer);
}

#[gpui::test]
fn actual_mutation_failed_cleanup_retains_both_roots_after_window_exit(cx: &mut TestAppContext) {
    let timer = timer();
    let h = Harness::new(cx, &timer, Mode::FailedCleanup);
    h.dispatch(cx);
    await_action(cx, &h.scene.app, |_| {
        h.state.started.available_permits() == 1
    });
    h.scene.close(cx);
    h.blocked();
    h.state.release.add_permits(1);
    await_action(cx, &h.scene.app, |a| a.failed_action_ids().len() == 1);
    assert_eq!(h.scene.app.borrow().accepted_action_count(), 1);
    assert!(!h.scene.app.borrow().can_shutdown());
    let original = h.reference();
    let saved = timer
        .block_on(h.host_store.operation_observation(&original))
        .unwrap()
        .unwrap();
    assert!(
        matches!(saved.outcome, OperationOutcome::Returned { value } if value == json!("saved"))
    );
    assert!(matches!(saved.cleanup, OperationCleanup::Failed { .. }));
    assert!(timer.block_on(h.host.shutdown()).is_err());
    let weak_app = Rc::downgrade(&h.scene.app);
    let weak_session = h.state.session.lock().unwrap().clone().unwrap();
    let caller_root = h.caller_dir.path().join("root");
    let host_root = h.host_dir.path().join("root");
    if let Some(caller) = h.caller.upgrade() {
        timer.block_on(caller.shutdown()).unwrap();
    }
    timer.block_on(h.host_store.shutdown()).unwrap();
    let Harness {
        scene,
        host,
        state,
        caller,
        host_store,
        caller_dir: _caller_dir,
        caller_ns,
        host_dir: _host_dir,
        host_ns,
        source: _,
    } = h;
    drop(scene);
    drop(host);
    drop(state);
    drop(caller);
    drop(host_store);
    cx.run_until_parked();
    assert!(
        weak_app.upgrade().is_some(),
        "the independent GPUI Global retains the original owner"
    );
    assert!(weak_session.upgrade().is_some());
    assert!(RootOwner::open(&caller_root, &caller_ns).is_err());
    assert!(RootOwner::open(&host_root, &host_ns).is_err());
}

#[gpui::test]
fn independent_foreground_owner_consumes_actual_sql_receipt_and_close_without_a_gpui_waiter(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let h = Harness::new(cx, &timer, Mode::SqlExit);
    let sql =
        rusqlite::Connection::open(h.caller_dir.path().join("root").join(ROOT_DATABASE)).unwrap();
    sql.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let owner = cx.update(|cx| actions::own(h.scene.app.clone(), None, None, cx).unwrap());
    let mut app = h.scene.app.borrow_mut();
    let revision = app.frame(&h.scene.root).unwrap().revision;
    app.dispatch(&h.scene.root, revision, "read", Value::Null)
        .unwrap();
    app.capture_actions(&h.scene.root, |_| unreachable!("resource-free mutation"))
        .unwrap();
    let task = app.start_task(&h.scene.root).unwrap().unwrap();
    drop(app);
    // This observer is deliberately lost. The actual foreground owner retains it.
    drop(owner.start(task));
    let wait = |predicate: &dyn Fn() -> bool| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !predicate() {
            owner.pump().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    };
    wait(&|| h.state.original.lock().unwrap().is_some());
    h.scene.close(cx);
    h.blocked();
    sql.execute_batch("COMMIT;").unwrap();
    // Do not run the GPUI executor: actual preparation receipt starts close here.
    wait(&|| h.state.close_started.available_permits() == 1);
    h.blocked();
    assert_eq!(h.state.calls.load(Ordering::SeqCst), 0);
    assert!(!h.scene.app.borrow().can_shutdown());
    h.state.close_release.add_permits(1);
    wait(&|| h.scene.app.borrow().can_shutdown());
    let saved = timer
        .block_on(h.caller.upgrade().unwrap().application_intents(
            h.source.deployment,
            InstanceId(vec![Uuid::from_u128(2)]),
            0,
            128,
        ))
        .unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].reference, h.reference());
    drop(sql);
    h.finish(cx, &timer);
}
