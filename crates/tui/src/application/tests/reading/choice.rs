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

use super::dialog::{column, dialog, pending, root_count};
use super::*;

fn choice() -> Value {
    json!({"kind":"choice","key":"thinking","label":"Thinking","options":
        (0..12).map(|i| json!({"key":format!("level-{i}"),"label":format!("Level {i:02}")})).collect::<Vec<_>>(),
        "current":"level-8","foreground":0xffffff,"background":0x17171a,
        "border":0x888888,"selected":0x224466,"accent":0x4488ff})
}
fn menu(p: &Presenter) -> Rect {
    state(p).choices["thinking"]
        .geometry
        .as_ref()
        .unwrap()
        .menu
        .as_ref()
        .unwrap()
        .rect
}
fn revision(p: &Presenter) -> u64 {
    p.application.frame(&p.root).unwrap().revision
}

#[test]
fn choice_plain_and_reading_commit_stable_key_after_native_navigation() {
    for reading in [false, true] {
        let mut value = choice();
        value["ascii"] = json!(reading);
        let (_runtime, mut p, _) = if reading {
            native(vec![value])
        } else {
            let shared = Rc::new(RefCell::new(value));
            let (runtime, presenter) = layout::native(
                layout::View::InteractiveScene(shared.clone()),
                Rc::new(Cell::new(true)),
            );
            (runtime, presenter, shared)
        };
        draw(&mut p, 30, 6);
        draw(&mut p, 30, 6);
        focus_choice(&mut p);
        let trigger = p.focus.clone().unwrap();
        let before = revision(&p);
        p.input(key(KeyCode::Enter)).unwrap();
        assert!(!p.physical_admitted());
        p.input(key(KeyCode::Down)).unwrap();
        p.input(key(KeyCode::Enter)).unwrap();
        assert_eq!(revision(&p), before);
        draw(&mut p, 30, 6);
        assert_eq!(p.physical_path.len(), 1);
        p.input(key(KeyCode::Down)).unwrap();
        p.input(key(KeyCode::Down)).unwrap();
        p.input(key(KeyCode::Down)).unwrap();
        let buffer = draw(&mut p, 30, 6);
        assert_eq!(revision(&p), before);
        let rect = menu(&p);
        let last = rect.bottom() - 2;
        // Current selection and keyboard highlight are distinct, as in the old chooser.
        assert_eq!(
            buffer[(rect.x + 2, rect.y + 1)].symbol(),
            if reading { "*" } else { "●" }
        );
        assert_eq!(
            buffer[(rect.x, rect.y)].symbol(),
            if reading { "+" } else { "╭" }
        );
        assert_eq!(
            buffer[(rect.x + 2, last)].bg,
            ratatui::style::Color::Rgb(0x22, 0x44, 0x66)
        );
        let line: String = (rect.x..rect.right())
            .map(|x| buffer[(x, last)].symbol())
            .collect();
        assert!(line.contains("Level 11"), "{line:?}");
        // Hover another visible row, then repaint: the original clipped window stays put.
        mouse(&mut p, MouseEventKind::Moved, rect.x + 2, last - 1);
        draw(&mut p, 30, 6);
        assert_eq!(revision(&p), before);
        mouse(
            &mut p,
            MouseEventKind::Down(MouseButton::Left),
            rect.x + 2,
            last - 1,
        );
        assert_eq!(observation(&p), json!({"kind":"selected","key":"level-10"}));
        assert!(state(&p).dialogs.is_empty());
        draw(&mut p, 30, 6);
        assert!(p.focus.as_ref().unwrap().same(&trigger));
        close(&mut p);
    }
}

#[test]
fn nested_choice_cancel_returns_trigger_without_dismissing_parent() {
    for cancel in [KeyCode::Esc, KeyCode::Tab, KeyCode::BackTab] {
        let (_runtime, mut p, _) = native(vec![dialog(
            "modal",
            true,
            source("bg", "background 中\r\n"),
            column("form", vec![choice()]),
        )]);
        draw(&mut p, 40, 12);
        draw(&mut p, 40, 12);
        focus_choice(&mut p);
        let trigger = p.focus.clone().unwrap();
        let before = root_count(&p);
        p.input(key(KeyCode::Char(' '))).unwrap();
        draw(&mut p, 40, 12);
        assert_eq!(p.physical_path.len(), 2);
        p.input(key(cancel)).unwrap();
        assert_eq!(root_count(&p), before);
        assert!(state(&p).dialogs.contains_key("modal"));
        assert!(!state(&p).dialogs.contains_key("thinking"));
        draw(&mut p, 40, 12);
        assert_eq!(p.physical_path.len(), 1);
        assert!(p.focus.as_ref().unwrap().same(&trigger));
        close(&mut p);
    }
}

#[test]
fn choice_changed_options_require_flush_and_hidden_reopen_rejects_old_batch() {
    let (_runtime, mut p, props) = native(vec![choice()]);
    draw(&mut p, 30, 6);
    focus_choice(&mut p);
    p.input(key(KeyCode::Enter)).unwrap();
    draw(&mut p, 30, 6);
    props.borrow_mut()["content"]["children"][0]["options"][8]["key"] = json!("replacement");
    props.borrow_mut()["content"]["children"][0]["current"] = json!("replacement");
    refresh(&mut p);
    let before = root_count(&p);
    p.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(root_count(&p), before);
    draw(&mut p, 30, 6);
    let held = pending(&p, 30, 6);
    let old = p.physical_path.last().unwrap().clone();
    props.borrow_mut()["content"]["visible"] = json!(false);
    refresh(&mut p);
    assert_eq!(old.strong_count(), 0);
    props.borrow_mut()["content"]["visible"] = json!(true);
    refresh(&mut p);
    draw(&mut p, 30, 6);
    focus_choice(&mut p);
    p.input(key(KeyCode::Enter)).unwrap();
    p.present(held);
    assert!(!p.physical_admitted());
    draw(&mut p, 30, 6);
    p.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(
        observation(&p),
        json!({"kind":"selected","key":"replacement"})
    );
    close(&mut p);
}

#[test]
fn choice_outside_and_right_click_cancel_without_background_activation() {
    for button in [MouseButton::Left, MouseButton::Right] {
        let (_runtime, mut p, _) = native(vec![
            choice(),
            json!({"kind":"button","key":"back","label":"Back"}),
        ]);
        draw(&mut p, 40, 12);
        draw(&mut p, 40, 12);
        focus_choice(&mut p);
        p.input(key(KeyCode::Enter)).unwrap();
        draw(&mut p, 40, 12);
        let before = root_count(&p);
        mouse(&mut p, MouseEventKind::Down(button), 0, 11);
        assert_eq!(root_count(&p), before);
        assert!(state(&p).dialogs.is_empty());
        draw(&mut p, 40, 12);
        assert_eq!(p.focus.as_ref().unwrap().key, "thinking");
        close(&mut p);
    }
}

#[test]
fn choice_unplaceable_or_disabled_closes_without_resurrection() {
    for disabled in [false, true] {
        let (_runtime, mut p, props) = native(vec![choice()]);
        draw(&mut p, 30, 6);
        focus_choice(&mut p);
        p.input(key(KeyCode::Enter)).unwrap();
        draw(&mut p, 30, 6);
        if disabled {
            props.borrow_mut()["content"]["children"][0]["disabled"] = json!(true);
            refresh(&mut p);
            draw(&mut p, 30, 6);
            props.borrow_mut()["content"]["children"][0]["disabled"] = json!(false);
            refresh(&mut p);
        } else {
            draw(&mut p, 5, 2);
        }
        assert!(!state(&p).dialogs.contains_key("thinking"));
        draw(&mut p, 30, 6);
        assert!(p.physical_path.is_empty());
        close(&mut p);
    }
}

fn focus_choice(p: &mut Presenter) {
    for _ in 0..4 {
        if p.focus
            .as_ref()
            .is_some_and(|target| target.key == "thinking")
        {
            return;
        }
        p.input(key(KeyCode::Tab)).unwrap();
    }
    panic!("choice must be keyboard reachable");
}
