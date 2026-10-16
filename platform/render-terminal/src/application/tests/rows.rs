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
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn props() -> Value {
    json!({"revision":1,"rows":(0..6).map(|n|json!({"key":format!("row-{n}"),"text":format!(" ↳ pending {n}"),"actions":[{"key":"delete","label":"Delete","enabled":true}]})).collect::<Vec<_>>(),
        "shortcuts":[{"key":"delete","modifiers":[],"input":"delete","enabled":true}],
        "scrollbar":{"thumb":"┃","track":"│","thumbColor":0xffffffffu32,"trackColor":0x888888ffu32},"maxRows":3,"tabIndex":0,"foreground":0xffffffffu32,"muted":0x888888ffu32,"selected":0x222222ffu32,"hovered":0x333333ffu32})
}
fn mouse(kind: MouseEventKind, x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    })
}
fn text(buffer: &Buffer) -> String {
    buffer.content.iter().map(|cell| cell.symbol()).collect()
}
fn refresh(tui: &mut Presenter) {
    refresh_key(tui, "back");
}
fn refresh_key(tui: &mut Presenter, key: &str) {
    let rev = tui.application.frame(&tui.root).unwrap().revision;
    tui.application
        .dispatch(&tui.root.clone(), rev, key, json!({"kind":"click"}))
        .unwrap();
    // Props must commit before testing the still-unflushed physical targets.
    tui.pump_settled().unwrap();
}
fn close(mut tui: Presenter) {
    tui.surface_closed(&tui.surface(), Ok(())).unwrap();
    tui.shutdown_settled().unwrap();
}
#[test]
fn native_rows_hover_is_local_and_only_flushed_controls_can_activate() {
    let props = Rc::new(RefCell::new(props()));
    let (_runtime, mut tui) = layout::native(
        layout::View::Rows(props, Rc::new(Cell::new(false))),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 40, 10);
    let focus = tui.focus.clone().unwrap();
    assert_eq!(focus.key, "back");
    assert!(!text(&draw(&mut tui, 40, 10)).contains("Delete"));
    tui.input_settled(mouse(MouseEventKind::Moved, 2, 2))
        .unwrap();
    assert!(tui.focus.as_ref().unwrap().same(&focus));
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 0);
    let area = Rect::new(0, 0, 40, 10);
    let mut buffer = Buffer::empty(area);
    let pending = tui.paint(area, &mut buffer).unwrap();
    assert!(text(&buffer).contains("Delete"));
    // Visible in an unflushed candidate only: the last actual geometry is preview.
    tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 36, 2))
        .unwrap();
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 0);
    drop(pending);
    draw(&mut tui, 40, 10);
    tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 36, 2))
        .unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["observation"],
        json!({"revision":1,"row":"row-1","action":"delete"})
    );
    draw(&mut tui, 40, 10);
    tui.input_settled(key(KeyCode::Esc)).unwrap();
    assert!(tui.focus.as_ref().unwrap().same(&focus));
    close(tui);
}
#[test]
fn native_rows_selected_removal_exits_exact_focus_and_does_not_retarget_repeat() {
    let props = Rc::new(RefCell::new(props()));
    let (_runtime, mut tui) = layout::native(
        layout::View::Rows(props.clone(), Rc::new(Cell::new(false))),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 40, 10);
    tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 2, 2))
        .unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "rows");
    props.borrow_mut()["rows"].as_array_mut().unwrap().remove(1);
    props.borrow_mut()["revision"] = json!(2);
    refresh(&mut tui);
    assert_eq!(tui.focus.as_ref().unwrap().key, "back");
    let count = tui.application.frame(&tui.root).unwrap().state["count"].clone();
    let mut held = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    held.kind = KeyEventKind::Repeat;
    tui.input_settled(Event::Key(held)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["count"],
        count
    );
    draw(&mut tui, 40, 10);
    tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 2, 2))
        .unwrap();
    draw(&mut tui, 40, 10);
    tui.input_settled(key(KeyCode::Delete)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["observation"]["row"],
        "row-2"
    );
    draw(&mut tui, 40, 10);
    tui.input_settled(key(KeyCode::BackTab)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "back");
    close(tui);
}
#[test]
fn native_rows_scroll_navigation_and_modal_clear_keep_original_pointer_scope() {
    let props = Rc::new(RefCell::new(props()));
    let modal = Rc::new(Cell::new(false));
    let (_runtime, mut tui) = layout::native(
        layout::View::Rows(props, modal.clone()),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 40, 10);
    tui.input_settled(mouse(MouseEventKind::Moved, 2, 2))
        .unwrap();
    assert!(tui.hover.is_some());
    modal.set(true);
    refresh(&mut tui);
    // Opening is pending; input cannot reach the original background rows.
    tui.input_settled(mouse(MouseEventKind::Moved, 2, 2))
        .unwrap();
    assert!(tui.hover.is_none());
    draw(&mut tui, 40, 10);
    assert!(tui.scene.iter().all(|target| target.key != "rows"));
    modal.set(false);
    refresh(&mut tui);
    draw(&mut tui, 40, 10);
    tui.input_settled(mouse(MouseEventKind::ScrollDown, 2, 2))
        .unwrap();
    let buffer = draw(&mut tui, 40, 10);
    assert!(text(&buffer).contains("pending 5"));
    assert!(!text(&buffer).contains("pending 0"));
    assert_eq!(tui.focus.as_ref().unwrap().key, "back");
    tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 2, 1))
        .unwrap();
    tui.input_settled(key(KeyCode::Home)).unwrap();
    let buffer = draw(&mut tui, 40, 10);
    assert!(text(&buffer).contains("pending 0"));
    tui.input_settled(key(KeyCode::End)).unwrap();
    assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
    tui.input_settled(key(KeyCode::Tab)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "back");
    close(tui);
}

#[test]
fn native_rows_late_flush_does_not_undo_accepted_wheel_or_keyboard_navigation() {
    for wheel in [true, false] {
        let props = Rc::new(RefCell::new(props()));
        let (_runtime, mut tui) = layout::native(
            layout::View::Rows(props, Rc::new(Cell::new(false))),
            Rc::new(Cell::new(true)),
        );
        draw(&mut tui, 40, 10);
        tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 2, 1))
            .unwrap();
        draw(&mut tui, 40, 10);
        let area = Rect::new(0, 0, 40, 10);
        let mut buffer = Buffer::empty(area);
        let held = tui.paint(area, &mut buffer).unwrap();
        if wheel {
            tui.input_settled(mouse(MouseEventKind::ScrollDown, 2, 2))
                .unwrap();
        } else {
            tui.input_settled(key(KeyCode::End)).unwrap();
        }
        tui.present(held);
        let buffer = draw(&mut tui, 40, 10);
        assert!(text(&buffer).contains("pending 5"));
        assert!(!text(&buffer).contains("pending 0"));
        close(tui);
    }
}

#[test]
fn native_rows_held_navigation_moves_but_held_actions_do_not_repeat() {
    let props = Rc::new(RefCell::new(props()));
    let (_runtime, mut tui) = layout::native(
        layout::View::Rows(props, Rc::new(Cell::new(false))),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 40, 10);
    tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 2, 1))
        .unwrap();
    let mut repeat = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    repeat.kind = KeyEventKind::Repeat;
    tui.input_settled(Event::Key(repeat)).unwrap();
    draw(&mut tui, 40, 10);
    repeat.code = KeyCode::Delete;
    tui.input_settled(Event::Key(repeat)).unwrap();
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 0);
    tui.input_settled(key(KeyCode::Delete)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["observation"]["row"],
        "row-1"
    );
    close(tui);
}

#[test]
fn native_rows_consumed_under_modal_returns_to_original_composer() {
    for phase in [0, 1, 2] {
        let props = Rc::new(RefCell::new(props()));
        let modal = Rc::new(Cell::new(false));
        let (_runtime, mut tui) = layout::native(
            layout::View::Rows(props.clone(), modal.clone()),
            Rc::new(Cell::new(true)),
        );
        draw(&mut tui, 40, 10);
        let composer = tui.focus.clone().unwrap();
        tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 2, 2))
            .unwrap();
        assert_eq!(tui.focus.as_ref().unwrap().key, "rows");
        modal.set(true);
        if phase != 2 {
            refresh(&mut tui);
        }
        if phase == 0 {
            draw(&mut tui, 40, 10);
            assert_eq!(tui.focus.as_ref().unwrap().key, "close");
        }
        props.borrow_mut()["rows"].as_array_mut().unwrap().remove(1);
        props.borrow_mut()["revision"] = json!(2);
        refresh(&mut tui);
        draw(&mut tui, 40, 10);
        assert_eq!(tui.focus.as_ref().unwrap().key, "close");
        modal.set(false);
        refresh(&mut tui);
        draw(&mut tui, 40, 10);
        assert!(
            tui.focus
                .as_ref()
                .is_some_and(|focus| focus.same(&composer))
        );
        let count = tui.application.frame(&tui.root).unwrap().state["count"].clone();
        for code in [KeyCode::Delete, KeyCode::Enter] {
            let mut held = KeyEvent::new(code, KeyModifiers::NONE);
            held.kind = KeyEventKind::Repeat;
            tui.input_settled(Event::Key(held)).unwrap();
        }
        assert_eq!(
            tui.application.frame(&tui.root).unwrap().state["count"],
            count
        );
        // A fresh explicit selection can enter a remaining row; no stale exit follows it.
        tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 2, 2))
            .unwrap();
        draw(&mut tui, 40, 10);
        tui.input_settled(key(KeyCode::Delete)).unwrap();
        assert_eq!(
            tui.application.frame(&tui.root).unwrap().state["observation"]["row"],
            "row-2"
        );
        close(tui);
    }
}

#[test]
fn native_rows_consumed_under_nested_modals_keep_the_surviving_ancestor_return() {
    let props = Rc::new(RefCell::new(props()));
    let phase = Rc::new(Cell::new(0));
    let (_runtime, mut tui) = layout::native(
        layout::View::NestedRows(props.clone(), phase.clone()),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 40, 12);
    let original = tui.focus.clone().unwrap();
    assert_eq!(original.key, "root-composer");
    phase.set(1);
    refresh_key(&mut tui, "root-composer");
    draw(&mut tui, 40, 12);
    assert_eq!(tui.focus.as_ref().unwrap().key, "back");
    tui.input_settled(key(KeyCode::Tab)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "rows");
    phase.set(2);
    refresh_key(&mut tui, "root-composer");
    draw(&mut tui, 40, 12);
    assert_eq!(tui.focus.as_ref().unwrap().key, "close");
    props.borrow_mut()["rows"].as_array_mut().unwrap().remove(0);
    props.borrow_mut()["revision"] = json!(2);
    refresh_key(&mut tui, "root-composer");
    draw(&mut tui, 40, 12);
    phase.set(0);
    refresh_key(&mut tui, "root-composer");
    draw(&mut tui, 40, 12);
    assert!(
        tui.focus
            .as_ref()
            .is_some_and(|focus| focus.same(&original))
    );
    close(tui);
}

#[test]
fn native_rows_scrollbar_drag_uses_presented_track_without_selecting_or_taking_focus() {
    let props = Rc::new(RefCell::new(props()));
    let (_runtime, mut tui) = layout::native(
        layout::View::Rows(props, Rc::new(Cell::new(false))),
        Rc::new(Cell::new(true)),
    );
    let buffer = draw(&mut tui, 40, 10);
    let original = tui.focus.clone().unwrap();
    assert_eq!(buffer[(39, 1)].symbol(), "┃");
    assert_eq!(buffer[(39, 3)].symbol(), "│");
    tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 39, 3))
        .unwrap();
    assert!(tui.drag.is_some());
    assert!(tui.focus.as_ref().unwrap().same(&original));
    let buffer = draw(&mut tui, 40, 10);
    assert!(text(&buffer).contains("pending 5"));
    assert!(!text(&buffer).contains("pending 0"));
    assert_eq!(buffer[(39, 3)].symbol(), "┃");
    tui.input_settled(mouse(MouseEventKind::Drag(MouseButton::Left), 0, 0))
        .unwrap();
    assert!(text(&draw(&mut tui, 40, 10)).contains("pending 0"));
    tui.input_settled(mouse(MouseEventKind::Drag(MouseButton::Left), 0, 9))
        .unwrap();
    assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
    tui.input_settled(mouse(MouseEventKind::Up(MouseButton::Left), 0, 9))
        .unwrap();
    assert!(tui.drag.is_none());
    assert!(tui.focus.as_ref().unwrap().same(&original));
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 0);
    // Keyboard entry reveals its original selection without another arrow key.
    tui.input_settled(key(KeyCode::Tab)).unwrap();
    assert!(text(&draw(&mut tui, 40, 10)).contains("pending 0"));
    tui.input_settled(key(KeyCode::Delete)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["observation"]["row"],
        "row-0"
    );
    close(tui);
}

#[test]
fn native_rows_scrollbar_style_and_interruption_keep_original_capture() {
    for interruption in [Event::FocusLost, Event::Resize(30, 10)] {
        let props = Rc::new(RefCell::new(props()));
        props.borrow_mut()["scrollbar"]["thumb"] = json!("#");
        props.borrow_mut()["scrollbar"]["track"] = json!("|");
        let modal = Rc::new(Cell::new(false));
        let (_runtime, mut tui) = layout::native(
            layout::View::Rows(props.clone(), modal.clone()),
            Rc::new(Cell::new(true)),
        );
        let buffer = draw(&mut tui, 40, 10);
        assert_eq!(buffer[(39, 1)].symbol(), "#");
        assert_eq!(buffer[(39, 3)].symbol(), "|");
        tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 39, 1))
            .unwrap();
        let area = Rect::new(0, 0, 40, 10);
        let mut buffer = Buffer::empty(area);
        let held = tui.paint(area, &mut buffer).unwrap();
        tui.input_settled(mouse(MouseEventKind::Drag(MouseButton::Left), 0, 9))
            .unwrap();
        tui.present(held);
        assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
        tui.input_settled(interruption).unwrap();
        assert!(tui.drag.is_none());
        tui.input_settled(mouse(MouseEventKind::Drag(MouseButton::Left), 0, 0))
            .unwrap();
        assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
        tui.input_settled(mouse(MouseEventKind::Down(MouseButton::Left), 39, 3))
            .unwrap();
        modal.set(true);
        refresh(&mut tui);
        draw(&mut tui, 40, 10);
        assert!(tui.drag.is_none());
        modal.set(false);
        refresh(&mut tui);
        draw(&mut tui, 40, 10);
        tui.input_settled(mouse(MouseEventKind::Drag(MouseButton::Left), 39, 1))
            .unwrap();
        assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
        close(tui);
    }
}

#[test]
fn native_rows_keyboard_entry_selects_arriving_rows_and_survives_late_paint_and_modal() {
    let rows = Rc::new(RefCell::new(props()));
    rows.borrow_mut()["rows"] = json!([]);
    let modal = Rc::new(Cell::new(false));
    let (_runtime, mut tui) = layout::native(
        layout::View::Rows(rows.clone(), modal.clone()),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 40, 10);
    let composer = tui.focus.clone().unwrap();
    *rows.borrow_mut() = props();
    rows.borrow_mut()["entry"] = json!("row-5");
    refresh(&mut tui);
    draw(&mut tui, 40, 10);
    let area = Rect::new(0, 0, 40, 10);
    let mut buffer = Buffer::empty(area);
    let old = tui.paint(area, &mut buffer).unwrap();
    tui.input_settled(key(KeyCode::Tab)).unwrap();
    tui.present(old);
    assert_eq!(tui.focus.as_ref().unwrap().key, "rows");
    assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
    tui.input_settled(key(KeyCode::Delete)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["observation"]["row"],
        "row-5"
    );
    tui.input_settled(key(KeyCode::Up)).unwrap();
    draw(&mut tui, 40, 10);
    modal.set(true);
    refresh(&mut tui);
    draw(&mut tui, 40, 10);
    modal.set(false);
    refresh(&mut tui);
    draw(&mut tui, 40, 10);
    tui.input_settled(key(KeyCode::Delete)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["observation"]["row"],
        "row-4"
    );
    tui.input_settled(key(KeyCode::Esc)).unwrap();
    assert!(tui.focus.as_ref().unwrap().same(&composer));
    tui.input_settled(key(KeyCode::BackTab)).unwrap();
    draw(&mut tui, 40, 10);
    tui.input_settled(key(KeyCode::Delete)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["observation"]["row"],
        "row-5"
    );
    close(tui);
}

#[test]
fn native_rows_keyboard_entry_waits_for_current_props_to_be_presented() {
    let rows = Rc::new(RefCell::new(props()));
    let (_runtime, mut tui) = layout::native(
        layout::View::Rows(rows.clone(), Rc::new(Cell::new(false))),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 40, 10);
    let composer = tui.focus.clone().unwrap();
    rows.borrow_mut()["entry"] = json!("row-5");
    refresh(&mut tui);
    tui.input_settled(key(KeyCode::Tab)).unwrap();
    assert!(tui.focus.as_ref().unwrap().same(&composer));
    draw(&mut tui, 40, 10);
    tui.input_settled(key(KeyCode::Tab)).unwrap();
    assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
    tui.input_settled(key(KeyCode::Delete)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["observation"]["row"],
        "row-5"
    );
    close(tui);
}

fn focus_scene() -> Value {
    let mut row_props = props();
    row_props["entry"] = json!("row-5");
    json!({"kind":"stack","key":"root","direction":"column","gap":0,"padding":0,"background":0,"children":[
        {"kind":"button","key":"back","label":"Outside"},
        {"kind":"stack","key":"local","direction":"column","gap":0,"padding":0,"background":0,
         "focus_shortcuts":[{"key":"up","modifiers":[],"child":"rows","enabled":true}],
         "children":[
            {"kind":"component","key":"editor","binding":"editor","member":0,"on":{},"props":{
                "appearance":support::brand_appearance()["textarea"],"accessibilityLabel":"Composer",
                "readOnly":false,"disabled":false,"softWrap":true,"tabIndex":0,"initialText":""}},
            {"kind":"button","key":"send","label":"Send"}]},
        {"kind":"component","key":"rows","binding":"rows","member":0,"props":row_props,"on":{"activated":"row-activated"}}
    ]})
}
fn badge_scene() -> Value {
    let mut scene = focus_scene();
    scene["children"][1]["children"][0]["props"]["tabIndex"] = json!(1);
    scene["children"][2]["props"]["tabIndex"] = json!(2);
    scene["children"][2]["props"]["compactBelow"] = json!({"height":12,"maxRows":1});
    scene["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"button","key":"badge","label":"≡ 6","focus_child":"rows"}));
    scene
}
#[test]
fn queue_count_enters_last_row_and_resize_keeps_visible_action_and_original_draft() {
    for activation in [None, Some(KeyCode::Enter), Some(KeyCode::Char(' '))] {
        let source = Rc::new(RefCell::new(badge_scene()));
        let (_runtime, mut tui) = layout::native(
            layout::View::InteractiveScene(source),
            Rc::new(Cell::new(true)),
        );
        draw(&mut tui, 40, 12);
        let editor = tui
            .scene
            .iter()
            .find(|target| target.key == "observe")
            .unwrap()
            .clone();
        let area = editor.area.visible();
        tui.input_settled(mouse(
            MouseEventKind::Down(MouseButton::Left),
            area.x + 1,
            area.y + 1,
        ))
        .unwrap();
        tui.input_settled(Event::Paste("original draft".into()))
            .unwrap();
        draw(&mut tui, 40, 12);
        let row = tui
            .scene
            .iter()
            .find(|target| target.key == "rows")
            .unwrap()
            .area
            .visible();
        tui.input_settled(mouse(MouseEventKind::ScrollDown, row.x, row.y))
            .unwrap();
        draw(&mut tui, 40, 12);
        // While typing, shrinking must retain the wheel offset, not reveal the old selection.
        let buffer = draw(&mut tui, 40, 11);
        assert!(text(&buffer).contains("pending 3"));
        assert!(!text(&buffer).contains("pending 0"));
        draw(&mut tui, 40, 12);
        let count = tui.application.frame(&tui.root).unwrap().state["count"].clone();
        if let Some(code) = activation {
            tui.input_settled(key(KeyCode::BackTab)).unwrap();
            assert_eq!(tui.focus.as_ref().unwrap().key, "badge");
            let mut held = KeyEvent::new(code, KeyModifiers::NONE);
            held.kind = KeyEventKind::Repeat;
            tui.input_settled(Event::Key(held)).unwrap();
            assert_eq!(tui.focus.as_ref().unwrap().key, "badge");
            tui.input_settled(key(code)).unwrap();
        } else {
            let badge = tui
                .scene
                .iter()
                .find(|target| target.key == "badge")
                .unwrap()
                .area
                .visible();
            tui.input_settled(mouse(
                MouseEventKind::Down(MouseButton::Left),
                badge.x,
                badge.y,
            ))
            .unwrap();
        }
        assert_eq!(tui.focus.as_ref().unwrap().key, "rows");
        assert_eq!(
            tui.application.frame(&tui.root).unwrap().state["count"],
            count
        );
        assert!(text(&draw(&mut tui, 40, 12)).contains("pending 5"));
        // A settled three-row viewport must still show the selected last row when reduced to one.
        let buffer = draw(&mut tui, 40, 11);
        assert!(text(&buffer).contains("pending 5"));
        assert!(!text(&buffer).contains("pending 3"));
        assert_eq!(
            tui.scene
                .iter()
                .find(|target| target.key == "rows")
                .unwrap()
                .area
                .height,
            1
        );
        tui.input_settled(key(KeyCode::Delete)).unwrap();
        assert_eq!(
            tui.application.frame(&tui.root).unwrap().state["observation"]["row"],
            "row-5"
        );
        draw(&mut tui, 40, 11);
        tui.input_settled(key(KeyCode::Esc)).unwrap();
        assert!(tui.focus.as_ref().unwrap().same(&editor));
        assert!(text(&draw(&mut tui, 40, 12)).contains("original draft"));
        close(tui);
    }
}
#[test]
fn queue_count_cannot_enter_unflushed_replacement_hidden_disabled_or_modal_targets() {
    for change in ["replace", "hidden", "disabled", "modal"] {
        let source = Rc::new(RefCell::new(badge_scene()));
        let (_runtime, mut tui) = layout::native(
            layout::View::InteractiveScene(source.clone()),
            Rc::new(Cell::new(true)),
        );
        draw(&mut tui, 40, 12);
        let badge = tui
            .scene
            .iter()
            .find(|target| target.key == "badge")
            .unwrap()
            .area
            .visible();
        match change {
            "replace" => {
                source.borrow_mut()["children"]
                    .as_array_mut()
                    .unwrap()
                    .truncate(2);
                source.borrow_mut()["children"][1]["focus_shortcuts"] = json!([]);
                refresh(&mut tui);
                *source.borrow_mut() = badge_scene();
            }
            "hidden" => {
                let row = source.borrow()["children"][2].clone();
                source.borrow_mut()["children"][2] = json!({"kind":"stack","key":"hidden","direction":"column","gap":0,"padding":0,"background":0,"visible":false,"children":[row]});
            }
            "disabled" => source.borrow_mut()["children"][3]["disabled"] = json!(true),
            _ => {
                let background = source.borrow().clone();
                *source.borrow_mut() = json!({"kind":"dialog","key":"modal","label":"Modal","open":true,"width":30,"height":8,"background_color":0,"border_color":0xffffff,"background":background,"content":{"kind":"button","key":"close","label":"Close"}});
            }
        }
        refresh(&mut tui);
        tui.input_settled(mouse(
            MouseEventKind::Down(MouseButton::Left),
            badge.x,
            badge.y,
        ))
        .unwrap();
        assert!(tui.focus.as_ref().is_none_or(|focus| focus.key != "rows"));
        draw(&mut tui, 40, 12);
        if change == "replace" {
            tui.input_settled(mouse(
                MouseEventKind::Down(MouseButton::Left),
                badge.x,
                badge.y,
            ))
            .unwrap();
            assert_eq!(tui.focus.as_ref().unwrap().key, "rows");
        }
        close(tui);
    }
}
#[test]
fn native_focus_shortcut_is_local_and_selects_original_child_entry() {
    let source = Rc::new(RefCell::new(focus_scene()));
    let (_runtime, mut tui) = layout::native(
        layout::View::InteractiveScene(source),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 40, 10);
    let outside = tui.focus.clone().unwrap();
    assert_eq!(outside.key, "back");
    tui.input_settled(key(KeyCode::Up)).unwrap();
    assert!(tui.focus.as_ref().unwrap().same(&outside));
    tui.input_settled(key(KeyCode::Tab)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "observe");
    let mut held = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    held.kind = KeyEventKind::Repeat;
    tui.input_settled(Event::Key(held)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "observe");
    tui.input_settled(Event::Paste("draft".into())).unwrap();
    tui.input_settled(key(KeyCode::Up)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "observe");
    tui.input_settled(Event::Key(KeyEvent::new(
        KeyCode::Char('a'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    tui.input_settled(key(KeyCode::Backspace)).unwrap();
    draw(&mut tui, 40, 10);
    tui.input_settled(key(KeyCode::Tab)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "send");
    tui.input_settled(key(KeyCode::Up)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "send");
    tui.input_settled(key(KeyCode::BackTab)).unwrap();
    tui.input_settled(key(KeyCode::Up)).unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "rows");
    assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
    tui.input_settled(key(KeyCode::Delete)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["observation"]["row"],
        "row-5"
    );
    close(tui);
}
#[test]
fn native_focus_shortcut_never_retargets_an_unflushed_replacement_or_hidden_child() {
    for replace in [true, false] {
        let source = Rc::new(RefCell::new(focus_scene()));
        let (_runtime, mut tui) = layout::native(
            layout::View::InteractiveScene(source.clone()),
            Rc::new(Cell::new(true)),
        );
        draw(&mut tui, 40, 10);
        tui.input_settled(key(KeyCode::Tab)).unwrap();
        let composer = tui.focus.clone().unwrap();
        if replace {
            // Remove then recreate the same declared name: the old physical
            // shortcut must not resolve the replacement by that name.
            source.borrow_mut()["children"]
                .as_array_mut()
                .unwrap()
                .pop();
            source.borrow_mut()["children"][1]["focus_shortcuts"] = json!([]);
            refresh(&mut tui);
            *source.borrow_mut() = focus_scene();
            refresh(&mut tui);
        } else {
            let child = source.borrow()["children"][2].clone();
            source.borrow_mut()["children"][2] = json!({"kind":"stack","key":"hidden","direction":"column","gap":0,"padding":0,"background":0,"visible":false,"children":[child]});
            refresh(&mut tui);
        }
        tui.input_settled(key(KeyCode::Up)).unwrap();
        assert!(tui.focus.as_ref().unwrap().same(&composer));
        draw(&mut tui, 40, 10);
        tui.input_settled(key(KeyCode::Up)).unwrap();
        assert_eq!(
            tui.focus.as_ref().unwrap().key,
            if replace { "rows" } else { "observe" }
        );
        close(tui);
    }
}

#[test]
fn native_focus_shortcut_respects_disabled_scope_and_modal_boundary() {
    let mut scene = focus_scene();
    scene["focus_shortcuts"] = scene["children"][1]["focus_shortcuts"].clone();
    scene["children"][1]["focus_shortcuts"][0]["enabled"] = json!(false);
    let source = Rc::new(RefCell::new(scene));
    let (_runtime, mut tui) = layout::native(
        layout::View::InteractiveScene(source.clone()),
        Rc::new(Cell::new(true)),
    );
    draw(&mut tui, 40, 20);
    tui.input_settled(key(KeyCode::Tab)).unwrap();
    let editor = tui.focus.clone().unwrap();
    tui.input_settled(key(KeyCode::Up)).unwrap();
    assert!(tui.focus.as_ref().unwrap().same(&editor));
    let background = focus_scene();
    let mut foreground = background["children"][1].clone();
    foreground["key"] = json!("foreground");
    foreground["children"][0]["key"] = json!("foreground-editor");
    foreground["children"].as_array_mut().unwrap().pop();
    *source.borrow_mut() = json!({"kind":"dialog","key":"modal","label":"Modal","open":true,
        "width":30,"height":10,"background_color":0,"border_color":0xffffff,
        "background":background,"content":foreground});
    refresh(&mut tui);
    draw(&mut tui, 40, 20);
    let foreground = tui.focus.clone().unwrap();
    assert!(!foreground.same(&editor));
    assert_eq!(foreground.key, "observe");
    tui.input_settled(key(KeyCode::Up)).unwrap();
    assert!(tui.focus.as_ref().unwrap().same(&foreground));
    close(tui);
}
#[test]
fn focus_shortcuts_require_declared_children_and_unambiguous_chords() {
    let compile = |value: &Value| super::super::scene::compile(value);
    let mut scene = focus_scene();
    scene["children"][1]["focus_shortcuts"][0]["child"] = json!("missing");
    assert!(compile(&scene).is_err());
    let mut scene = badge_scene();
    scene["children"][3]["focus_child"] = json!("missing");
    assert!(compile(&scene).is_err());
    let mut scene = focus_scene();
    scene["children"][1]["shortcuts"] =
        json!([{"key":"up","modifiers":[],"input":"back","enabled":true}]);
    assert!(compile(&scene).is_err());
}
