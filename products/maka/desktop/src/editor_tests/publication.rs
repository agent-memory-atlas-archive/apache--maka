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
use maka_ui_runtime::{
    ComponentCall, ComponentOutput, Frame, NativeCapability, NativeComponent, NativeExport,
};
use serde_json::json;
use std::cell::Cell;
struct Parent {
    reject: Rc<Cell<bool>>,
}
impl NativeComponent<dyn widgets::Widget> for Parent {
    fn call(
        &self,
        call: ComponentCall<'_>,
        values: &maka_ui_runtime::abi::ValueBindings,
    ) -> maka_ui_runtime::Result<ComponentOutput<NativeView<dyn widgets::Widget>>> {
        let (mut state, revision) = match call {
            ComponentCall::Mount { .. } => (
                json!({"command":null,"observation":null,"receipts":[],"visible":true}),
                0,
            ),
            ComponentCall::Update { state, .. } => {
                (state.clone(), state["revision"].as_u64().unwrap() + 1)
            }
            ComponentCall::Event {
                state, key, input, ..
            } => {
                let mut next = state.clone();
                if key == "changed" {
                    if self.reject.get() {
                        return Err(maka_ui_runtime::Error::Invalid(
                            "healthy parent rejects observation".into(),
                        ));
                    }
                    next["observation"] = input.clone();
                    if let Some(receipt) = input.get("receipt").filter(|receipt| !receipt.is_null())
                    {
                        next["receipts"]
                            .as_array_mut()
                            .unwrap()
                            .push(receipt.clone());
                    }
                } else {
                    if let Some(command) = input.get("command") {
                        next["command"] = command.clone();
                    }
                    if let Some(visible) = input.get("visible") {
                        next["visible"] = visible.clone();
                    }
                }
                (next, state["revision"].as_u64().unwrap() + 1)
            }
        };
        state["revision"] = json!(revision);
        let view = widgets::scene::compile(
            &json!({"kind":"stack","key":"root","direction":"column","padding":16,"gap":12,"background":0x101010,"children":[
             {"kind":"stack","key":"presentation","direction":"column","gap":0,"padding":0,"background":0x101010,"visible":state["visible"],"children":[{"kind":"component","key":"nativeLabel","binding":"label","member":0,"on":{},"props":{"text":"Retained marker","appearance":values.get("brand").unwrap()[0]["label"]}},{"kind":"component","key":"editor","binding":"editor","member":0,"on":{"changed":"changed"},"props":{"appearance":values.get("brand").unwrap()[0]["textarea"],"accessibilityLabel":"Read-only draft","readOnly":true,"disabled":true,"softWrap":true,"tabIndex":-1,"initialText":"Start","command":state["command"]}}]},
             {"kind":"button","key":"parent","label":"Next parent transaction"}
            ]}),
        )?;
        Ok(Frame{revision,state,surface:Some(serde_json::from_value(json!({"kind":"window","binding":"surface","title":"Receipt publication","width":640,"height":400})).unwrap()),view}.output(vec![]))
    }
}
fn scene(
    cx: &mut TestAppContext,
    timer: &tokio::runtime::Runtime,
    reject: Rc<Cell<bool>>,
) -> Scene {
    use maka_plugin_core::package::{EntryTarget, Implementation};
    let (mut input, mut packages) = support::editor_fixture();
    let index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(index);
    let mut declaration = old
        .manifest()
        .application
        .as_ref()
        .unwrap()
        .declaration()
        .clone();
    let entry = declaration.entries.get_mut(&EntryTarget::Desktop).unwrap();
    entry.implementation = Implementation::Native {
        implementation: "test.publication.parent".into(),
    };
    entry.scene = None;
    let contract = entry.exports["App"].contract.clone();
    let parent = support::package("dynamic.app", declaration, None);
    input.instances.get_mut("app").unwrap().package = parent.application_ref().into();
    packages.push(parent);
    let mut registry = widgets::registry();
    registry
        .register("test.publication.parent", move |_, _| {
            Ok(BTreeMap::from([(
                "App".into(),
                NativeExport {
                    contract: contract.clone(),
                    value: NativeCapability::Component(Rc::new(Parent {
                        reject: reject.clone(),
                    })),
                },
            )]))
        })
        .unwrap();
    Scene::with_registry(
        cx,
        timer,
        false,
        (input, packages),
        registry,
        Default::default(),
    )
}
#[gpui::test]
fn healthy_parent_failure_republishes_receipt_with_unchanged_readonly_child(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let reject = Rc::new(Cell::new(false));
    let scene = scene(cx, &timer, reject.clone());
    scene.draw(cx);
    let owner = scene.owner();
    reject.set(true);
    cx.update_window(scene.window, |_, window, cx| {
        scene
            .command(
                EditorCommand {
                    owner: scene.editor_owner(cx),
                    sequence: 1,
                    based_on: owner.read(cx).revision,
                    edit: EditorEdit::Reset {
                        text: String::new(),
                    },
                },
                window,
                cx,
            )
            .unwrap()
    })
    .unwrap();
    cx.run_until_parked();
    let child_revision = scene.app.borrow().frame(&scene.editor).unwrap().revision;
    assert_eq!(
        scene.app.borrow().frame(&scene.editor).unwrap().state["pending"],
        true
    );
    let native_revision = cx.update(|cx| {
        assert!(owner.read(cx).state.read(cx).value().is_empty());
        assert!(!owner.read(cx).state.read(cx).can_undo());
        owner.read(cx).revision
    });
    reject.set(false);
    cx.update_window(scene.window, |_, window, cx| {
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "parent", json!({}))
            .unwrap();
        assert_eq!(
            scene.app.borrow().frame(&scene.editor).unwrap().revision,
            child_revision,
            "unchanged props do not stage a new child frame"
        );
        scene
            .surface
            .update(cx, |surface, cx| surface.sync_native(window, cx).unwrap());
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(
        scene.app.borrow().frame(&scene.editor).unwrap().state["pending"],
        false
    );
    assert_eq!(
        scene.app.borrow().frame(&scene.root).unwrap().state["observation"]["receipt"]["sequence"],
        1
    );
    cx.update(|cx| {
        assert_eq!(
            owner.read(cx).revision,
            native_revision,
            "publication does not replay Reset"
        )
    });
    scene.close(cx);
}

#[gpui::test]
fn retained_presentation_blocks_old_child_slots_but_settles_hidden_editor_receipts(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = scene(cx, &timer, Rc::new(Cell::new(false)));
    cx.activate_accessibility(scene.window);
    scene.draw(cx);
    let owner = scene.owner();
    let entity = owner.entity_id();
    let child_revision = scene.app.borrow().frame(&scene.editor).unwrap().revision;
    let old_child = cx
        .update_window(scene.window, |_, window, cx| {
            scene.surface.update(cx, |surface, cx| {
                surface.scene(window, cx).unwrap().entry.children["nativeLabel"].clone()
            })
        })
        .unwrap();
    // Label has no presentation hook: this exercises the framework gate itself,
    // rather than an editor or scene recipe independently hiding its own render.
    cx.with_paused_automatic_drawing(|cx| {
        cx.update_window(scene.window, |_, window, cx| {
            let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
            scene
                .app
                .borrow_mut()
                .dispatch(&scene.root, revision, "parent", json!({"visible":false}))
                .unwrap();
            scene
                .surface
                .update(cx, |surface, cx| surface.sync_native(window, cx).unwrap());
            assert_eq!(
                scene.app.borrow().frame(&scene.editor).unwrap().revision,
                child_revision
            );
            assert!(!scene.resources.borrow().owners[&scene.editor.id().to_string()].visible);
            assert_eq!(owner.entity_id(), entity);
        })
        .unwrap();
        gpui_kit::VisualTestContext::from_window(scene.window, cx).draw(
            point(px(0.), px(0.)),
            size(px(640.), px(400.)),
            |window, cx| {
                let resources = scene.resources.borrow();
                let native = &resources.owners[&old_child.entry.lease.mount().id().to_string()];
                let raw = old_child
                    .entry
                    .frame
                    .view
                    .view
                    .render(native.state.as_ref(), &old_child.slots(), window, cx)
                    .unwrap();
                gpui_kit::div()
                    .child(raw)
                    .child(old_child.render(window, cx).unwrap())
                    .on_children_prepainted(|bounds, _, _| {
                        assert_eq!(bounds.len(), 2);
                        assert!(
                            bounds[0].size.width > px(0.) && bounds[0].size.height > px(0.),
                            "the native Label itself has no hidden-render hook"
                        );
                        assert_eq!(
                            bounds[1].size,
                            size(px(0.), px(0.)),
                            "a retained child render obeys current ancestor presentation"
                        );
                    })
                    .into_any_element()
            },
        );
    });
    scene.draw(cx);
    assert!(
        !cx.accessibility_update(scene.window)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.label() == Some("Read-only draft"))
    );
    cx.update_window(scene.window, |_, window, cx| {
        scene
            .command(
                EditorCommand {
                    owner: scene.editor_owner(cx),
                    sequence: 1,
                    based_on: owner.read(cx).revision,
                    edit: EditorEdit::Reset {
                        text: "Hidden reset".into(),
                    },
                },
                window,
                cx,
            )
            .unwrap();
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(
        scene.app.borrow().frame(&scene.root).unwrap().state["observation"]["receipt"]["sequence"],
        1
    );
    cx.update(|cx| {
        assert_eq!(
            owner.read(cx).state.read(cx).value().as_ref(),
            "Hidden reset"
        );
        assert!(!owner.read(cx).state.read(cx).can_undo());
    });
    cx.update_window(scene.window, |_, window, cx| {
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "parent", json!({"visible":true}))
            .unwrap();
        scene
            .surface
            .update(cx, |surface, cx| surface.sync_native(window, cx).unwrap());
    })
    .unwrap();
    scene.draw(cx);
    assert_eq!(scene.owner().entity_id(), entity);
    cx.update(|cx| {
        let state = owner.read(cx).state.read(cx);
        assert!(
            state.presentation().is_disabled(),
            "show restores the original disabled props"
        );
        assert!(state.presentation().is_readonly());
    });
    assert!(
        cx.accessibility_update(scene.window)
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.label() == Some("Read-only draft"))
    );
    scene.close(cx);
}

#[gpui::test]
fn consecutive_original_editor_receipts_survive_candidate_rejection_and_release_custody(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let reject = Rc::new(Cell::new(false));
    let scene = scene(cx, &timer, reject.clone());
    scene.draw(cx);
    let owner = scene.owner();
    let budget = scene.resources.borrow().budget.clone();
    let mut originals = Vec::new();
    reject.set(true);
    cx.update_window(scene.window, |_, window, cx| {
        let based_on = owner.read(cx).revision;
        for (sequence, text) in [(1, "A"), (2, "B")] {
            scene.command(EditorCommand {
                owner: scene.editor_owner(cx), sequence, based_on,
                edit: EditorEdit::Append { text: text.into() },
            }, window, cx).unwrap();
            originals.push(serde_json::to_value(owner.read(cx).receipt.as_ref().unwrap()).unwrap());
        }
        assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "StartA");
        assert_eq!(originals[0]["outcome"]["status"], "applied");
        assert_eq!(originals[1]["outcome"], json!({"status":"conflict","reason":"revision"}));
        assert!(scene.app.borrow().frame(&scene.root).unwrap().state["receipts"].as_array().unwrap().is_empty());
        let occupied = budget.reserve(maka_ui_runtime::budget::Kind::Encoded, 64 * 1024 * 1024 - budget.usage().0).unwrap();
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene.app.borrow_mut().dispatch(&scene.root, revision, "parent", json!({"command":{
            "owner":scene.editor_owner(cx),"sequence":3,"basedOn":based_on+1,"edit":{"kind":"append","text":"C"}
        }})).unwrap();
        assert!(scene.commit_in(window, cx).is_err(), "new native effect requires precommit custody");
        assert_eq!(scene.app.borrow().frame(&scene.root).unwrap().revision, revision);
        assert_eq!(owner.read(cx).state.read(cx).value().as_ref(), "StartA");
        assert_eq!(serde_json::to_value(owner.read(cx).receipt.as_ref().unwrap()).unwrap(), originals[1]);
        drop(occupied);
    }).unwrap();
    reject.set(false);
    cx.update_window(scene.window, |_, window, cx| {
        let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
        scene
            .app
            .borrow_mut()
            .dispatch(&scene.root, revision, "parent", json!({}))
            .unwrap();
        scene.settle_in(window, cx).unwrap();
    })
    .unwrap();
    await_action(cx, &scene.app, |app| {
        app.frame(&scene.root).unwrap().state["receipts"]
            .as_array()
            .unwrap()
            .len()
            >= 2
            && !app.ui_pending()
    });
    assert_eq!(
        scene.app.borrow().frame(&scene.root).unwrap().state["receipts"],
        json!(originals)
    );
    drop(owner);
    scene.close(cx);
    await_shutdown(cx, &scene.app);
    assert_eq!(
        budget.usage(),
        (0, 0, 0),
        "retirement returns every publication permit and original ACK"
    );
}
