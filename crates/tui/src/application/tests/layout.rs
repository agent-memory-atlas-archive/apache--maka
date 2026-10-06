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
use std::{any::Any, cell::Cell, collections::BTreeMap, rc::Rc};

enum View {
    Scene(Value),
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
            View::Flush => NativeView::<dyn Widget>::new(
                Rc::new(Flush),
                BTreeMap::from([("flushed".into(), json!({"type":"integer","minimum":0}))]),
                vec![],
            )?,
        };
        Ok(Frame {
            revision,
            state: json!({"revision":revision,"count":count}),
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
    fn create(&self) -> Box<dyn Any> {
        Box::new(None::<u16>)
    }
    fn measure(&self, _: &Slots<'_>, width: u16) -> maka_ui::Result<(u16, u16)> {
        Ok((width, 1))
    }
    fn paint(&self, slots: &mut Slots<'_>, area: Rect, _: &mut Buffer) -> maka_ui::Result<()> {
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
fn native(view: View, accept: Rc<Cell<bool>>) -> (tokio::runtime::Runtime, Presenter) {
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
    let last = presenter.scene.last().unwrap().area;
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
    presenter.surface_closed(Ok(())).unwrap();
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
    presenter.surface_closed(Ok(())).unwrap();
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
    presenter.surface_closed(Ok(())).unwrap();
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
        presenter.surface_closed(Ok(())).unwrap();
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
    presenter.surface_closed(Ok(())).unwrap();
    presenter.shutdown().unwrap();
}
