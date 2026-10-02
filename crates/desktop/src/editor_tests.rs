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
        (mut input, mut packages): (
            maka_plugin_core::composition::Input,
            Vec<maka_plugins::package::Package>,
        ),
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
        let mut application = Application::new(
            &input,
            packages,
            timer.handle().clone(),
            Default::default(),
            widgets::registry(),
        )
        .unwrap();
        let root = application.mount_launch(serde_json::json!({})).unwrap();
        let editor = application.child_mount(&root, "editor").unwrap().clone();
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
