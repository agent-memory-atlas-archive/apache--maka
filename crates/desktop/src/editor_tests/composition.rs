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
use gpui_kit::{
    Element, ElementId, GlobalElementId, InspectorElementId, LayoutId, Pixels, Window, uniform_list,
};
use maka_plugin_core::package::{EntryTarget, Implementation};
use maka_ui::{
    ComponentCall, ComponentOutput, ComponentSlot, Frame, NativeCapability, NativeComponent,
    NativeExport, NativeView,
};
use serde_json::{Value, json};
use std::{any::Any, cell::Cell};

struct Layout {
    list: bool,
    retry: bool,
    duplicate: bool,
}
impl widgets::Widget for Layout {
    fn create(&self, _: widgets::LiveInput, _: &mut Window, _: &mut App) -> Box<dyn Any> {
        Box::new(Rc::new(Cell::new(0usize)))
    }
    fn render(
        &self,
        state: &dyn Any,
        slots: &widgets::Slots,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let count = state.downcast_ref::<Rc<Cell<usize>>>().unwrap().clone();
        let keys = slots.keys().to_vec();
        if self.list {
            let slots = slots.clone();
            return uniform_list("plugin-children", keys.len(), move |range, window, cx| {
                range
                    .map(|i| {
                        count.set(count.get() + 1);
                        div()
                            .h(px(128.))
                            .child(slots.render(&keys[i], window, cx).unwrap())
                            .into_any_element()
                    })
                    .collect::<Vec<_>>()
            })
            .size_full()
            .into_any_element();
        }
        let mut content = Vec::new();
        for key in &keys {
            content.push(slots.render(key, window, cx).unwrap());
        }
        if self.duplicate {
            content.push(slots.render(&keys[0], window, cx).unwrap());
        }
        let content = div().flex().flex_col().children(content).into_any_element();
        if self.retry {
            Retry {
                attempt: div()
                    .flex()
                    .flex_col()
                    .children(
                        keys.iter()
                            .map(|key| slots.render(key, window, cx).unwrap()),
                    )
                    .into_any_element(),
                content,
            }
            .into_any_element()
        } else {
            content
        }
    }
}
// Actual GPUI autoscroll-style retry: child prepaint is rolled back, then the
// same measured elements are prepainted at their final geometry.
struct Retry {
    attempt: AnyElement,
    content: AnyElement,
}
impl IntoElement for Retry {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Retry {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let attempt = self.attempt.request_layout(window, cx);
        let final_layout = self.content.request_layout(window, cx);
        (
            window.request_layout(gpui_kit::Style::default(), [attempt, final_layout], cx),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let attempt: Result<(), ()> = window.transact(|window| {
            self.attempt.prepaint(window, cx);
            Err(())
        });
        assert!(attempt.is_err());
        self.content.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.content.paint(window, cx);
    }
}
struct Parent {
    list: bool,
    retry: bool,
    duplicate: bool,
}
impl NativeComponent<dyn widgets::Widget> for Parent {
    fn call(
        &self,
        call: ComponentCall<'_>,
        values: &maka_ui::abi::ValueBindings,
    ) -> maka_ui::Result<ComponentOutput<Presentation<dyn widgets::Widget>>> {
        let mut state = match call {
            ComponentCall::Mount { .. } => {
                json!({"revision":0,"version":0,"show":true,"native":true})
            }
            ComponentCall::Update { state, .. } | ComponentCall::Event { state, .. } => {
                state.clone()
            }
        };
        if let ComponentCall::Event { key, .. } = call {
            state["revision"] = json!(state["revision"].as_u64().unwrap() + 1);
            match key {
                "tree" => state["native"] = json!(!state["native"].as_bool().unwrap()),
                "hide" => state["show"] = json!(false),
                _ => state["version"] = json!(state["version"].as_u64().unwrap() + 1),
            }
        }
        let mut slots = vec![ComponentSlot {
            key: "right".into(),
            binding: "counter".into(),
            member: 0,
            props: json!({"label":format!("Right {}",state["version"]),"initial":20}),
            on: BTreeMap::new(),
        }];
        if state["show"] == true {
            slots.push(ComponentSlot {
                key: "editor".into(),
                binding: "editor".into(),
                member: 0,
                props: json!({"initialText":"Start","placeholder":"Write here","command":null,"appearance":values.get("brand").unwrap()[0]["textarea"],"accessibilityLabel":"Write a draft","readOnly":false,"disabled":false,"softWrap":true,"tabIndex":0}),
                on: BTreeMap::new(),
            });
        }
        slots.push(ComponentSlot {
            key: "left".into(),
            binding: "counter".into(),
            member: 0,
            props: json!({"label":format!("Left {}",state["version"]),"initial":10}),
            on: BTreeMap::new(),
        });
        let view = if state["native"] == true {
            Presentation::Native(NativeView::new(
                Rc::new(Layout {
                    list: self.list,
                    retry: self.retry,
                    duplicate: self.duplicate,
                }) as Rc<dyn widgets::Widget>,
                ["parent", "hide", "tree"]
                    .into_iter()
                    .map(|k| (k.into(), json!(true)))
                    .collect(),
                slots,
            )?)
        } else {
            Presentation::Tree(Node::Stack {
                key: "layout".into(),
                direction: Direction::Column,
                gap: 0,
                padding: 0,
                background: 0,
                children: slots
                    .into_iter()
                    .map(Node::Component)
                    .chain(std::iter::once(Node::Button {
                        key: "tree".into(),
                        label: "Native".into(),
                    }))
                    .collect(),
            })
        };
        Ok(Frame {
            revision: state["revision"].as_u64().unwrap(),
            state,
            surface: Some(SurfaceDeclaration::Window {
                binding: "surface".into(),
                title: "Native layout".into(),
                width: 640,
                height: 400,
            }),
            view,
        }
        .output(vec![]))
    }
}
fn scene(
    cx: &mut TestAppContext,
    timer: &tokio::runtime::Runtime,
    list: bool,
    retry: bool,
    duplicate: bool,
) -> Scene {
    let (mut input, mut packages) = support::editor_fixture();
    let i = packages
        .iter()
        .position(|p| p.manifest().id == "dynamic.app")
        .unwrap();
    let old = packages.remove(i);
    let mut declaration = old.manifest().application.clone().unwrap();
    let entry = declaration.entries.get_mut(&EntryTarget::Desktop).unwrap();
    let contract = entry.exports["App"].contract.clone();
    entry.implementation = Implementation::Native {
        implementation: "test.layout".into(),
    };
    let root = support::package("native.layout", declaration, None);
    input.instances.get_mut("app").unwrap().package = root.application_ref().into();
    packages.push(root);
    let mut native = widgets::registry();
    native
        .register("test.layout", move |_, _values| {
            Ok(BTreeMap::from([(
                "App".into(),
                NativeExport {
                    contract: contract.clone(),
                    value: NativeCapability::Component(Rc::new(Parent {
                        list,
                        retry,
                        duplicate,
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
        native,
        Default::default(),
    )
}
fn capture(scene: &Scene, cx: &mut TestAppContext) -> Rc<RenderScene> {
    cx.update_window(scene.window, |_, window, cx| {
        scene
            .surface
            .update(cx, |s, cx| s.scene(window, cx).unwrap())
    })
    .unwrap()
}
fn dispatch(scene: &Scene, cx: &mut TestAppContext, key: &str) {
    let revision = scene.app.borrow().frame(&scene.root).unwrap().revision;
    scene
        .app
        .borrow_mut()
        .dispatch(&scene.root, revision, key, Value::Null)
        .unwrap();
    scene.draw(cx);
}
#[gpui::test]
fn native_slots_virtual_measurement_repaint_preserve_editor_and_composing_state(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = scene(cx, &timer, true, false, false);
    scene.draw(cx);
    let snapshot = capture(&scene, cx);
    assert_eq!(snapshot.slots().keys(), ["right", "editor", "left"]);
    let calls = scene.resources.borrow().owners[&scene.root.id().to_string()]
        .state
        .downcast_ref::<Rc<Cell<usize>>>()
        .unwrap()
        .clone();
    assert!(
        calls.get() > 3,
        "actual uniform_list measures and then paints the same slot"
    );
    let owner = scene.owner();
    let entity = owner.entity_id();
    cx.update_window(scene.window, |_, window, cx| {
        owner.update(cx, |s, cx| {
            s.state
                .read(cx)
                .presentation()
                .focus_handle()
                .focus(window, cx);
            s.replace_and_mark_text_in_range(Some(0..5), "你好", Some(2..2), window, cx);
        })
    })
    .unwrap();
    cx.run_until_parked();
    dispatch(&scene, cx, "parent");
    assert_eq!(scene.owner().entity_id(), entity);
    cx.update_window(scene.window, |_, window, cx| {
        owner.update(cx, |s, cx| {
            assert!(s.marked_text_range(window, cx).is_some());
            assert_eq!(s.state.read(cx).text().to_string(), "你好");
        })
    })
    .unwrap();
    scene.draw(cx);
    scene.draw(cx);
    assert_eq!(scene.owner().entity_id(), entity);
    drop(snapshot);
    drop(owner);
    scene.close(cx);
}
#[gpui::test]
fn native_slots_old_owner_and_detached_children_never_retarget_after_same_mount_rebuild(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = scene(cx, &timer, false, false, false);
    scene.draw(cx);
    let snapshot = capture(&scene, cx);
    let old = snapshot.slots();
    drop(snapshot);
    dispatch(&scene, cx, "tree");
    dispatch(&scene, cx, "tree");
    cx.update_window(scene.window, |_, window, cx| {
        assert!(old.render("left", window, cx).is_err())
    })
    .unwrap();
    let current = capture(&scene, cx);
    let slots = current.slots();
    drop(current);
    dispatch(&scene, cx, "hide");
    assert!(scene.app.borrow().is_retiring(&scene.editor));
    cx.update_window(scene.window, |_, window, cx| {
        assert!(slots.render("editor", window, cx).is_err())
    })
    .unwrap();
    drop(old);
    drop(slots);
    scene.draw(cx);
    cx.run_until_parked();
    assert!(scene.app.borrow().frame(&scene.editor).is_err());
    scene.close(cx);
}
#[gpui::test]
fn retaining_one_components_slots_does_not_block_sibling_editor_cleanup(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = scene(cx, &timer, false, false, false);
    scene.draw(cx);
    let snapshot = capture(&scene, cx);
    let left = snapshot.entry.children["left"].slots();
    drop(snapshot);
    dispatch(&scene, cx, "hide");
    cx.run_until_parked();
    assert!(
        scene.app.borrow().frame(&scene.editor).is_err(),
        "unrelated sibling owns no editor lease"
    );
    assert!(left.keys().is_empty());
    drop(left);
    scene.close(cx);
}
#[gpui::test]
fn native_slots_prepaint_retry_rolls_back_claims_before_final_placement(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = scene(cx, &timer, false, true, false);
    scene.draw(cx);
    scene.draw(cx);
    dispatch(&scene, cx, "parent");
    scene.close(cx);
}
#[gpui::test]
#[should_panic(expected = "element placed more than once")]
fn native_slots_reject_two_actual_placements_of_one_mount(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = scene(cx, &timer, false, false, true);
    scene.draw(cx);
}

struct CachedChild {
    scene: Rc<RenderScene>,
    builds: Rc<Cell<usize>>,
}
impl Render for CachedChild {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.builds.set(self.builds.get() + 1);
        self.scene.render(window, cx).unwrap()
    }
}
struct CachedRoot {
    child: Entity<CachedChild>,
    duplicate: bool,
}
impl Render for CachedRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div().size_full().child(
            self.child
                .clone()
                .cached(gpui_kit::StyleRefinement::default().w(px(200.)).h(px(120.))),
        );
        if self.duplicate {
            root.child(
                self.child
                    .read(cx)
                    .scene
                    .clone()
                    .render(window, cx)
                    .unwrap(),
            )
        } else {
            root
        }
    }
}
fn cached(cx: &mut TestAppContext, scene: &Scene) -> (Entity<CachedRoot>, Rc<Cell<usize>>) {
    let snapshot = capture(scene, cx);
    let child = snapshot.entry.children["left"].clone();
    drop(snapshot);
    let builds = Rc::new(Cell::new(0));
    let root = cx
        .update_window(scene.window, |_, window, cx| {
            window.replace_root(cx, |_, cx| CachedRoot {
                child: cx.new(|_| CachedChild {
                    scene: child,
                    builds: builds.clone(),
                }),
                duplicate: false,
            })
        })
        .unwrap();
    (root, builds)
}
#[gpui::test]
fn cached_native_child_replays_placements_without_rebuilding_or_double_claiming(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = scene(cx, &timer, false, false, false);
    scene.draw(cx);
    let (root, builds) = cached(cx, &scene);
    for _ in 0..4 {
        cx.update_window(scene.window, |_, window, cx| {
            root.update(cx, |_, cx| cx.notify());
            window.draw(cx).clear(cx);
        })
        .unwrap();
    }
    assert_eq!(
        builds.get(),
        1,
        "actual retained subtree replayed across frames"
    );
    drop(root);
    scene.close(cx);
}
#[gpui::test]
#[should_panic(expected = "element placed more than once")]
fn cached_placement_cannot_be_reused_alongside_another_placement_of_same_mount(
    cx: &mut TestAppContext,
) {
    let timer = timer();
    let scene = scene(cx, &timer, false, false, false);
    scene.draw(cx);
    let (root, builds) = cached(cx, &scene);
    cx.update_window(scene.window, |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
    assert_eq!(builds.get(), 1);
    cx.update_window(scene.window, |_, window, cx| {
        root.update(cx, |root, cx| {
            root.duplicate = true;
            cx.notify();
        });
        window.draw(cx).clear(cx);
    })
    .unwrap();
}

struct Blank;
impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}
#[gpui::test]
fn retained_slots_and_input_reject_a_different_platform_window(cx: &mut TestAppContext) {
    let timer = timer();
    let scene = scene(cx, &timer, false, false, false);
    scene.draw(cx);
    let snapshot = capture(&scene, cx);
    let slots = snapshot.slots();
    let input = snapshot
        .entry
        .input
        .bind("parent".into(), |_: &()| Value::Null);
    let other = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Blank))
            .unwrap()
            .0
    });
    cx.update_window(other, |_, window, cx| {
        assert!(slots.render("editor", window, cx).is_err());
        input(&(), window, cx);
        window.remove_window();
    })
    .unwrap();
    assert_eq!(scene.app.borrow().frame(&scene.root).unwrap().revision, 0);
    drop(input);
    drop(slots);
    drop(snapshot);
    scene.close(cx);
}
