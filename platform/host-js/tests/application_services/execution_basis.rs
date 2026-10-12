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
use maka_host_runtime::application::Binding;

struct FixedTarget(Option<Value>);
impl NativeOwner for FixedTarget {
    fn execution_basis(&self) -> Option<Value> {
        self.0.clone()
    }
    fn close(&self) -> BoxFuture<'static, Result<()>> {
        Box::pin(async { Ok(()) })
    }
}
fn native(target: Option<Value>) -> NativeRegistry {
    registry_with_owner(Arc::new(FixedTarget(target)))
}
fn registry_with_owner(owner: Arc<dyn NativeOwner>) -> NativeRegistry {
    let mut native = NativeRegistry::default();
    native
        .insert(
            "native.echo".into(),
            Arc::new(move |_, _| {
                let owner = owner.clone();
                Box::pin(async move {
                    NativeActivation {
                        exports: Ok(BTreeMap::from([(
                            "echo".into(),
                            Published {
                                contract: contract(MethodMode::Query),
                                value: Arc::new(Echo {
                                    calls: Arc::new(AtomicUsize::new(0)),
                                }) as Arc<dyn Handler>,
                            },
                        )])),
                        owner,
                    }
                })
            }),
        )
        .unwrap();
    native
}
async fn capture(input: Input, packages: Vec<Package>, native: NativeRegistry) -> Binding {
    let issuer = Issuer::default();
    let app = Application::start(
        input,
        packages,
        native,
        issuer.clone(),
        Arc::new(Grant(issuer)),
        Some(maka_host_js::factory(Limits::default())),
    )
    .unwrap();
    app.ready().await.unwrap();
    let client = app.client();
    let binding = client
        .bind(client.activation(&id(2)).unwrap(), "echo".into())
        .await
        .unwrap()
        .remove(0);
    // Retaining the comparison facts does not retain the provider's resources.
    app.shutdown().await.unwrap();
    binding
}

#[tokio::test]
async fn execution_basis_preserves_configuration_and_native_target_across_restarts() {
    let provider = native_package();
    let consumer = consumer();
    let packages = vec![provider.clone(), consumer.clone()];
    let mut source = input(&provider, &consumer);
    source
        .instances
        .get_mut("provider")
        .unwrap()
        .configuration
        .values
        .insert("nested".into(), json!({"a":1,"b":2}));
    let original = capture(
        source.clone(),
        packages.clone(),
        native(Some(json!({"target":"one"}))),
    )
    .await;
    // Cosmetic paths and unrelated consumer configuration do not change the
    // actual provider. JSON object order is not an execution fact either.
    let mut same = source.clone();
    same.instances
        .get_mut("b")
        .unwrap()
        .configuration
        .values
        .insert("unrelated".into(), json!(true));
    let mut renamed = same.instances.remove("provider").unwrap();
    renamed
        .configuration
        .values
        .insert("nested".into(), json!({"b":2,"a":1}));
    same.instances.insert("renamed".into(), renamed);
    for name in ["a", "b"] {
        same.instances.get_mut(name).unwrap().bindings.insert(
            "echo".into(),
            vec![Target::Instance {
                path: vec!["renamed".into()],
                export: "echo".into(),
            }],
        );
    }
    let equivalent = capture(
        same,
        packages.clone(),
        native(Some(json!({"target":"one"}))),
    )
    .await;
    assert_ne!(
        original.registration_epoch(),
        equivalent.registration_epoch()
    );
    assert_eq!(original.execution_basis(), equivalent.execution_basis());
    let changed_target = capture(
        source.clone(),
        packages.clone(),
        native(Some(json!({"target":"two"}))),
    )
    .await;
    assert_ne!(original.execution_basis(), changed_target.execution_basis());
    source
        .instances
        .get_mut("provider")
        .unwrap()
        .configuration
        .values
        .insert("nested".into(), json!({"a":2,"b":2}));
    let changed_configuration =
        capture(source, packages, native(Some(json!({"target":"one"})))).await;
    assert_ne!(
        original.execution_basis(),
        changed_configuration.execution_basis()
    );
}

#[tokio::test]
async fn execution_basis_tracks_captured_dependencies_and_optional_absence() {
    let leaf = native_package();
    let mut wrapper_entry = entry(
        true,
        Implementation::JavaScript {
            module: "host.mjs".into(),
            realm: Realm::Shared,
        },
        MethodMode::Query,
    );
    wrapper_entry.imports.insert(
        "echo".into(),
        Import {
            contract: contract(MethodMode::Query),
            cardinality: Cardinality::Optional,
        },
    );
    let wrapper = package(
        "wrapper",
        wrapper_entry,
        Some(format!(
            "export const service = {{ echo(value) {{ return value; }} }}; export function activate() {{ return {{ echo: {{ contract: {}, service: ['service'] }} }}; }}",
            serde_json::to_string(&contract(MethodMode::Query)).unwrap()
        )),
    );
    let consumer = consumer();
    let mut source = input(&wrapper, &consumer);
    source.instances.insert("leaf".into(), node(&leaf, 4, true));
    source
        .instances
        .get_mut("provider")
        .unwrap()
        .bindings
        .insert(
            "echo".into(),
            vec![Target::Instance {
                path: vec!["leaf".into()],
                export: "echo".into(),
            }],
        );
    let packages = vec![wrapper, consumer, leaf.clone()];
    let original = capture(source.clone(), packages.clone(), native(Some(Value::Null))).await;
    let equivalent = capture(source.clone(), packages.clone(), native(Some(Value::Null))).await;
    assert_eq!(original.execution_basis(), equivalent.execution_basis());
    let mut changed = source.clone();
    changed
        .instances
        .get_mut("leaf")
        .unwrap()
        .configuration
        .values
        .insert("route".into(), json!("different"));
    let changed_configuration = capture(changed, packages.clone(), native(Some(Value::Null))).await;
    assert_ne!(
        original.execution_basis(),
        changed_configuration.execution_basis()
    );
    let replacement = package(
        "replacement",
        entry(
            true,
            Implementation::Native {
                implementation: "native.echo".into(),
            },
            MethodMode::Query,
        ),
        None,
    );
    let mut changed = source.clone();
    changed.instances.get_mut("leaf").unwrap().package = replacement.application_ref().into();
    let mut replaced_packages = packages.clone();
    replaced_packages.push(replacement);
    let changed_package = capture(changed, replaced_packages, native(Some(Value::Null))).await;
    assert_ne!(
        original.execution_basis(),
        changed_package.execution_basis()
    );
    let opaque = capture(source.clone(), packages.clone(), native(None)).await;
    let restarted_opaque = capture(source.clone(), packages.clone(), native(None)).await;
    assert_ne!(opaque.execution_basis(), restarted_opaque.execution_basis());
    source
        .instances
        .get_mut("provider")
        .unwrap()
        .bindings
        .remove("echo");
    let absent = capture(source, packages, native(Some(Value::Null))).await;
    assert_ne!(original.execution_basis(), absent.execution_basis());
}

#[tokio::test]
async fn execution_basis_panic_keeps_native_owner_for_actual_close() {
    struct Panics(Arc<AtomicUsize>);
    impl NativeOwner for Panics {
        fn execution_basis(&self) -> Option<Value> {
            panic!("fixture basis panic")
        }
        fn close(&self) -> BoxFuture<'static, Result<()>> {
            let closed = self.0.clone();
            Box::pin(async move {
                closed.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }
    }
    let closed = Arc::new(AtomicUsize::new(0));
    let provider = native_package();
    let consumer = consumer();
    let issuer = Issuer::default();
    let app = Application::start(
        input(&provider, &consumer),
        vec![provider, consumer],
        registry_with_owner(Arc::new(Panics(closed.clone()))),
        issuer.clone(),
        Arc::new(Grant(issuer)),
        Some(maka_host_js::factory(Limits::default())),
    )
    .unwrap();
    assert!(
        app.ready()
            .await
            .unwrap_err()
            .contains("native execution basis panicked")
    );
    app.shutdown().await.unwrap();
    assert_eq!(closed.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn native_activation_hook_starts_once_after_publish_and_panic_keeps_real_cleanup() {
    struct Hook {
        started: Arc<AtomicUsize>,
        settled: Arc<AtomicUsize>,
        closed: Arc<AtomicUsize>,
        jobs: tokio_util::task::TaskTracker,
        stop: CancellationToken,
        panic: bool,
    }
    impl NativeOwner for Hook {
        fn activated(&self) {
            assert_eq!(self.started.fetch_add(1, Ordering::SeqCst), 0);
            let stop = self.stop.clone();
            let settled = self.settled.clone();
            self.jobs.spawn(async move {
                stop.cancelled().await;
                settled.fetch_add(1, Ordering::SeqCst);
            });
            assert!(!self.panic, "fixture post-publication panic");
        }
        fn close(&self) -> BoxFuture<'static, Result<()>> {
            let jobs = self.jobs.clone();
            let stop = self.stop.clone();
            let closed = self.closed.clone();
            Box::pin(async move {
                jobs.close();
                stop.cancel();
                jobs.wait().await;
                closed.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }
    }
    for (reject, panic) in [(false, false), (true, false), (false, true)] {
        let started = Arc::new(AtomicUsize::new(0));
        let settled = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicUsize::new(0));
        let hook = Arc::new(Hook {
            started: started.clone(),
            settled: settled.clone(),
            closed: closed.clone(),
            jobs: tokio_util::task::TaskTracker::new(),
            stop: CancellationToken::new(),
            panic,
        });
        let mut native = NativeRegistry::default();
        native
            .insert(
                "native.echo".into(),
                Arc::new(move |_, _| {
                    let hook = hook.clone();
                    Box::pin(async move {
                        NativeActivation {
                            owner: hook,
                            exports: if reject {
                                Err("fixture rejected exports".into())
                            } else {
                                Ok(BTreeMap::from([(
                                    "echo".into(),
                                    Published {
                                        contract: contract(MethodMode::Query),
                                        value: Arc::new(Echo {
                                            calls: Arc::new(AtomicUsize::new(0)),
                                        })
                                            as Arc<dyn Handler>,
                                    },
                                )]))
                            },
                        }
                    })
                }),
            )
            .unwrap();
        let provider = native_package();
        let consumer = consumer();
        let issuer = Issuer::default();
        let app = Application::start(
            input(&provider, &consumer),
            vec![provider, consumer],
            native,
            issuer.clone(),
            Arc::new(Grant(issuer)),
            Some(maka_host_js::factory(Limits::default())),
        )
        .unwrap();
        assert_eq!(app.ready().await.is_err(), reject || panic);
        app.shutdown().await.unwrap();
        assert_eq!(started.load(Ordering::SeqCst), usize::from(!reject));
        assert_eq!(settled.load(Ordering::SeqCst), usize::from(!reject));
        assert_eq!(closed.load(Ordering::SeqCst), 1);
    }
}
