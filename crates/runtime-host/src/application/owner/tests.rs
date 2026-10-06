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
    composition::{Configuration, Environment, Node, PackageUse, Target},
    contract::{Availability, Cardinality, Export, Import, Method, Transport},
    package::{Declaration, EntryDeclaration, EntryTarget},
};
use maka_plugins::call::Identity;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
struct Allowed;
impl Policy for Allowed {
    fn admit(&self, _: Invocation<'_>) -> Result<()> {
        Ok(())
    }
}
struct Count(Arc<AtomicUsize>);
impl Handler for Count {
    fn call(&self, _: String, input: Value, _: Context) -> BoxFuture<'static, Result<Value>> {
        let count = self.0.clone();
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(input)
        })
    }
}
#[tokio::test]
async fn final_admission_rejects_failed_consumer_before_terminal_notification_is_polled() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let contract = Contract {
            name: "example.Echo".into(),
            version: 1,
            transport: Transport::Local,
            interface: Interface::Service {
                methods: BTreeMap::from([(
                    "echo".into(),
                    Method {
                        mode: MethodMode::Query,
                        input: json!({}),
                        output: json!({}),
                        resources: BTreeMap::new(),
                    },
                )]),
            },
        };
        let mut packages = Packages::default();
        let mut nodes = BTreeMap::new();
        for (name, key, provider) in [("provider", 1, true), ("consumer", 2, false)] {
            let package = PackageRef {
                id: name.into(),
                revision: format!("sha256-{:064x}", key),
            };
            packages
                .insert(
                    package.clone(),
                    Declaration {
                        composition: None,
                        entries: BTreeMap::from([(
                            EntryTarget::Host,
                            EntryDeclaration {
                                scene: None,
                                implementation: if provider {
                                    Implementation::Native {
                                        implementation: "native".into(),
                                    }
                                } else {
                                    Implementation::JavaScript {
                                        module: "consumer.mjs".into(),
                                        realm: Realm::Shared,
                                    }
                                },
                                configuration: json!({}),
                                imports: if provider {
                                    BTreeMap::new()
                                } else {
                                    BTreeMap::from([(
                                        "echo".into(),
                                        Import {
                                            contract: contract.clone(),
                                            cardinality: Cardinality::One,
                                        },
                                    )])
                                },
                                exports: if provider {
                                    BTreeMap::from([(
                                        "echo".into(),
                                        Export {
                                            contract: contract.clone(),
                                            availability: Availability::Required,
                                        },
                                    )])
                                } else {
                                    BTreeMap::new()
                                },
                            },
                        )]),
                    },
                )
                .unwrap();
            nodes.insert(
                name.into(),
                Node {
                    key: Uuid::from_u128(key),
                    package: package.into(),
                    use_package: PackageUse::Entry,
                    configuration: Configuration::default(),
                    parameters: Configuration::default(),
                    bindings: if provider {
                        BTreeMap::new()
                    } else {
                        BTreeMap::from([(
                            "echo".into(),
                            vec![Target::Instance {
                                path: vec!["provider".into()],
                                export: "echo".into(),
                            }],
                        )])
                    },
                    disabled: false,
                    overrides: Vec::new(),
                },
            );
        }
        let input = Input {
            environment: Environment {
                location: ExecutionLocation::Host,
                presentation: None,
            },
            instances: nodes,
            launch: None,
        };
        let plan = Plan::prepare(&input, &packages, &RemoteExports::new()).unwrap();
        let waiting_replacement = plan.instances()[&InstanceId(vec![Uuid::from_u128(2)])].clone();
        let issuer = Issuer::default();
        let mut owner = Owner::new(
            plan,
            input,
            BTreeMap::new(),
            NativeRegistry::default(),
            issuer.clone(),
            Arc::new(Allowed),
            Limits::default(),
            watch::channel(Snapshot::default()).0,
            Weak::new(),
            None,
        );
        owner.pending.clear();
        owner.starting = false;
        let provider = owner
            .runtime
            .begin(&InstanceId(vec![Uuid::from_u128(1)]))
            .unwrap();
        let consumer = owner
            .runtime
            .begin(&InstanceId(vec![Uuid::from_u128(2)]))
            .unwrap();
        owner
            .activations
            .insert(provider.instance().clone(), provider.clone());
        owner
            .activations
            .insert(consumer.instance().clone(), consumer.clone());
        let count = Arc::new(AtomicUsize::new(0));
        owner
            .runtime
            .publish(
                &provider,
                BTreeMap::from([(
                    "echo".into(),
                    Published {
                        contract: contract.clone(),
                        value: Service::new(
                            contract,
                            ImplementationValue::Native(Arc::new(Count(count.clone()))),
                        )
                        .unwrap(),
                    },
                )]),
            )
            .map_err(|e| e.error)
            .unwrap();
        let resource = owner
            .runtime
            .own_resource(&consumer, "actual consumer module")
            .unwrap();
        let vm = owner.new_vm().unwrap();
        let module = owner.vms[&vm]
            .load(
                "consumer.mjs".into(),
                "export function activate(){return {}; }".into(),
            )
            .unwrap();
        module.call(vec!["activate".into()], vec![]).await.unwrap();
        owner.owned.insert(
            consumer.instance().clone(),
            Owned {
                imports: Arc::new(BTreeMap::new()),
                resource,
                close: Some(Box::pin(async move {
                    module.close().await.map_err(|e| e.to_string())
                })),
                initializing: false,
                closing: false,
                native_owner: None,
                vm: Some(vm),
            },
        );
        owner
            .runtime
            .publish(&consumer, BTreeMap::new())
            .map_err(|e| e.error)
            .unwrap();
        let binding = owner.capture_bindings(&consumer, "echo").unwrap().remove(0);
        owner.vms[&vm].terminate("actual consumer execution domain failure");
        owner.vms[&vm].failed().await;
        // Deliberately leave the notification unpolled. The command must check
        // actual domain health before accepting work from this consumer.
        assert_eq!(
            owner.runtime.status(&consumer).unwrap().phase,
            Phase::Active
        );
        let scope = issuer
            .issue(
                Identity::Remote {
                    request_id: Uuid::new_v4(),
                },
                CancellationToken::new(),
            )
            .unwrap();
        let (reply, result) = oneshot::channel();
        owner.command(Command::Call(Box::new(Call {
            kind: CallKind::Query(reply),
            parent: None,
            binding,
            method: "echo".into(),
            input: Value::Null,
            scope,
            attachments: Attachments::new(),
            capture: None,
            resources: Arc::new(Semaphore::new(0))
                .try_acquire_many_owned(0)
                .unwrap(),
            slot: Arc::new(Semaphore::new(1)).try_acquire_owned().unwrap(),
            bytes: Arc::new(Semaphore::new(1)).try_acquire_owned().unwrap(),
        })));
        assert!(
            result
                .await
                .unwrap()
                .err()
                .unwrap()
                .contains("domain terminated")
        );
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(owner.jobs.is_empty());
        assert_eq!(
            owner.runtime.status(&provider).unwrap().phase,
            Phase::Active
        );
        assert_eq!(
            owner.runtime.status(&consumer).unwrap().phase,
            Phase::Retiring
        );
        // A new candidate is waiting on this old, already retiring domain.
        // Its delayed actual failure must not erase that replacement.
        owner.starting = true;
        owner.pending.push(waiting_replacement);
        owner.retire_vm(vm, "delayed original domain failure".into());
        assert!(owner.starting);
        assert_eq!(owner.pending.len(), 1);
        assert!(owner.startup_error.is_none());
        assert!(!owner.closing);
        owner.pending.clear();
        owner.starting = false;
        let original_vm = owner.vms[&vm].clone();
        owner.drive();
        while !owner.jobs.is_empty() {
            let done = owner.jobs.next().await.unwrap();
            owner.complete(done);
            owner.drive();
        }
        assert!(original_vm.is_terminated());
        assert!(owner.vms.is_empty());
        assert!(!owner.closing);
        assert_eq!(
            owner.runtime.status(&provider).unwrap().phase,
            Phase::Active
        );
        // Healthy module replacement closes the real isolate, even while a
        // later plan is starting; its terminal notification is not a failure.
        for _ in 0..16 {
            let activation = owner.runtime.begin(consumer.instance()).unwrap();
            owner
                .activations
                .insert(activation.instance().clone(), activation.clone());
            let resource = owner
                .runtime
                .own_resource(&activation, "healthy module")
                .unwrap();
            let index = owner.new_vm().unwrap();
            let vm = owner.vms[&index].clone();
            let module = vm
                .load(
                    "healthy.mjs".into(),
                    "export function activate(){return {}; }".into(),
                )
                .unwrap();
            module.call(vec!["activate".into()], vec![]).await.unwrap();
            assert!(!vm.is_terminated());
            owner.owned.insert(
                activation.instance().clone(),
                Owned {
                    imports: Arc::new(BTreeMap::new()),
                    resource,
                    close: Some(Box::pin(async move {
                        module.close().await.map_err(|e| e.to_string())
                    })),
                    initializing: false,
                    closing: false,
                    native_owner: None,
                    vm: Some(index),
                },
            );
            owner
                .runtime
                .publish(&activation, BTreeMap::new())
                .map_err(|e| e.error)
                .unwrap();
            owner.runtime.retire(&activation).unwrap();
            owner.drive();
            while !owner.jobs.is_empty() {
                let done = owner.jobs.next().await.unwrap();
                owner.complete(done);
                owner.drive();
            }
            assert!(vm.is_terminated());
            assert!(owner.vms.is_empty());
            assert!(owner.runtime.status(&activation).is_err());
            owner.starting = true;
            owner.retire_vm(index, "normal shutdown notification".into());
            assert!(owner.starting);
            assert!(owner.startup_error.is_none());
            owner.starting = false;
        }
        owner.stop();
        owner.drive();
        while let Some(done) = owner.jobs.next().await {
            owner.complete(done);
            owner.drive();
            if owner.closed {
                break;
            }
        }
        assert!(owner.closed);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn retained_old_head_after_new_explicit_adoption_does_not_poison_readiness() {
    use maka_event_log::root::{RootNamespaces, RootOwner};
    use maka_plugin_core::composition::{Environment, ExecutionLocation};
    let dir = tempfile::tempdir().unwrap();
    let ns = RootNamespaces {
        ownership: dir.path().join("owners"),
        control: dir.path().join("control"),
    };
    let store = Arc::new(
        maka_event_log::EventLog::for_root(Arc::new(
            RootOwner::create(&dir.path().join("root"), &ns).unwrap(),
        ))
        .await
        .unwrap(),
    );
    let input = Input {
        environment: Environment {
            location: ExecutionLocation::Host,
            presentation: None,
        },
        instances: BTreeMap::new(),
        launch: None,
    };
    let id = Uuid::new_v4();
    let first = store
        .commit_application(id, 0, input.clone(), vec![], vec![])
        .await
        .unwrap();
    let cursor = store.subscribe_application(first.clone()).unwrap();
    let second = store
        .commit_application(id, 1, input.clone(), vec![], vec![])
        .await
        .unwrap();
    // This is a real completed SQL head read retained across another owner turn.
    let (cursor, observed) = next_change(cursor).await;
    assert_eq!(observed.as_ref().unwrap(), &second);
    let third = store
        .commit_application(id, 2, input.clone(), vec![], vec![])
        .await
        .unwrap();
    let native = NativeRegistry::default();
    let (plan, sources) = prepare(&input, vec![], &native, true).unwrap();
    let (updates, _) = watch::channel(Snapshot::default());
    let mut owner = Owner::new(
        plan,
        input,
        sources,
        native,
        Issuer::default(),
        Arc::new(Allowed),
        Limits::default(),
        updates,
        Weak::new(),
        Some((first, store.clone())),
    );
    owner
        .adopt(store.load_application_at(&third).await.unwrap().unwrap())
        .unwrap();
    owner.drive();
    owner.observe_source(observed.unwrap());
    let (_, latest) = next_change(cursor).await;
    owner.observe_source(latest.unwrap());
    let snapshot = owner.snapshot();
    assert_eq!(snapshot.desired, Some(third.clone()));
    assert_eq!(snapshot.effective, Some(third));
    assert!(snapshot.startup_error.is_none());
    assert!(owner.loading_adoption.is_none());
    assert!(owner.check_adoption_source(&second).is_err());
    drop(owner);
    Arc::try_unwrap(store).ok().unwrap().close().await.unwrap();
}
