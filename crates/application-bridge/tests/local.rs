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

#[path = "local/subscriptions.rs"]
mod subscriptions;
mod support;
use maka_application_bridge::{Configuration, Connections, Grant, Host};
use maka_event_log::{
    EventLog,
    root::{RootNamespaces, RootOwner},
};
use maka_plugin_core::{
    composition::PresentationTarget,
    deployment::DeploymentRef,
    operation::{OperationReply, Receipt},
};
use maka_ui::Application;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use uuid::Uuid;
struct Fixture {
    timer: tokio::runtime::Runtime,
    directory: tempfile::TempDir,
    namespaces: RootNamespaces,
    caller: Arc<EventLog>,
    source: DeploymentRef,
    host: Uuid,
}
impl Fixture {
    fn new() -> Self {
        Self::from_sources(support::host(), support::ui(PresentationTarget::Desktop))
    }
    fn from_sources(
        host_sources: (
            maka_plugin_core::composition::Input,
            Vec<maka_plugins::package::Package>,
        ),
        ui_sources: (
            maka_plugin_core::composition::Input,
            Vec<maka_plugins::package::Package>,
            Vec<maka_plugin_core::deployment::RemoteObservation>,
        ),
    ) -> Self {
        let timer = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_time()
            .build()
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let namespaces = RootNamespaces {
            ownership: directory.path().join("owners"),
            control: directory.path().join("control"),
        };
        let open = |name| {
            let owner =
                Arc::new(RootOwner::create(&directory.path().join(name), &namespaces).unwrap());
            Arc::new(timer.block_on(EventLog::for_root(owner)).unwrap())
        };
        let caller = open("caller");
        let host_store = open("host");
        let host = Uuid::new_v4();
        let (input, packages) = host_sources;
        timer
            .block_on(host_store.commit_application(host, 0, input, vec![], packages))
            .unwrap();
        timer.block_on(host_store.shutdown()).unwrap();
        drop(host_store);
        let (input, packages, observations) = ui_sources;
        let source = timer
            .block_on(caller.commit_application(Uuid::new_v4(), 0, input, observations, packages))
            .unwrap();
        Self {
            timer,
            directory,
            namespaces,
            caller,
            source,
            host,
        }
    }
    fn configuration(&self, read: bool, recovery: bool) -> Configuration {
        Configuration {
            hosts: vec![Host {
                name: "local".into(),
                root: self.directory.path().join("host"),
                deployment: self.host,
                consumer: support::identity(13),
                imports: BTreeMap::from([
                    (
                        "business".into(),
                        Grant {
                            methods: if read {
                                BTreeSet::from(["read".into(), "write".into()])
                            } else {
                                BTreeSet::from(["write".into()])
                            },
                            recovery: false,
                        },
                    ),
                    (
                        "receipts".into(),
                        Grant {
                            methods: BTreeSet::from(["read".into()]),
                            recovery,
                        },
                    ),
                ]),
            }],
        }
    }
    fn connect(&self, config: Configuration) -> Connections {
        self.timer
            .block_on(Connections::start(
                &self.caller,
                self.source.deployment,
                config,
                &self.namespaces,
                maka_runtime_host::application::NativeRegistry::default(),
            ))
            .unwrap()
    }
    fn ui(&self, connections: &mut Connections) -> Application {
        self.ui_target(connections, PresentationTarget::Desktop)
    }
    fn ui_target(&self, connections: &mut Connections, target: PresentationTarget) -> Application {
        self.timer
            .block_on(Application::start_from_store(
                self.caller.clone(),
                connections.source().clone(),
                target,
                self.timer.handle().clone(),
                Default::default(),
                crate::support::counter::scene::registry(target),
                connections.take_registrations(),
            ))
            .unwrap()
    }
    fn blocked(&self, name: &str) {
        assert_eq!(
            RootOwner::open(&self.directory.path().join(name), &self.namespaces)
                .err()
                .unwrap()
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    fn reopened(&self, name: &str) {
        drop(RootOwner::open(&self.directory.path().join(name), &self.namespaces).unwrap());
    }
}
fn action(
    f: &Fixture,
    app: &mut Application,
    mount: &maka_plugin_core::runtime::Mount,
    key: &str,
) -> Value {
    let revision = app.frame(mount).unwrap().revision;
    app.dispatch(mount, revision, key, Value::Null).unwrap();
    app.capture_actions(mount, |_| panic!("resource-free request"))
        .unwrap();
    let mut task = app.start_task(mount).unwrap().expect("actual task");
    loop {
        let completion = f.timer.block_on(task.future);
        match app.finish_task(task.id, completion).unwrap() {
            Some(next) => task = next,
            None => break,
        }
    }
    assert!(app.start_task(mount).unwrap().is_none());
    assert_eq!(app.deliver_action_results(mount).unwrap(), 1);
    app.frame(mount).unwrap().state["outcome"].clone()
}
fn stop(
    f: &Fixture,
    mut app: Application,
    mount: &maka_plugin_core::runtime::Mount,
    connections: &mut Connections,
) {
    app.retire(mount).unwrap();
    app.close(mount, Ok(())).unwrap();
    app.shutdown().unwrap();
    drop(app);
    f.blocked("host");
    f.timer.block_on(connections.close()).unwrap();
    f.reopened("host");
}
#[test]
fn local_query_mutation_and_original_receipt_use_actual_dual_roots_and_selected_source() {
    let f = Fixture::new();
    let mut connections = f.connect(f.configuration(true, true));
    f.blocked("caller");
    f.blocked("host");
    let (input, packages, observations) = support::ui(PresentationTarget::Desktop);
    let newer = f
        .timer
        .block_on(f.caller.commit_application(
            f.source.deployment,
            1,
            input,
            observations,
            packages,
        ))
        .unwrap();
    assert_ne!(newer, connections.source().clone());
    let mut app = f.ui(&mut connections);
    assert_eq!(app.source(), Some(&f.source));
    let mount = app.mount_launch(json!({})).unwrap();
    assert_eq!(
        action(&f, &mut app, &mount, "read"),
        json!({"status":"succeeded","value":{"effects":0}})
    );
    let outcome = action(&f, &mut app, &mount, "write");
    let reply: OperationReply = serde_json::from_value(outcome["reply"].clone()).unwrap();
    assert!(matches!(reply.receipt, Receipt::Recorded { .. }));
    let history = f
        .timer
        .block_on(
            f.caller
                .application_intents(f.source.deployment, support::identity(2), 0, 10),
        )
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].reference, reply.reference);
    let recovered = action(&f, &mut app, &mount, "recover");
    let original: OperationReply = serde_json::from_value(recovered["value"].clone()).unwrap();
    assert_eq!(original, reply);
    assert_eq!(
        action(&f, &mut app, &mount, "read")["value"],
        json!({"effects":1})
    );
    stop(&f, app, &mount, &mut connections);
    let owner = Arc::new(RootOwner::open(&f.directory.path().join("host"), &f.namespaces).unwrap());
    let journal = f.timer.block_on(EventLog::for_root(owner)).unwrap();
    let saved = f
        .timer
        .block_on(journal.operation_observation(&reply.reference))
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(saved.outcome).unwrap()["value"],
        json!({"effects":1})
    );
    f.timer.block_on(journal.close()).unwrap();
    // An EventLog handle itself retains its Root lease even after SQL shutdown.
    // Consume the real last caller handle before checking actual Root reopen.
    let Fixture {
        timer,
        directory,
        namespaces,
        caller,
        ..
    } = f;
    timer
        .block_on(Arc::try_unwrap(caller).ok().unwrap().close())
        .unwrap();
    drop(RootOwner::open(&directory.path().join("caller"), &namespaces).unwrap());
}
#[test]
fn ordinary_methods_and_original_recovery_require_separate_explicit_grants() {
    let f = Fixture::new();
    let mut connections = f.connect(f.configuration(false, false));
    let mut app = f.ui(&mut connections);
    let mount = app.mount_launch(json!({})).unwrap();
    assert_eq!(action(&f, &mut app, &mount, "read")["status"], "failed");
    let outcome = action(&f, &mut app, &mount, "write");
    let original: OperationReply = serde_json::from_value(outcome["reply"].clone()).unwrap();
    let recovered = action(&f, &mut app, &mount, "recover");
    let denied: OperationReply = serde_json::from_value(recovered["value"].clone()).unwrap();
    assert_eq!(denied.reference, original.reference);
    assert!(matches!(denied.receipt, Receipt::Unavailable { .. }));
    assert!(!app.is_retiring(&mount));
    stop(&f, app, &mount, &mut connections);
    f.timer.block_on(f.caller.shutdown()).unwrap();
}
#[test]
fn failed_connection_preflight_drains_every_started_host_without_unlocking_the_caller() {
    let f = Fixture::new();
    let mut configuration = f.configuration(true, true);
    configuration.hosts[0].name = "undeclared".into();
    let result = f.timer.block_on(Connections::start(
        &f.caller,
        f.source.deployment,
        configuration,
        &f.namespaces,
        maka_runtime_host::application::NativeRegistry::default(),
    ));
    assert!(matches!(result,Err(reason) if reason.contains("observation")));
    f.blocked("caller");
    f.reopened("host");
    let mut configuration = f.configuration(true, true);
    configuration.hosts[0]
        .imports
        .get_mut("business")
        .unwrap()
        .methods
        .insert("undeclared".into());
    let result = f.timer.block_on(Connections::start(
        &f.caller,
        f.source.deployment,
        configuration,
        &f.namespaces,
        maka_runtime_host::application::NativeRegistry::default(),
    ));
    assert!(matches!(result,Err(reason) if reason.contains("undeclared service method")));
    f.reopened("host");
    f.timer.block_on(f.caller.shutdown()).unwrap();
}
#[test]
fn startup_configuration_is_closed_and_bounded_before_opening_any_host() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("connections.json");
    for data in [
        br#"{"hosts":[],"approved":true}"#.as_slice(),
        br#"{"hosts":[{"name":"local","credential":"fake"}]}"#.as_slice(),
    ] {
        std::fs::write(&path, data).unwrap();
        assert!(Configuration::read(&path).is_err());
    }
    std::fs::write(&path, vec![b' '; 1_048_577]).unwrap();
    assert!(Configuration::read(&path).err().unwrap().contains("1 MiB"));
    std::fs::write(&path, br#"{"hosts":[]}"#).unwrap();
    assert!(Configuration::read(&path).unwrap().hosts.is_empty());
}

#[test]
fn compiled_native_availability_keeps_real_file_and_root_until_actual_close() {
    use futures_util::future::BoxFuture;
    use maka_plugin_core::runtime::Published;
    use maka_runtime_host::application::{
        Context, Handler, NativeActivation, NativeOwner, NativeRegistry,
    };
    use std::sync::Mutex;
    struct FileOwner {
        file: Arc<Mutex<Option<std::fs::File>>>,
        closing: Arc<tokio::sync::Semaphore>,
        release: Arc<tokio::sync::Semaphore>,
    }
    impl Handler for FileOwner {
        fn call(
            &self,
            _: String,
            _: Value,
            _: Context,
        ) -> BoxFuture<'static, Result<Value, String>> {
            Box::pin(async { Ok(json!({"effects":0})) })
        }
    }
    impl NativeOwner for FileOwner {
        fn close(&self) -> BoxFuture<'static, Result<(), String>> {
            let file = self.file.clone();
            let closing = self.closing.clone();
            let release = self.release.clone();
            Box::pin(async move {
                closing.add_permits(1);
                release.acquire_owned().await.unwrap().forget();
                file.lock().unwrap().take();
                Ok(())
            })
        }
    }
    let f = Fixture::new();
    let owner = Arc::new(RootOwner::open(&f.directory.path().join("host"), &f.namespaces).unwrap());
    let store = f.timer.block_on(EventLog::for_root(owner)).unwrap();
    let (input, packages) = support::native_host("native.File");
    f.timer
        .block_on(store.commit_application(f.host, 1, input, vec![], packages))
        .unwrap();
    f.timer.block_on(store.close()).unwrap();
    let file = Arc::new(Mutex::new(Some(
        std::fs::File::create(f.directory.path().join("native-file")).unwrap(),
    )));
    let closing = Arc::new(tokio::sync::Semaphore::new(0));
    let release = Arc::new(tokio::sync::Semaphore::new(0));
    let resource = Arc::new(FileOwner {
        file: file.clone(),
        closing: closing.clone(),
        release: release.clone(),
    });
    let mut native = NativeRegistry::default();
    native
        .insert(
            "native.File".into(),
            Arc::new(move |_, _| {
                let resource = resource.clone();
                Box::pin(async move {
                    NativeActivation {
                        exports: Ok(BTreeMap::from([(
                            "Counter".into(),
                            Published {
                                contract: support::business(),
                                value: resource.clone() as Arc<dyn Handler>,
                            },
                        )])),
                        owner: resource,
                    }
                })
            }),
        )
        .unwrap();
    let mut connections = f
        .timer
        .block_on(Connections::start(
            &f.caller,
            f.source.deployment,
            f.configuration(true, true),
            &f.namespaces,
            native,
        ))
        .unwrap();
    let close = f.timer.spawn(async move {
        connections.close().await.unwrap();
        connections
    });
    f.timer.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(3), closing.acquire_owned())
            .await
            .unwrap()
            .unwrap()
            .forget();
    });
    f.blocked("host");
    file.lock().unwrap().as_ref().unwrap().metadata().unwrap();
    assert!(!close.is_finished());
    release.add_permits(1);
    let mut connections = f.timer.block_on(close).unwrap();
    assert!(file.lock().unwrap().is_none());
    f.reopened("host");
    f.timer.block_on(connections.close()).unwrap();
    f.timer.block_on(f.caller.shutdown()).unwrap();
}
