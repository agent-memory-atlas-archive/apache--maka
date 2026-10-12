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
use crate::bridge_support as host_fixture;
use maka_event_log::{
    EventLog,
    root::{RootNamespaces, RootOwner},
};
use maka_host_runtime::application::{
    Context, Handler, NativeActivation, NativeOwner, NativeRegistry,
};
use maka_plugin_core::{
    composition::{Environment, ExecutionLocation, Input, Target},
    contract::{Cardinality, Import},
    deployment::RemoteObservation,
    package::{EntryTarget, Implementation},
    plan::RemoteCapability,
    runtime::Published,
};
use maka_transport::{Connections, HostExecutable};
use std::sync::atomic::{AtomicBool, Ordering};

struct CloseWrites {
    store: Arc<EventLog>,
    closing: Arc<tokio::sync::Semaphore>,
    release: Arc<tokio::sync::Semaphore>,
    committed: Arc<AtomicBool>,
    marker: uuid::Uuid,
}
impl Handler for CloseWrites {
    fn call(
        &self,
        _: String,
        _: serde_json::Value,
        _: Context,
    ) -> futures_util::future::BoxFuture<'static, Result<serde_json::Value, String>> {
        Box::pin(async { Ok(serde_json::json!({"effects":0})) })
    }
}
impl NativeOwner for CloseWrites {
    fn close(&self) -> futures_util::future::BoxFuture<'static, Result<(), String>> {
        let store = self.store.clone();
        let closing = self.closing.clone();
        let release = self.release.clone();
        let committed = self.committed.clone();
        let marker = self.marker;
        Box::pin(async move {
            closing.add_permits(1);
            release
                .acquire_owned()
                .await
                .map_err(|error| error.to_string())?
                .forget();
            store
                .commit_application(
                    marker,
                    0,
                    Input {
                        environment: Environment {
                            location: ExecutionLocation::Host,
                            presentation: None,
                        },
                        instances: BTreeMap::new(),
                        recovery: None,
                        launch: None,
                    },
                    vec![],
                    vec![],
                )
                .await
                .map_err(|error| error.to_string())?;
            committed.store(true, Ordering::Release);
            Ok(())
        })
    }
}

#[gpui::test]
fn actual_quit_drains_cohost_native_sql_write_before_shared_store_close(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let timer = timer();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root");
    let namespaces = RootNamespaces {
        ownership: directory.path().join("owners"),
        control: directory.path().join("control"),
    };
    let store = Arc::new(
        timer
            .block_on(EventLog::for_root(Arc::new(
                RootOwner::create(&root, &namespaces).unwrap(),
            )))
            .unwrap(),
    );
    let (mut host_input, originals) = host_fixture::host();
    let mut host_packages = Vec::new();
    for old in originals {
        if old.manifest().id != "local.business" {
            host_packages.push(old);
            continue;
        }
        let mut declaration = old.manifest().application.clone().unwrap();
        declaration
            .entries
            .get_mut(&EntryTarget::Host)
            .unwrap()
            .implementation = Implementation::Native {
            implementation: "cohost.close-writes".into(),
        };
        let package = host_fixture::counter::package("local.business", declaration, None);
        host_input.instances.get_mut("business").unwrap().package =
            package.application_ref().into();
        host_packages.push(package);
    }
    let host = uuid::Uuid::new_v4();
    timer
        .block_on(store.commit_application(host, 0, host_input, vec![], host_packages))
        .unwrap();
    let (mut input, mut packages) = support::editor_fixture();
    let index = packages
        .iter()
        .position(|package| package.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(index);
    let mut declaration = old.manifest().application.clone().unwrap();
    declaration
        .entries
        .get_mut(&EntryTarget::Desktop)
        .unwrap()
        .imports
        .insert(
            "cohost".into(),
            Import {
                contract: host_fixture::business(),
                cardinality: Cardinality::One,
            },
        );
    let package = support::package(
        "dynamic.app",
        declaration,
        Some(
            std::str::from_utf8(old.file("index.mjs").unwrap())
                .unwrap()
                .into(),
        ),
    );
    for node in input.instances.values_mut() {
        if node.package == old.application_ref().into() {
            node.package = package.application_ref().into();
            node.bindings.insert(
                "cohost".into(),
                vec![Target::Remote {
                    deployment: "local".into(),
                    instance: host_fixture::identity(11),
                    export: "Counter".into(),
                }],
            );
        }
    }
    packages.push(package);
    let observation = RemoteObservation {
        capability: RemoteCapability {
            deployment: "local".into(),
            instance: host_fixture::identity(11),
            export: "Counter".into(),
        },
        declaration: maka_plugin_core::contract::Export {
            contract: host_fixture::business(),
            availability: maka_plugin_core::contract::Availability::Required,
        },
    };
    let ui = timer
        .block_on(store.commit_application(
            uuid::Uuid::new_v4(),
            0,
            input,
            vec![observation],
            packages,
        ))
        .unwrap();
    let closing = Arc::new(tokio::sync::Semaphore::new(0));
    let release = Arc::new(tokio::sync::Semaphore::new(0));
    let committed = Arc::new(AtomicBool::new(false));
    let marker = uuid::Uuid::new_v4();
    let mut native = NativeRegistry::default();
    let closing_factory = closing.clone();
    let release_factory = release.clone();
    let committed_factory = committed.clone();
    native
        .insert(
            "cohost.close-writes".into(),
            Arc::new(move |context, _| {
                let resource = Arc::new(CloseWrites {
                    store: context.persistence().unwrap().store().clone(),
                    closing: closing_factory.clone(),
                    release: release_factory.clone(),
                    committed: committed_factory.clone(),
                    marker,
                });
                Box::pin(async move {
                    NativeActivation {
                        exports: Ok(BTreeMap::from([(
                            "Counter".into(),
                            Published {
                                contract: host_fixture::business(),
                                value: resource.clone() as Arc<dyn Handler>,
                            },
                        )])),
                        owner: resource,
                    }
                })
            }),
        )
        .unwrap();
    let configuration = serde_json::from_value(serde_json::json!({"hosts":[{
        "name":"local", "location":{"kind":"cohost"}, "deployment":host, "endpoints":{"business":{"consumer":host_fixture::identity(13),"import":"business","grant":{"methods":["read"],"recovery":false}}}
    }]})).unwrap();
    let mut connections = timer
        .block_on(Connections::prepare(
            store.clone(),
            ui.deployment,
            PresentationTarget::Desktop,
            configuration,
            namespaces.clone(),
            HostExecutable::new("absent-cohost-executable".into(), vec![]).unwrap(),
            native,
            Some(maka_host_js::factory(Default::default())),
        ))
        .unwrap();
    timer.block_on(connections.start()).unwrap();
    let mut application = timer
        .block_on(Application::start_from_store(
            store.clone(),
            ui.clone(),
            PresentationTarget::Desktop,
            timer.handle().clone(),
            maka_ui_js::execution(Default::default()),
            widgets::registry(),
            connections.registrations(),
        ))
        .unwrap();
    let connections = Arc::new(tokio::sync::Mutex::new(connections));
    let mount = application.mount_launch(serde_json::json!({})).unwrap();
    let application = Rc::new(RefCell::new(application));
    let owner = cx.update(|cx| {
        gpui_kit::init(cx);
        let owner = actions::own(
            application.clone(),
            Some(connections.clone()),
            Some(store.clone()),
            cx,
        )
        .unwrap();
        owner.open_launch(mount, serde_json::json!({}), cx).unwrap();
        owner
    });
    let scene = Scene::from_launch(&owner, cx);
    scene.draw(cx);
    let editor = scene.owner();
    let editor_id = editor.entity_id();
    cx.update_window(scene.window, |_, window, cx| {
        editor.update(cx, |state, cx| {
            state
                .state
                .read(cx)
                .presentation()
                .focus_handle()
                .focus(window, cx);
            state.replace_text_in_range(Some(0..5), "原草稿🙂", window, cx)
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert_eq!(editor.read(cx).state.read(cx).value().as_ref(), "原草稿🙂"));
    let loaded = timer
        .block_on(store.load_application(ui.deployment))
        .unwrap()
        .unwrap();
    let next = timer
        .block_on(store.commit_application(
            ui.deployment,
            1,
            loaded.source.input,
            loaded.source.remote_exports,
            loaded.packages,
        ))
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while application.borrow().source() != Some(&next) {
        cx.run_until_parked();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    scene.draw(cx);
    let (original, original_window, _) = owner.current_launch().unwrap();
    assert_eq!(original, scene.root);
    assert_eq!(original_window, scene.window);
    assert_eq!(scene.owner().entity_id(), editor_id);
    cx.update(|cx| assert_eq!(editor.read(cx).state.read(cx).value().as_ref(), "原草稿🙂"));
    scene.close(cx);
    assert!(
        !scene.app.borrow().can_shutdown(),
        "held original native entity is not a cleanup receipt"
    );
    cx.update(|_| drop(editor));
    cx.run_until_parked();
    assert!(
        scene.app.borrow().can_shutdown(),
        "{}",
        scene.resources.borrow().cleanup_diagnostics()
    );
    cx.update(|cx| actions::quit_when_drained(&scene.app, cx));
    cx.run_until_parked();
    timer.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(5), closing.acquire())
            .await
            .unwrap()
            .unwrap()
            .forget();
    });
    cx.run_until_parked();
    assert!(cx.update(|cx| cx.has_global::<actions::Owner>()));
    assert!(!committed.load(Ordering::Acquire));
    assert!(
        timer
            .block_on(store.application_head(host))
            .unwrap()
            .is_some(),
        "shared SQL must still accept original Host work"
    );
    release.add_permits(1);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while cx.update(|cx| cx.has_global::<actions::Owner>()) {
        cx.run_until_parked();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(committed.load(Ordering::Acquire));
    use futures_util::FutureExt;
    assert!(matches!(store.shutdown().now_or_never(), Some(Ok(()))));
    drop(scene);
    drop(owner);
    drop(application);
    drop(connections);
    drop(store);
    cx.run_until_parked();
    let reopened = timer
        .block_on(EventLog::for_root(Arc::new(
            RootOwner::open(&root, &namespaces).unwrap(),
        )))
        .unwrap();
    assert!(
        timer
            .block_on(reopened.application_head(marker))
            .unwrap()
            .is_some()
    );
    timer.block_on(reopened.close()).unwrap();
}
