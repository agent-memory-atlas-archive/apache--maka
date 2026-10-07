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
use maka_plugin_core::package::{EntryTarget, Implementation};
use maka_ui::{
    ComponentCall, ComponentOutput, Frame, NativeCapability, NativeComponent, NativeExport,
    NativeView,
};
use ratatui::style::{Color, Modifier};
use std::{any::Any, cell::Cell, collections::BTreeMap, rc::Rc};

pub(super) enum View {
    Scene(Value),
    ChangingScene(Rc<std::cell::RefCell<Value>>),
    StandaloneSource(Rc<std::cell::RefCell<Value>>),
    Reading(Rc<std::cell::RefCell<Value>>),
    ReadingSlot(Rc<std::cell::RefCell<Value>>),
    Flush,
}
struct Root {
    view: View,
    accept: Rc<Cell<bool>>,
}
impl NativeComponent<dyn Widget> for Root {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let observation = match &call {
            ComponentCall::Event { key, input, .. }
                if matches!(*key, "__reading_observation" | "reading-observed") =>
            {
                (*input).clone()
            }
            ComponentCall::Update { state, .. } | ComponentCall::Event { state, .. } => {
                state["observation"].clone()
            }
            _ => Value::Null,
        };
        let (revision, count) = match call {
            ComponentCall::Mount { .. } => (0, 0),
            ComponentCall::Update { state, .. } => (
                state["revision"].as_u64().unwrap() + 1,
                state["count"].as_u64().unwrap(),
            ),
            ComponentCall::Event {
                state, key, input, ..
            } => {
                if !self.accept.get() {
                    return Err(Error::Invalid("injected feedback rejection".into()));
                }
                let count = if matches!(self.view, View::Flush) {
                    assert_eq!(key, "flushed");
                    input.as_u64().unwrap()
                } else {
                    state["count"].as_u64().unwrap() + 1
                };
                (state["revision"].as_u64().unwrap() + 1, count)
            }
        };
        let view = match &self.view {
            View::Scene(source) => super::super::scene::compile(
                source,
                std::time::Instant::now() + std::time::Duration::from_secs(1),
            )?,
            View::ChangingScene(source) => super::super::scene::compile(
                &source.borrow(),
                std::time::Instant::now() + std::time::Duration::from_secs(1),
            )?,
            View::StandaloneSource(source) => {
                let view = super::super::source::compile(&source.borrow())?;
                NativeView::<dyn Widget>::new(
                    view.view,
                    BTreeMap::from([("back".into(), json!(true))]),
                    vec![],
                )?
                .with_declaration(source.borrow().clone())?
            }
            View::ReadingSlot(props) => super::super::scene::compile(
                &json!({
                    "kind":"stack","key":"container","direction":"column","gap":0,"padding":0,"background":0,"children":[
                        {"kind":"component","key":"reader","binding":"reading","member":0,"props":props.borrow().clone(),"on":{"observed":"reading-observed"}},
                        {"kind":"button","key":"back","label":"Back"}
                    ]
                }),
                std::time::Instant::now() + std::time::Duration::from_secs(1),
            )?,
            View::Reading(props) => {
                super::super::reading::Component
                    .call(
                        ComponentCall::Mount {
                            props: &props.borrow(),
                        },
                        &Default::default(),
                    )?
                    .frame
                    .view
            }
            View::Flush => NativeView::<dyn Widget>::new(
                Rc::new(Flush),
                BTreeMap::from([("flushed".into(), json!({"type":"integer","minimum":0}))]),
                vec![],
            )?,
        };
        Ok(Frame {
            revision,
            state: json!({"revision":revision,"count":count,"observation":observation}),
            surface: Some(Surface::Terminal {
                binding: "surface".into(),
            }),
            view,
        }
        .output(vec![]))
    }
}
struct Flush;
impl Widget for Flush {
    fn create(&self, _: NativeContext) -> Box<dyn Any> {
        Box::new(None::<u16>)
    }
    fn measure(&self, _: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u32)> {
        Ok((width, 1))
    }
    fn paint(&self, slots: &mut Slots<'_>, area: Placement, _: &mut Buffer) -> maka_ui::Result<()> {
        slots.native_geometry(Box::new(area.width))
    }
    fn present(&self, state: &mut dyn Any, geometry: Box<dyn Any>) -> Option<NativeInput> {
        let seen = state.downcast_mut::<Option<u16>>().unwrap();
        let width = *geometry.downcast::<u16>().unwrap();
        if *seen == Some(width) {
            return None;
        }
        *seen = Some(width);
        Some(NativeInput {
            key: "flushed".into(),
            value: json!(width),
        })
    }
}
pub(super) fn native(view: View, accept: Rc<Cell<bool>>) -> (tokio::runtime::Runtime, Presenter) {
    let (runtime, mut input, mut packages) = fixture("return frame(state);");
    let index = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let original = packages.remove(index);
    let mut declaration = original.manifest().application.clone().unwrap();
    let entry = declaration.entries.get_mut(&EntryTarget::Tui).unwrap();
    entry.scene = None;
    entry.implementation = Implementation::Native {
        implementation: "test.layout".into(),
    };
    if matches!(view, View::Reading(_) | View::ReadingSlot(_)) {
        use maka_plugin_core::{
            composition::Target,
            contract::{Availability, Cardinality, Export, Import},
            package::{Declaration, EntryDeclaration},
        };
        let source_contract = maka_ui::source::source_contract();
        entry.imports.insert(
            "source".into(),
            Import {
                contract: source_contract.clone(),
                cardinality: Cardinality::One,
            },
        );
        let source = support::package(
            "test.source",
            Declaration {
                entries: BTreeMap::from([(
                    EntryTarget::Tui,
                    EntryDeclaration {
                        scene: None,
                        implementation: Implementation::Native {
                            implementation: "maka.ui.source".into(),
                        },
                        configuration: json!({"type":"object"}),
                        imports: BTreeMap::new(),
                        exports: BTreeMap::from([(
                            "SourceText".into(),
                            Export {
                                contract: source_contract,
                                availability: Availability::Required,
                            },
                        )]),
                    },
                )]),
                composition: None,
            },
            None,
        );
        input.instances.get_mut("app").unwrap().bindings.insert(
            "source".into(),
            vec![Target::Instance {
                path: vec!["source".into()],
                export: "SourceText".into(),
            }],
        );
        input
            .instances
            .insert("source".into(), support::node(&source, 150));
        packages.push(source);
        if matches!(view, View::ReadingSlot(_)) {
            let contract = maka_ui::reading::reading_contract();
            entry.imports.insert(
                "reading".into(),
                Import {
                    contract: contract.clone(),
                    cardinality: Cardinality::One,
                },
            );
            let reading = support::package(
                "test.reading",
                Declaration {
                    entries: BTreeMap::from([(
                        EntryTarget::Tui,
                        EntryDeclaration {
                            scene: None,
                            implementation: Implementation::Native {
                                implementation: "maka.ui.reading".into(),
                            },
                            configuration: json!({"type":"object"}),
                            imports: BTreeMap::from([(
                                "source".into(),
                                Import {
                                    contract: maka_ui::source::source_contract(),
                                    cardinality: Cardinality::One,
                                },
                            )]),
                            exports: BTreeMap::from([(
                                "ReadingArea".into(),
                                Export {
                                    contract,
                                    availability: Availability::Required,
                                },
                            )]),
                        },
                    )]),
                    composition: None,
                },
                None,
            );
            let mut node = support::node(&reading, 151);
            node.bindings.insert(
                "source".into(),
                vec![Target::Instance {
                    path: vec!["source".into()],
                    export: "SourceText".into(),
                }],
            );
            input.instances.insert("reading".into(), node);
            input.instances.get_mut("app").unwrap().bindings.insert(
                "reading".into(),
                vec![Target::Instance {
                    path: vec!["reading".into()],
                    export: "ReadingArea".into(),
                }],
            );
            packages.push(reading);
        }
    }
    let contract = entry.exports["App"].contract.clone();
    let package = support::package(
        "dynamic.app",
        declaration,
        Some(
            std::str::from_utf8(original.file("index.mjs").unwrap())
                .unwrap()
                .to_owned(),
        ),
    );
    input.instances.get_mut("app").unwrap().package = package.application_ref().into();
    packages.push(package);
    let root = Rc::new(Root { view, accept });
    let mut registry = registry();
    registry
        .register("test.layout", move |_, _| {
            Ok(BTreeMap::from([(
                "App".into(),
                NativeExport {
                    contract: contract.clone(),
                    value: NativeCapability::Component(root.clone()),
                },
            )]))
        })
        .unwrap();
    let presenter = Presenter::new(
        &input,
        packages,
        runtime.handle().clone(),
        registry,
        json!({}),
    )
    .unwrap();
    (runtime, presenter)
}
fn scene(wrap: bool) -> Value {
    json!({"kind":"stack","key":"root","direction":"column","gap":1,"padding":0,"background":0,"children":[
        {"kind":"text","key":"heading","text":"Message source","color":0xffffff},
        {"kind":"stack","key":"body","direction":"column","gap":0,"padding":0,"background":0,"layout":{"grow":1},"children":[
            {"kind":"text","key":"source","text":"中文😀 source\n".repeat(2000),"color":0xffffff}]},
        {"kind":"stack","key":"controls","direction":"row","gap":1,"padding":0,"background":0,"layout":{"wrap":wrap},"children":[
            {"kind":"button","key":"back","label":"Back"},{"kind":"button","key":"next","label":"Next"},{"kind":"button","key":"last","label":"End"}]}
    ]})
}
#[test]
fn actual_long_scene_keeps_footer_painted_and_native_click_bound_to_flush() {
    let (_runtime, mut presenter) = native(View::Scene(scene(false)), Rc::new(Cell::new(true)));
    let area = Rect::new(0, 0, 30, 10);
    let mut buffer = Buffer::empty(area);
    let pending = presenter.paint(area, &mut buffer).unwrap();
    assert!(presenter.scene.is_empty());
    assert_eq!(buffer[(0, 9)].symbol(), "[");
    presenter.present(pending);
    assert_eq!(
        presenter
            .scene
            .iter()
            .map(|target| target.key.as_str())
            .collect::<Vec<_>>(),
        ["back", "next", "last"]
    );
    let last = presenter.scene.last().unwrap().area.visible();
    assert_eq!(last.y, 9);
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: last.x,
            row: last.y,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
    assert_eq!(
        presenter.application.frame(&presenter.root).unwrap().state["count"],
        1
    );
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    presenter.shutdown().unwrap();
}
#[test]
fn narrow_wrapped_footer_is_measured_before_the_body_viewport() {
    let (_runtime, mut presenter) = native(View::Scene(scene(true)), Rc::new(Cell::new(true)));
    draw(&mut presenter, 17, 10);
    assert_eq!(
        presenter
            .scene
            .iter()
            .map(|target| (target.key.as_str(), target.area.y))
            .collect::<Vec<_>>(),
        [("back", 7), ("next", 7), ("last", 9)]
    );
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    presenter.shutdown().unwrap();
}
#[test]
fn post_flush_native_observation_coalesces_and_retries_without_guest_paint() {
    let accept = Rc::new(Cell::new(false));
    let (_runtime, mut presenter) = native(View::Flush, accept.clone());
    let original = presenter.resources.owners[&presenter.root.id()]
        .identity
        .clone();
    let area = Rect::new(0, 0, 10, 3);
    let mut buffer = Buffer::empty(area);
    let _discarded = presenter.paint(area, &mut buffer).unwrap();
    presenter.sync_native().unwrap();
    assert_eq!(
        presenter.application.frame(&presenter.root).unwrap().state["count"],
        0
    );
    draw(&mut presenter, 10, 3);
    presenter.sync_native().unwrap(); // Rejection retains the original native pending observation.
    assert_eq!(
        presenter.application.frame(&presenter.root).unwrap().state["count"],
        0
    );
    draw(&mut presenter, 20, 3); // A newer successful flush replaces that one pending observation.
    accept.set(true);
    presenter.sync_native().unwrap();
    assert_eq!(
        presenter.application.frame(&presenter.root).unwrap().state["count"],
        20
    );
    assert_eq!(
        presenter.application.frame(&presenter.root).unwrap().state["revision"],
        1
    );
    draw(&mut presenter, 20, 3);
    presenter.sync_native().unwrap();
    assert_eq!(
        presenter.application.frame(&presenter.root).unwrap().state["revision"],
        1
    );
    assert!(Rc::ptr_eq(
        &original,
        &presenter.resources.owners[&presenter.root.id()].identity
    ));
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    presenter.shutdown().unwrap();
}

#[test]
fn allocated_row_width_remeasures_wrapped_body_before_footer_placement() {
    for (wrap, width, footer_y) in [(false, 100, 6), (true, 15, 8)] {
        let source = json!({"kind":"stack","key":"root","direction":"column","gap":1,"padding":0,"background":0,"children":[
          {"kind":"stack","key":"row","direction":"row","gap":1,"padding":0,"background":0,"layout":{"wrap":wrap},"children":[
            {"kind":"stack","key":"narrow","direction":"column","gap":0,"padding":0,"background":0,"layout":{"basis":10},"children":[
              {"kind":"text","key":"body","text":"a".repeat(50),"color":0xffffff}]},
            {"kind":"button","key":"after","label":"After"}]},
          {"kind":"button","key":"footer","label":"Footer"}]});
        let (_runtime, mut presenter) = native(View::Scene(source), Rc::new(Cell::new(true)));
        let buffer = draw(&mut presenter, width, 12);
        for y in 0..5 {
            assert_eq!(buffer[(0, y)].symbol(), "a");
            assert_eq!(buffer[(9, y)].symbol(), "a");
        }
        assert_eq!(
            presenter
                .scene
                .iter()
                .find(|target| target.key == "footer")
                .unwrap()
                .area
                .y,
            footer_y
        );
        presenter
            .surface_closed(&presenter.surface(), Ok(()))
            .unwrap();
        presenter.shutdown().unwrap();
    }
}

#[test]
fn deep_legal_rows_share_one_measurement_pass_instead_of_expanding_recursively() {
    let mut source = json!({"kind":"text","key":"leaf","text":"word ".repeat(40),"color":0xffffff});
    for depth in 1..=31 {
        source = json!({"kind":"stack","key":format!("row-{depth}"),"direction":"row","gap":0,"padding":0,"background":0,"layout":{"basis":depth},"children":[source]});
    }
    let (_runtime, mut presenter) = native(View::Scene(source), Rc::new(Cell::new(true)));
    let frame = presenter.application.frame(&presenter.root).unwrap();
    let node = (frame.view.view.as_ref() as &dyn Any)
        .downcast_ref::<super::super::scene::Node>()
        .unwrap();
    let measurements = super::super::paint::Measurements::guarded(4096);
    presenter
        .measure(&presenter.root, node, 80, &measurements)
        .unwrap();
    assert!(measurements.requests() <= 32 * 32 * 3);
    // Real painting also uses one ephemeral pass; no cache enters accepted resources.
    let before = presenter
        .application
        .frame(&presenter.root)
        .unwrap()
        .state
        .clone();
    draw(&mut presenter, 80, 24);
    assert_eq!(
        presenter.application.frame(&presenter.root).unwrap().state,
        before
    );
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    presenter.shutdown().unwrap();
}

fn scroll_scene(policy: &str, text: &str) -> Value {
    json!({"kind":"stack","key":"root","direction":"column","gap":0,"padding":0,"background":0,"children":[
        {"kind":"stack","key":"body","direction":"column","gap":0,"padding":0,"background":0,"layout":{"grow":1},"scroll":policy,"children":[
            {"kind":"text","key":"source","text":text,"color":0xffffff}]},
        {"kind":"button","key":"back","label":"Back"}]})
}
fn flushed(presenter: &mut Presenter, width: u16, height: u16) -> Buffer {
    presenter.sync_native().unwrap();
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    let scene = presenter.paint(area, &mut buffer).unwrap();
    presenter.present(scene);
    buffer
}
fn wheel(presenter: &mut Presenter, down: bool, column: u16, row: u16) {
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: if down {
                MouseEventKind::ScrollDown
            } else {
                MouseEventKind::ScrollUp
            },
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
}
fn scroll_state(presenter: &Presenter, key: &str) -> (u32, u32) {
    let state = presenter.resources.owners[&presenter.root.id()]
        .state
        .downcast_ref::<super::super::scroll::SceneState>()
        .unwrap();
    let scroll = &state.scrolls[key];
    (scroll.offset, scroll.presented.as_ref().unwrap().offset)
}
#[test]
fn native_scroll_has_real_clipped_cells_and_does_not_ack_unflushed_or_superseded_intent() {
    let text = (0..30).map(|n| format!("row {n:02}\n")).collect::<String>();
    let (_runtime, mut presenter) = native(
        View::Scene(scroll_scene("manual", &text)),
        Rc::new(Cell::new(true)),
    );
    let first = flushed(&mut presenter, 12, 6);
    assert_eq!(first[(4, 0)].symbol(), "0");
    wheel(&mut presenter, true, 1, 1);
    assert_eq!(scroll_state(&presenter, "body"), (3, 0));
    let area = Rect::new(0, 0, 12, 6);
    let mut discarded = Buffer::empty(area);
    let stale = presenter.paint(area, &mut discarded).unwrap();
    assert_eq!(discarded[(5, 0)].symbol(), "3");
    assert_eq!(discarded[(0, 5)].symbol(), "[");
    wheel(&mut presenter, true, 1, 1);
    presenter.present(stale);
    assert_eq!(scroll_state(&presenter, "body"), (6, 0));
    let next = flushed(&mut presenter, 12, 6);
    assert_eq!(next[(5, 0)].symbol(), "6");
    assert_eq!(scroll_state(&presenter, "body"), (6, 6));
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    presenter.shutdown().unwrap();
}
#[test]
fn wheel_and_page_keys_bubble_from_the_original_button_target() {
    let mut source = scroll_scene("manual", "");
    source["children"][0]["children"] = json!(
        (0..20)
            .map(|n| json!({"kind":"button","key":format!("b{n}"),"label":format!("Item {n}")}))
            .collect::<Vec<_>>()
    );
    let (_runtime, mut presenter) = native(View::Scene(source), Rc::new(Cell::new(true)));
    flushed(&mut presenter, 20, 7);
    wheel(&mut presenter, true, 2, 0);
    assert_eq!(scroll_state(&presenter, "body").0, 3);
    flushed(&mut presenter, 20, 7);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::PageDown,
            KeyModifiers::NONE,
        )))
        .unwrap();
    assert_eq!(scroll_state(&presenter, "body").0, 9);
    assert_eq!(
        presenter.application.frame(&presenter.root).unwrap().state["count"],
        0
    );
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    presenter.shutdown().unwrap();
}
#[test]
fn follow_end_uses_actual_flush_and_manual_interaction_survives_recipe_updates() {
    let source = Rc::new(std::cell::RefCell::new(scroll_scene(
        "follow_end",
        "0\n1\n2\n3\n4\n5\n6\n7",
    )));
    let (_runtime, mut presenter) = native(
        View::ChangingScene(source.clone()),
        Rc::new(Cell::new(true)),
    );
    flushed(&mut presenter, 10, 5);
    assert_eq!(scroll_state(&presenter, "body"), (4, 4));
    let owner = presenter.resources.owners[&presenter.root.id()]
        .identity
        .clone();
    source.borrow_mut()["children"][0]["children"][0]["text"] =
        json!("0\n1\n2\n3\n4\n5\n6\n7\n8\n9");
    refresh(&mut presenter);
    flushed(&mut presenter, 10, 5);
    assert_eq!(scroll_state(&presenter, "body"), (6, 6));
    wheel(&mut presenter, false, 1, 1);
    source.borrow_mut()["children"][0]["children"][0]["text"] =
        json!("0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11");
    refresh(&mut presenter);
    flushed(&mut presenter, 10, 5);
    assert_eq!(scroll_state(&presenter, "body"), (3, 3));
    assert!(Rc::ptr_eq(
        &owner,
        &presenter.resources.owners[&presenter.root.id()].identity
    ));
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    presenter.shutdown().unwrap();
}
#[test]
fn anchor_keeps_gap_offset_and_original_child_after_hide_and_prepend() {
    let mut initial = scroll_scene("anchor", "A");
    initial["children"][0]["gap"] = json!(10);
    initial["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"text","key":"second","text":"B\nC\nD\nE\nF\nG", "color":0xffffff}));
    let source = Rc::new(std::cell::RefCell::new(initial));
    let (_runtime, mut presenter) = native(
        View::ChangingScene(source.clone()),
        Rc::new(Cell::new(true)),
    );
    flushed(&mut presenter, 10, 5);
    wheel(&mut presenter, true, 1, 1);
    flushed(&mut presenter, 10, 5);
    flushed(&mut presenter, 10, 5);
    assert_eq!(scroll_state(&presenter, "body"), (3, 3));
    source.borrow_mut()["children"][0]["visible"] = json!(false);
    refresh(&mut presenter);
    flushed(&mut presenter, 10, 5);
    assert_eq!(scroll_state(&presenter, "body"), (3, 3));
    source.borrow_mut()["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .insert(
            0,
            json!({"kind":"text","key":"prefix","text":"prefix\n".repeat(100),"color":0xffffff}),
        );
    source.borrow_mut()["children"][0]["visible"] = json!(true);
    refresh(&mut presenter);
    flushed(&mut presenter, 10, 5);
    assert_eq!(scroll_state(&presenter, "body"), (114, 114));
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    presenter.shutdown().unwrap();
}

fn refresh(presenter: &mut Presenter) {
    let revision = presenter
        .application
        .frame(&presenter.root)
        .unwrap()
        .revision;
    presenter
        .application
        .dispatch(&presenter.root, revision, "back", json!({"kind":"click"}))
        .unwrap();
    presenter.sync_native().unwrap();
}

#[test]
fn disabled_buttons_retain_layout_but_revoke_click_dispatch_and_tab_targets() {
    let source = Rc::new(std::cell::RefCell::new(json!({
        "kind":"stack","key":"root","direction":"column","gap":0,"padding":0,"background":0,
        "children":[
            {"kind":"button","key":"save","label":"Save","color":0x416653},
            {"kind":"button","key":"back","label":"Back"}
        ]
    })));
    let (_runtime, mut presenter) = native(
        View::ChangingScene(source.clone()),
        Rc::new(Cell::new(true)),
    );
    let first = flushed(&mut presenter, 20, 3);
    assert_eq!(first[(0, 0)].fg, Color::Rgb(0x41, 0x66, 0x53));
    assert_eq!(first[(0, 1)].fg, Color::Reset);
    let owner = presenter.resources.owners[&presenter.root.id()]
        .identity
        .clone();
    assert_eq!(presenter.focus.as_ref().unwrap().key, "save");
    source.borrow_mut()["children"][0]["disabled"] = json!(true);
    refresh(&mut presenter);
    let count = presenter.application.frame(&presenter.root).unwrap().state["count"].clone();
    let revision = presenter
        .application
        .frame(&presenter.root)
        .unwrap()
        .revision;
    assert!(
        presenter
            .application
            .dispatch(&presenter.root, revision, "save", json!({"kind":"click"}))
            .is_err()
    );
    assert!(
        presenter.input(key(KeyCode::Enter)).is_err(),
        "old flushed target cannot invoke a newly disabled action"
    );
    let disabled = flushed(&mut presenter, 20, 3);
    for y in 0..3 {
        for x in 0..20 {
            assert_eq!(
                first[(x, y)].symbol(),
                disabled[(x, y)].symbol(),
                "disabled retains button geometry"
            );
        }
    }
    assert!(
        !disabled[(0, 0)].modifier.contains(Modifier::DIM),
        "explicit theme color is not dimmed again"
    );
    assert_eq!(presenter.scene.len(), 1);
    assert_eq!(presenter.focus.as_ref().unwrap().key, "back");
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
    presenter.input(key(KeyCode::Tab)).unwrap();
    assert_eq!(presenter.focus.as_ref().unwrap().key, "back");
    assert_eq!(
        presenter.application.frame(&presenter.root).unwrap().state["count"],
        count
    );
    source.borrow_mut()["children"][0]["disabled"] = json!(false);
    refresh(&mut presenter);
    flushed(&mut presenter, 20, 3);
    assert!(Rc::ptr_eq(
        &owner,
        &presenter.resources.owners[&presenter.root.id()].identity
    ));
    presenter.input(key(KeyCode::Tab)).unwrap();
    assert_eq!(presenter.focus.as_ref().unwrap().key, "save");
    let before = presenter.application.frame(&presenter.root).unwrap().state["count"]
        .as_u64()
        .unwrap();
    presenter.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(
        presenter.application.frame(&presenter.root).unwrap().state["count"],
        before + 1
    );
    source.borrow_mut()["children"][1]["disabled"] = json!(true);
    refresh(&mut presenter);
    let default_disabled = flushed(&mut presenter, 20, 3);
    assert_eq!(default_disabled[(0, 1)].fg, Color::Reset);
    assert!(default_disabled[(0, 1)].modifier.contains(Modifier::DIM));
    assert_eq!(presenter.scene.len(), 1);
    assert_eq!(presenter.scene[0].key, "save");
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    presenter.shutdown().unwrap();
}

#[test]
fn native_scroll_reaches_content_beyond_u16_terminal_height() {
    let mut source = scroll_scene("manual", "");
    source["children"][0]["children"] = json!((0..4).map(|n| json!({"kind":"text", "key":format!("part{n}"), "text":"p\n".repeat(20000), "color":0xffffff})).collect::<Vec<_>>());
    let (_runtime, mut presenter) = native(View::Scene(source), Rc::new(Cell::new(true)));
    flushed(&mut presenter, 10, 6);
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)))
        .unwrap();
    let end = flushed(&mut presenter, 10, 6);
    assert_eq!(scroll_state(&presenter, "body"), (79999, 79999));
    assert_eq!(end[(0, 0)].symbol(), "p");
    assert_eq!(end[(0, 5)].symbol(), "[");
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    presenter.shutdown().unwrap();
}
#[test]
fn nested_scroll_consumes_innermost_wheel_then_bubbles_at_its_boundary() {
    let mut source = scroll_scene("manual", "");
    source["children"][0]["children"] = json!([
        {"kind":"stack","key":"inner","direction":"column","gap":0,"padding":0,"background":0,"scroll":"manual","layout":{"basis":3},"children":[
            {"kind":"text","key":"inner-text","text":"0\n1\n2\n3\n4\n5\n6\n7\n8\n9","color":0xffffff}]},
        {"kind":"text","key":"outer-text","text":"tail\n".repeat(30),"color":0xffffff}]);
    let (_runtime, mut presenter) = native(View::Scene(source), Rc::new(Cell::new(true)));
    flushed(&mut presenter, 10, 6);
    for _ in 0..3 {
        wheel(&mut presenter, true, 1, 1);
        flushed(&mut presenter, 10, 6);
    }
    assert_eq!(scroll_state(&presenter, "inner"), (7, 7));
    assert_eq!(scroll_state(&presenter, "body"), (0, 0));
    wheel(&mut presenter, true, 1, 1);
    flushed(&mut presenter, 10, 6);
    assert_eq!(scroll_state(&presenter, "body"), (3, 3));
    assert_eq!(scroll_state(&presenter, "inner"), (7, 7));
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    presenter.shutdown().unwrap();
}
