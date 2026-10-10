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
use serde_json::Value;
use std::cell::RefCell;

fn content() -> Value {
    json!({"kind":"stack","key":"reading","direction":"column","gap":0,"padding":0,
        "background":0,"children":[],"scroll":"manual"})
}
#[test]
fn omitted_command_preserves_sequence_and_same_number_cannot_change_target() {
    let values = maka_ui::abi::ValueBindings::default();
    let props = json!({"content":content(),"command":{"sequence":1,"target":{"kind":"end"}}});
    let first = Component
        .call(ComponentCall::Mount { props: &props }, &values)
        .unwrap();
    let omitted = json!({"content":content(),"command":null});
    let next = Component
        .call(
            ComponentCall::Update {
                state: &first.frame.state,
                props: &omitted,
            },
            &values,
        )
        .unwrap();
    assert_eq!(next.frame.state["command"], props["command"]);
    let changed = json!({"content":content(),"command":{"sequence":1,"target":{"kind":"start"}}});
    assert!(
        Component
            .call(
                ComponentCall::Update {
                    state: &next.frame.state,
                    props: &changed
                },
                &values
            )
            .is_err()
    );
}

use crate::{Input, scene};
use gpui_kit::base::TextSelectionLayer;
use gpui_kit::{
    Context, InteractiveElement, Modifiers, ParentElement, Render, ScrollDelta, ScrollWheelEvent,
    Styled, TestAppContext, TouchPhase, div, point, px,
};

struct Harness {
    recipe: Rc<dyn Widget>,
    state: Box<dyn Any>,
    slots: Slots,
    renders: std::cell::Cell<u64>,
    block_long_press: Rc<std::cell::Cell<bool>>,
    foreground: Option<(gpui_kit::InputScope, gpui_kit::FocusHandle)>,
}
impl Render for Harness {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        let block = self.block_long_press.clone();
        let mut root = div()
            .size_full()
            .child(TextSelectionLayer)
            .child(gpui_kit::canvas(
                |_, _, _| (),
                move |_, _, window, _| {
                    window.on_mouse_event(move |_: &gpui_kit::LongPressEvent, phase, window, _| {
                        if phase.capture() && block.get() {
                            window.prevent_default();
                        }
                    })
                },
            ))
            .child(
                self.recipe
                    .render(self.state.as_ref(), &self.slots, window, cx)
                    .unwrap(),
            );
        if let Some((scope, focus)) = &self.foreground {
            root = root.child(
                scope.element(
                    div()
                        .id("reading-foreground")
                        .track_focus(focus)
                        .child(TextSelectionLayer),
                ),
            );
        }
        root
    }
}
fn input() -> LiveInput {
    LiveInput::new(
        Rc::new(RefCell::new(Some(Input(Rc::new(|_, _, _, _| {
            Ok(Default::default())
        }))))),
        crate::NativeBudget::default(),
        std::sync::Arc::new(maka_plugins::call::Resources::default()),
    )
}
fn scene_source(children: Value) -> Value {
    json!({"kind":"stack","key":"reading","direction":"column","gap":0,"padding":0,
        "background":0xffffff,"children":children,"scroll":"follow_end",
        "style":{"width":{"unit":"pixels","value":240},"height":{"unit":"pixels","value":100}}})
}
fn compile_recipe(source: &Value, command: Option<ReadingCommand>) -> Rc<dyn Widget> {
    Component
        .call(
            ComponentCall::Mount {
                props: &json!({"content":source,"command":command}),
            },
            &Default::default(),
        )
        .unwrap()
        .frame
        .view
        .view
}
fn empty_slots() -> Slots {
    Slots::new(
        vec![],
        Input(Rc::new(|_, _, _, _| Ok(Default::default()))),
        |_, _, _| unreachable!(),
    )
}

#[gpui::test]
fn logical_identity_markers_preserve_original_scene_geometry(cx: &mut TestAppContext) {
    for child in [
        json!({"kind":"text","key":"row","text":"Original text", "color":0,"style":{"height":{"unit":"pixels","value":400},"shrink":0}}),
        json!({"kind":"button","key":"row","label":"Run","style":{"width":{"unit":"fraction","value":0.75},"height":{"unit":"pixels","value":160},"shrink":0}}),
        json!({"kind":"stack","key":"row","direction":"column","gap":3,"padding":5,"background":0,"style":{"width":{"unit":"fraction","value":0.8},"grow":1,"shrink":2,"minHeight":120},"children":[{"kind":"text","key":"nested","text":"Nested", "color":0}]}),
    ] {
        let source = scene_source(json!([child]));
        let baseline = scene::compile(&source, Instant::now() + Duration::from_secs(1))
            .unwrap()
            .view;
        let reading = compile_recipe(&source, None);
        let mut shapes = vec![];
        for (reading, recipe) in [(false, baseline), (true, reading)] {
            let (view, window_cx) = cx.add_window_view(|window, cx| {
                let mut state = recipe.create(input(), window, cx);
                recipe.presentation(state.as_mut(), true, window, cx);
                recipe.commit(state.as_mut(), window, cx);
                Harness {
                    recipe,
                    state,
                    slots: empty_slots(),
                    renders: Default::default(),
                    block_long_press: Default::default(),
                    foreground: None,
                }
            });
            window_cx.update(|window, cx| window.draw(cx).clear(cx));
            shapes.push(window_cx.update(|_, cx| {
                let view = view.read(cx);
                let handle = if reading {
                    owner(view.state.as_ref()).read(cx).handle.clone()
                } else {
                    (view.recipe.as_ref() as &dyn Any)
                        .downcast_ref::<Node>()
                        .unwrap()
                        .reading_handle(view.state.as_ref())
                };
                (
                    handle.bounds().size,
                    handle.bounds_for_item(0),
                    handle.max_offset(),
                    handle.offset(),
                )
            }));
        }
        assert_eq!(
            shapes[0], shapes[1],
            "an identity marker cannot change flex or percentage geometry"
        );
    }
}

fn appearance() -> maka_ui::markdown::MarkdownAppearance {
    use maka_ui::appearance::{Color, FontAppearance, FontFamily, TextAppearance};
    let text = TextAppearance {
        font: FontAppearance {
            family: FontFamily("UI".into()),
            fallbacks: vec![],
            size: 16.,
            line_height: 1.5,
            weight: 400,
        },
        foreground: Color(0x000000ff),
    };
    maka_ui::markdown::MarkdownAppearance {
        terminal: None,
        body: text.clone(),
        code: text.clone(),
        headings: std::array::from_fn(|_| text.clone()),
        muted: Color(0x444444ff),
        link: Color(0x004499ff),
        selection: Color(0x4466ff55),
        code_background: Color(0xffffff00),
        table_background: Color(0xffffff00),
        border: Color(0xddddddff),
        dark: false,
        paragraph_gap: 8.,
    }
}
#[gpui::test]
fn original_markdown_completed_selection_and_owner_gap_stay_in_reading_scope(
    cx: &mut TestAppContext,
) {
    cx.with_paused_automatic_drawing(|cx| {
    cx.update(gpui_kit::init);
    let text = "**Original** selection 中文";
    let props = maka_ui::markdown::MarkdownProps {
        animate: false,
        ascii: false,
        source: text.into(),
        appearance: appearance(),
        selectable: true,
        disabled: false,
        focus_on_pointer: true,
    };
    let source = scene_source(
        json!([{"kind":"component","key":"document","binding":"markdown","member":0,"props":serde_json::to_value(&props).unwrap(),"on":{}}]),
    );
    let recipe = compile_recipe(&source, None);
    let document = Rc::new(RefCell::new(None));
    let notify = Rc::new(RefCell::new(None));
    let document_out = document.clone();
    let notify_out = notify.clone();
    let inner = Rc::new(RefCell::new(None));
    let inner_out = inner.clone();
    let block_long_press = Rc::new(std::cell::Cell::new(true));
    let block_out = block_long_press.clone();
    let (view, window_cx) = cx.add_window_view(move |window, cx| {
        let crate::markdown::TestDocument { widget: markdown, state: markdown_state, document: doc, notify: owner_notify } =
            crate::markdown::test_document(&props, input(), window, cx);
        *document_out.borrow_mut() = Some(doc);
        *notify_out.borrow_mut() = Some(owner_notify);
        let mut state = recipe.create(input(), window, cx);
        recipe.presentation(state.as_mut(), true, window, cx);
        recipe.commit(state.as_mut(), window, cx);
        let slots = Slots::new(
            vec!["document".into()],
            Input(Rc::new(|_, _, _, _| Ok(Default::default()))),
            move |_, window, cx| {
                markdown.render(markdown_state.as_ref(), &empty_slots(), window, cx)
            },
        );
        *inner_out.borrow_mut() = Some(owner(state.as_ref()).clone());
        let outer_source = scene_source(json!([{"kind":"component","key":"nested","binding":"reading","member":0,"props":{"content":source,"command":null},"on":{}}]));
        let outer = compile_recipe(&outer_source, None);
        let mut outer_state = outer.create(input(), window, cx);
        outer.presentation(outer_state.as_mut(), true, window, cx);
        outer.commit(outer_state.as_mut(), window, cx);
        let outer_slots = Slots::new(vec!["nested".into()], Input(Rc::new(|_, _, _, _| Ok(Default::default()))),
            move |_, window, cx| recipe.render(state.as_ref(), &slots, window, cx));
        Harness {
            recipe: outer,
            state: outer_state,
            slots: outer_slots,
            renders: Default::default(), block_long_press: block_out, foreground: None,
        }
    });
    window_cx.run_until_parked();
    window_cx.update(|window, cx| window.draw(cx).clear(cx));
    let state = inner.borrow().as_ref().unwrap().clone();
    let scope = window_cx.update(|_, cx| state.read(cx).scope);
    let before = window_cx.update(|_, cx| state.read(cx).handle.reading_witness());
    window_cx.update(|window, cx| {
        assert!(state.read(cx).eligible(window, cx));
        document
            .borrow()
            .as_ref()
            .unwrap()
            .update(cx, |doc, cx| doc.select_all(cx));
        assert!(TextSelection::has_selection_in_scope(scope, window, cx));
        assert!(!TextSelection::has_selection_in_scope(
            TextSelectionScopeId::new(),
            window,
            cx
        ));
        assert!(!state.read(cx).eligible(window, cx));
        assert!(
            before.is_current(&state.read(cx).handle),
            "completed selection need not scroll"
        );
        assert!(TextSelection::selected_text(window, cx).contains("Original selection 中文"));
    });
    window_cx.run_until_parked();
    window_cx.simulate_event(gpui_kit::LongPressEvent {
        phase: TouchPhase::Started,
        start_position: point(px(16.), px(16.)),
        position: point(px(16.), px(16.)),
    });
    window_cx.update(|window, cx| {
        assert!(TextSelection::has_selection_in_scope(scope, window, cx),
            "a prevented long press must not clear the original completed local selection");
        assert!(TextSelection::selected_text(window, cx).contains("Original selection 中文"));
    });
    block_long_press.set(false);
    window_cx.simulate_event(gpui_kit::LongPressEvent {
        phase: TouchPhase::Started,
        start_position: point(px(239.), px(12.)),
        position: point(px(239.), px(12.)),
    });
    window_cx.update(|window, cx| {
        assert!(TextSelection::has_selection_in_scope(scope, window, cx),
            "a blank long press must not switch scope or clear completed selection");
        assert!(TextSelection::selected_text(window, cx).contains("Original selection 中文"));
    });
    // The foreground has no selection. A global lookup would incorrectly resume
    // background follow; the original Reading observer must retain its own group.
    window_cx.update(|window, cx| view.update(cx, |view, cx| {
        let focus = cx.focus_handle().tab_stop(false);
        view.foreground = Some((window.input_scope(&focus, cx), focus));
        cx.notify();
    }));
    window_cx.update(|window, cx| window.draw(cx).clear(cx));
    window_cx.update(|window, cx| {
        assert!(!TextSelection::has_selection_in_scope(scope, window, cx));
        assert_eq!(state.read(cx).selection.as_ref().unwrap().has_selection_in_scope(scope, cx), Some(true));
        assert!(!state.read(cx).eligible(window, cx));
    });
    window_cx.run_until_parked();
    window_cx.update(|window, cx| window.draw(cx).clear(cx));
    // Force the actual native Markdown Owner (which recreates TextView) to
    // rebuild independently; the retained parent must restore inherited scope.
    let renders = window_cx.update(|_, cx| view.read(cx).renders.get());
    window_cx.update(|_, cx| notify.borrow().as_ref().unwrap()(cx));
    window_cx.update(|window, cx| window.draw(cx).clear(cx));
    window_cx.update(|window, cx| {
        assert_eq!(
            view.read(cx).renders.get(),
            renders,
            "the Markdown gap rebuild must retain the parent"
        );
        assert_eq!(state.read(cx).selection.as_ref().unwrap().has_selection_in_scope(scope, cx), Some(true));
        assert!(!state.read(cx).eligible(window, cx));
    });
    window_cx.update(|_, cx| document.borrow().as_ref().unwrap().update(cx, |doc, cx| doc.push_str("\n\nFresh source append 中文", cx)));
    window_cx.run_until_parked();
    window_cx.update(|window, cx| window.draw(cx).clear(cx));
    window_cx.update(|window, cx| {
        assert_eq!(state.read(cx).selection.as_ref().unwrap().has_selection_in_scope(scope, cx), Some(true));
        assert!(!state.read(cx).eligible(window, cx));
        view.update(cx, |view, cx| { view.foreground.take(); cx.notify(); });
    });
    window_cx.update(|window, cx| window.draw(cx).clear(cx));
    window_cx.update(|window, cx| {
        assert!(TextSelection::selected_text(window, cx).contains("Original selection 中文"));
        document.borrow().as_ref().unwrap().update(cx, |doc, cx| doc.focus_handle().focus(window, cx));
    });
    window_cx.dispatch_action(gpui_kit::base::input::Copy);
    window_cx.update(|_, cx| {
        assert!(cx.read_from_clipboard().unwrap().text().unwrap().starts_with("Original selection 中文"));
    });
    window_cx.update(|window, cx| {
        TextSelection::clear(window, cx);
        assert!(state.read(cx).eligible(window, cx));
    });
    // Move the original, unnotified Markdown entity into a new inherited scope.
    // Retained layout and scroll-layer caches must not replay its old registration.
    let moved_scope = TextSelectionScopeId::new();
    state.update(window_cx, |state, _| state.scope = moved_scope);
    window_cx.update(|_, cx| view.update(cx, |_, cx| cx.notify()));
    window_cx.update(|window, cx| window.draw(cx).clear(cx));
    window_cx.update(|window, cx| {
        document.borrow().as_ref().unwrap().update(cx, |doc, cx| doc.select_all(cx));
        assert!(TextSelection::has_selection_in_scope(moved_scope, window, cx));
        assert!(!TextSelection::has_selection_in_scope(scope, window, cx));
    });
    });
}

#[gpui::test]
fn original_wheel_changes_reading_intent_and_cancels_pending_command(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
    let source = scene_source(
        json!([{"kind":"text","key":"row","text":"Long source","color":0,"style":{"height":{"unit":"pixels","value":400},"shrink":0}}]),
    );
    let recipe = compile_recipe(&source, None);
    let (view, window_cx) = cx.add_window_view(|window, cx| {
        let mut state = recipe.create(input(), window, cx);
        recipe.presentation(state.as_mut(), true, window, cx);
        recipe.commit(state.as_mut(), window, cx);
        Harness {
            recipe,
            state,
            slots: empty_slots(),
            renders: Default::default(), block_long_press: Default::default(), foreground: None,
        }
    });
    window_cx.update(|window, cx| window.draw(cx).clear(cx));
    let state = window_cx.update(|_, cx| owner(view.read(cx).state.as_ref()).clone());
    assert!(state.read_with(window_cx, |state, _| state.handle.painted_observation().unwrap().at_end));
    window_cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.recipe = compile_recipe(
                &source,
                Some(ReadingCommand {
                    sequence: 1,
                    target: ReadingTarget::Start,
                }),
            );
            view.recipe.commit(view.state.as_mut(), window, cx);
            cx.notify();
        });
    });
    let witness = window_cx.update(|_, cx| state.read(cx).handle.reading_witness());
    assert_eq!(state.update(window_cx, |state, _| state.observation()).pending_command, Some(1));
    window_cx.simulate_event(ScrollWheelEvent {
        position: point(px(50.), px(50.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(100.))),
        modifiers: Modifiers::default(),
        touch_phase: TouchPhase::Moved,
    });
    assert!(!state.read_with(window_cx, |state, _| witness.is_current(&state.handle)));
    assert_eq!(
        state.update(window_cx, |state, _| state.observation()).receipt,
        Some(ReadingReceipt {
            sequence: 1,
            status: ReadingStatus::Cancelled
        })
    );
    window_cx.update(|window, cx| {
        assert!(!state.read(cx).eligible(window, cx));
        window.draw(cx).clear(cx);
        assert!(!state.read(cx).handle.painted_observation().unwrap().at_end);
    });
    window_cx.simulate_event(ScrollWheelEvent {
        position: point(px(50.), px(50.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(-1000.))),
        modifiers: Modifiers::default(),
        touch_phase: TouchPhase::Moved,
    });
    window_cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert!(state.read(cx).handle.painted_observation().unwrap().at_end);
        assert!(state.read(cx).eligible(window, cx));
        assert_eq!(state.query(cx, |state, _| state.observation()).receipt.unwrap().status, ReadingStatus::Cancelled);
    });
    });
}

#[gpui::test]
fn model_button_viewport_reveals_every_choice_in_both_directions(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        let mut source = scene_source(Value::Array((0..16).map(|at| json!({
            "kind":"button", "key":format!("choose-{at}"),
            "label":format!("Model {at} with a long display name · Research Account"),
            "style":{"width":{"unit":"fraction","value":1},"height":{"unit":"pixels","value":32},"shrink":0,"minWidth":0,"truncate":true}
        })).collect()));
        source["scroll"] = json!("manual");
        source["gap"] = json!(2);
        source["style"]["height"] = json!({"unit":"pixels","value":160});
        let initial = compile_recipe(&source, None);
        let (view, window_cx) = cx.add_window_view(move |window, cx| {
            gpui_kit::init(cx);
            let mut state = initial.create(input(), window, cx);
            initial.presentation(state.as_mut(), true, window, cx);
            initial.commit(state.as_mut(), window, cx);
            Harness { recipe: initial, state, slots: empty_slots(), renders: Default::default(), block_long_press: Default::default(), foreground: None }
        });
        window_cx.update(|window, cx| window.draw(cx).clear(cx));
        let state = window_cx.update(|_, cx| owner(view.read(cx).state.as_ref()).clone());
        assert!(!state.update(window_cx, |state, _| state.observation()).viewport.unwrap().visible_child_keys.contains(&"choose-15".into()));
        for (sequence, at) in (0..16).chain((0..16).rev()).enumerate() {
            let sequence = sequence as u64 + 1;
            let key = format!("choose-{at}");
            window_cx.update(|window, cx| view.update(cx, |view, cx| {
                view.recipe = compile_recipe(&source, Some(ReadingCommand { sequence, target: ReadingTarget::RevealChild { key: key.clone() } }));
                view.recipe.commit(view.state.as_mut(), window, cx);
                cx.notify();
            }));
            window_cx.update(|window, cx| window.draw(cx).clear(cx));
            let observation = state.update(window_cx, |state, _| state.observation());
            assert_eq!(observation.receipt, Some(ReadingReceipt { sequence, status: ReadingStatus::Applied }));
            assert!(observation.viewport.unwrap().visible_child_keys.contains(&key), "active model must actually paint inside the native viewport: {key}");
        }
    });
}

#[gpui::test]
fn native_commands_settle_only_on_paint_and_observations_do_not_poll(cx: &mut TestAppContext) {
    cx.with_paused_automatic_drawing(|cx| {
        let mut source = scene_source(json!([
            {"kind":"text","key":"first","text":"First","color":0,"style":{"height":{"unit":"pixels","value":160},"shrink":0}},
            {"kind":"text","key":"last","text":"Last","color":0,"style":{"height":{"unit":"pixels","value":160},"shrink":0}},
            {"kind":"stack","key":"hidden","visible":false,"direction":"column","gap":0,"padding":0,"background":0,"children":[]}
        ]));
        let observations = Rc::new(RefCell::new(Vec::new()));
        let out = observations.clone();
        let live = LiveInput::new(Rc::new(RefCell::new(Some(Input(Rc::new(move |key, value, _, _| {
            assert_eq!(key, OBSERVED);
            out.borrow_mut().push(serde_json::from_value::<ReadingObservation>(value).unwrap());
            Ok(Default::default())
        }))))), crate::NativeBudget::default(), std::sync::Arc::new(maka_plugins::call::Resources::default()));
        let initial = compile_recipe(&source, None);
        let (view, window_cx) = cx.add_window_view(move |window, cx| {
            let mut state = initial.create(live, window, cx);
            initial.presentation(state.as_mut(), true, window, cx);
            initial.commit(state.as_mut(), window, cx);
            Harness { recipe: initial, state, slots: empty_slots(), renders: Default::default(), block_long_press: Default::default(), foreground: None }
        });
        window_cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(observations.borrow().is_empty(), "paint must not call a plugin");
        window_cx.update(|window, cx| { window.simulate_next_frame(cx); });
        assert!(observations.borrow().last().unwrap().viewport.as_ref().unwrap().at_end);
        let state = window_cx.update(|_, cx| owner(view.read(cx).state.as_ref()).clone());
        for (sequence, target, expected) in [
            (1, ReadingTarget::Start, ReadingStatus::Applied),
            (2, ReadingTarget::RevealChild { key: "last".into() }, ReadingStatus::Applied),
            (3, ReadingTarget::RevealChild { key: "hidden".into() }, ReadingStatus::Rejected),
            (4, ReadingTarget::RevealChild { key: "missing".into() }, ReadingStatus::Rejected),
            (5, ReadingTarget::End, ReadingStatus::Applied),
        ] {
            window_cx.update(|window, cx| view.update(cx, |view, cx| {
                view.recipe = compile_recipe(&source, Some(ReadingCommand { sequence, target }));
                view.recipe.commit(view.state.as_mut(), window, cx);
                cx.notify();
            }));
            if expected == ReadingStatus::Applied {
                assert_eq!(state.update(window_cx, |state, _| state.observation()).pending_command, Some(sequence));
            }
            window_cx.update(|window, cx| window.draw(cx).clear(cx));
            window_cx.update(|window, cx| { window.simulate_next_frame(cx); });
            assert_eq!(state.update(window_cx, |state, _| state.observation()).receipt, Some(ReadingReceipt { sequence, status: expected }));
        }
        assert!(state.read_with(window_cx, |state, _| state.handle.painted_observation().unwrap().at_end));
        window_cx.simulate_event(ScrollWheelEvent {
            position: point(px(50.), px(50.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(100.))),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        window_cx.update(|window, cx| window.draw(cx).clear(cx));
        window_cx.update(|window, cx| { window.simulate_next_frame(cx); });
        assert!(!observations.borrow().last().unwrap().viewport.as_ref().unwrap().at_end,
            "a real wheel paint must publish the current viewport even through native caches");
        let count = observations.borrow().len();
        window_cx.update(|window, cx| window.draw(cx).clear(cx));
        window_cx.update(|window, cx| { window.simulate_next_frame(cx); });
        assert_eq!(observations.borrow().len(), count, "unchanged paint must not poll");
        source["visible"] = json!(false);
        window_cx.update(|window, cx| view.update(cx, |view, cx| {
            view.recipe = compile_recipe(&source, Some(ReadingCommand { sequence: 6, target: ReadingTarget::Start }));
            view.recipe.commit(view.state.as_mut(), window, cx);
            cx.notify();
        }));
        assert_eq!(state.update(window_cx, |state, _| state.observation()).receipt, Some(ReadingReceipt { sequence: 6, status: ReadingStatus::Rejected }));
        window_cx.update(|_, cx| {
            let view = view.read(cx);
            let reading = (view.recipe.as_ref() as &dyn Any).downcast_ref::<Reading>().unwrap();
            assert!(!reading.node.reading_visible(state.read(cx).scene.as_ref()), "old painted end must not admit a hidden recipe");
        });
    });
}

mod dialog;
