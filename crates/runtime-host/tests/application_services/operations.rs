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
use maka_event_log::{
    EventLog,
    root::{RootNamespaces, RootOwner},
};
use maka_plugin_core::operation::{OperationCleanup, OperationOutcome};
use maka_runtime_host::application::RecoveryInvocation;
use std::sync::atomic::AtomicBool;

async fn store() -> (tempfile::TempDir, RootNamespaces, Arc<EventLog>) {
    let dir = tempfile::tempdir().unwrap();
    let namespaces = RootNamespaces {
        ownership: dir.path().join("owners"),
        control: dir.path().join("control"),
    };
    let owner = Arc::new(RootOwner::create(&dir.path().join("root"), &namespaces).unwrap());
    let store = Arc::new(EventLog::for_root(owner).await.unwrap());
    (dir, namespaces, store)
}
fn mutation_packages() -> (Package, Package) {
    let provider = package(
        "native",
        entry(
            true,
            Implementation::Native {
                implementation: "native.echo".into(),
            },
            MethodMode::Mutation,
        ),
        None,
    );
    let consumer = package(
        "consumer",
        entry(
            false,
            Implementation::JavaScript {
                module: "host.mjs".into(),
                realm: Realm::Shared,
            },
            MethodMode::Mutation,
        ),
        Some("export function activate() { return {}; }".into()),
    );
    (provider, consumer)
}
struct RecoveryGrant {
    issuer: Issuer,
    recovery: Arc<AtomicBool>,
}
impl Policy for RecoveryGrant {
    fn admit(&self, invocation: Invocation<'_>) -> Result<()> {
        Grant(self.issuer.clone()).admit(invocation)
    }
    fn recover(&self, invocation: RecoveryInvocation<'_>) -> Result<()> {
        if self.issuer.evidence::<bool>(invocation.scope) == Some(&true)
            && self.recovery.load(Ordering::SeqCst)
        {
            Ok(())
        } else {
            Err("historical grant denied".into())
        }
    }
}
struct VerifiedEcho {
    store: Arc<EventLog>,
    calls: Arc<AtomicUsize>,
}
impl Handler for VerifiedEcho {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value>> {
        let store = self.store.clone();
        let calls = self.calls.clone();
        Box::pin(async move {
            let reference = context.operation().expect("actual operation context");
            let observation = store
                .operation_observation(reference)
                .await
                .unwrap()
                .expect("dispatch must precede effect");
            assert_eq!(observation.outcome, OperationOutcome::Unresolved);
            assert_eq!(observation.cleanup, OperationCleanup::Unconfirmed);
            if input == json!("escaped-fail") {
                calls.fetch_add(1, Ordering::SeqCst);
                return Err("\u{00}\n\"".repeat(64 * 1024));
            }
            Echo { calls }.call(method, input, context).await
        })
    }
}
async fn launch(
    store: Arc<EventLog>,
    issuer: &Issuer,
    recovery: Arc<AtomicBool>,
    handler: Arc<dyn Handler>,
) -> (Application, Uuid) {
    let (provider, consumer) = mutation_packages();
    let deployment = Uuid::new_v4();
    store
        .commit_application(
            deployment,
            0,
            input(&provider, &consumer),
            vec![],
            vec![provider, consumer],
        )
        .await
        .unwrap();
    let app = Application::start_from_store(
        store,
        deployment,
        registry_contract(handler, contract(MethodMode::Mutation)),
        issuer.clone(),
        Arc::new(RecoveryGrant {
            issuer: issuer.clone(),
            recovery,
        }),
        Limits::default(),
    )
    .await
    .unwrap();
    app.ready().await.unwrap();
    (app, deployment)
}
async fn close(store: Arc<EventLog>) {
    Arc::try_unwrap(store)
        .ok()
        .expect("no live storage owner")
        .close()
        .await
        .unwrap();
}

#[tokio::test]
async fn real_mutation_commits_before_effect_and_duplicate_requires_recovery_grant() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let (_dir, _ns, store) = store().await;
        let issuer = Issuer::default();
        let recovery = Arc::new(AtomicBool::new(false));
        let calls = Arc::new(AtomicUsize::new(0));
        let (app, _) = launch(
            store.clone(),
            &issuer,
            recovery.clone(),
            Arc::new(VerifiedEcho {
                store: store.clone(),
                calls: calls.clone(),
            }),
        )
        .await;
        let client = app.client();
        let consumer = client.activation(&id(2)).unwrap();
        let binding = client
            .bind(consumer.clone(), "echo".into())
            .await
            .unwrap()
            .remove(0);
        assert!(
            client
                .start(
                    binding.clone(),
                    "echo".into(),
                    json!("original"),
                    scope(&issuer, true),
                    Attachments::new()
                )
                .await
                .is_err()
        );
        assert!(
            client
                .mutate(
                    binding.clone(),
                    "echo".into(),
                    json!("original"),
                    scope(&issuer, true),
                    Uuid::nil()
                )
                .await
                .is_err()
        );
        assert!(
            client
                .mutate(
                    binding.clone(),
                    "echo".into(),
                    json!("original"),
                    scope(&issuer, false),
                    Uuid::new_v4()
                )
                .await
                .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let operation = Uuid::new_v4();
        let accepted = client
            .mutate(
                binding.clone(),
                "echo".into(),
                json!("original"),
                scope(&issuer, true),
                operation,
            )
            .await
            .unwrap();
        let reference = accepted.recovery.clone().unwrap();
        assert_eq!(accepted.result().await.unwrap(), json!("original"));
        let observation = store
            .operation_observation(&reference)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(observation.cleanup, OperationCleanup::Confirmed);
        assert!(
            client
                .recover(consumer.clone(), reference.clone(), scope(&issuer, true))
                .await
                .unwrap_err()
                .contains("historical grant denied")
        );
        assert!(
            client
                .mutate(
                    binding.clone(),
                    "echo".into(),
                    json!("original"),
                    scope(&issuer, true),
                    operation
                )
                .await
                .unwrap()
                .result()
                .await
                .unwrap_err()
                .contains("historical grant denied")
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        recovery.store(true, Ordering::SeqCst);
        assert_eq!(
            client
                .recover(consumer.clone(), reference.clone(), scope(&issuer, true))
                .await
                .unwrap()
                .unwrap(),
            observation
        );
        assert_eq!(
            client
                .mutate(
                    binding.clone(),
                    "echo".into(),
                    json!("original"),
                    scope(&issuer, true),
                    operation
                )
                .await
                .unwrap()
                .result()
                .await
                .unwrap(),
            json!("original")
        );
        assert!(
            client
                .mutate(
                    binding,
                    "echo".into(),
                    json!("changed"),
                    scope(&issuer, true),
                    operation
                )
                .await
                .unwrap()
                .result()
                .await
                .unwrap_err()
                .contains("conflicts")
        );
        let foreign = Issuer::default();
        assert!(
            client
                .recover(consumer.clone(), reference.clone(), scope(&foreign, true))
                .await
                .is_err()
        );
        let cancelled = scope(&issuer, true);
        cancelled.cancellation.cancel();
        assert!(
            client
                .recover(consumer.clone(), reference.clone(), cancelled)
                .await
                .is_err()
        );
        client.retire(consumer.clone()).await.unwrap();
        assert!(
            client
                .recover(consumer, reference.clone(), scope(&issuer, true))
                .await
                .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            store
                .operation_observation(&reference)
                .await
                .unwrap()
                .unwrap(),
            observation
        );
        app.shutdown().await.unwrap();
        drop(app);
        drop(client);
        close(store).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn unknown_result_is_not_replayed_and_default_recovery_policy_denies() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let (_dir, _ns, store) = store().await;
        let (provider, consumer) = mutation_packages();
        let deployment = Uuid::new_v4();
        store
            .commit_application(
                deployment,
                0,
                input(&provider, &consumer),
                vec![],
                vec![provider, consumer],
            )
            .await
            .unwrap();
        let issuer = Issuer::default();
        let calls = Arc::new(AtomicUsize::new(0));
        let app = Application::start_from_store(
            store.clone(),
            deployment,
            registry_contract(
                Arc::new(VerifiedEcho {
                    store: store.clone(),
                    calls: calls.clone(),
                }),
                contract(MethodMode::Mutation),
            ),
            issuer.clone(),
            Arc::new(Grant(issuer.clone())),
            Limits::default(),
        )
        .await
        .unwrap();
        app.ready().await.unwrap();
        let client = app.client();
        let consumer = client.activation(&id(2)).unwrap();
        let binding = client
            .bind(consumer.clone(), "echo".into())
            .await
            .unwrap()
            .remove(0);
        for input in ["fail", "bad-result", "escaped-fail"] {
            let operation = Uuid::new_v4();
            let accepted = client
                .mutate(
                    binding.clone(),
                    "echo".into(),
                    json!(input),
                    scope(&issuer, true),
                    operation,
                )
                .await
                .unwrap();
            let reference = accepted.recovery.clone().unwrap();
            assert!(accepted.result().await.is_err());
            let observation = store
                .operation_observation(&reference)
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(
                observation.outcome,
                OperationOutcome::Unknown { .. }
            ));
            assert_eq!(observation.cleanup, OperationCleanup::Confirmed);
            assert!(
                client
                    .mutate(
                        binding.clone(),
                        "echo".into(),
                        json!(input),
                        scope(&issuer, true),
                        operation
                    )
                    .await
                    .unwrap()
                    .result()
                    .await
                    .unwrap_err()
                    .contains("not authorized")
            );
            assert!(
                client
                    .recover(consumer.clone(), reference.clone(), scope(&issuer, true))
                    .await
                    .unwrap_err()
                    .contains("not authorized")
            );
            assert_eq!(
                store
                    .operation_observation(&reference)
                    .await
                    .unwrap()
                    .unwrap(),
                observation
            );
        }
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        app.shutdown().await.unwrap();
        drop(binding);
        drop(app);
        drop(client);
        close(store).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn original_receipt_survives_restart_and_new_composition_without_provider() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let (dir, ns, store) = store().await;
        let issuer = Issuer::default();
        let calls = Arc::new(AtomicUsize::new(0));
        let recovery = Arc::new(AtomicBool::new(true));
        let (app, deployment) = launch(
            store.clone(),
            &issuer,
            recovery.clone(),
            Arc::new(VerifiedEcho {
                store: store.clone(),
                calls: calls.clone(),
            }),
        )
        .await;
        let client = app.client();
        let binding = client
            .bind(client.activation(&id(2)).unwrap(), "echo".into())
            .await
            .unwrap()
            .remove(0);
        let accepted = client
            .mutate(
                binding,
                "echo".into(),
                json!("original"),
                scope(&issuer, true),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        let reference = accepted.recovery.clone().unwrap();
        accepted.result().await.unwrap();
        let mut declaration = entry(
            false,
            Implementation::JavaScript {
                module: "host.mjs".into(),
                realm: Realm::Shared,
            },
            MethodMode::Query,
        );
        declaration.imports.clear();
        let replacement = package(
            "replacement",
            declaration,
            Some("export function activate() { return {}; }".into()),
        );
        let mut replacement_node = node(&replacement, 2, true);
        replacement_node.bindings.clear();
        let replacement_input = Input {
            environment: Environment {
                location: ExecutionLocation::Host,
                presentation: None,
            },
            instances: BTreeMap::from([("recovery-client".into(), replacement_node)]),
            launch: None,
        };
        store
            .commit_application(deployment, 1, replacement_input, vec![], vec![replacement])
            .await
            .unwrap();
        // The accepted original owner can still dispatch against source revision one
        // after revision two has replaced desired composition and removed its provider.
        let old_binding = client
            .bind(client.activation(&id(2)).unwrap(), "echo".into())
            .await
            .unwrap()
            .remove(0);
        let old_accepted = client
            .mutate(
                old_binding,
                "echo".into(),
                json!("old-owner"),
                scope(&issuer, true),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        assert_eq!(
            old_accepted.recovery.as_ref().unwrap().source,
            reference.source
        );
        assert_eq!(old_accepted.result().await.unwrap(), json!("old-owner"));
        assert_eq!(
            store
                .load_application_at(&reference.source)
                .await
                .unwrap()
                .unwrap()
                .source
                .reference,
            reference.source
        );
        app.shutdown().await.unwrap();
        drop(app);
        drop(client);
        close(store).await;
        let owner = Arc::new(RootOwner::open(&dir.path().join("root"), &ns).unwrap());
        let reopened = Arc::new(EventLog::for_root(owner).await.unwrap());
        let new_issuer = Issuer::default();
        let new_app = Application::start_from_store(
            reopened.clone(),
            deployment,
            NativeRegistry::default(),
            new_issuer.clone(),
            Arc::new(RecoveryGrant {
                issuer: new_issuer.clone(),
                recovery,
            }),
            Limits::default(),
        )
        .await
        .unwrap();
        new_app.ready().await.unwrap();
        let client = new_app.client();
        let consumer = client.activation(&id(2)).unwrap();
        assert!(client.activation(&id(1)).is_err());
        let observation = client
            .recover(consumer, reference.clone(), scope(&new_issuer, true))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            observation.outcome,
            OperationOutcome::Returned {
                value: json!("original")
            }
        );
        assert_eq!(observation.cleanup, OperationCleanup::Confirmed);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let (_other_dir, _other_ns, other_store) = self::store().await;
        let current = reopened
            .load_application(deployment)
            .await
            .unwrap()
            .unwrap();
        other_store
            .commit_application(
                deployment,
                0,
                current.source.input,
                vec![],
                current.packages,
            )
            .await
            .unwrap();
        let other_app = Application::start_from_store(
            other_store.clone(),
            deployment,
            NativeRegistry::default(),
            new_issuer.clone(),
            Arc::new(RecoveryGrant {
                issuer: new_issuer.clone(),
                recovery: Arc::new(AtomicBool::new(true)),
            }),
            Limits::default(),
        )
        .await
        .unwrap();
        other_app.ready().await.unwrap();
        let other_client = other_app.client();
        assert!(
            other_client
                .recover(
                    other_client.activation(&id(2)).unwrap(),
                    reference,
                    scope(&new_issuer, true)
                )
                .await
                .unwrap_err()
                .contains("foreign operation state domain")
        );
        other_app.shutdown().await.unwrap();
        drop(other_app);
        drop(other_client);
        close(other_store).await;
        new_app.shutdown().await.unwrap();
        drop(new_app);
        drop(client);
        close(reopened).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn javascript_mutation_receives_original_reference_and_dispatches_once() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let (_dir,_ns,store)=store().await;let issuer=Issuer::default();let recovery=Arc::new(AtomicBool::new(true));
        let contract=contract(MethodMode::Mutation);
        let source=format!(r#"let calls=0;
            export const service={{echo(input, observation) {{
                if (!observation.operation || observation.operation.method !== 'echo' || observation.operation.source.revision !== 1) throw Error('missing original operation observation');
                calls++; return input+':'+calls;
            }}}};
            export function activate() {{return {{echo:{{contract:{},service:['service']}}}};}}
        "#,serde_json::to_string(&contract).unwrap());
        let provider=package("script",entry(true,Implementation::JavaScript {module:"host.mjs".into(),realm:Realm::Dedicated},MethodMode::Mutation),Some(source));
        let consumer=mutation_packages().1;let deployment=Uuid::new_v4();store.commit_application(deployment,0,input(&provider,&consumer),vec![],vec![provider,consumer]).await.unwrap();
        let app=Application::start_from_store(store.clone(),deployment,NativeRegistry::default(),issuer.clone(),Arc::new(RecoveryGrant {issuer:issuer.clone(),recovery}),Limits::default()).await.unwrap();app.ready().await.unwrap();
        let client=app.client();let binding=client.bind(client.activation(&id(2)).unwrap(),"echo".into()).await.unwrap().remove(0);let operation=Uuid::new_v4();
        let accepted=client.mutate(binding.clone(),"echo".into(),json!("script"),scope(&issuer,true),operation).await.unwrap();let reference=accepted.recovery.clone().unwrap();assert_eq!(accepted.result().await.unwrap(),json!("script:1"));
        assert_eq!(client.mutate(binding,"echo".into(),json!("script"),scope(&issuer,true),operation).await.unwrap().result().await.unwrap(),json!("script:1"));
        assert_eq!(store.operation_observation(&reference).await.unwrap().unwrap().outcome,OperationOutcome::Returned {value:json!("script:1")});
        app.shutdown().await.unwrap();drop(app);drop(client);close(store).await;
    }).await.unwrap();
}

#[tokio::test]
async fn javascript_forwarded_mutation_preserves_original_evidence_and_parent_cleanup() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let (_dir,_ns,store)=store().await;let issuer=Issuer::default();let calls=Arc::new(AtomicUsize::new(0));let operation=Uuid::new_v4();
        let (sink,_)=mutation_packages();
        let mut forwarder_entry=entry(true,Implementation::JavaScript {module:"host.mjs".into(),realm:Realm::Shared},MethodMode::Query);
        forwarder_entry.imports.insert("child".into(), Import {contract:contract(MethodMode::Mutation),cardinality:Cardinality::One});
        let source=format!(r#"
            export const service={{async echo(input,_observation,context) {{
                return (await context.invoke({{operation:'mutate',import:'child',provider:0,method:'echo',input,operationId:'{operation}'}}))+':parent';
            }}}};
            export function activate() {{return {{echo:{{contract:{},service:['service']}}}};}}
        "#,serde_json::to_string(&contract(MethodMode::Query)).unwrap());
        let forwarder=package("forwarder",forwarder_entry,Some(source));let consumer=consumer();
        let mut input=input(&forwarder,&consumer);let parent=input.instances.get_mut("provider").unwrap();parent.key=Uuid::from_u128(4);
        parent.bindings.insert("child".into(),vec![Target::Instance {path:vec!["sink".into()],export:"echo".into()}]);input.instances.insert("sink".into(),node(&sink,1,true));
        let deployment=Uuid::new_v4();store.commit_application(deployment,0,input,vec![],vec![sink,forwarder,consumer]).await.unwrap();
        let app=Application::start_from_store(store.clone(),deployment,registry_contract(Arc::new(VerifiedEcho {store:store.clone(),calls:calls.clone()}),contract(MethodMode::Mutation)),issuer.clone(),Arc::new(RecoveryGrant {issuer:issuer.clone(),recovery:Arc::new(AtomicBool::new(true))}),Limits::default()).await.unwrap();app.ready().await.unwrap();
        let client=app.client();let binding=client.bind(client.activation(&id(2)).unwrap(),"echo".into()).await.unwrap().remove(0);
        for _ in 0..2 {assert_eq!(client.start(binding.clone(),"echo".into(),json!("child"),scope(&issuer,true),Attachments::new()).await.unwrap().result().await.unwrap(),json!("child:parent"));}
        assert_eq!(calls.load(Ordering::SeqCst),1);
        app.shutdown().await.unwrap();drop(binding);drop(app);drop(client);close(store).await;
    }).await.unwrap();
}

struct HeldMutation {
    store: Arc<EventLog>,
    started: Arc<Semaphore>,
    release: Arc<Semaphore>,
    resource_started: Arc<Semaphore>,
    resource_release: Arc<Semaphore>,
}
impl Handler for HeldMutation {
    fn call(&self, _: String, input: Value, context: Context) -> BoxFuture<'static, Result<Value>> {
        let store = self.store.clone();
        let reference = context.operation().unwrap().clone();
        let scope = context.scope().clone();
        let started = self.started.clone();
        let release = self.release.clone();
        let resource_started = self.resource_started.clone();
        let resource_release = self.resource_release.clone();
        Box::pin(async move {
            assert_eq!(
                store
                    .operation_observation(&reference)
                    .await
                    .unwrap()
                    .unwrap()
                    .outcome,
                OperationOutcome::Unresolved
            );
            started.add_permits(1);
            permit(&release).await;
            let mut ticket = scope.resources.reserve().unwrap();
            ticket.start();
            tokio::spawn(async move {
                resource_started.add_permits(1);
                permit(&resource_release).await;
                ticket.complete(Ok(()));
            });
            Ok(input)
        })
    }
}
#[tokio::test]
async fn dropped_root_and_all_observers_keep_mutation_resources_native_close_and_root_lease() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let (dir, ns, store) = store().await;
        let issuer = Issuer::default();
        let started = Arc::new(Semaphore::new(0));
        let release = Arc::new(Semaphore::new(0));
        let resource_started = Arc::new(Semaphore::new(0));
        let resource_release = Arc::new(Semaphore::new(0));
        let close_started = Arc::new(Semaphore::new(0));
        let close_release = Arc::new(Semaphore::new(0));
        let handler: Arc<dyn Handler> = Arc::new(HeldMutation {
            store: store.clone(),
            started: started.clone(),
            release: release.clone(),
            resource_started: resource_started.clone(),
            resource_release: resource_release.clone(),
        });
        let mut registry = NativeRegistry::default();
        let closing = close_started.clone();
        let closed = close_release.clone();
        registry
            .insert(
                "native.echo".into(),
                Arc::new(move |_| {
                    let handler = handler.clone();
                    let closing = closing.clone();
                    let closed = closed.clone();
                    Box::pin(async move {
                        NativeActivation {
                            exports: Ok(BTreeMap::from([(
                                "echo".into(),
                                Published {
                                    contract: contract(MethodMode::Mutation),
                                    value: handler,
                                },
                            )])),
                            owner: native_owner(move || {
                                let closing = closing.clone();
                                let closed = closed.clone();
                                Box::pin(async move {
                                    closing.add_permits(1);
                                    permit(&closed).await;
                                    Ok(())
                                })
                            }),
                        }
                    })
                }),
            )
            .unwrap();
        let (provider, consumer) = mutation_packages();
        let deployment = Uuid::new_v4();
        store
            .commit_application(
                deployment,
                0,
                input(&provider, &consumer),
                vec![],
                vec![provider, consumer],
            )
            .await
            .unwrap();
        let app = Application::start_from_store(
            store.clone(),
            deployment,
            registry,
            issuer.clone(),
            Arc::new(Grant(issuer.clone())),
            Limits::default(),
        )
        .await
        .unwrap();
        app.ready().await.unwrap();
        let client = app.client();
        let binding = client
            .bind(client.activation(&id(2)).unwrap(), "echo".into())
            .await
            .unwrap()
            .remove(0);
        let accepted = client
            .mutate(
                binding,
                "echo".into(),
                json!("held"),
                scope(&issuer, true),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        let reference = accepted.recovery.clone().unwrap();
        permit(&started).await;
        let weak = Arc::downgrade(&store);
        drop(accepted);
        drop(app);
        drop(client);
        drop(store);
        assert!(RootOwner::open(&dir.path().join("root"), &ns).is_err());
        release.add_permits(1);
        permit(&resource_started).await;
        let observation = weak
            .upgrade()
            .unwrap()
            .operation_observation(&reference)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(observation.outcome, OperationOutcome::Unresolved);
        assert_eq!(observation.cleanup, OperationCleanup::Unconfirmed);
        assert_eq!(close_started.available_permits(), 0);
        assert!(RootOwner::open(&dir.path().join("root"), &ns).is_err());
        resource_release.add_permits(1);
        permit(&close_started).await;
        let observation = weak
            .upgrade()
            .unwrap()
            .operation_observation(&reference)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            observation.outcome,
            OperationOutcome::Returned {
                value: json!("held")
            }
        );
        assert_eq!(observation.cleanup, OperationCleanup::Confirmed);
        assert!(RootOwner::open(&dir.path().join("root"), &ns).is_err());
        close_release.add_permits(1);
        loop {
            match RootOwner::open(&dir.path().join("root"), &ns) {
                Ok(owner) => {
                    let reopened = EventLog::for_root(Arc::new(owner)).await.unwrap();
                    assert_eq!(
                        reopened
                            .operation_observation(&reference)
                            .await
                            .unwrap()
                            .unwrap(),
                        observation
                    );
                    reopened.close().await.unwrap();
                    break;
                }
                Err(_) => tokio::task::yield_now().await,
            }
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn returned_mutation_and_failed_cleanup_are_separate_persistent_facts() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let (dir, ns, store) = store().await;
        let issuer = Issuer::default();
        let calls = Arc::new(AtomicUsize::new(0));
        let (app, _) = launch(
            store.clone(),
            &issuer,
            Arc::new(AtomicBool::new(true)),
            Arc::new(VerifiedEcho {
                store: store.clone(),
                calls: calls.clone(),
            }),
        )
        .await;
        let client = app.client();
        let binding = client
            .bind(client.activation(&id(2)).unwrap(), "echo".into())
            .await
            .unwrap()
            .remove(0);
        let accepted = client
            .mutate(
                binding,
                "echo".into(),
                json!("resource-fail"),
                scope(&issuer, true),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        let reference = accepted.recovery.clone().unwrap();
        assert_eq!(accepted.result().await.unwrap(), json!("resource-fail"));
        let observation = store
            .operation_observation(&reference)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            observation.outcome,
            OperationOutcome::Returned {
                value: json!("resource-fail")
            }
        );
        assert!(matches!(
            observation.cleanup,
            OperationCleanup::Failed { .. }
        ));
        assert!(app.shutdown().await.is_err());
        drop(app);
        drop(client);
        drop(store);
        tokio::task::yield_now().await;
        assert!(RootOwner::open(&dir.path().join("root"), &ns).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    })
    .await
    .unwrap();
}

struct UncertainSettlement {
    store: Arc<EventLog>,
    database: std::path::PathBuf,
    calls: Arc<AtomicUsize>,
}
impl Handler for UncertainSettlement {
    fn call(&self, _: String, input: Value, context: Context) -> BoxFuture<'static, Result<Value>> {
        let store = self.store.clone();
        let reference = context.operation().unwrap().clone();
        let database = self.database.clone();
        let calls = self.calls.clone();
        Box::pin(async move {
            assert_eq!(
                store
                    .operation_observation(&reference)
                    .await
                    .unwrap()
                    .unwrap()
                    .outcome,
                OperationOutcome::Unresolved
            );
            calls.fetch_add(1, Ordering::SeqCst);
            let connection = rusqlite::Connection::open(database).unwrap();
            connection.execute_batch("CREATE TRIGGER reject_operation_settlement BEFORE UPDATE OF outcome_json ON application_operations BEGIN SELECT RAISE(ABORT,'test settlement unavailable'); END;").unwrap();
            Ok(input)
        })
    }
}
#[tokio::test]
async fn uncertain_durable_settlement_does_not_replay_effect_or_invent_cleanup_failure() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let (dir, _ns, store) = store().await;
        let issuer = Issuer::default();
        let calls = Arc::new(AtomicUsize::new(0));
        let database = dir
            .path()
            .join("root")
            .join(maka_event_log::root::ROOT_DATABASE);
        let (app, _) = launch(
            store.clone(),
            &issuer,
            Arc::new(AtomicBool::new(true)),
            Arc::new(UncertainSettlement {
                store: store.clone(),
                database: database.clone(),
                calls: calls.clone(),
            }),
        )
        .await;
        let client = app.client();
        let consumer = client.activation(&id(2)).unwrap();
        let binding = client
            .bind(consumer.clone(), "echo".into())
            .await
            .unwrap()
            .remove(0);
        let operation = Uuid::new_v4();
        let accepted = client
            .mutate(
                binding.clone(),
                "echo".into(),
                json!("effect"),
                scope(&issuer, true),
                operation,
            )
            .await
            .unwrap();
        let reference = accepted.recovery.clone().unwrap();
        assert!(
            accepted
                .result()
                .await
                .unwrap_err()
                .contains("settlement outcome unknown")
        );
        assert!(client.snapshot().cleanup_error.is_none());
        let observation = client
            .recover(consumer, reference.clone(), scope(&issuer, true))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(observation.outcome, OperationOutcome::Unresolved);
        assert_eq!(observation.cleanup, OperationCleanup::Unconfirmed);
        assert!(
            client
                .mutate(
                    binding,
                    "echo".into(),
                    json!("effect"),
                    scope(&issuer, true),
                    operation
                )
                .await
                .unwrap()
                .result()
                .await
                .unwrap_err()
                .contains("no replay")
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        app.shutdown().await.unwrap();
        drop(app);
        drop(client);
        close(store).await;
        let connection = rusqlite::Connection::open(database).unwrap();
        connection
            .execute_batch("DROP TRIGGER reject_operation_settlement;")
            .unwrap();
    })
    .await
    .unwrap();
}
