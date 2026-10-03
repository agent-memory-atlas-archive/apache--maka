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
use gpui_kit::{AnyWindowHandle, Entity, EntityInputHandler};
use maka_ui::editor::{
    EditorCommand, EditorConflict, EditorEdit, EditorObservation, EditorOutcome, EditorReadError,
};
use std::collections::BTreeMap;
use widgets::editor::EditorOwner;

struct Scene {
    window: AnyWindowHandle,
    surface: Entity<Surface>,
    app: Rc<RefCell<DesktopApplication>>,
    resources: Rc<RefCell<resources::NativeResources>>,
    root: Mount,
    editor: Mount,
}
impl Scene {
    fn new(cx: &mut TestAppContext, timer: &tokio::runtime::Runtime, fail_sibling: bool) -> Self {
        Self::from_packages(cx, timer, fail_sibling, support::editor_fixture())
    }
    fn from_packages(
        cx: &mut TestAppContext,
        timer: &tokio::runtime::Runtime,
        fail_sibling: bool,
        fixture: (
            maka_plugin_core::composition::Input,
            Vec<maka_plugins::package::Package>,
        ),
    ) -> Self {
        Self::with_registry(
            cx,
            timer,
            fail_sibling,
            fixture,
            widgets::registry(),
            Default::default(),
        )
    }
    fn with_registry(
        cx: &mut TestAppContext,
        timer: &tokio::runtime::Runtime,
        fail_sibling: bool,
        (mut input, mut packages): (
            maka_plugin_core::composition::Input,
            Vec<maka_plugins::package::Package>,
        ),
        native: maka_ui::NativeRegistry<dyn widgets::Widget>,
        remotes: maka_ui::actions::RemoteServices,
    ) -> Self {
        if fail_sibling {
            let index = packages
                .iter()
                .position(|p| p.manifest().id == "dynamic.counter")
                .unwrap();
            let old = packages.remove(index);
            let source=std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap().replace(support::CHILD_UPDATE,"if(props.label==='Right 1')throw new Error('failed sibling');return frame({...state,label:props.label,revision:state.revision+1});");
            let counter = support::package(
                "dynamic.counter",
                old.manifest().application.clone().unwrap(),
                Some(source),
            );
            input.instances.get_mut("counter").unwrap().package = counter.application_ref().into();
            packages.push(counter);
        }
        let application = Application::new(
            &input,
            packages,
            timer.handle().clone(),
            Default::default(),
            native,
            remotes,
        )
        .unwrap();
        Self::from_application(cx, application)
    }
    fn from_application(cx: &mut TestAppContext, mut application: DesktopApplication) -> Self {
        let root = application.mount_launch(serde_json::json!({})).unwrap();
        let outer = application.child_mount(&root, "editor").unwrap().clone();
        let editor = application
            .child_mount(&outer, "draft")
            .cloned()
            .unwrap_or(outer);
        let app = Rc::new(RefCell::new(application));
        let (wake, receipts) = tokio::sync::mpsc::unbounded_channel();
        let resources = Rc::new(RefCell::new(resources::NativeResources::new(
            root.clone(),
            wake,
        )));
        let (window, surface) = cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.), px(0.)),
                        size(px(640.), px(400.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    window_owner::start(
                        window,
                        cx,
                        resources.clone(),
                        app.clone(),
                        receipts,
                        |outcome, _| outcome.unwrap(),
                    )
                    .unwrap()
                    .detach();
                    let surface = cx.new(|_| Surface {
                        application: app.clone(),
                        resources: resources.clone(),
                        mount: root.clone(),
                    });
                    surface.update(cx, |surface, cx| surface.sync_native(window, cx).unwrap());
                    surface
                },
            )
            .unwrap()
        });
        Self {
            window,
            surface,
            app,
            resources,
            root,
            editor,
        }
    }
    fn owner(&self) -> Entity<EditorOwner> {
        self.resources.borrow().owners[&self.editor.id().to_string()]
            .state
            .downcast_ref::<Entity<EditorOwner>>()
            .unwrap()
            .clone()
    }
    fn draw(&self, cx: &mut TestAppContext) {
        cx.update_window(self.window, |_, window, cx| {
            self.surface
                .update(cx, |s, cx| s.sync_native(window, cx).unwrap());
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    fn command(
        &self,
        command: EditorCommand,
        window: &mut gpui_kit::Window,
        cx: &mut App,
    ) -> maka_ui::Result<()> {
        let revision = self.app.borrow().frame(&self.root).unwrap().revision;
        self.app.borrow_mut().dispatch(
            &self.root,
            revision,
            "parent",
            serde_json::json!({"command":command}),
        )?;
        self.surface.update(cx, |s, cx| s.sync_native(window, cx))
    }
    fn close(&self, cx: &mut TestAppContext) {
        cx.update_window(self.window, |_, window, _| window.remove_window())
            .unwrap();
        cx.run_until_parked();
    }
}
fn timer() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_time()
        .build()
        .unwrap()
}

#[gpui::test]
fn editor_native_revision_ime_barrier_and_command_receipts_are_independent_of_paint(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = Scene::new(cx, &timer, false);
    scene.draw(cx);
    let owner = scene.owner();
    let observation = || {
        serde_json::from_value::<EditorObservation>(
            scene.app.borrow().frame(&scene.editor).unwrap().state["observation"].clone(),
        )
        .unwrap()
    };
    cx.update_window(scene.window, |_, window, cx| {
        owner.update(cx, |s, cx| {
            s.replace_text_in_range(Some(0..5), "Draft", window, cx)
        })
    })
    .unwrap();
    cx.run_until_parked();
    let resource = observation().snapshot.text;
    cx.update_window(scene.window, |_, window, cx| {
        assert_eq!(
            owner
                .update(cx, |s, cx| s.read_text(&resource, 0..5, window, cx))
                .unwrap(),
            "Draft"
        )
    })
    .unwrap();
    let mut published = Vec::new();
    cx.update_window(scene.window, |_, window, cx| {
        let native = owner.read(cx).revision;
        owner
            .read(cx)
            .state
            .clone()
            .update(cx, |s, cx| s.set_selected_range(0..2, cx));
        scene
            .command(
                EditorCommand {
                    sequence: 1,
                    based_on: native,
                    edit: EditorEdit::Replace {
                        text: "Wrong".into(),
                    },
                },
                window,
                cx,
            )
            .unwrap();
        assert_eq!(
            owner.read(cx).receipt.as_ref().unwrap().outcome,
            EditorOutcome::Conflict {
                reason: EditorConflict::Revision
            }
        );
        assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "Draft");
        assert_eq!(owner.read(cx).revision, native + 1);
        // No second command is admitted before this result is published.
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        assert!(
            scene
                .command(
                    EditorCommand {
                        sequence: 2,
                        based_on: native + 1,
                        edit: EditorEdit::Reset {
                            text: "Early".into()
                        }
                    },
                    window,
                    cx
                )
                .is_err()
        );
        assert_eq!(
            scene.app.borrow().frame(&scene.root).unwrap().revision,
            revision
        );
        assert_eq!(owner.read(cx).receipt.as_ref().unwrap().sequence, 1);
    })
    .unwrap();
    cx.run_until_parked();
    published.push(observation().receipt.unwrap().sequence);
    cx.update_window(scene.window, |_, window, cx| {
        // Two native IME calls in one effect cycle advance causality before any
        // paint or observation publication occurs.
        let before = owner.read(cx).revision;
        owner.update(cx, |s, cx| {
            s.replace_and_mark_text_in_range(Some(0..2), "n", Some(1..1), window, cx);
            s.replace_and_mark_text_in_range(None, "ni", Some(2..2), window, cx);
        });
        assert_eq!(owner.read(cx).revision, before + 2);
        let based_on = owner.read(cx).revision;
        scene
            .command(
                EditorCommand {
                    sequence: 2,
                    based_on,
                    edit: EditorEdit::Reset {
                        text: "Clear".into(),
                    },
                },
                window,
                cx,
            )
            .unwrap();
        assert_eq!(
            owner.read(cx).receipt.as_ref().unwrap().outcome,
            EditorOutcome::Conflict {
                reason: EditorConflict::Composition
            }
        );
        assert!(
            owner
                .update(cx, |s, cx| s.marked_text_range(window, cx))
                .is_some()
        );
    })
    .unwrap();
    cx.run_until_parked();
    published.push(observation().receipt.unwrap().sequence);
    cx.update_window(scene.window, |_, window, cx| {
        let revision = owner.read(cx).revision;
        owner.update(cx, |s, cx| s.unmark_text(window, cx));
        assert_eq!(owner.read(cx).revision, revision + 1);
        scene
            .command(
                EditorCommand {
                    sequence: 3,
                    based_on: revision,
                    edit: EditorEdit::Reset {
                        text: "Late".into(),
                    },
                },
                window,
                cx,
            )
            .unwrap();
        assert_eq!(
            owner.read(cx).receipt.as_ref().unwrap().outcome,
            EditorOutcome::Conflict {
                reason: EditorConflict::Revision
            }
        );
    })
    .unwrap();
    cx.run_until_parked();
    published.push(observation().receipt.unwrap().sequence);
    cx.update_window(scene.window, |_, window, cx| {
        let based_on = owner.read(cx).revision;
        let command = EditorCommand {
            sequence: 4,
            based_on,
            edit: EditorEdit::Reset {
                text: "Reset".into(),
            },
        };
        scene.command(command.clone(), window, cx).unwrap();
        scene.command(command, window, cx).unwrap();
        assert_eq!(owner.read(cx).revision, based_on + 1);
        assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "Reset");
        assert_eq!(
            owner.read(cx).receipt.as_ref().unwrap().outcome,
            EditorOutcome::Applied
        );
    })
    .unwrap();
    cx.run_until_parked();
    published.push(observation().receipt.unwrap().sequence);
    assert_eq!(
        published,
        vec![1, 2, 3, 4],
        "every admitted command publishes its settlement"
    );
    assert!(observation().snapshot.marked.is_none());
    assert_eq!(
        scene.app.borrow().frame(&scene.editor).unwrap().state["pending"],
        false
    );
    scene.draw(cx);
    scene.close(cx);
}

#[gpui::test]
fn editor_large_text_observations_use_revision_checked_bounded_resources(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = Scene::new(cx, &timer, false);
    scene.draw(cx);
    let owner = scene.owner();
    let text = format!("中🌟{}", "x".repeat(1024 * 1024));
    cx.update_window(scene.window, |_, window, cx| {
        owner.update(cx, |s, cx| {
            s.replace_text_in_range(Some(0..5), &text, window, cx)
        })
    })
    .unwrap();
    cx.run_until_parked();
    let frame = scene
        .app
        .borrow()
        .frame(&scene.editor)
        .unwrap()
        .state
        .clone();
    assert!(
        serde_json::to_vec(&frame).unwrap().len() < 4096,
        "observation never transports the whole document"
    );
    let observation: EditorObservation =
        serde_json::from_value(frame["observation"].clone()).unwrap();
    assert_eq!(observation.snapshot.text.bytes, text.len());
    let reference = observation.snapshot.text;
    cx.update_window(scene.window, |_, window, cx| {
        owner.update(cx, |s, cx| {
            assert_eq!(s.read_text(&reference, 0..7, window, cx).unwrap(), "中🌟");
            assert!(matches!(
                s.read_text(&reference, 1..2, window, cx),
                Err(EditorReadError::Range)
            ));
            assert!(matches!(
                s.read_text(&reference, 0..reference.bytes, window, cx),
                Err(EditorReadError::Range)
            ));
            let mut foreign = reference.clone();
            foreign.owner = "another owner".into();
            assert!(matches!(
                s.read_text(&foreign, 0..7, window, cx),
                Err(EditorReadError::Owner)
            ));
            let mut adjusted = None;
            assert_eq!(
                s.text_for_range(0..3, &mut adjusted, window, cx).unwrap(),
                "中🌟"
            );
            assert_eq!(adjusted, Some(0..3));
            s.replace_text_in_range(Some(3..3), "!", window, cx);
            assert!(matches!(
                s.read_text(&reference, 0..7, window, cx),
                Err(EditorReadError::Revision)
            ));
        });
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(
        scene.app.borrow().frame(&scene.editor).unwrap().state["observation"]["snapshot"]["text"]["bytes"],
        text.len() + 1
    );
    scene.close(cx);
}

#[gpui::test]
fn failed_composed_batch_cannot_execute_staged_editor_reset(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = Scene::new(cx, &timer, true);
    scene.draw(cx);
    let owner = scene.owner();
    cx.update_window(scene.window, |_, window, cx| {
        owner.update(cx, |s, cx| {
            s.replace_text_in_range(Some(0..5), "Draft", window, cx)
        });
        let based_on = owner.read(cx).revision;
        assert!(
            scene
                .command(
                    EditorCommand {
                        sequence: 1,
                        based_on,
                        edit: EditorEdit::Reset {
                            text: "Must not escape".into()
                        }
                    },
                    window,
                    cx
                )
                .is_err()
        );
        assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "Draft");
        assert_eq!(owner.read(cx).revision, based_on);
        assert!(owner.read(cx).receipt.is_none());
        assert_eq!(scene.app.borrow().frame(&scene.root).unwrap().revision, 0);
    })
    .unwrap();
    scene.close(cx);
}

#[gpui::test]
fn detached_editor_owner_drops_before_real_kit_entity_cleanup_ack(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = Scene::new(cx, &timer, false);
    scene.draw(cx);
    let engine = cx.update(|cx| scene.owner().read(cx).state.clone());
    scene
        .app
        .borrow_mut()
        .dispatch(&scene.root, 0, "toggle", serde_json::Value::Null)
        .unwrap();
    scene
        .resources
        .borrow_mut()
        .collect(&mut scene.app.borrow_mut())
        .unwrap();
    assert!(
        !scene
            .resources
            .borrow()
            .owners
            .contains_key(&scene.editor.id().to_string())
    );
    scene.draw(cx);
    assert!(scene.app.borrow().is_retiring(&scene.editor));
    assert!(
        scene.app.borrow().frame(&scene.editor).is_ok(),
        "real Kit engine remains held independently of recipe/owner"
    );
    cx.update(|_| drop(engine));
    cx.run_until_parked();
    assert!(scene.app.borrow().frame(&scene.editor).is_err());
    scene.close(cx);
}

#[gpui::test]
fn public_editor_events_preserve_a_same_batch_command_until_receipt_and_do_not_leak_failed_intents(
    cx: &mut TestAppContext,
) {
    use maka_plugin_core::{contract::Interface, package::EntryTarget};
    let timer = timer();
    for fail_clear in [false, true] {
        let (mut input, mut packages) = support::editor_events_fixture();
        let index = packages
            .iter()
            .position(|p| p.manifest().id == "dynamic.counter")
            .unwrap();
        let old = packages.remove(index);
        let mut declaration = old.manifest().application.clone().unwrap();
        let contract = &mut declaration
            .entries
            .get_mut(&EntryTarget::Desktop)
            .unwrap()
            .exports
            .get_mut("Counter")
            .unwrap()
            .contract;
        let old_contract = serde_json::to_string(contract).unwrap();
        let Interface::Component { events, .. } = &mut contract.interface else {
            unreachable!()
        };
        events.insert(
            "set".into(),
            serde_json::to_value(schemars::schema_for!(EditorCommand)).unwrap(),
        );
        events.insert("clear".into(), serde_json::json!({"type":"null"}));
        let contract = contract.clone();
        let source=std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap()
            .replace(&old_contract,&serde_json::to_string(&contract).unwrap())
            .replace("event(state,event){return frame({...state,count:state.count+1,revision:state.revision+1});}","event(state,event){const output=frame({...state,count:state.count+1,revision:state.revision+1});output.events=[{name:'set',value:event.event.command},{name:'clear',value:null}];return output;}");
        let counter = support::package("dynamic.counter", declaration, Some(source));
        input.instances.get_mut("counter").unwrap().package = counter.application_ref().into();
        packages.push(counter);
        let index = packages
            .iter()
            .position(|p| p.manifest().id == "dynamic.app")
            .unwrap();
        let old = packages.remove(index);
        let mut declaration = old.manifest().application.clone().unwrap();
        declaration
            .entries
            .get_mut(&EntryTarget::Desktop)
            .unwrap()
            .imports
            .get_mut("counter")
            .unwrap()
            .contract = contract;
        let receive = if fail_clear {
            "if(event.kind==='component'&&event.name==='clear')throw new Error('reject after staged command');"
        } else {
            "if(event.kind==='component'&&event.name==='clear')return frame({...state,command:null,revision:state.revision+1});"
        };
        let source=std::str::from_utf8(old.file("index.mjs").unwrap()).unwrap()
            .replace("binding:'counter',member:0,on:{},props:","binding:'counter',member:0,on:{set:'command_set',clear:'command_clear'},props:")
            .replace(support::EDITOR_RECEIVE,&format!("{receive}if(event.kind==='component'&&event.name==='set')return frame({{...state,command:event.event,revision:state.revision+1}});{}",support::EDITOR_RECEIVE));
        let root = support::package("dynamic.app", declaration, Some(source));
        input.instances.get_mut("app").unwrap().package = root.application_ref().into();
        packages.push(root);
        let scene = Scene::from_packages(cx, &timer, false, (input, packages));
        scene.draw(cx);
        let owner = scene.owner();
        let left = scene
            .app
            .borrow()
            .child_mount(&scene.root, "left")
            .unwrap()
            .clone();
        let based_on = cx.update(|cx| owner.read(cx).revision);
        let command = EditorCommand {
            sequence: 1,
            based_on,
            edit: EditorEdit::Replace {
                text: "Accepted".into(),
            },
        };
        let result = scene.app.borrow_mut().dispatch(
            &left,
            0,
            "inc",
            serde_json::json!({"command":command}),
        );
        cx.update_window(scene.window, |_, window, cx| {
            assert_eq!(
                owner.read(cx).state.read(cx).value().as_ref(),
                "Start",
                "pre-commit preparation does not execute native commands"
            );
            assert!(owner.read(cx).receipt.is_none());
            if fail_clear {
                assert!(result.is_err());
                assert_eq!(
                    scene.app.borrow().frame(&scene.editor).unwrap().state["pending"],
                    false
                );
                assert_eq!(scene.app.borrow().frame(&left).unwrap().revision, 0);
            } else {
                result.unwrap();
                assert!(scene.app.borrow().frame(&scene.root).unwrap().state["command"].is_null());
                assert_eq!(
                    scene.app.borrow().frame(&scene.editor).unwrap().state["pending"],
                    true
                );
                scene
                    .surface
                    .update(cx, |s, cx| s.sync_native(window, cx).unwrap());
                assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "Accepted");
            }
        })
        .unwrap();
        cx.run_until_parked();
        if !fail_clear {
            let receipt =
                scene.app.borrow().frame(&scene.root).unwrap().state["observation"]["receipt"]
                    .clone();
            assert_eq!(receipt["sequence"], 1);
            assert_eq!(
                scene.app.borrow().frame(&scene.editor).unwrap().state["pending"],
                false
            );
            cx.update_window(scene.window, |_, window, cx| {
                let revision = owner.read(cx).revision;
                scene
                    .command(
                        EditorCommand {
                            sequence: 2,
                            based_on: revision,
                            edit: EditorEdit::Replace {
                                text: "Next".into(),
                            },
                        },
                        window,
                        cx,
                    )
                    .unwrap();
                assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "Next");
            })
            .unwrap();
            cx.run_until_parked();
            assert_eq!(
                scene.app.borrow().frame(&scene.root).unwrap().state["observation"]["receipt"]["sequence"],
                2
            );
        }
        scene.close(cx);
    }
}

fn await_action(
    cx: &mut TestAppContext,
    app: &Rc<RefCell<DesktopApplication>>,
    ready: impl Fn(&DesktopApplication) -> bool,
) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !ready(&app.borrow()) {
        cx.run_until_parked();
        assert!(
            std::time::Instant::now() < deadline,
            "cross-executor action did not complete"
        );
        std::thread::yield_now();
    }
    cx.run_until_parked();
}
#[gpui::test]
fn real_composer_snapshot_precedes_reset_and_capture_conflict_does_not_fake_scene_rollback(
    cx: &mut TestAppContext,
) {
    // The production executor uses Tokio workers, whose completions wake GPUI from another thread.
    cx.executor().allow_parking();
    for conflict in [false, true] {
        let timer = timer();
        let scene = Scene::from_packages(cx, &timer, false, support::resource_fixture());
        scene.draw(cx);
        let owner = scene.owner();
        cx.update_window(scene.window, |_, window, cx| {
            owner.update(cx, |o, cx| {
                o.replace_text_in_range(Some(0..5), "Draft", window, cx)
            })
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(scene.window,|_,window,cx| {
            let native=owner.read(cx).revision;let revision=scene.app.borrow().frame(&scene.root).unwrap().revision;
            scene.app.borrow_mut().dispatch(&scene.root,revision,"read",serde_json::json!({"capture":if conflict {serde_json::json!({"kind":"at_revision","revision":native+100})} else {serde_json::json!({"kind":"at_revision","revision":native})},"command":{"sequence":1,"basedOn":native,"edit":{"kind":"reset","text":"Reset"}}})).unwrap();
            assert_eq!(owner.read(cx).state.read(cx).value().as_ref(),"Draft");
            scene.surface.update(cx,|s,cx|s.sync_native(window,cx).unwrap());
            assert_eq!(owner.read(cx).state.read(cx).value().as_ref(),"Reset");
        }).unwrap();
        await_action(cx, &scene.app, |a| {
            a.frame(&scene.root).unwrap().state["outcome"].is_object()
        });
        let value = scene.app.borrow().frame(&scene.root).unwrap().state["outcome"].clone();
        if conflict {
            assert_eq!(value["status"], "not_accepted");
        } else {
            assert_eq!(value["status"], "succeeded");
            assert_eq!(value["value"]["text"], "Draft");
        }
        assert_eq!(scene.app.borrow().accepted_action_count(), 0);
        drop(owner);
        scene.close(cx);
    }
}
struct HeldRead {
    release: std::sync::Arc<tokio::sync::Semaphore>,
    panic: bool,
    snapshot: std::sync::Arc<
        std::sync::Mutex<Option<std::sync::Weak<dyn maka_plugins::call::ReadResource>>>,
    >,
}
impl maka_ui::actions::NativeService for HeldRead {
    fn call(&self, request: maka_ui::actions::ServiceRequest) -> maka_ui::actions::ServiceFuture {
        let resources = request.resources;
        *self.snapshot.lock().unwrap() = Some(std::sync::Arc::downgrade(&resources["text"]));
        let release = self.release.clone();
        let panic = self.panic;
        Box::pin(async move {
            let _permit = release.acquire_owned().await.unwrap();
            assert_eq!(resources["text"].read(0..5).unwrap(), b"Start");
            assert!(
                !panic,
                "controlled worker panic after actual window cleanup"
            );
            maka_ui::actions::ServiceCompletion {
                result: Err("read completed but cleanup confirmation unavailable".into()),
                cleanup: Err("controlled cleanup failure".into()),
            }
        })
    }
}
#[gpui::test]
fn application_owner_preserves_failed_and_panicked_work_after_real_window_cleanup_and_task_exit(
    cx: &mut TestAppContext,
) {
    // Exercise the real external worker rather than GPUI's deterministic test dispatcher.
    cx.executor().allow_parking();
    for panic in [false, true] {
        let timer = timer();
        let (mut input, mut packages) = support::resource_fixture();
        let index = packages
            .iter()
            .position(|p| p.manifest().id == "native.reader")
            .unwrap();
        let old = packages.remove(index);
        let mut declaration = old.manifest().application.clone().unwrap();
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
            panic,
            snapshot: snapshot.clone(),
        });
        let mut native = widgets::registry();
        native
            .register("test.held-reader", move |_, _values| {
                Ok(BTreeMap::from([(
                    "Reader".into(),
                    maka_ui::NativeExport {
                        contract: maka_ui::text::reader_contract(),
                        value: maka_ui::NativeCapability::Service(service.clone()),
                    },
                )]))
            })
            .unwrap();
        let scene = Scene::with_registry(
            cx,
            &timer,
            false,
            (input, packages),
            native,
            Default::default(),
        );
        cx.update(|cx| actions::own(scene.app.clone(), None, None, cx).unwrap());
        scene.draw(cx);
        cx.update_window(scene.window, |_, window, cx| {
            let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
            scene
                .app
                .borrow_mut()
                .dispatch(&scene.root, revision, "read", serde_json::Value::Null)
                .unwrap();
            scene
                .surface
                .update(cx, |s, cx| s.sync_native(window, cx).unwrap());
        })
        .unwrap();
        assert_eq!(scene.app.borrow().accepted_action_count(), 1);
        scene.close(cx);
        assert!(
            scene.resources.borrow().surface_closed,
            "real window, entity and retained-frame cleanup must complete while accepted snapshot remains"
        );
        assert!(scene.app.borrow().retiring_mounts().is_empty());
        assert!(!scene.app.borrow().can_shutdown());
        release.add_permits(1);
        await_action(cx, &scene.app, |a| a.failed_action_ids().len() == 1);
        let id = scene.app.borrow().failed_action_ids()[0];
        let weak = Rc::downgrade(&scene.app);
        drop(scene);
        cx.run_until_parked();
        let app = weak
            .upgrade()
            .expect("application owner outlives all windows/executor tasks");
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
        assert!(
            app.borrow_mut()
                .complete_action_cleanup(id, Err("still unknown".into()))
                .is_err()
        );
        assert_eq!(app.borrow().accepted_action_count(), 1);
        app.borrow_mut()
            .complete_action_cleanup(id, Ok(()))
            .unwrap();
        assert!(
            snapshot
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .upgrade()
                .is_none()
        );
        cx.update(|cx| actions::quit_when_drained(&app, cx));
        drop(app);
        cx.run_until_parked();
        assert!(weak.upgrade().is_none());
    }
}

struct HostRead {
    fail_cleanup: bool,
    started: std::sync::Arc<tokio::sync::Semaphore>,
    release: std::sync::Arc<tokio::sync::Semaphore>,
    snapshot: std::sync::Arc<
        std::sync::Mutex<Option<std::sync::Weak<dyn maka_plugins::call::ReadResource>>>,
    >,
}
impl maka_runtime_host::application::Handler for HostRead {
    fn call(
        &self,
        _: String,
        input: serde_json::Value,
        context: maka_runtime_host::application::Context,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static>,
    > {
        if self.fail_cleanup {
            let mut ticket = context.scope().resources.reserve().unwrap();
            ticket.start();
            ticket.complete(Err("Host snapshot cleanup unconfirmed".into()));
        }
        let attachments = context.attachments().clone();
        *self.snapshot.lock().unwrap() = Some(std::sync::Arc::downgrade(&attachments["text"]));
        let started = self.started.clone();
        let release = self.release.clone();
        Box::pin(async move {
            started.add_permits(1);
            release.acquire_owned().await.unwrap().forget();
            let range: maka_ui::text::ReadText = serde_json::from_value(input).unwrap();
            let text = String::from_utf8(attachments["text"].read(range.start..range.end)?)
                .map_err(|e| e.to_string())?;
            assert_eq!(text, "Start");
            // The real Rope snapshot still enforces the native text contract.
            assert!(attachments["text"].read(0..64 * 1024 + 1).is_err());
            serde_json::to_value(maka_ui::text::TextChunk {
                text,
                metadata: attachments["text"].metadata(),
            })
            .map_err(|e| e.to_string())
        })
    }
}
struct UiReadGrant {
    source: maka_ui::actions::ServiceSource,
    approved: bool,
}
struct HostGrant(maka_plugins::call::Issuer);
impl maka_runtime_host::application::Policy for HostGrant {
    fn admit(
        &self,
        invocation: maka_runtime_host::application::Invocation<'_>,
    ) -> Result<(), String> {
        if self
            .0
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
            Err("native read grant denied".into())
        }
    }
}
struct HostClose;
impl maka_runtime_host::application::NativeOwner for HostClose {
    fn close(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'static>>
    {
        Box::pin(async { Ok(()) })
    }
}
struct UiReadScopes {
    issuer: maka_plugins::call::Issuer,
    approved: std::sync::Arc<std::sync::atomic::AtomicBool>,
    observed: std::sync::Arc<std::sync::Mutex<Option<maka_ui::actions::ServiceSource>>>,
}
impl maka_application_bridge::Scopes for UiReadScopes {
    fn scope(
        &self,
        source: &maka_ui::actions::ServiceSource,
    ) -> Result<maka_plugins::call::Scope, String> {
        *self.observed.lock().unwrap() = Some(source.clone());
        self.issuer
            .issue_with(
                maka_plugins::call::Identity::Remote {
                    request_id: source.request_id(),
                },
                UiReadGrant {
                    source: source.clone(),
                    approved: self.approved.load(std::sync::atomic::Ordering::SeqCst),
                },
                tokio_util::sync::CancellationToken::new(),
            )
            .map_err(|e| e.to_string())
    }
}
async fn host_service_fixture(
    query: std::sync::Arc<dyn maka_runtime_host::application::Handler>,
    issuer: maka_plugins::call::Issuer,
    store: std::sync::Arc<maka_event_log::EventLog>,
    contract: maka_plugin_core::contract::Contract,
) -> maka_runtime_host::application::Application {
    use maka_plugin_core::{
        composition::{
            Configuration, Environment, ExecutionLocation, Input, Node, PackageUse, Target,
        },
        contract::{Availability, Cardinality, Export, Import},
        package::{Declaration, EntryDeclaration, EntryTarget, Implementation},
        runtime::Published,
    };
    use maka_runtime_host::application::{Application, NativeActivation, NativeRegistry};
    let provider = support::package(
        "host.reader",
        Declaration {
            entries: BTreeMap::from([(
                EntryTarget::Host,
                EntryDeclaration {
                    implementation: Implementation::Native {
                        implementation: "test.host-reader".into(),
                    },
                    configuration: serde_json::json!({"type":"object"}),
                    imports: BTreeMap::new(),
                    exports: BTreeMap::from([(
                        "Reader".into(),
                        Export {
                            contract: contract.clone(),
                            availability: Availability::Required,
                        },
                    )]),
                },
            )]),
            composition: None,
        },
        None,
    );
    let consumer = support::package(
        "host.consumer",
        Declaration {
            entries: BTreeMap::from([(
                EntryTarget::Host,
                EntryDeclaration {
                    implementation: Implementation::Native {
                        implementation: "test.host-consumer".into(),
                    },
                    configuration: serde_json::json!({"type":"object"}),
                    imports: BTreeMap::from([(
                        "reader".into(),
                        Import {
                            contract: contract.clone(),
                            cardinality: Cardinality::One,
                        },
                    )]),
                    exports: BTreeMap::new(),
                },
            )]),
            composition: None,
        },
        None,
    );
    let nodes = [
        ("reader", 11, &provider, true),
        ("consumer", 12, &consumer, false),
    ]
    .into_iter()
    .map(|(name, id, p, provider)| {
        (
            name.into(),
            Node {
                key: uuid::Uuid::from_u128(id),
                package: p.application_ref().into(),
                use_package: PackageUse::Entry,
                configuration: Configuration::default(),
                parameters: Configuration::default(),
                bindings: if provider {
                    BTreeMap::new()
                } else {
                    BTreeMap::from([(
                        "reader".into(),
                        vec![Target::Instance {
                            path: vec!["reader".into()],
                            export: "Reader".into(),
                        }],
                    )])
                },
                disabled: false,
                overrides: Vec::new(),
            },
        )
    })
    .collect();
    let input = Input {
        environment: Environment {
            location: ExecutionLocation::Host,
            presentation: None,
        },
        instances: nodes,
        launch: None,
    };
    let mut native = NativeRegistry::default();
    native
        .insert(
            "test.host-reader".into(),
            std::sync::Arc::new(move |_| {
                let query = query.clone();
                let contract = contract.clone();
                Box::pin(async move {
                    NativeActivation {
                        exports: Ok(BTreeMap::from([(
                            "Reader".into(),
                            Published {
                                contract,
                                value: query,
                            },
                        )])),
                        owner: std::sync::Arc::new(HostClose),
                    }
                })
            }),
        )
        .unwrap();
    native
        .insert(
            "test.host-consumer".into(),
            std::sync::Arc::new(|_| {
                Box::pin(async {
                    NativeActivation {
                        exports: Ok(BTreeMap::new()),
                        owner: std::sync::Arc::new(HostClose),
                    }
                })
            }),
        )
        .unwrap();
    let deployment = uuid::Uuid::new_v4();
    store
        .commit_application(deployment, 0, input, vec![], vec![provider, consumer])
        .await
        .unwrap();
    Application::start_from_store(
        store,
        deployment,
        native,
        issuer.clone(),
        std::sync::Arc::new(HostGrant(issuer)),
        Default::default(),
    )
    .await
    .unwrap()
}
fn stored_root(
    timer: &tokio::runtime::Runtime,
) -> (
    tempfile::TempDir,
    maka_event_log::root::RootNamespaces,
    std::sync::Arc<maka_event_log::EventLog>,
) {
    let dir = tempfile::tempdir().unwrap();
    let ns = maka_event_log::root::RootNamespaces {
        ownership: dir.path().join("owners"),
        control: dir.path().join("control"),
    };
    let owner = std::sync::Arc::new(
        maka_event_log::root::RootOwner::create(&dir.path().join("root"), &ns).unwrap(),
    );
    let store = std::sync::Arc::new(
        timer
            .block_on(maka_event_log::EventLog::for_root(owner))
            .unwrap(),
    );
    (dir, ns, store)
}
#[gpui::test]
fn host_query_keeps_real_kit_snapshot_after_edit_remove_and_window_cleanup(
    cx: &mut TestAppContext,
) {
    host_query_scene(cx, false);
}
#[gpui::test]
fn host_query_cleanup_failure_keeps_remote_work_and_snapshot_after_window_exit(
    cx: &mut TestAppContext,
) {
    host_query_scene(cx, true);
}
fn host_query_scene(cx: &mut TestAppContext, fail_cleanup: bool) {
    cx.executor().allow_parking();
    let timer = timer();
    let started = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
    let release = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
    let snapshot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let issuer = maka_plugins::call::Issuer::default();
    let (host_dir, host_ns, host_store) = stored_root(&timer);
    let host_root = host_dir.path().join("root");
    let host = timer.block_on(host_service_fixture(
        std::sync::Arc::new(HostRead {
            fail_cleanup,
            started: started.clone(),
            release: release.clone(),
            snapshot: snapshot.clone(),
        }),
        issuer.clone(),
        host_store,
        {
            let mut contract = maka_ui::text::reader_contract();
            contract.transport = maka_plugin_core::contract::Transport::Remote;
            contract
        },
    ));
    timer.block_on(host.ready()).unwrap();
    let client = host.client();
    let consumer = client
        .activation(&maka_plugin_core::composition::InstanceId(vec![
            uuid::Uuid::from_u128(12),
        ]))
        .unwrap();
    let binding = timer
        .block_on(client.bind(consumer, "reader".into()))
        .unwrap()
        .remove(0);
    let registration_epoch = binding.registration_epoch();
    let approved = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
    let bridge = std::sync::Arc::new(maka_application_bridge::LocalService::new(
        client,
        binding,
        std::sync::Arc::new(UiReadScopes {
            issuer,
            approved: approved.clone(),
            observed: observed.clone(),
        }),
    ));
    let (mut input, mut packages) = support::resource_fixture();
    input.instances.remove("reader");
    packages.retain(|p| p.manifest().id != "native.reader");
    let index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(index);
    let mut declaration = old.manifest().application.clone().unwrap();
    let mut contract = maka_ui::text::reader_contract();
    contract.transport = maka_plugin_core::contract::Transport::Remote;
    declaration
        .entries
        .get_mut(&maka_plugin_core::package::EntryTarget::Desktop)
        .unwrap()
        .imports
        .get_mut("reader")
        .unwrap()
        .contract = contract.clone();
    let package = support::package(
        "dynamic.app",
        declaration,
        Some(
            std::str::from_utf8(old.file("index.mjs").unwrap())
                .unwrap()
                .to_owned(),
        ),
    );
    let target = maka_plugin_core::plan::RemoteCapability {
        deployment: "host-a".into(),
        instance: maka_plugin_core::composition::InstanceId(vec![uuid::Uuid::from_u128(11)]),
        export: "Reader".into(),
    };
    let app = input.instances.get_mut("app").unwrap();
    app.package = package.application_ref().into();
    let consumer_id = maka_plugin_core::composition::InstanceId(vec![app.key]);
    app.bindings.insert(
        "reader".into(),
        vec![maka_plugin_core::composition::Target::Remote {
            deployment: target.deployment.clone(),
            instance: target.instance.clone(),
            export: target.export.clone(),
        }],
    );
    packages.push(package);
    let remotes = maka_ui::actions::RemoteServices {
        declarations: BTreeMap::from([(
            target.clone(),
            maka_plugin_core::contract::Export {
                contract: contract.clone(),
                availability: maka_plugin_core::contract::Availability::Required,
            },
        )]),
        registrations: vec![maka_ui::actions::RemoteService {
            target,
            registration_epoch,
            contract,
            implementation: bridge,
        }],
    };
    let (dir, namespaces, store) = stored_root(&timer);
    let root = dir.path().join("root");
    let deployment = uuid::Uuid::new_v4();
    let observations = remotes
        .declarations
        .into_iter()
        .map(
            |(capability, declaration)| maka_plugin_core::deployment::RemoteObservation {
                capability,
                declaration,
            },
        )
        .collect();
    let ui_source = timer
        .block_on(store.commit_application(deployment, 0, input, observations, packages))
        .unwrap();
    let application = timer
        .block_on(Application::start_from_store(
            store.clone(),
            ui_source.clone(),
            PresentationTarget::Desktop,
            timer.handle().clone(),
            Default::default(),
            widgets::registry(),
            remotes.registrations,
        ))
        .unwrap();
    drop(store);
    let scene = Scene::from_application(cx, application);
    cx.update(|cx| actions::own(scene.app.clone(), None, None, cx).unwrap());
    scene.draw(cx);
    // Source facts alone cannot authorize a call in the independent Host kernel.
    let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
    scene
        .app
        .borrow_mut()
        .dispatch(&scene.root, revision, "read", serde_json::Value::Null)
        .unwrap();
    cx.update_window(scene.window, |_, window, cx| {
        scene
            .surface
            .update(cx, |s, cx| s.sync_native(window, cx).unwrap());
    })
    .unwrap();
    await_action(cx, &scene.app, |a| a.accepted_action_count() == 0);
    assert_eq!(started.available_permits(), 0);
    assert!(snapshot.lock().unwrap().is_none());
    let rejected = scene.app.borrow().frame(&scene.root).unwrap().state.clone();
    assert!(rejected.to_string().contains("native read grant denied"));
    approved.store(true, std::sync::atomic::Ordering::SeqCst);

    let owner = scene.owner();
    cx.update_window(scene.window, |_, window, cx| {
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "read", serde_json::Value::Null)
            .unwrap();
        scene
            .surface
            .update(cx, |s, cx| s.sync_native(window, cx).unwrap());
        owner.update(cx, |o, cx| {
            o.replace_text_in_range(Some(0..5), "After", window, cx)
        });
    })
    .unwrap();
    // Native construction now happens inside the tracked worker. Drive the
    // real foreground task that starts it instead of blocking that executor.
    await_action(cx, &scene.app, |_| started.available_permits() == 1);
    started.try_acquire().unwrap().forget();
    let source = observed.lock().unwrap().clone().unwrap();
    assert_eq!(source.deployment(), Some(&ui_source));
    assert_eq!(source.consumer().instance(), &consumer_id);
    assert_eq!(source.mount(), &scene.root);
    assert!(!source.request_id().is_nil());
    let weak = snapshot.lock().unwrap().clone().unwrap();
    assert!(weak.upgrade().is_some());
    cx.update_window(scene.window, |_, window, cx| {
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "remove", serde_json::Value::Null)
            .unwrap();
        scene
            .surface
            .update(cx, |s, cx| s.sync_native(window, cx).unwrap());
    })
    .unwrap();
    let editor = cx.update(|cx| owner.read(cx).state.clone());
    let editor_released = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = editor_released.clone();
    let subscription = cx.update(|cx| {
        cx.observe_release(&editor, move |_, _| {
            observed.store(true, std::sync::atomic::Ordering::SeqCst);
        })
    });
    drop(editor);
    drop(owner);
    scene.close(cx);
    assert!(scene.resources.borrow().surface_closed);
    assert!(editor_released.load(std::sync::atomic::Ordering::SeqCst));
    assert!(
        maka_event_log::root::RootOwner::open(&root, &namespaces).is_err(),
        "accepted query retains the UI Root after actual native destruction"
    );
    assert!(
        maka_event_log::root::RootOwner::open(&host_root, &host_ns).is_err(),
        "the independent Host execution owns its own original Root"
    );
    assert!(weak.upgrade().is_some());
    assert_eq!(
        host.client()
            .snapshot()
            .instances
            .iter()
            .find(|s| s.activation.instance()
                == &maka_plugin_core::composition::InstanceId(vec![uuid::Uuid::from_u128(11)]))
            .unwrap()
            .calls,
        1
    );
    release.add_permits(1);
    if fail_cleanup {
        await_action(cx, &scene.app, |a| a.failed_action_ids().len() == 1);
        assert_eq!(scene.app.borrow().accepted_action_count(), 1);
        assert!(scene.app.borrow_mut().shutdown().is_err());
        assert!(weak.upgrade().is_some());
        assert!(
            host.client()
                .snapshot()
                .cleanup_error
                .unwrap()
                .contains("Host snapshot cleanup unconfirmed")
        );
        assert!(timer.block_on(host.shutdown()).is_err());
        assert!(maka_event_log::root::RootOwner::open(&root, &namespaces).is_err());
        assert!(maka_event_log::root::RootOwner::open(&host_root, &host_ns).is_err());
        drop(host);
        cx.run_until_parked();
        assert!(weak.upgrade().is_some());
    } else {
        await_action(cx, &scene.app, |a| a.accepted_action_count() == 0);
        assert!(weak.upgrade().is_none());
        timer.block_on(host.shutdown()).unwrap();
        scene.app.borrow_mut().shutdown().unwrap();
        timer.block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                for (path, ns) in [(&root, &namespaces), (&host_root, &host_ns)] {
                    loop {
                        match maka_event_log::root::RootOwner::open(path, ns) {
                            Ok(owner) => break drop(owner),
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                tokio::task::yield_now().await
                            }
                            Err(e) => panic!("{e}"),
                        }
                    }
                }
            })
            .await
            .unwrap();
        });
    }
    drop(subscription);
    let application = Rc::downgrade(&scene.app);
    drop(scene);
    cx.run_until_parked();
    if fail_cleanup {
        assert!(
            application.upgrade().is_some(),
            "failed query remains owned after all visual observers exit"
        );
        assert!(maka_event_log::root::RootOwner::open(&root, &namespaces).is_err());
        assert!(maka_event_log::root::RootOwner::open(&host_root, &host_ns).is_err());
    }
}

#[path = "editor_tests/mutations.rs"]
mod mutations;

#[gpui::test]
fn empty_host_quit_waits_for_actual_caller_sql_close_after_accepted_writer_waiter_is_lost(
    cx: &mut TestAppContext,
) {
    use maka_event_log::{
        EventLog,
        root::{ROOT_DATABASE, RootNamespaces, RootOwner},
    };
    // This case owns real Tokio and SQL threads, rather than the test dispatcher.
    cx.executor().allow_parking();
    let timer = timer();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root");
    let namespaces = RootNamespaces {
        ownership: directory.path().join("owners"),
        control: directory.path().join("control"),
    };
    let caller = Arc::new(
        timer
            .block_on(EventLog::for_root(Arc::new(
                RootOwner::create(&root, &namespaces).unwrap(),
            )))
            .unwrap(),
    );
    let (input, packages) = support::editor_fixture();
    let source = timer
        .block_on(caller.commit_application(uuid::Uuid::new_v4(), 0, input, vec![], packages))
        .unwrap();
    let application = timer
        .block_on(Application::start_from_store(
            caller.clone(),
            source,
            PresentationTarget::Desktop,
            timer.handle().clone(),
            Default::default(),
            widgets::registry(),
            vec![],
        ))
        .unwrap();
    let scene = Scene::from_application(cx, application);
    cx.update(|cx| actions::own(scene.app.clone(), None, Some(caller.clone()), cx).unwrap());
    scene.draw(cx);
    let blocker = rusqlite::Connection::open(root.join(ROOT_DATABASE)).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let pending = uuid::Uuid::new_v4();
    let (input, packages) = support::editor_fixture();
    assert!(
        timer
            .block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_millis(50),
                    caller.commit_application(pending, 0, input, vec![], packages),
                )
                .await
            })
            .is_err()
    );
    scene.close(cx);
    assert!(
        scene.app.borrow().can_shutdown(),
        "UI has no outstanding service work"
    );
    cx.update(|cx| actions::quit_when_drained(&scene.app, cx));
    cx.run_until_parked();
    assert!(cx.update(|cx| cx.has_global::<actions::Owner>()));
    assert!(RootOwner::open(&root, &namespaces).is_err());
    blocker.execute_batch("COMMIT;").unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while cx.update(|cx| cx.has_global::<actions::Owner>()) {
        cx.run_until_parked();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    // The cached receipt must already be ready when the quit gate releases.
    use futures_util::FutureExt;
    assert!(matches!(caller.shutdown().now_or_never(), Some(Ok(()))));
    drop(blocker);
    drop(scene);
    drop(caller);
    cx.run_until_parked();
    let reopened = timer
        .block_on(EventLog::for_root(Arc::new(
            RootOwner::open(&root, &namespaces).unwrap(),
        )))
        .unwrap();
    assert!(
        timer
            .block_on(reopened.load_application(pending))
            .unwrap()
            .is_some()
    );
    timer.block_on(reopened.close()).unwrap();
}

mod composition;
