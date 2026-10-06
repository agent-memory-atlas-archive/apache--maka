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
    contract::{Availability, Export},
    deployment::management::{
        ManagedDeployment, PackageInstalled, deployment_manager_contract, package_bundle_type,
    },
    package::{EntryTarget, Implementation},
};
use maka_plugins::{call::ReadResource, package::Package};
use maka_ui::actions::*;
use std::sync::Mutex;

fn fixture() -> Fixture {
    fixture_with(maka_runtime_host::application::management::DEPLOYMENT_MANAGER)
}
fn fixture_with(implementation: &str) -> Fixture {
    let contract = deployment_manager_contract();
    let (mut host, originals) = support::host();
    let mut packages = Vec::new();
    for old in originals {
        if old.manifest().id == "local.receipts" {
            packages.push(old);
            continue;
        }
        let mut declaration = old.manifest().application.clone().unwrap();
        let entry = declaration.entries.get_mut(&EntryTarget::Host).unwrap();
        if old.manifest().id == "local.business" {
            entry.implementation = Implementation::Native {
                implementation: implementation.into(),
            };
            entry.exports = BTreeMap::from([(
                "Management".into(),
                Export {
                    contract: contract.clone(),
                    availability: Availability::Required,
                },
            )]);
        } else {
            entry.imports.get_mut("business").unwrap().contract = contract.clone();
        }
        let package = support::counter::package(&old.manifest().id, declaration, None);
        for node in host.instances.values_mut() {
            if node.package == old.application_ref().into() {
                node.package = package.application_ref().into();
            }
        }
        packages.push(package);
    }
    let maka_plugin_core::composition::Target::Instance { export, .. } = &mut host
        .instances
        .get_mut("gateway")
        .unwrap()
        .bindings
        .get_mut("business")
        .unwrap()[0]
    else {
        panic!("gateway binding")
    };
    *export = "Management".into();
    let (mut ui, mut ui_packages, mut observations) = support::ui(PresentationTarget::Desktop);
    let old = ui_packages.pop().unwrap();
    let mut declaration = old.manifest().application.clone().unwrap();
    for entry in declaration.entries.values_mut() {
        entry.imports.get_mut("business").unwrap().contract = contract.clone();
    }
    let mut files = old.files().clone();
    files.insert(
        maka_plugins::package::MANIFEST_FILE.into(),
        serde_json::to_vec(
            &json!({"schemaVersion":2,"id":old.manifest().id,"application":declaration}),
        )
        .unwrap(),
    );
    for module in ["index.mjs", "terminal.mjs"] {
        let source = String::from_utf8(files[module].clone()).unwrap().replace(
            "input:recovery?state.reference:null",
            "input:recovery?state.reference:{}",
        );
        files.insert(module.into(), source.into_bytes());
    }
    let package = Package::new(files).unwrap();
    ui.instances.get_mut("app").unwrap().package = package.application_ref().into();
    ui_packages.push(package);
    let maka_plugin_core::composition::Target::Remote { export, .. } = &mut ui
        .instances
        .get_mut("app")
        .unwrap()
        .bindings
        .get_mut("business")
        .unwrap()[0]
    else {
        panic!("UI binding")
    };
    *export = "Management".into();
    let observation = observations
        .iter_mut()
        .find(|o| o.capability.export == "Counter")
        .unwrap();
    observation.capability.export = "Management".into();
    observation.declaration.contract = contract;
    Fixture::from_sources((host, packages), (ui, ui_packages, observations))
}
struct Capture {
    inner: Arc<dyn NativeService>,
    source: Arc<Mutex<Option<ServiceSource>>>,
}
impl NativeService for Capture {
    fn call(&self, request: ServiceRequest) -> ServiceFuture {
        *self.source.lock().unwrap() = Some(request.source.clone());
        self.inner.call(request)
    }
}
struct Bundle(Vec<u8>);
impl ReadResource for Bundle {
    fn resource_type(&self) -> maka_plugin_core::contract::ResourceType {
        package_bundle_type()
    }
    fn bytes(&self) -> usize {
        self.0.len()
    }
    fn metadata(&self) -> Value {
        json!({})
    }
    fn read(&self, range: std::ops::Range<usize>) -> Result<Vec<u8>, String> {
        self.0.get(range).map(<[u8]>::to_vec).ok_or("range".into())
    }
}
#[test]
fn actual_child_manager_install_uses_original_grants_dual_roots_and_journaled_immutable_resource() {
    for install_allowed in [false, true] {
        let f = fixture();
        let mut config = f.configuration(true, true);
        config.hosts[0].imports.get_mut("business").unwrap().methods = if install_allowed {
            BTreeSet::from(["read".into(), "install".into(), "replace".into()])
        } else {
            BTreeSet::from(["read".into()])
        };
        let mut connections = f.connect(config);
        let mut registrations = connections.registrations();
        let registration = registrations
            .iter_mut()
            .find(|r| r.target.export == "Management")
            .unwrap();
        let actual = registration.implementation.clone();
        let captured = Arc::new(Mutex::new(None));
        registration.implementation = Arc::new(Capture {
            inner: actual.clone(),
            source: captured.clone(),
        });
        let mut app = f
            .timer
            .block_on(Application::start_from_store(
                f.caller.clone(),
                f.source.clone(),
                f.target,
                f.timer.handle().clone(),
                Default::default(),
                support::counter::scene::registry(f.target),
                registrations,
            ))
            .unwrap();
        let mount = app.mount_launch(json!({})).unwrap();
        let initial: ManagedDeployment =
            serde_json::from_value(action(&f, &mut app, &mount, "read")["value"].clone()).unwrap();
        assert_eq!(initial.source.reference.deployment, f.host);
        let source = captured.lock().unwrap().take().unwrap();
        let mut bytes = support::host().1.remove(0).files().clone();
        let manifest: Value =
            serde_json::from_slice(&bytes[maka_plugins::package::MANIFEST_FILE]).unwrap();
        let mut manifest = manifest;
        manifest["id"] = json!("installed.waits");
        bytes.insert(
            maka_plugins::package::MANIFEST_FILE.into(),
            serde_json::to_vec(&manifest).unwrap(),
        );
        bytes.insert(
            "index.mjs".into(),
            b"throw Error('install must never execute this package');".to_vec(),
        );
        let package = Package::new(bytes).unwrap();
        let bundle = Arc::new(Bundle(package.to_bundle()));
        let weak = Arc::downgrade(&bundle);
        let session = actual
            .mutation(ServiceRequest {
                source,
                method: "install".into(),
                input: json!({"expectedRevision":1}),
                resources: BTreeMap::from([("package".into(), bundle as Arc<dyn ReadResource>)]),
            })
            .unwrap();
        let reference = f.timer.block_on(session.clone().prepare());
        let original = if install_allowed {
            let reference = reference.unwrap();
            assert_eq!(reference.source, initial.source.reference);
            let intent = f
                .timer
                .block_on(f.caller.record_application_intent(
                    f.source.clone(),
                    support::identity(2),
                    reference.clone(),
                ))
                .unwrap();
            let completion = f.timer.block_on(session.clone().dispatch(intent));
            completion.cleanup.unwrap();
            let Receipt::Recorded {
                outcome: maka_plugin_core::operation::OperationOutcome::Returned { value },
                cleanup: maka_plugin_core::operation::OperationCleanup::Confirmed,
            } = &completion.reply.receipt
            else {
                panic!("{:?}", completion.reply)
            };
            let installed: PackageInstalled =
                serde_json::from_value(value["value"].clone()).unwrap();
            assert_eq!(installed.source.deployment, f.host);
            assert_eq!(installed.source.revision, 2);
            assert_eq!(installed.package, package.application_ref());
            assert_ne!(installed.source.state_domain, f.source.state_domain);
            Some(completion.reply)
        } else {
            assert!(reference.err().unwrap().contains("granted"));
            None
        };
        f.timer.block_on(session.clone().close()).unwrap();
        assert!(weak.upgrade().is_none());
        drop(session);
        drop(actual);
        let current: ManagedDeployment =
            serde_json::from_value(action(&f, &mut app, &mount, "read")["value"].clone()).unwrap();
        assert_eq!(
            current.source.reference.revision,
            if install_allowed { 2 } else { 1 }
        );
        assert_eq!(
            current.packages.contains(&package.application_ref()),
            install_allowed
        );
        assert_eq!(
            current.source.input.instances.len(),
            initial.source.input.instances.len()
        );
        stop(&f, app, &mount, &mut connections);
        let store = f
            .timer
            .block_on(EventLog::for_root(Arc::new(
                RootOwner::open(&f.directory.path().join("host"), &f.namespaces).unwrap(),
            )))
            .unwrap();
        if let Some(original) = original {
            let recorded = f
                .timer
                .block_on(store.operation_observation(&original.reference))
                .unwrap()
                .unwrap();
            assert!(matches!(
                recorded.outcome,
                maka_plugin_core::operation::OperationOutcome::Returned { .. }
            ));
            let loaded = f
                .timer
                .block_on(store.load_application(f.host))
                .unwrap()
                .unwrap();
            assert_eq!(
                loaded
                    .packages
                    .iter()
                    .find(|p| p.application_ref() == package.application_ref())
                    .unwrap()
                    .files(),
                package.files()
            );
        }
        f.timer.block_on(store.close()).unwrap();
        f.timer.block_on(f.caller.shutdown()).unwrap();
    }
}

#[test]
fn cohost_manager_installs_ui_bytes_and_adopts_without_reopening_root_or_replacing_host() {
    for install_allowed in [false, true] {
        let f = fixture_with(maka_application_bridge::UI_DEPLOYMENT_MANAGER);
        let seed = f
            .timer
            .block_on(EventLog::for_root(Arc::new(
                RootOwner::open(&f.directory.path().join("host"), &f.namespaces).unwrap(),
            )))
            .unwrap();
        let loaded = f
            .timer
            .block_on(seed.load_application(f.host))
            .unwrap()
            .unwrap();
        f.timer.block_on(seed.close()).unwrap();
        let host = f
            .timer
            .block_on(f.caller.commit_application(
                f.host,
                0,
                loaded.source.input,
                loaded.source.remote_exports,
                loaded.packages,
            ))
            .unwrap();
        let mut config = f.configuration(true, true);
        config.hosts[0].location = HostLocation::Cohost {};
        config.hosts[0].imports.get_mut("business").unwrap().methods = if install_allowed {
            BTreeSet::from(["read".into(), "install".into(), "replace".into()])
        } else {
            BTreeSet::from(["read".into()])
        };
        // No child executable exists. Cohost must use the original owned Store.
        let mut connections = f
            .timer
            .block_on(Connections::prepare(
                f.caller.clone(),
                f.source.deployment,
                f.target,
                config,
                f.namespaces.clone(),
                HostExecutable::new("absent-cohost-executable".into(), vec![]).unwrap(),
                Default::default(),
            ))
            .unwrap();
        f.timer.block_on(connections.start()).unwrap();
        f.reopened("host");
        let mut registrations = connections.registrations();
        let registration = registrations
            .iter_mut()
            .find(|r| r.target.export == "Management")
            .unwrap();
        let actual = registration.implementation.clone();
        let epoch = registration.registration_epoch;
        let captured = Arc::new(Mutex::new(None));
        registration.implementation = Arc::new(Capture {
            inner: actual.clone(),
            source: captured.clone(),
        });
        let mut app = f
            .timer
            .block_on(Application::start_from_store(
                f.caller.clone(),
                f.source.clone(),
                f.target,
                f.timer.handle().clone(),
                Default::default(),
                support::counter::scene::registry(f.target),
                registrations,
            ))
            .unwrap();
        let mount = app.mount_launch(json!({})).unwrap();
        let initial: ManagedDeployment =
            serde_json::from_value(action(&f, &mut app, &mount, "read")["value"].clone()).unwrap();
        assert_eq!(initial.source.reference, f.source);
        let source = captured.lock().unwrap().take().unwrap();
        let mut bytes = support::host().1.remove(0).files().clone();
        let mut manifest: Value =
            serde_json::from_slice(&bytes[maka_plugins::package::MANIFEST_FILE]).unwrap();
        manifest["id"] = json!("cohost.installed.waits");
        bytes.insert(
            maka_plugins::package::MANIFEST_FILE.into(),
            serde_json::to_vec(&manifest).unwrap(),
        );
        bytes.insert(
            "index.mjs".into(),
            b"throw Error('installation must not activate');".to_vec(),
        );
        let package = Package::new(bytes).unwrap();
        let bundle = Arc::new(Bundle(package.to_bundle()));
        let weak = Arc::downgrade(&bundle);
        let session = actual
            .mutation(ServiceRequest {
                source,
                method: "install".into(),
                input: json!({"expectedRevision":1}),
                resources: BTreeMap::from([("package".into(), bundle as Arc<dyn ReadResource>)]),
            })
            .unwrap();
        let prepared = f.timer.block_on(session.clone().prepare());
        let original = if install_allowed {
            let reference = prepared.unwrap();
            assert_eq!(reference.source, host);
            let intent = f
                .timer
                .block_on(f.caller.record_application_intent(
                    f.source.clone(),
                    support::identity(2),
                    reference,
                ))
                .unwrap();
            let completion = f.timer.block_on(session.clone().dispatch(intent));
            completion.cleanup.unwrap();
            let Receipt::Recorded {
                outcome: maka_plugin_core::operation::OperationOutcome::Returned { value },
                cleanup: maka_plugin_core::operation::OperationCleanup::Confirmed,
            } = &completion.reply.receipt
            else {
                panic!("{:?}", completion.reply)
            };
            let installed: PackageInstalled =
                serde_json::from_value(value["value"].clone()).unwrap();
            assert_eq!(installed.source.deployment, f.source.deployment);
            assert_eq!(installed.source.state_domain, host.state_domain);
            assert_eq!(installed.source.revision, 2);
            assert_eq!(installed.package, package.application_ref());
            let update = f
                .timer
                .block_on(app.load_committed_update(installed.source.clone()).unwrap())
                .unwrap();
            let refreshed = f
                .timer
                .block_on(connections.approve_source(
                    &f.caller,
                    installed.source,
                    BTreeSet::from([1]),
                ))
                .unwrap();
            let same = refreshed
                .iter()
                .find(|r| r.target.export == "Management")
                .unwrap();
            assert_eq!(same.registration_epoch, epoch);
            assert!(Arc::ptr_eq(&same.implementation, &actual));
            app.adopt_update(update, refreshed).unwrap();
            assert!(
                app.frame(&mount).is_ok(),
                "original UI mount survives unchanged code"
            );
            Some(completion.reply)
        } else {
            assert!(prepared.err().unwrap().contains("granted"));
            None
        };
        f.timer.block_on(session.clone().close()).unwrap();
        assert!(weak.upgrade().is_none());
        drop(session);
        let current: ManagedDeployment =
            serde_json::from_value(action(&f, &mut app, &mount, "read")["value"].clone()).unwrap();
        assert_eq!(
            current.source.reference.revision,
            if install_allowed { 2 } else { 1 }
        );
        assert_eq!(
            f.timer.block_on(f.caller.application_head(f.host)).unwrap(),
            Some(host)
        );
        if install_allowed {
            let request_source = captured.lock().unwrap().take().unwrap();
            let caller_source = request_source.deployment().unwrap().clone();
            let consumer = request_source.consumer().instance().clone();
            let replace = actual
                .mutation(ServiceRequest {
                    source: request_source,
                    method: "replace".into(),
                    input: serde_json::to_value(
                        maka_plugin_core::deployment::management::DeploymentReplace {
                            expected_revision: 2,
                            input: current.source.input,
                            remote_exports: current.source.remote_exports,
                            packages: current.packages,
                        },
                    )
                    .unwrap(),
                    resources: BTreeMap::new(),
                })
                .unwrap();
            let reference = f.timer.block_on(replace.clone().prepare()).unwrap();
            assert_eq!(reference.source.deployment, f.host);
            let intent = f
                .timer
                .block_on(
                    f.caller
                        .record_application_intent(caller_source, consumer, reference),
                )
                .unwrap();
            let completion = f.timer.block_on(replace.clone().dispatch(intent));
            completion.cleanup.unwrap();
            let Receipt::Recorded {
                outcome: maka_plugin_core::operation::OperationOutcome::Returned { value },
                cleanup: maka_plugin_core::operation::OperationCleanup::Confirmed,
            } = completion.reply.receipt
            else {
                panic!("{:?}", completion.reply)
            };
            let next: DeploymentRef = serde_json::from_value(value["value"].clone()).unwrap();
            assert_eq!(next.revision, 3);
            assert_eq!(next.deployment, f.source.deployment);
            f.timer.block_on(replace.clone().close()).unwrap();
            drop(replace);
            let update = f
                .timer
                .block_on(app.load_committed_update(next.clone()).unwrap())
                .unwrap();
            let registrations = f
                .timer
                .block_on(connections.approve_source(&f.caller, next, BTreeSet::from([1])))
                .unwrap();
            app.adopt_update(update, registrations).unwrap();
            assert!(app.frame(&mount).is_ok());
        }
        if let Some(original) = original {
            assert!(matches!(
                f.timer
                    .block_on(f.caller.operation_observation(&original.reference))
                    .unwrap()
                    .unwrap()
                    .outcome,
                maka_plugin_core::operation::OperationOutcome::Returned { .. }
            ));
        }
        app.retire(&mount).unwrap();
        app.close(&mount, Ok(())).unwrap();
        app.shutdown().unwrap();
        drop(app);
        drop(actual);
        f.timer.block_on(connections.close()).unwrap();
        f.blocked("caller");
        assert!(
            f.timer
                .block_on(f.caller.load_application(f.source.deployment))
                .unwrap()
                .is_some()
        );
        f.timer.block_on(f.caller.shutdown()).unwrap();
        drop(connections);
        let Fixture {
            caller,
            directory,
            namespaces,
            timer: _timer,
            ..
        } = f;
        drop(caller);
        drop(RootOwner::open(&directory.path().join("caller"), &namespaces).unwrap());
    }
}
