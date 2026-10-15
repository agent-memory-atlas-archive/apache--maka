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
use gpui_kit::{Modifiers, MouseButton, PlatformInput, ScrollDelta, ScrollWheelEvent, TouchPhase};
use maka_plugin_core::{
    composition::Target,
    contract::{
        Availability, Cardinality, Contract, Export, Import, Interface, Method, MethodMode,
        Transport,
    },
    package::{Declaration, EntryDeclaration, EntryTarget, Implementation},
};
use maka_ui_runtime::{
    NativeCapability, NativeExport,
    actions::{NativeService, ServiceCompletion, ServiceFuture, ServiceRequest},
};
use serde_json::{Value, json};
use std::sync::Arc;

struct History;
impl NativeService for History {
    fn call(&self, request: ServiceRequest) -> ServiceFuture {
        // Native presentation witnesses must never enter the business request.
        assert!(request.resources.is_empty());
        assert_eq!(request.method, "tail");
        assert_eq!(request.input, Value::Null);
        Box::pin(async {
            ServiceCompletion {
                result: Ok(json!({"source":"Replacement source"})),
                cleanup: Ok(()),
            }
        })
    }
}
fn fixture(
    tall: bool,
) -> (
    maka_plugin_core::composition::Input,
    Vec<maka_packages::Package>,
    Contract,
) {
    let (mut input, mut packages) =
        support::fixture("return frame({...state,revision:state.revision+1});");
    let index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let original = packages.remove(index);
    let mut declaration = original.manifest().application.clone().unwrap();
    let history = Contract {
        name: "test.History".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Service {
            methods: BTreeMap::from([(
                "tail".into(),
                Method {
                    progress: None,
                    mode: MethodMode::Query,
                    input: json!({"type":"null"}),
                    output: json!({"type":"object"}),
                    resources: BTreeMap::new(),
                },
            )]),
        },
    };
    let native_package =
        |id: &str, implementation: &str, name: &str, contract: Contract, imports| {
            support::package(
                id,
                Declaration {
                    composition: None,
                    entries: BTreeMap::from([(
                        EntryTarget::Desktop,
                        EntryDeclaration {
                            scene: None,
                            implementation: Implementation::Native {
                                implementation: implementation.into(),
                            },
                            configuration: json!({"type":"object"}),
                            imports,
                            exports: BTreeMap::from([(
                                name.into(),
                                Export {
                                    contract,
                                    availability: Availability::Required,
                                },
                            )]),
                        },
                    )]),
                },
                None,
            )
        };
    let reading = maka_ui_runtime::reading::reading_contract();
    let markdown = maka_ui_runtime::markdown::markdown_contract();
    let area = native_package(
        "test.reading",
        "maka.ui.reading",
        "ReadingArea",
        reading.clone(),
        BTreeMap::from([(
            "markdown".into(),
            Import {
                contract: markdown.clone(),
                cardinality: Cardinality::One,
            },
        )]),
    );
    let document = native_package(
        "test.markdown",
        "maka.ui.markdown",
        "Markdown",
        markdown,
        BTreeMap::new(),
    );
    let service = native_package(
        "test.history",
        "test.history",
        "History",
        history.clone(),
        BTreeMap::new(),
    );
    let entry = declaration.entries.get_mut(&EntryTarget::Desktop).unwrap();
    entry.imports.insert(
        "reading".into(),
        Import {
            contract: reading,
            cardinality: Cardinality::One,
        },
    );
    entry.imports.insert(
        "history".into(),
        Import {
            contract: history.clone(),
            cardinality: Cardinality::One,
        },
    );
    let text = support::brand_appearance()["label"].clone();
    let appearance = json!({"body":text,"code":text,"headings":[text,text,text,text,text,text],"muted":0x444444ffu32,
        "link":0x004499ffu32,"selection":0x4466ff55u32,"codeBackground":0xffffff00u32,"tableBackground":0xffffff00u32,
        "border":0xddddddffu32,"dark":false,"paragraphGap":8});
    let source = r#"
    const contract=CONTRACT,appearance=APPEARANCE,tall=TALL;
    export function activate(){return {App:{contract,kind:'component',component:['app']}};}
    function frame(state){
        const content={kind:'stack',key:'reading',direction:'column',padding:0,gap:0,background:0xffffff,scroll:'follow_end',
            style:{width:{unit:'pixels',value:320},height:{unit:'pixels',value:100}},
            children:[...(tall?[{kind:'text',key:'spacer',text:'Earlier source',color:0,style:{height:{unit:'pixels',value:300},shrink:0}}]:[]),
                {kind:'component',key:state.replaced?'replacement':'document',binding:'markdown',member:0,on:{},
                    props:{source:state.source,appearance,selectable:true,disabled:false}}]};
        return {actions:[],events:[],frame:{revision:state.revision,state,
            surface:{kind:'window',binding:'surface',title:'Reading continuation',width:640,height:400},
            view:{kind:'stack',key:'root',direction:'column',padding:0,gap:0,background:0xffffff,
                children:[{kind:'component',key:'editor',binding:'reading',member:0,props:{content,command:null},on:{observed:'observed'}},{kind:'button',key:'tail',label:'Read latest'}]}}};
    }
    export const app={mount(){return frame({revision:0,source:'Original selection 中文',replaced:false});},
        update(state){return frame({...state,revision:state.revision+1});},
        event(state,event){const next={...state,revision:state.revision+1};
            if(event.kind==='component')next.observation=event.event;
            if(event.kind==='action'){next.outcome=event.event;if(event.event.status==='succeeded'){next.source=event.event.value.source;next.replaced=true;}}
            const out=frame(next);
            if(event.kind==='raw'&&event.key==='tail')out.actions.push({key:'tail_done',binding:'history',member:0,method:'tail',input:null,resources:{},continuations:[{child:'editor',method:'tail',input:null}]});
            return out;}};
    "#
    .replace("CONTRACT", &serde_json::to_string(&entry.exports["App"].contract).unwrap())
    .replace("APPEARANCE", &appearance.to_string()).replace("TALL", if tall { "true" } else { "false" });
    let app = support::package("dynamic.app", declaration, Some(source));
    let app_node = input.instances.get_mut("app").unwrap();
    app_node.package = app.application_ref().into();
    for (binding, path, export) in [
        ("reading", "reading", "ReadingArea"),
        ("history", "history", "History"),
    ] {
        app_node.bindings.insert(
            binding.into(),
            vec![Target::Instance {
                path: vec![path.into()],
                export: export.into(),
            }],
        );
    }
    let mut area_node = support::node(&area, 3);
    area_node.bindings.insert(
        "markdown".into(),
        vec![Target::Instance {
            path: vec!["markdown".into()],
            export: "Markdown".into(),
        }],
    );
    input.instances.insert("reading".into(), area_node);
    input
        .instances
        .insert("markdown".into(), support::node(&document, 4));
    input
        .instances
        .insert("history".into(), support::node(&service, 5));
    packages.extend([app, area, document, service]);
    (input, packages, history)
}

#[gpui::test]
fn actual_reading_tail_checks_wheel_and_completed_selection_before_whole_scene_publication(
    cx: &mut TestAppContext,
) {
    cx.with_paused_automatic_drawing(|cx| {
        let timer = timer();
        for change in ["none", "wheel", "selection", "drag"] {
            let (input, packages, contract) = fixture(matches!(change, "none" | "wheel"));
            let mut registry = widgets::registry();
            registry.register("test.history", move |_, _| Ok(BTreeMap::from([("History".into(), NativeExport {
                contract: contract.clone(), value: NativeCapability::Service(Arc::new(History)),
            })]))).unwrap();
            let scene = Scene::with_registry(cx, &timer, false, (input, packages), registry, Default::default());
            scene.draw(cx);
            cx.update_window(scene.window, |_, window, cx| window.simulate_next_frame(cx)).unwrap();
            let reading = scene.editor.clone();
            let document = scene.app.borrow().child_mount(&reading, "document").unwrap().clone();
            let owner = scene.resources.borrow().owners[&document.id().to_string()].identity.clone();
            assert_eq!(scene.app.borrow().frame(&scene.root).unwrap().state["observation"]["viewport"]["atEnd"], true);
            let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
            scene.app.borrow_mut().dispatch(&scene.root, revision, "tail", Value::Null).unwrap();
            cx.update_window(scene.window, |_, window, cx| {
                scene.commit_in(window, cx).unwrap();
                scene.app.borrow_mut().capture_actions(&scene.root, |target| scene.resources.borrow_mut().capture(target, window, cx)).unwrap();
            }).unwrap();
            let task = scene.app.borrow_mut().start_task(&scene.root).unwrap().unwrap();
            let completion = timer.block_on(task.future);
            assert!(matches!(&completion, maka_ui_runtime::actions::TaskCompletion::Query(result) if result.result.as_ref().unwrap()["source"] == "Replacement source"));
            scene.app.borrow_mut().finish_task(task.id, completion).unwrap();
            cx.update_window(scene.window, |_, window, cx| {
                if change == "wheel" {
                    window.dispatch_event(PlatformInput::ScrollWheel(ScrollWheelEvent {
                        position: point(px(30.), px(40.)), delta: ScrollDelta::Pixels(point(px(0.), px(100.))),
                        modifiers: Modifiers::default(), touch_phase: TouchPhase::Moved,
                    }), cx);
                } else if matches!(change, "selection" | "drag") {
                    window.dispatch_event(PlatformInput::MouseDown(gpui_kit::MouseDownEvent {
                        button: MouseButton::Left, position: point(px(30.), px(20.)), modifiers: Modifiers::default(), click_count: 1, first_mouse: false,
                    }), cx);
                    if change == "selection" {
                    window.dispatch_event(PlatformInput::MouseUp(gpui_kit::MouseUpEvent {
                        button: MouseButton::Left, position: point(px(30.), px(12.)), modifiers: Modifiers::default(), click_count: 1,
                    }), cx);
                    window.focused(cx).unwrap().dispatch_action(&gpui_kit::base::input::SelectAll, window, cx);
                    assert_eq!(gpui_kit::base::TextSelection::selected_text(window, cx), "Original selection 中文\n");
                    } else {
                        assert_eq!(gpui_kit::base::TextSelection::selected_text(window, cx), "");
                    }
                }
                // The plugin's last atEnd is still optimistic: no new paint/frame callback.
                assert_eq!(scene.app.borrow().frame(&scene.root).unwrap().state["observation"]["viewport"]["atEnd"], true);
                assert_eq!(scene.surface.update(cx, |surface, cx| surface.deliver_action_results(window, cx)).unwrap(), 1);
            }).unwrap();
            scene.settle(cx);
            let state = scene.app.borrow().frame(&scene.root).unwrap().state.clone();
            assert_eq!(state["outcome"]["status"], if change == "none" { "succeeded" } else { "stale" });
            assert_eq!(state["source"], if change == "none" { "Replacement source" } else { "Original selection 中文" });
            scene.draw(cx);
            if change != "none" {
                assert_eq!(scene.app.borrow().child_mount(&reading, "document").unwrap(), &document);
                assert!(Rc::ptr_eq(&owner, &scene.resources.borrow().owners[&document.id().to_string()].identity));
            } else {
                assert!(scene.app.borrow().child_mount(&reading, "document").is_err());
                assert!(scene.app.borrow().child_mount(&reading, "replacement").is_ok());
            }
            if change == "selection" {
                cx.update_window(scene.window, |_, window, cx| {
                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("copy sentinel".into()));
                    window.focused(cx).unwrap().dispatch_action(&gpui_kit::base::input::Copy, window, cx);
                    assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), "Original selection 中文");
                }).unwrap();
            }
            if change == "drag" {
                cx.update_window(scene.window, |_, window, cx| {
                    window.dispatch_event(PlatformInput::MouseMove(gpui_kit::MouseMoveEvent {
                        position: point(px(160.), px(20.)), pressed_button: Some(MouseButton::Left), modifiers: Modifiers::default(),
                    }), cx);
                    window.dispatch_event(PlatformInput::MouseUp(gpui_kit::MouseUpEvent {
                        button: MouseButton::Left, position: point(px(160.), px(20.)), modifiers: Modifiers::default(), click_count: 1,
                    }), cx);
                    window.draw(cx).clear(cx);
                    let selected = gpui_kit::base::TextSelection::selected_text(window, cx);
                    assert!(!selected.is_empty(), "original collapsed gesture must still extend");
                    assert!("Original selection 中文\n".contains(selected.trim_end_matches('\n')));
                    window.focused(cx).unwrap().dispatch_action(&gpui_kit::base::input::Copy, window, cx);
                    assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), selected.trim_end_matches('\n'));
                }).unwrap();
            }
            assert!(!scene.app.borrow().is_retiring(&scene.root));
            assert_eq!(scene.app.borrow().accepted_action_count(), 0);
            scene.close_and_drain(cx);
        }
    });
}

#[gpui::test]
fn committed_addition_preserves_original_reading_selection_copy_and_viewport(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    for select in [true, false] {
        let (_dir, _namespaces, store) = stored_root(&timer);
        let deployment = uuid::Uuid::new_v4();
        let (mut input, mut packages, contract) = fixture(!select);
        if !select {
            let index = packages
                .iter()
                .position(|p| p.manifest().id == "dynamic.app")
                .unwrap();
            let original = packages.remove(index);
            let source = std::str::from_utf8(original.file("index.mjs").unwrap())
                .unwrap()
                .replace("scroll:'follow_end'", "scroll:'manual'");
            let package = support::package(
                "dynamic.app",
                original.manifest().application.clone().unwrap(),
                Some(source),
            );
            input.instances.get_mut("app").unwrap().package = package.application_ref().into();
            packages.push(package);
        }
        let mut registry = widgets::registry();
        registry
            .register("test.history", move |_, _| {
                Ok(BTreeMap::from([(
                    "History".into(),
                    NativeExport {
                        contract: contract.clone(),
                        value: NativeCapability::Service(Arc::new(History)),
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
        let app = timer
            .block_on(Application::start_from_store(
                store.clone(),
                original,
                PresentationTarget::Desktop,
                timer.handle().clone(),
                maka_ui_js::execution(Default::default()),
                registry,
                vec![],
            ))
            .unwrap();
        let scene = Scene::from_application(cx, app);
        scene.draw(cx);
        cx.update_window(scene.window, |_, window, cx| window.simulate_next_frame(cx))
            .unwrap();
        let reading = scene.editor.clone();
        let document = scene
            .app
            .borrow()
            .child_mount(&reading, "document")
            .unwrap()
            .clone();
        let reading_owner = scene.resources.borrow().owners[&reading.id().to_string()]
            .identity
            .clone();
        let document_owner = scene.resources.borrow().owners[&document.id().to_string()]
            .identity
            .clone();
        let initial_revision = scene.app.borrow().frame(&scene.root).unwrap().state["observation"]
            ["readingRevision"]
            .as_u64()
            .unwrap();
        cx.update_window(scene.window, |_, window, cx| {
            if select {
                window.dispatch_event(
                    PlatformInput::MouseDown(gpui_kit::MouseDownEvent {
                        button: MouseButton::Left,
                        position: point(px(30.), px(20.)),
                        modifiers: Modifiers::default(),
                        click_count: 1,
                        first_mouse: false,
                    }),
                    cx,
                );
                window.dispatch_event(
                    PlatformInput::MouseUp(gpui_kit::MouseUpEvent {
                        button: MouseButton::Left,
                        position: point(px(30.), px(12.)),
                        modifiers: Modifiers::default(),
                        click_count: 1,
                    }),
                    cx,
                );
                window.focused(cx).unwrap().dispatch_action(
                    &gpui_kit::base::input::SelectAll,
                    window,
                    cx,
                );
                assert_eq!(
                    gpui_kit::base::TextSelection::selected_text(window, cx),
                    "Original selection 中文\n"
                );
            } else {
                window.dispatch_event(
                    PlatformInput::ScrollWheel(ScrollWheelEvent {
                        position: point(px(30.), px(40.)),
                        delta: ScrollDelta::Pixels(point(px(0.), px(-50.))),
                        modifiers: Modifiers::default(),
                        touch_phase: TouchPhase::Moved,
                    }),
                    cx,
                );
            }
        })
        .unwrap();
        scene.draw(cx);
        cx.update_window(scene.window, |_, window, cx| window.simulate_next_frame(cx))
            .unwrap();
        scene.draw(cx);
        let before = scene.app.borrow().frame(&scene.root).unwrap().state["observation"].clone();
        if !select {
            assert_eq!(before["viewport"]["atEnd"], false);
            assert!(before["readingRevision"].as_u64().unwrap() > initial_revision);
        }
        let mut extra = input.instances["history"].clone();
        extra.key = uuid::Uuid::new_v4();
        input.instances.insert("unused-history".into(), extra);
        let next = timer
            .block_on(store.commit_application(deployment, 1, input, vec![], packages))
            .unwrap();
        let load = cx.update(|cx| {
            scene
                .surface
                .read(cx)
                .load_committed_update(next.clone())
                .unwrap()
        });
        let update = timer.block_on(load).unwrap();
        cx.update_window(scene.window, |_, window, cx| {
            scene.surface.update(cx, |surface, cx| {
                surface.adopt_update(update, vec![], window, cx).unwrap()
            });
        })
        .unwrap();
        scene.draw(cx);
        cx.update_window(scene.window, |_, window, cx| window.simulate_next_frame(cx))
            .unwrap();
        scene.draw(cx);
        assert_eq!(scene.app.borrow().deployment_status().effective, Some(next));
        assert_eq!(
            scene
                .app
                .borrow()
                .child_mount(&reading, "document")
                .unwrap(),
            &document
        );
        assert!(Rc::ptr_eq(
            &reading_owner,
            &scene.resources.borrow().owners[&reading.id().to_string()].identity
        ));
        assert!(Rc::ptr_eq(
            &document_owner,
            &scene.resources.borrow().owners[&document.id().to_string()].identity
        ));
        assert_eq!(
            scene.app.borrow().frame(&scene.root).unwrap().state["observation"],
            before
        );
        if select {
            cx.update_window(scene.window, |_, window, cx| {
                assert_eq!(
                    gpui_kit::base::TextSelection::selected_text(window, cx),
                    "Original selection 中文\n"
                );
                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()));
                window.focused(cx).unwrap().dispatch_action(
                    &gpui_kit::base::input::Copy,
                    window,
                    cx,
                );
                assert_eq!(
                    cx.read_from_clipboard().unwrap().text().unwrap(),
                    "Original selection 中文"
                );
            })
            .unwrap();
        }
        drop(reading_owner);
        drop(document_owner);
        scene.close(cx);
        scene.app.borrow_mut().shutdown().unwrap();
        drop(scene);
        timer
            .block_on(Arc::try_unwrap(store).ok().unwrap().close())
            .unwrap();
    }
}
