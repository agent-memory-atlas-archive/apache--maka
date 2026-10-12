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
use crate::Input;
use gpui_kit::{AnyWindowHandle, EntityInputHandler, TestAppContext};
use serde_json::json;
use std::cell::RefCell;
struct Harness(Entity<Owner>);
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(400.)).h(px(180.)).child(self.0.clone())
    }
}
fn props() -> TerminalViewProps {
    serde_json::from_value(json!({"source":"task","accessibilityLabel":"Terminal","controller":uuid::Uuid::new_v4(),"acknowledgement":null,
        "output":{"mode":"pty","screen":"界é🙂 prompt","scrollback":"one\ntwo\nthree","cols":40,"rows":6,"cursor":{"x":6,"y":0,"visible":true},"alternateScreen":false,"truncated":false,"redacted":false},
        "appearance":{"font":{"family":"Menlo","fallbacks":[],"size":14,"lineHeight":1,"weight":400},"foreground":4294967295u32},"background":255,"caret":4294967295u32,"rows":6,"tabIndex":0})).unwrap()
}
fn draw(cx: &mut TestAppContext, window: AnyWindowHandle) {
    cx.update_window(window, |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
}
fn harness(cx: &mut TestAppContext) -> (Entity<Owner>, AnyWindowHandle, LiveInput) {
    let live = LiveInput::new(
        Rc::new(RefCell::new(Some(Input(Rc::new(|_, _, _, _| {
            Ok(Default::default())
        }))))),
        crate::NativeBudget::default(),
        std::sync::Arc::new(maka_capabilities::call::Resources::default()),
    );
    let bound = live.clone();
    let (root, _) = cx.add_window_view(move |window, cx| {
        gpui_kit::init(cx);
        let props = props();
        let owner = Owner::new(&props, bound, window, cx);
        owner.update(cx, |owner, cx| {
            owner.visibility(true, window, cx);
            owner.apply(&props, window, cx);
        });
        Harness(owner)
    });
    let owner = cx.update(|cx| root.read(cx).0.clone());
    let window = cx.windows()[0];
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        owner.read(cx).focus.clone().focus(window, cx)
    })
    .unwrap();
    draw(cx, window);
    acknowledge(cx, window, &owner);
    (owner, window, live)
}
fn batch(cx: &mut TestAppContext, owner: &Entity<Owner>) -> Option<TerminalInputBatch> {
    cx.update(|cx| owner.read(cx).core.observation().batch.clone())
}
fn acknowledge(cx: &mut TestAppContext, window: AnyWindowHandle, owner: &Entity<Owner>) {
    cx.update_window(window, |_, window, cx| {
        owner.update(cx, |owner, cx| {
            let Some(batch) = owner.core.observation().batch.clone() else {
                return;
            };
            let mut props = owner.props.clone();
            props.acknowledgement = Some(TerminalInputAcknowledgement {
                owner: owner.core.observation().owner,
                controller: batch.controller,
                sequence: batch.sequence,
                accepted: true,
            });
            owner.apply(&props, window, cx);
        })
    })
    .unwrap();
    draw(cx, window);
}
#[gpui::test]
fn terminal_desktop_keyboard_ime_and_retirement_follow_native_focus(cx: &mut TestAppContext) {
    let (owner, window, live) = harness(cx);
    for (key, name, modifiers) in [
        ("tab", "tab", json!([])),
        ("escape", "escape", json!([])),
        ("ctrl-c", "c", json!(["ctrl"])),
    ] {
        cx.simulate_keystrokes(window, key);
        draw(cx, window);
        let input = serde_json::to_value(batch(cx, &owner).unwrap().actions).unwrap();
        assert_eq!(
            input[0],
            json!({"type":"key","key":name,"modifiers":modifiers})
        );
        acknowledge(cx, window, &owner);
    }
    cx.update_window(window, |_, window, cx| {
        owner.update(cx, |owner, cx| {
            owner.replace_and_mark_text_in_range(None, "中🙂", Some(1..3), window, cx);
            assert!(owner.core.observation().batch.is_none());
            assert_eq!(owner.marked, "中🙂");
            owner.replace_and_mark_text_in_range(Some(2..3), "X", None, window, cx);
            assert_eq!(owner.marked, "中🙂"); // reject half a surrogate
            owner.replace_text_in_range(None, "中文🙂", window, cx);
        })
    })
    .unwrap();
    draw(cx, window);
    assert_eq!(
        batch(cx, &owner).unwrap().actions,
        vec![maka_ui_runtime::terminal::input::InputAction::Text(
            "中文🙂".into()
        )]
    );
    acknowledge(cx, window, &owner);
    cx.simulate_input(window, "é");
    draw(cx, window);
    assert_eq!(
        batch(cx, &owner).unwrap().actions,
        vec![maka_ui_runtime::terminal::input::InputAction::Text(
            "é".into()
        )]
    );
    acknowledge(cx, window, &owner);
    cx.update_window(window, |_, window, cx| {
        owner.update(cx, |owner, cx| owner.visibility(false, window, cx))
    })
    .unwrap();
    cx.simulate_input(window, "hidden");
    draw(cx, window);
    assert!(batch(cx, &owner).is_none());
    cx.update_window(window, |_, window, cx| {
        owner.update(cx, |owner, cx| {
            owner.visibility(true, window, cx);
            owner.focus.focus(window, cx);
        })
    })
    .unwrap();
    draw(cx, window);
    live.0.borrow_mut().take();
    cx.simulate_input(window, "retired");
    draw(cx, window);
    assert!(batch(cx, &owner).is_none());
}
#[test]
fn terminal_desktop_cell_positions_keep_wide_combining_and_emoji_clusters() {
    let text = "AB界é🙂cd";
    let spans: Vec<_> = cell_spans(text)
        .into_iter()
        .map(|(range, column)| (&text[range], column))
        .collect();
    assert_eq!(
        spans,
        vec![("AB", 0), ("界", 2), ("é", 4), ("🙂", 5), ("cd", 7)]
    );
    assert_eq!(super::input::replace("🙂", Some(1..2), "x"), None);
}

#[gpui::test]
fn terminal_desktop_readonly_copy_does_not_require_a_controller(cx: &mut TestAppContext) {
    let (owner, window, _live) = harness(cx);
    cx.update_window(window, |_, window, cx| {
        owner.update(cx, |owner, cx| {
            let mut props = owner.props.clone();
            props.controller = None;
            props.acknowledgement = None;
            owner.apply(&props, window, cx);
        })
    })
    .unwrap();
    draw(cx, window);
    cx.simulate_keystrokes(window, "cmd-c");
    draw(cx, window);
    let text = cx.update(|cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(text.as_deref(), Some("界é🙂 prompt"));
    cx.simulate_input(window, "never");
    draw(cx, window);
    assert!(batch(cx, &owner).is_none());
}

#[gpui::test]
fn terminal_desktop_release_leaves_the_only_tab_stop(cx: &mut TestAppContext) {
    let (owner, window, _live) = harness(cx);
    cx.simulate_keystrokes(window, "ctrl-]");
    draw(cx, window);
    cx.update_window(window, |_, window, cx| {
        assert!(!owner.read(cx).focus.is_focused(window));
        assert!(!owner.read(cx).core.captured());
    })
    .unwrap();
    cx.simulate_input(window, "released");
    draw(cx, window);
    assert!(batch(cx, &owner).is_none());
}

#[gpui::test]
fn terminal_desktop_pointer_cells_and_font_resize_preserve_input_order(cx: &mut TestAppContext) {
    use gpui_kit::{Modifiers, MouseButton, VisualTestContext};
    let (owner, window, _live) = harness(cx);
    let position = cx.update(|cx| {
        let g = owner.read(cx).geometry.as_ref().unwrap();
        g.bounds.origin + point(g.cell * 2.5, g.line * 1.5)
    });
    for (button, name) in [
        (MouseButton::Middle, "middle"),
        (MouseButton::Right, "right"),
    ] {
        for event in ["press", "move", "release"] {
            let mut visual = VisualTestContext::from_window(window, cx);
            match event {
                "press" => visual.simulate_mouse_down(position, button, Modifiers::default()),
                "move" => visual.simulate_mouse_move(position, button, Modifiers::default()),
                _ => visual.simulate_mouse_up(position, button, Modifiers::default()),
            }
            draw(cx, window);
            let actions = serde_json::to_value(batch(cx, &owner).unwrap().actions).unwrap();
            assert_eq!(
                actions[0],
                json!({"type":"mouse","x":2,"y":1,"event":event,"button":name,"modifiers":[]})
            );
            acknowledge(cx, window, &owner);
        }
    }
    cx.update_window(window, |_, window, cx| {
        owner.update(cx, |owner, cx| {
            owner.replace_and_mark_text_in_range(None, "中🙂", Some(1..3), window, cx);
            let mut props = owner.props.clone();
            props.appearance.font.size = 20.;
            owner.apply(&props, window, cx);
            assert!(owner.marked.is_empty());
            assert_eq!(owner.selected, 0..0);
        })
    })
    .unwrap();
    draw(cx, window);
    let resize = batch(cx, &owner).unwrap();
    assert!(resize.size.is_some());
    assert!(resize.actions.is_empty());
    cx.simulate_input(window, "after resize");
    draw(cx, window);
    assert_eq!(batch(cx, &owner), Some(resize.clone()));
    acknowledge(cx, window, &owner);
    // Individual platform commits retain their order behind the resize, but
    // queued text does not require another round trip for each character.
    let typed = batch(cx, &owner).unwrap();
    assert_eq!(typed.controller, resize.controller);
    assert_eq!(typed.sequence, resize.sequence + 1);
    assert!(typed.size.is_none());
    assert_eq!(
        typed.actions,
        "after resize"
            .chars()
            .map(|ch| maka_ui_runtime::terminal::input::InputAction::Text(ch.to_string()))
            .collect::<Vec<_>>()
    );
    acknowledge(cx, window, &owner);
    assert!(batch(cx, &owner).is_none());
}
