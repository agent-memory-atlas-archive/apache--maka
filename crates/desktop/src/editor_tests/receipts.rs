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
    operation::receipt_reader_contract,
    package::{Declaration, EntryDeclaration, Implementation},
};
use maka_runtime_host::application::{
    Application as HostApplication, Invocation, NativeActivation, NativeRegistry, Policy,
    RecoveryInvocation, operations::RECEIPT_READER,
};
use maka_ui::operations::{CALLER_HISTORY, caller_history_contract};
use std::sync::atomic::AtomicBool;

struct ReceiptGrant {
    issuer: maka_plugins::call::Issuer,
    recovery: Arc<AtomicBool>,
    observed: Arc<Mutex<Vec<InstanceId>>>,
}
impl Policy for ReceiptGrant {
    fn admit(&self, invocation: Invocation<'_>) -> Result<(), String> {
        HostGrant(self.issuer.clone()).admit(invocation)
    }
    fn recover(&self, invocation: RecoveryInvocation<'_>) -> Result<(), String> {
        self.observed
            .lock()
            .unwrap()
            .push(invocation.consumer.instance().clone());
        if self.recovery.load(Ordering::SeqCst)
            && self
                .issuer
                .evidence::<UiReadGrant>(invocation.scope)
                .is_some_and(|grant| {
                    grant.approved
                        && invocation.scope.identity
                            == maka_plugins::call::Identity::Remote {
                                request_id: grant.source.request_id(),
                            }
                })
        {
            Ok(())
        } else {
            Err("historical grant denied".into())
        }
    }
}
struct CurrentBusiness(Arc<AtomicUsize>);
impl NativeService for CurrentBusiness {
    fn mutation(&self, _: ServiceRequest) -> Result<Arc<dyn NativeMutation>, String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err("recovery must not prepare current business mutations".into())
    }
    fn call(&self, _: ServiceRequest) -> ServiceFuture {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            ServiceCompletion {
                result: Err("current business service is not the original reader".into()),
                cleanup: Ok(()),
            }
        })
    }
}

async fn reader_source(store: &EventLog, original: &DeploymentRef) {
    let loaded = store
        .load_application(original.deployment)
        .await
        .unwrap()
        .unwrap();
    let mut input = loaded.source.input;
    let mut packages = Vec::new();
    for old in loaded.packages {
        let mut declaration = old.manifest().application.clone().unwrap();
        let entry = declaration.entries.get_mut(&EntryTarget::Host).unwrap();
        let node = if old.manifest().id == "host.reader" {
            entry.implementation = Implementation::Native {
                implementation: RECEIPT_READER.into(),
            };
            entry.exports = BTreeMap::from([(
                "Receipts".into(),
                Export {
                    contract: receipt_reader_contract(),
                    availability: Availability::Required,
                },
            )]);
            input.instances.get_mut("reader").unwrap()
        } else {
            entry.imports.get_mut("reader").unwrap().contract = receipt_reader_contract();
            if let Target::Instance { export, .. } = &mut input
                .instances
                .get_mut("consumer")
                .unwrap()
                .bindings
                .get_mut("reader")
                .unwrap()[0]
            {
                *export = "Receipts".into();
            }
            input.instances.get_mut("consumer").unwrap()
        };
        let package = support::package(&old.manifest().id, declaration, None);
        node.package = package.application_ref().into();
        packages.push(package);
    }
    store
        .commit_application(original.deployment, 1, input, vec![], packages)
        .await
        .unwrap();
}
async fn start_reader(
    store: Arc<EventLog>,
    source: &DeploymentRef,
    issuer: &maka_plugins::call::Issuer,
    recovery: Arc<AtomicBool>,
    observed: Arc<Mutex<Vec<InstanceId>>>,
) -> HostApplication {
    let mut native = NativeRegistry::default();
    native
        .insert(
            "test.host-consumer".into(),
            Arc::new(|_, _| {
                Box::pin(async {
                    NativeActivation {
                        exports: Ok(BTreeMap::new()),
                        owner: Arc::new(HostClose),
                    }
                })
            }),
        )
        .unwrap();
    let app = HostApplication::start_from_store(
        store,
        source.deployment,
        native,
        issuer.clone(),
        Arc::new(ReceiptGrant {
            issuer: issuer.clone(),
            recovery,
            observed,
        }),
        Default::default(),
    )
    .await
    .unwrap();
    app.ready().await.unwrap();
    app
}
async fn recovery_source(
    store: &EventLog,
    original: &DeploymentRef,
) -> (DeploymentRef, RemoteObservation, RemoteObservation) {
    let loaded = store
        .load_application(original.deployment)
        .await
        .unwrap()
        .unwrap();
    let mut input = loaded.source.input;
    let mut packages = loaded.packages;
    let mut observations = loaded.source.remote_exports;
    let business = observations[0].clone();
    assert_eq!(business.capability.deployment, "replacement-host");
    let old = packages.remove(
        packages
            .iter()
            .position(|p| p.manifest().id == "dynamic.app")
            .unwrap(),
    );
    let mut declaration = old.manifest().application.clone().unwrap();
    let entry = declaration.entries.get_mut(&EntryTarget::Desktop).unwrap();
    entry.imports.insert(
        "history".into(),
        Import {
            contract: caller_history_contract(),
            cardinality: Cardinality::One,
        },
    );
    entry.imports.insert(
        "receipts".into(),
        Import {
            contract: receipt_reader_contract(),
            cardinality: Cardinality::One,
        },
    );
    let source = std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap();
    let start = source.find("        const next=").unwrap();
    let end = source[start..].find("        return frame(next);").unwrap() + start;
    let source = format!(
        "{}{}{}",
        &source[..start],
        r#"
        const next={...state,revision:state.revision+1};
        if(event.kind==='action') {
            if(event.key==='history_done' && event.event.status==='succeeded') {
                next.history=event.event.value;
                const out=frame(next);
                if(next.history.length) out.actions.push({key:'receipt_done',binding:'receipts',member:0,method:'read',input:next.history[0].reference,resources:{}});
                return out;
            }
            next.outcome=event.event;
        } else if(event.kind==='component') next.observation=event.event;
        else if(event.key==='read') {
            next.outcome=null;
            const out=frame(next);
            out.actions.push({key:'history_done',binding:'history',member:0,method:'page',input:{after:0,limit:16},resources:{}});
            return out;
        }
"#,
        &source[end..]
    );
    let root = support::package("dynamic.app", declaration, Some(source));
    let history = support::package(
        "native.history",
        Declaration {
            entries: BTreeMap::from([(
                EntryTarget::Desktop,
                EntryDeclaration {
                    scene: None,
                    implementation: Implementation::Native {
                        implementation: CALLER_HISTORY.into(),
                    },
                    configuration: json!({"type":"object","additionalProperties":false}),
                    imports: BTreeMap::new(),
                    exports: BTreeMap::from([(
                        "History".into(),
                        Export {
                            contract: caller_history_contract(),
                            availability: Availability::Required,
                        },
                    )]),
                },
            )]),
            composition: None,
        },
        None,
    );
    input
        .instances
        .insert("history".into(), support::node(&history, 22));
    let app = input.instances.get_mut("app").unwrap();
    app.package = root.application_ref().into();
    app.bindings.insert(
        "history".into(),
        vec![Target::Instance {
            path: vec!["history".into()],
            export: "History".into(),
        }],
    );
    let receipts = RemoteObservation {
        capability: RemoteCapability {
            deployment: "original-receipts".into(),
            instance: InstanceId(vec![Uuid::from_u128(11)]),
            export: "Receipts".into(),
        },
        declaration: Export {
            contract: receipt_reader_contract(),
            availability: Availability::Required,
        },
    };
    app.bindings.insert(
        "receipts".into(),
        vec![Target::Remote {
            deployment: receipts.capability.deployment.clone(),
            instance: receipts.capability.instance.clone(),
            export: receipts.capability.export.clone(),
        }],
    );
    observations.push(receipts.clone());
    packages.extend([root, history]);
    let source = store
        .commit_application(original.deployment, 2, input, observations, packages)
        .await
        .unwrap();
    (source, business, receipts)
}
fn dispatch_history(scene: &Scene, cx: &mut TestAppContext) {
    cx.update_window(scene.window, |_, window, cx| {
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "read", Value::Null)
            .unwrap();
        scene
            .surface
            .update(cx, |s, cx| s.sync_native(window, cx).unwrap());
    })
    .unwrap();
    await_action(cx, &scene.app, |a| {
        a.frame(&scene.root)
            .unwrap()
            .state
            .get("outcome")
            .is_some_and(|outcome| !outcome.is_null())
    });
}
fn outcome(scene: &Scene) -> Value {
    scene.app.borrow().frame(&scene.root).unwrap().state["outcome"].clone()
}

#[gpui::test]
fn caller_history_recovers_lost_original_receipt_after_both_roots_reopen_and_business_rebinds(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let h = Harness::new(cx, &timer, Mode::LostReply);
    h.dispatch(cx);
    await_action(cx, &h.scene.app, |_| {
        h.state.started.available_permits() == 1
    });
    h.state.release.add_permits(1);
    await_action(cx, &h.scene.app, |a| {
        a.frame(&h.scene.root)
            .unwrap()
            .state
            .get("outcome")
            .is_some()
    });
    let original = h.reference();
    let lost: OperationReply = serde_json::from_value(h.outcome()["reply"].clone()).unwrap();
    assert_eq!(lost.reference, original);
    assert!(matches!(lost.receipt, Receipt::Unavailable { .. }));
    let saved = timer
        .block_on(h.host_store.operation_observation(&original))
        .unwrap()
        .unwrap();
    assert_eq!(
        saved.outcome,
        OperationOutcome::Returned {
            value: json!("saved")
        }
    );
    assert_eq!(saved.cleanup, OperationCleanup::Confirmed);
    let caller = h.caller.upgrade().unwrap();
    let (current, business, receipts) = timer.block_on(recovery_source(&caller, &h.source));
    timer.block_on(reader_source(&h.host_store, &original.source));
    h.scene.close(cx);
    await_action(cx, &h.scene.app, |a| a.can_shutdown());
    cx.update(|cx| actions::quit_when_drained(&h.scene.app, cx));
    timer.block_on(h.host.shutdown()).unwrap();
    timer.block_on(caller.shutdown()).unwrap();
    timer.block_on(h.host_store.shutdown()).unwrap();
    let Harness {
        scene,
        host,
        state,
        caller: old_caller,
        host_store,
        caller_dir,
        caller_ns,
        host_dir,
        host_ns,
        source: _,
    } = h;
    drop(scene);
    drop(host);
    drop(old_caller);
    drop(host_store);
    drop(caller);
    cx.run_until_parked();
    assert!(state.caller.upgrade().is_none());
    let caller = Arc::new(
        timer
            .block_on(EventLog::for_root(Arc::new(
                RootOwner::open(&caller_dir.path().join("root"), &caller_ns).unwrap(),
            )))
            .unwrap(),
    );
    let host_store = Arc::new(
        timer
            .block_on(EventLog::for_root(Arc::new(
                RootOwner::open(&host_dir.path().join("root"), &host_ns).unwrap(),
            )))
            .unwrap(),
    );
    let issuer = maka_plugins::call::Issuer::default();
    let recovery = Arc::new(AtomicBool::new(false));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let reader = timer.block_on(start_reader(
        host_store.clone(),
        &original.source,
        &issuer,
        recovery.clone(),
        seen.clone(),
    ));
    assert_ne!(reader.source(), Some(&original.source));
    let client = reader.client();
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
    let reader_activation = binding.provider().clone();
    let observed = Arc::new(Mutex::new(None));
    let reader_bridge = Arc::new(maka_application_bridge::LocalService::new(
        client.clone(),
        binding.clone(),
        Arc::new(UiReadScopes {
            issuer,
            approved: Arc::new(AtomicBool::new(true)),
            observed: observed.clone(),
        }),
    ));
    let business_calls = Arc::new(AtomicUsize::new(0));
    let registrations = vec![
        RemoteService {
            target: business.capability,
            registration_epoch: Uuid::new_v4(),
            contract: business.declaration.contract,
            implementation: Arc::new(CurrentBusiness(business_calls.clone())),
        },
        RemoteService {
            target: receipts.capability,
            registration_epoch: binding.registration_epoch(),
            contract: receipts.declaration.contract,
            implementation: reader_bridge,
        },
    ];
    let application = timer
        .block_on(Application::start_from_store(
            caller.clone(),
            current.clone(),
            PresentationTarget::Desktop,
            timer.handle().clone(),
            Default::default(),
            widgets::registry(),
            registrations,
        ))
        .unwrap();
    let scene = Scene::from_application(cx, application);
    cx.update(|cx| actions::own(scene.app.clone(), None, None, cx).unwrap());
    scene.draw(cx);
    assert_eq!(scene.app.borrow().source(), Some(&current));
    dispatch_history(&scene, cx);
    let denied: OperationReply = serde_json::from_value(outcome(&scene)["value"].clone()).unwrap();
    assert_eq!(denied.reference, original);
    assert!(matches!(denied.receipt, Receipt::Unavailable { .. }));
    recovery.store(true, Ordering::SeqCst);
    dispatch_history(&scene, cx);
    let reply: OperationReply = serde_json::from_value(outcome(&scene)["value"].clone()).unwrap();
    assert_eq!(reply.reference, original);
    assert_eq!(
        reply.receipt,
        Receipt::Recorded {
            outcome: saved.outcome,
            cleanup: saved.cleanup
        }
    );
    let history: Vec<maka_event_log::applications::intents::CallerIntent> = serde_json::from_value(
        scene.app.borrow().frame(&scene.root).unwrap().state["history"].clone(),
    )
    .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].reference, original);
    assert_ne!(history[0].caller, current);
    assert_eq!(
        observed.lock().unwrap().as_ref().unwrap().deployment(),
        Some(&current)
    );
    assert!(seen.lock().unwrap().len() >= 3);
    assert!(
        seen.lock()
            .unwrap()
            .iter()
            .all(|consumer| consumer == reader_activation.instance())
    );
    timer.block_on(client.retire(reader_activation)).unwrap();
    dispatch_history(&scene, cx);
    assert_eq!(outcome(&scene)["status"], "failed");
    assert!(!scene.app.borrow().is_retiring(&scene.root));
    assert_eq!(business_calls.load(Ordering::SeqCst), 0);
    assert_eq!(state.calls.load(Ordering::SeqCst), 1);
    scene.close(cx);
    await_action(cx, &scene.app, |a| a.can_shutdown());
    cx.update(|cx| actions::quit_when_drained(&scene.app, cx));
    timer.block_on(reader.shutdown()).unwrap();
    timer.block_on(caller.shutdown()).unwrap();
    timer.block_on(host_store.shutdown()).unwrap();
    drop(scene);
    drop(reader);
    drop(client);
    drop(binding);
    drop(caller);
    drop(host_store);
    drop(state);
    cx.run_until_parked();
    assert!(RootOwner::open(&caller_dir.path().join("root"), &caller_ns).is_ok());
    assert!(RootOwner::open(&host_dir.path().join("root"), &host_ns).is_ok());
}
