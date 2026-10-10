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

#[path = "application_connections/deferred.rs"]
mod deferred;
#[path = "application_connections/management.rs"]
mod management;
#[path = "application_connections/resource_mutations.rs"]
mod resource_mutations;
#[path = "application_connections/subscriptions.rs"]
mod subscriptions;
#[allow(dead_code)]
#[path = "../../application-bridge/tests/support/mod.rs"]
mod support;
use maka_application_bridge::{
    Configuration, Connections, Grant, Host, HostExecutable, HostLocation,
};
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
    target: PresentationTarget,
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
            .enable_all()
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
        let target = input.environment.presentation.unwrap();
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
            target,
        }
    }
    fn configuration(&self, read: bool, recovery: bool) -> Configuration {
        Configuration {
            hosts: vec![Host {
                native: Vec::new(),
                name: "local".into(),
                location: HostLocation::Process {
                    state_domain: Some(
                        maka_event_log::root::resolve(&self.directory.path().join("host"))
                            .unwrap()
                            .root_id()
                            .into(),
                    ),
                    root: self.directory.path().join("host"),
                },
                deployment: self.host,
                endpoints: BTreeMap::<String, Grant>::from([
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
                ])
                .into_iter()
                .map(|(name, grant)| {
                    (
                        name.clone(),
                        maka_application_bridge::HostEndpoint {
                            consumer: support::identity(13),
                            import: name,
                            grant,
                        },
                    )
                })
                .collect(),
            }],
        }
    }
    fn prepare(&self, config: Configuration) -> Result<Connections, String> {
        self.timer.block_on(Connections::prepare(
            self.caller.clone(),
            self.source.deployment,
            self.target,
            config,
            self.namespaces.clone(),
            HostExecutable::new(
                std::path::PathBuf::from(env!("CARGO_BIN_EXE_maka")),
                vec!["application".into(), "serve".into()],
            )
            .unwrap(),
            Default::default(),
        ))
    }
    fn connect(&self, config: Configuration) -> Connections {
        let mut connections = self.prepare(config).unwrap();
        self.timer.block_on(connections.start()).unwrap();
        connections
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
                connections.registrations(),
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
    assert_eq!(
        app.deliver_action_results(mount, |_, _, _| Ok(false))
            .unwrap(),
        1
    );
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
fn child_query_mutation_and_original_receipt_use_actual_dual_roots_and_selected_source() {
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
fn preflight_and_failed_child_start_do_not_unlock_the_actual_caller() {
    let f = Fixture::new();
    let mut configuration = f.configuration(true, true);
    configuration.hosts[0].name = "undeclared".into();
    assert!(
        matches!(f.prepare(configuration), Err(reason) if reason.contains("unconfigured Host"))
    );
    f.blocked("caller");
    f.reopened("host");
    let mut configuration = f.configuration(true, true);
    configuration.hosts[0]
        .endpoints
        .get_mut("business")
        .unwrap()
        .grant
        .methods
        .insert("undeclared".into());
    let mut connections = f.prepare(configuration).unwrap();
    assert!(f.timer.block_on(connections.start()).is_err());
    f.timer.block_on(connections.close()).unwrap();
    f.reopened("host");
    f.blocked("caller");
    f.timer.block_on(f.caller.shutdown()).unwrap();
}

#[test]
fn presentation_is_rejected_before_a_child_or_host_root_is_opened() {
    let f = Fixture::new();
    let result = f.timer.block_on(Connections::prepare(
        f.caller.clone(),
        f.source.deployment,
        PresentationTarget::Tui,
        f.configuration(true, true),
        f.namespaces.clone(),
        HostExecutable::new("does-not-exist".into(), vec![]).unwrap(),
        Default::default(),
    ));
    assert!(matches!(result, Err(reason) if reason.contains("presentation")));
    f.reopened("host");
    f.blocked("caller");
    f.timer.block_on(f.caller.shutdown()).unwrap();
}

#[test]
fn cancelling_start_observation_retains_the_same_child_for_actual_close() {
    let (input, originals) = support::host();
    let mut packages = Vec::new();
    for old in originals {
        if old.manifest().id != "local.business" {
            packages.push(old);
            continue;
        }
        let declaration = old.manifest().application.clone().unwrap();
        let source = format!(
            r#"
            const began=Date.now();while(Date.now()-began<300){{}}
            export function activate(){{return {{Counter:{{contract:{},service:['service']}}}};}}
            export const service={{read(){{return {{effects:0}};}},write(){{return {{accepted:true}};}}}};
        "#,
            serde_json::to_string(&support::business()).unwrap()
        );
        let package = support::counter::package("local.business", declaration, Some(source));
        packages.push(package);
    }
    let mut input = input;
    input.instances.get_mut("business").unwrap().package = packages
        .iter()
        .find(|p| p.manifest().id == "local.business")
        .unwrap()
        .application_ref()
        .into();
    let f = Fixture::from_sources((input, packages), support::ui(PresentationTarget::Desktop));
    let mut wal = f
        .directory
        .path()
        .join("host")
        .join(maka_event_log::root::ROOT_DATABASE)
        .into_os_string();
    wal.push("-wal");
    let wal = std::path::PathBuf::from(wal);
    assert!(
        !wal.exists(),
        "the seeded original writer has actually closed"
    );
    let mut connections = f.prepare(f.configuration(true, true)).unwrap();
    f.timer.block_on(async {
        let startup = connections.start();
        tokio::pin!(startup);
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                tokio::select! {
                    biased;
                    result = &mut startup => panic!("expected an independently owned child still starting: {result:?}"),
                    () = tokio::time::sleep(std::time::Duration::from_millis(1)) => {
                        // Observe real child SQL startup without competing for
                        // its Root lease while it is acquiring ownership.
                        if wal.exists() {
                            break;
                        }
                    }
                }
            }
        }).await.unwrap();
        // Dropping this observer releases only its mutable borrow. The original
        // child has actually acquired its Root; the parent still owns it.
    });
    f.blocked("caller");
    f.timer.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(10), connections.close())
            .await
            .unwrap()
            .unwrap();
    });
    f.reopened("host");
    f.timer.block_on(f.caller.shutdown()).unwrap();
}

#[test]
fn failed_second_child_initialization_drains_the_first_actual_host() {
    let (mut input, packages, mut observations) = support::ui(PresentationTarget::Desktop);
    let mut second = input.instances["app"].clone();
    second.key = Uuid::new_v4();
    for targets in second.bindings.values_mut() {
        for target in targets {
            if let maka_plugin_core::composition::Target::Remote { deployment, .. } = target {
                *deployment = "secondary".into();
            }
        }
    }
    let extra: Vec<_> = observations
        .iter()
        .cloned()
        .map(|mut observation| {
            observation.capability.deployment = "secondary".into();
            observation
        })
        .collect();
    observations.extend(extra);
    input.instances.insert("second".into(), second);
    let f = Fixture::from_sources(support::host(), (input, packages, observations));
    let mut configuration = f.configuration(true, true);
    let mut second = f.configuration(true, true).hosts.pop().unwrap();
    second.name = "secondary".into();
    second.location = HostLocation::Process {
        state_domain: None,
        root: f.directory.path().join("absent"),
    };
    configuration.hosts.push(second);
    let mut connections = f.prepare(configuration).unwrap();
    assert!(f.timer.block_on(connections.start()).is_err());
    f.blocked("host");
    f.blocked("caller");
    f.timer.block_on(connections.close()).unwrap();
    f.reopened("host");
    f.blocked("caller");
    assert!(!f.directory.path().join("absent").exists());
    f.timer.block_on(f.caller.shutdown()).unwrap();
}

#[test]
fn actual_child_sql_initialization_and_cursor_errors_confirm_cleanup_without_ready() {
    use sqlx::Connection;
    for cursor in [false, true] {
        let f = Fixture::new();
        f.timer.block_on(async {
            let mut raw = sqlx::SqliteConnection::connect_with(&sqlx::sqlite::SqliteConnectOptions::new()
                .filename(f.directory.path().join("host").join(maka_event_log::root::ROOT_DATABASE)))
                .await.unwrap();
            sqlx::raw_sql(if cursor {
                "PRAGMA ignore_check_constraints=ON; INSERT INTO event_log(sequence,event_id,kind,event_json) VALUES(-1,'invalid','diagnostic','{}');"
            } else { "PRAGMA application_id=123;" }).execute(&mut raw).await.unwrap();
            raw.close().await.unwrap();
        });
        let mut connections = f.prepare(f.configuration(true, true)).unwrap();
        assert!(f.timer.block_on(connections.start()).is_err());
        f.timer.block_on(connections.close()).unwrap();
        f.reopened("host");
        f.blocked("caller");
        f.timer.block_on(f.caller.shutdown()).unwrap();
    }
}

#[test]
fn exact_source_approval_reuses_original_host_services_and_revokes_unused_consumers() {
    let f = Fixture::new();
    let mut connections = f.connect(f.configuration(true, true));
    let originals = connections.registrations();
    let mut old = f.ui(&mut connections);
    let old_mount = old.mount_launch(json!({})).unwrap();
    assert_eq!(
        action(&f, &mut old, &old_mount, "read")["status"],
        "succeeded"
    );
    let (mut input, packages, observations) = support::ui(PresentationTarget::Desktop);
    input.instances.get_mut("app").unwrap().key = Uuid::from_u128(22);
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
    let registrations = f
        .timer
        .block_on(connections.approve_source(&f.caller, newer.clone(), BTreeSet::from([1])))
        .unwrap();
    for registered in &registrations {
        let original = originals
            .iter()
            .find(|original| original.target == registered.target)
            .unwrap();
        assert_eq!(original.registration_epoch, registered.registration_epoch);
        assert!(Arc::ptr_eq(
            &original.implementation,
            &registered.implementation
        ));
    }
    f.blocked("host");
    let mut current = f
        .timer
        .block_on(Application::start_from_store(
            f.caller.clone(),
            newer.clone(),
            f.target,
            f.timer.handle().clone(),
            Default::default(),
            support::counter::scene::registry(f.target),
            registrations,
        ))
        .unwrap();
    let mount = current.mount_launch(json!({})).unwrap();
    assert_eq!(
        action(&f, &mut current, &mount, "read")["status"],
        "succeeded"
    );
    assert_eq!(
        action(&f, &mut old, &old_mount, "read")["status"],
        "succeeded"
    );
    f.timer
        .block_on(connections.approve_source(&f.caller, newer, BTreeSet::new()))
        .unwrap();
    assert_eq!(action(&f, &mut old, &old_mount, "read")["status"], "failed");
    assert_eq!(
        action(&f, &mut current, &mount, "read")["status"],
        "succeeded"
    );
    let (mut input, packages, observations) = support::ui(PresentationTarget::Desktop);
    input.instances.get_mut("app").unwrap().disabled = true;
    input.launch = None;
    let empty = f
        .timer
        .block_on(f.caller.commit_application(
            f.source.deployment,
            2,
            input,
            observations,
            packages,
        ))
        .unwrap();
    assert!(
        f.timer
            .block_on(connections.approve_source(&f.caller, empty, BTreeSet::new()))
            .unwrap()
            .is_empty()
    );
    assert_eq!(action(&f, &mut current, &mount, "read")["status"], "failed");
    old.retire(&old_mount).unwrap();
    old.close(&old_mount, Ok(())).unwrap();
    old.shutdown().unwrap();
    drop(old);
    stop(&f, current, &mount, &mut connections);
    f.timer.block_on(f.caller.shutdown()).unwrap();
}

#[test]
fn invalid_actual_endpoint_candidate_does_not_revoke_original_child_approval() {
    let f = Fixture::new();
    let mut connections = f.connect(f.configuration(true, true));
    let originals = connections.registrations();
    let mut app = f.ui(&mut connections);
    let mount = app.mount_launch(json!({})).unwrap();
    let (mut input, packages, mut observations) = support::ui(PresentationTarget::Desktop);
    let target = input
        .instances
        .get_mut("app")
        .unwrap()
        .bindings
        .get_mut("business")
        .unwrap()
        .first_mut()
        .unwrap();
    let maka_plugin_core::composition::Target::Remote { instance, .. } = target else {
        panic!("remote target")
    };
    *instance = support::identity(99);
    observations
        .iter_mut()
        .find(|o| o.capability.export == "Counter")
        .unwrap()
        .capability
        .instance = support::identity(99);
    let invalid = f
        .timer
        .block_on(f.caller.commit_application(
            f.source.deployment,
            1,
            input,
            observations,
            packages,
        ))
        .unwrap();
    assert!(
        f.timer
            .block_on(connections.approve_source(&f.caller, invalid, BTreeSet::new()))
            .err()
            .unwrap()
            .contains("configured grants")
    );
    for (before, after) in originals.iter().zip(connections.registrations()) {
        assert_eq!(before.registration_epoch, after.registration_epoch);
        assert!(Arc::ptr_eq(&before.implementation, &after.implementation));
    }
    assert_eq!(action(&f, &mut app, &mount, "read")["status"], "succeeded");
    assert_eq!(action(&f, &mut app, &mount, "write")["status"], "operation");
    f.blocked("host");
    stop(&f, app, &mount, &mut connections);
    f.timer.block_on(f.caller.shutdown()).unwrap();
}

#[test]
fn pinned_host_domain_is_checked_by_the_owned_child_before_application_start() {
    let f = Fixture::new();
    let mut configuration = f.configuration(true, true);
    if let HostLocation::Process { state_domain, .. } = &mut configuration.hosts[0].location {
        *state_domain = Some("f".repeat(64));
    }
    let mut connections = f.prepare(configuration).unwrap();
    assert!(f.timer.block_on(connections.start()).is_err());
    assert!(connections.registrations().is_empty());
    f.timer.block_on(connections.close()).unwrap();
    f.reopened("host");
    f.timer.block_on(f.caller.shutdown()).unwrap();
}
