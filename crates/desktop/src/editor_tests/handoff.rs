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

fn replacement(
    mut input: maka_plugin_core::composition::Input,
    mut packages: Vec<maka_plugins::package::Package>,
) -> (
    maka_plugin_core::composition::Input,
    Vec<maka_plugins::package::Package>,
) {
    let index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(index);
    let source = std::str::from_utf8(old.file("index.mjs").unwrap())
        .unwrap()
        .replace("Native editor product", "Replacement product")
        .replace("Immutable draft", "Replacement product");
    assert!(source.as_bytes() != old.file("index.mjs").unwrap());
    let package = support::package(
        "dynamic.app",
        old.manifest().application.clone().unwrap(),
        Some(source),
    );
    input.instances.get_mut("app").unwrap().package = package.application_ref().into();
    packages.push(package);
    (input, packages)
}
#[gpui::test]
fn replacement_waits_for_original_native_editor_release_and_opens_fresh_window(
    cx: &mut TestAppContext,
) {
    for quitting in [false, true] {
        let timer = timer();
        let (_dir, _ns, store) = stored_root(&timer);
        let deployment = uuid::Uuid::new_v4();
        let (input, packages) = support::editor_fixture();
        let original = timer
            .block_on(store.commit_application(
                deployment,
                0,
                input.clone(),
                vec![],
                packages.clone(),
            ))
            .unwrap();
        let mut app = timer
            .block_on(Application::start_from_store(
                store.clone(),
                original.clone(),
                PresentationTarget::Desktop,
                timer.handle().clone(),
                Default::default(),
                widgets::registry(),
                vec![],
            ))
            .unwrap();
        let root = app.mount_launch(serde_json::json!({})).unwrap();
        let app = Rc::new(RefCell::new(app));
        let owner = cx.update(|cx| {
            gpui_kit::init(cx);
            let owner = actions::own(app.clone(), None, None, cx).unwrap();
            owner.open_launch(root, serde_json::json!({}), cx).unwrap();
            owner
        });
        let old = Scene::from_launch(&owner, cx);
        old.draw(cx);
        let native = old.owner();
        let engine = cx.update(|cx| native.read(cx).state.clone());
        let old_id = native.entity_id();
        cx.update_window(old.window, |_, window, cx| {
            native.update(cx, |state, cx| {
                state.replace_text_in_range(Some(0..5), "原草稿🙂", window, cx)
            });
        })
        .unwrap();
        old.draw(cx);
        let (input, packages) = replacement(input, packages);
        let next = timer
            .block_on(store.commit_application(deployment, 1, input, vec![], packages))
            .unwrap();
        let update = timer
            .block_on(owner.load_committed_update(next.clone()).unwrap())
            .unwrap();
        cx.update(|cx| owner.adopt_update(update, vec![], cx).unwrap());
        cx.run_until_parked();
        assert!(owner.replacing());
        assert!(owner.current_launch().is_none());
        assert!(!old.resources.borrow().surface_closed);
        assert!(app.borrow().frame(&old.root).is_ok());
        assert_eq!(app.borrow().deployment_status().effective, Some(original));
        assert!(!cx.update(|cx| owner.resume_launch(cx).unwrap()));
        if quitting {
            assert!(!cx.update(actions::request_quit));
            assert!(owner.load_committed_update(next.clone()).is_err());
        }
        // Dropping both actual native Entity handles still requires GPUI release effects.
        cx.update(|_| {
            drop(engine);
            drop(native);
        });
        cx.run_until_parked();
        assert!(
            old.resources.borrow().surface_closed,
            "old native root still held after entity release; failure={:?}; {}",
            owner.failure(),
            old.resources.borrow().cleanup_diagnostics()
        );
        if quitting {
            assert!(owner.current_launch().is_none());
            assert!(!cx.update(|cx| owner.resume_launch(cx).unwrap()));
            assert!(cx.update(|cx| cx.windows().is_empty()));
            assert!(app.borrow().can_shutdown());
            drop(old);
            drop(owner);
            drop(app);
            timer
                .block_on(std::sync::Arc::try_unwrap(store).ok().unwrap().close())
                .unwrap();
            continue;
        }
        assert!(
            owner.current_launch().is_some(),
            "confirmed cleanup did not resume; deployment_error={:?}, failure={:?}, replacing={}",
            app.borrow().deployment_status().error,
            owner.failure(),
            owner.replacing()
        );
        let fresh = Scene::from_launch(&owner, cx);
        fresh.draw(cx);
        assert!(old.resources.borrow().surface_closed);
        assert!(app.borrow().frame(&old.root).is_err());
        assert_ne!(fresh.root, old.root);
        assert_ne!(fresh.window.window_id(), old.window.window_id());
        assert_ne!(fresh.owner().entity_id(), old_id);
        assert_eq!(app.borrow().deployment_status().effective, Some(next));
        assert!(
            matches!(app.borrow().frame(&fresh.root).unwrap().surface.as_ref(), Some(SurfaceDeclaration::Window { title, .. }) if title == "Replacement product")
        );
        let new_text = cx.update(|cx| fresh.owner().read(cx).state.read(cx).value().to_string());
        assert_eq!(new_text, "Start");
        assert!(owner.failure().is_none());
        fresh.close(cx);
        app.borrow_mut().shutdown().unwrap();
        drop(fresh);
        drop(old);
        drop(owner);
        drop(app);
        timer
            .block_on(std::sync::Arc::try_unwrap(store).ok().unwrap().close())
            .unwrap();
    }
}

#[gpui::test]
fn replacement_preserves_original_snapshot_work_and_only_waits_for_its_provider(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    for (replace_provider, quitting) in [(false, false), (true, false), (false, true), (true, true)]
    {
        let timer = timer();
        let (_dir, _ns, store) = stored_root(&timer);
        let deployment = uuid::Uuid::new_v4();
        let (mut input, mut packages) = support::resource_fixture();
        let index = packages
            .iter()
            .position(|p| p.manifest().id == "native.reader")
            .unwrap();
        let old_package = packages.remove(index);
        let mut declaration = old_package.manifest().application.clone().unwrap();
        declaration
            .entries
            .get_mut(&maka_plugin_core::package::EntryTarget::Desktop)
            .unwrap()
            .implementation = maka_plugin_core::package::Implementation::Native {
            implementation: "test.held-reader".into(),
        };
        let package = support::package("native.reader", declaration, None);
        input.instances.get_mut("reader").unwrap().package = package.application_ref().into();
        packages.push(package);
        let release = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
        let snapshot = std::sync::Arc::new(std::sync::Mutex::new(None));
        let service = std::sync::Arc::new(HeldRead {
            release: release.clone(),
            panic: false,
            snapshot: snapshot.clone(),
        });
        let mut native = widgets::registry();
        native
            .register("test.held-reader", move |_, _| {
                Ok(BTreeMap::from([(
                    "Reader".into(),
                    maka_ui::NativeExport {
                        contract: maka_ui::text::reader_contract(),
                        value: maka_ui::NativeCapability::Service(service.clone()),
                    },
                )]))
            })
            .unwrap();
        let original = timer
            .block_on(store.commit_application(
                deployment,
                0,
                input.clone(),
                vec![],
                packages.clone(),
            ))
            .unwrap();
        let mut application = timer
            .block_on(Application::start_from_store(
                store.clone(),
                original.clone(),
                PresentationTarget::Desktop,
                timer.handle().clone(),
                Default::default(),
                native,
                vec![],
            ))
            .unwrap();
        let root = application.mount_launch(serde_json::json!({})).unwrap();
        let app = Rc::new(RefCell::new(application));
        let owner = cx.update(|cx| {
            gpui_kit::init(cx);
            let owner = actions::own(app.clone(), None, None, cx).unwrap();
            owner.open_launch(root, serde_json::json!({}), cx).unwrap();
            owner
        });
        let old = Scene::from_launch(&owner, cx);
        old.draw(cx);
        cx.update_window(old.window, |_, window, cx| {
            let revision = app.borrow().frame(&old.root).unwrap().revision;
            app.borrow_mut()
                .dispatch(&old.root, revision, "read", serde_json::Value::Null)
                .unwrap();
            old.surface
                .update(cx, |surface, cx| surface.sync_native(window, cx).unwrap());
        })
        .unwrap();
        await_action(cx, &app, |_| snapshot.lock().unwrap().is_some());
        assert_eq!(app.borrow().accepted_action_count(), 1);
        let (mut input, packages) = replacement(input, packages);
        if replace_provider {
            input
                .instances
                .get_mut("reader")
                .unwrap()
                .configuration
                .values
                .insert("revision".into(), serde_json::json!(2));
        }
        let next = timer
            .block_on(store.commit_application(deployment, 1, input, vec![], packages))
            .unwrap();
        let update = timer
            .block_on(owner.load_committed_update(next.clone()).unwrap())
            .unwrap();
        cx.update(|cx| owner.adopt_update(update, vec![], cx).unwrap());
        cx.run_until_parked();
        assert!(old.resources.borrow().surface_closed);
        assert!(app.borrow().frame(&old.root).is_err());
        assert_eq!(app.borrow().accepted_action_count(), 1);
        assert!(
            snapshot
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .upgrade()
                .is_some()
        );
        assert_eq!(owner.current_launch().is_some(), !replace_provider);
        if replace_provider {
            assert!(!cx.update(|cx| owner.resume_launch(cx).unwrap()));
            assert_eq!(app.borrow().deployment_status().effective, Some(original));
        } else {
            assert_eq!(
                app.borrow().deployment_status().effective,
                Some(next.clone())
            );
        }
        if quitting {
            assert!(!cx.update(actions::request_quit));
            cx.run_until_parked();
            assert!(owner.current_launch().is_none());
            assert!(!cx.update(|cx| owner.resume_launch(cx).unwrap()));
            assert!(cx.update(|cx| cx.windows().is_empty()));
            assert_eq!(app.borrow().accepted_action_count(), 1);
        }
        release.add_permits(1);
        await_action(cx, &app, |a| a.failed_action_ids().len() == 1);
        let id = app.borrow().failed_action_ids()[0];
        assert_eq!(
            owner.current_launch().is_some(),
            !replace_provider && !quitting
        );
        assert!(
            app.borrow_mut()
                .complete_action_cleanup(id, Err("still unconfirmed".into()))
                .is_err()
        );
        assert_eq!(app.borrow().accepted_action_count(), 1);
        app.borrow_mut()
            .complete_action_cleanup(id, Ok(()))
            .unwrap();
        cx.update(|cx| actions::quit_when_drained(&app, cx));
        cx.run_until_parked();
        if quitting {
            assert!(owner.current_launch().is_none());
            assert!(!cx.update(|cx| owner.resume_launch(cx).unwrap()));
            assert!(app.borrow().can_shutdown());
            assert_eq!(app.borrow().accepted_action_count(), 0);
            assert!(
                snapshot
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .upgrade()
                    .is_none()
            );
            drop(old);
            drop(owner);
            drop(app);
            timer
                .block_on(std::sync::Arc::try_unwrap(store).ok().unwrap().close())
                .unwrap();
            continue;
        }
        let fresh = Scene::from_launch(&owner, cx);
        fresh.draw(cx);
        assert_ne!(fresh.root, old.root);
        assert!(Rc::ptr_eq(&fresh.app, &app));
        assert_eq!(app.borrow().accepted_action_count(), 0);
        assert!(
            snapshot
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .upgrade()
                .is_none()
        );
        assert_eq!(app.borrow().deployment_status().effective, Some(next));
        assert!(owner.failure().is_none());
        fresh.close(cx);
        app.borrow_mut().shutdown().unwrap();
        drop(fresh);
        drop(old);
        drop(owner);
        drop(app);
        timer
            .block_on(std::sync::Arc::try_unwrap(store).ok().unwrap().close())
            .unwrap();
    }
}

#[gpui::test]
fn recovery_root_waits_for_original_editor_release_and_keeps_independent_input(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let (mut input, mut packages) = support::editor_fixture();
    let index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old_package = packages.remove(index);
    let source = std::str::from_utf8(old_package.file("index.mjs").unwrap()).unwrap();
    let changed = source.replace("const next={...state,revision:state.revision+1};", "throw new Error('primary handler failed');const next={...state,revision:state.revision+1};");
    assert_ne!(source, changed);
    let package = support::package(
        "dynamic.app",
        old_package.manifest().application.clone().unwrap(),
        Some(changed),
    );
    input.instances.get_mut("app").unwrap().package = package.application_ref().into();
    packages.push(package);
    support::add_recovery(&mut input, &mut packages);
    let mut app = Application::new(
        &input,
        packages,
        timer.handle().clone(),
        Default::default(),
        widgets::registry(),
        Default::default(),
    )
    .unwrap();
    let root = app.mount_bootstrap(serde_json::json!({})).unwrap();
    let app = Rc::new(RefCell::new(app));
    let owner = cx.update(|cx| {
        gpui_kit::init(cx);
        let owner = actions::own(app.clone(), None, None, cx).unwrap();
        owner.open_launch(root, serde_json::json!({}), cx).unwrap();
        owner
    });
    let old = Scene::from_launch(&owner, cx);
    old.draw(cx);
    let native = old.owner();
    let engine = cx.update(|cx| native.read(cx).state.clone());
    cx.update_window(old.window, |_, window, cx| {
        let revision = app.borrow().frame(&old.root).unwrap().revision;
        assert!(
            app.borrow_mut()
                .dispatch(&old.root, revision, "parent", serde_json::json!({}))
                .is_err()
        );
        old.surface
            .update(cx, |s, cx| s.sync_native(window, cx))
            .unwrap();
    })
    .unwrap();
    cx.run_until_parked();
    assert!(owner.replacing());
    assert!(owner.current_launch().is_none());
    assert!(!old.resources.borrow().surface_closed);
    assert!(!cx.update(|cx| owner.resume_launch(cx).unwrap()));
    cx.update(|_| {
        drop(engine);
        drop(native);
    });
    cx.run_until_parked();
    assert!(old.resources.borrow().surface_closed);
    assert!(app.borrow().frame(&old.root).is_err());
    let (fresh, window, surface) = owner.current_launch().expect("ordinary recovery root");
    assert_ne!(fresh, old.root);
    assert_ne!(window.window_id(), old.window.window_id());
    assert_eq!(
        app.borrow().root_kind(&fresh).unwrap(),
        maka_plugin_core::composition::RootKind::Recovery
    );
    cx.update_window(window, |_, window, cx| {
        let revision = app.borrow().frame(&fresh).unwrap().revision;
        app.borrow_mut()
            .dispatch(&fresh, revision, "inc", serde_json::json!({}))
            .unwrap();
        surface
            .update(cx, |s, cx| s.sync_native(window, cx))
            .unwrap()
            .unwrap();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert_eq!(app.borrow().frame(&fresh).unwrap().state["count"], 1);
    cx.update_window(window, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
    assert!(app.borrow().can_shutdown());
    app.borrow_mut().shutdown().unwrap();
}
