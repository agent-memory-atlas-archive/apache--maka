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
use maka_ui::{
    ComponentCall, ComponentOutput, Frame, NativeCapability, NativeComponent, NativeExport,
};
use serde_json::json;
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
struct Parent {
    reject: Rc<Cell<bool>>,
}
impl NativeComponent<dyn widgets::Widget> for Parent {
    fn call(
        &self,
        call: ComponentCall<'_>,
        values: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn widgets::Widget>>> {
        let (mut state, revision) = match call {
            ComponentCall::Mount { .. } => (json!({"command":null,"observation":null}), 0),
            ComponentCall::Update { state, .. } => {
                (state.clone(), state["revision"].as_u64().unwrap() + 1)
            }
            ComponentCall::Event {
                state, key, input, ..
            } => {
                let mut next = state.clone();
                if key == "changed" {
                    if self.reject.get() {
                        return Err(maka_ui::Error::Invalid(
                            "healthy parent rejects observation".into(),
                        ));
                    }
                    next["observation"] = input.clone();
                } else if let Some(command) = input.get("command") {
                    next["command"] = command.clone();
                }
                (next, state["revision"].as_u64().unwrap() + 1)
            }
        };
        state["revision"] = json!(revision);
        let view = widgets::scene::compile(
            &json!({"kind":"stack","key":"root","direction":"column","padding":16,"gap":12,"background":0x101010,"children":[
             {"kind":"component","key":"editor","binding":"editor","member":0,"on":{"changed":"changed"},"props":{"appearance":values.get("brand").unwrap()[0]["textarea"],"accessibilityLabel":"Read-only draft","readOnly":true,"disabled":true,"softWrap":true,"tabIndex":-1,"initialText":"Start","command":state["command"]}},
             {"kind":"button","key":"parent","label":"Next parent transaction"}
            ]}),
            Instant::now() + Duration::from_secs(1),
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
    let mut declaration = old.manifest().application.clone().unwrap();
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
