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
    tui.sync_native().unwrap();
}
fn close(mut tui: Presenter) {
    tui.surface_closed(&tui.surface(), Ok(())).unwrap();
    tui.shutdown().unwrap();
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
    tui.input(mouse(MouseEventKind::Moved, 2, 2)).unwrap();
    assert!(tui.focus.as_ref().unwrap().same(&focus));
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 0);
    let area = Rect::new(0, 0, 40, 10);
    let mut buffer = Buffer::empty(area);
    let pending = tui.paint(area, &mut buffer).unwrap();
    assert!(text(&buffer).contains("Delete"));
    // Visible in an unflushed candidate only: the last actual geometry is preview.
    tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 36, 2))
        .unwrap();
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 0);
    drop(pending);
    draw(&mut tui, 40, 10);
    tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 36, 2))
        .unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["observation"],
        json!({"revision":1,"row":"row-1","action":"delete"})
    );
    draw(&mut tui, 40, 10);
    tui.input(key(KeyCode::Esc)).unwrap();
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
    tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 2, 2))
        .unwrap();
    assert_eq!(tui.focus.as_ref().unwrap().key, "rows");
    props.borrow_mut()["rows"].as_array_mut().unwrap().remove(1);
    props.borrow_mut()["revision"] = json!(2);
    refresh(&mut tui);
    assert_eq!(tui.focus.as_ref().unwrap().key, "back");
    let count = tui.application.frame(&tui.root).unwrap().state["count"].clone();
    let mut held = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    held.kind = KeyEventKind::Repeat;
    tui.input(Event::Key(held)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["count"],
        count
    );
    draw(&mut tui, 40, 10);
    tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 2, 2))
        .unwrap();
    draw(&mut tui, 40, 10);
    tui.input(key(KeyCode::Delete)).unwrap();
    assert_eq!(
        tui.application.frame(&tui.root).unwrap().state["observation"]["row"],
        "row-2"
    );
    draw(&mut tui, 40, 10);
    tui.input(key(KeyCode::BackTab)).unwrap();
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
    tui.input(mouse(MouseEventKind::Moved, 2, 2)).unwrap();
    assert!(tui.hover.is_some());
    modal.set(true);
    refresh(&mut tui);
    // Opening is pending; input cannot reach the original background rows.
    tui.input(mouse(MouseEventKind::Moved, 2, 2)).unwrap();
    assert!(tui.hover.is_none());
    draw(&mut tui, 40, 10);
    assert!(tui.scene.iter().all(|target| target.key != "rows"));
    modal.set(false);
    refresh(&mut tui);
    draw(&mut tui, 40, 10);
    tui.input(mouse(MouseEventKind::ScrollDown, 2, 2)).unwrap();
    let buffer = draw(&mut tui, 40, 10);
    assert!(text(&buffer).contains("pending 5"));
    assert!(!text(&buffer).contains("pending 0"));
    assert_eq!(tui.focus.as_ref().unwrap().key, "back");
    tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 2, 1))
        .unwrap();
    tui.input(key(KeyCode::Home)).unwrap();
    let buffer = draw(&mut tui, 40, 10);
    assert!(text(&buffer).contains("pending 0"));
    tui.input(key(KeyCode::End)).unwrap();
    assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
    tui.input(key(KeyCode::Tab)).unwrap();
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
        tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 2, 1))
            .unwrap();
        draw(&mut tui, 40, 10);
        let area = Rect::new(0, 0, 40, 10);
        let mut buffer = Buffer::empty(area);
        let held = tui.paint(area, &mut buffer).unwrap();
        if wheel {
            tui.input(mouse(MouseEventKind::ScrollDown, 2, 2)).unwrap();
        } else {
            tui.input(key(KeyCode::End)).unwrap();
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
    tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 2, 1))
        .unwrap();
    let mut repeat = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    repeat.kind = KeyEventKind::Repeat;
    tui.input(Event::Key(repeat)).unwrap();
    draw(&mut tui, 40, 10);
    repeat.code = KeyCode::Delete;
    tui.input(Event::Key(repeat)).unwrap();
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 0);
    tui.input(key(KeyCode::Delete)).unwrap();
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
        tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 2, 2))
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
            tui.input(Event::Key(held)).unwrap();
        }
        assert_eq!(
            tui.application.frame(&tui.root).unwrap().state["count"],
            count
        );
        // A fresh explicit selection can enter a remaining row; no stale exit follows it.
        tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 2, 2))
            .unwrap();
        draw(&mut tui, 40, 10);
        tui.input(key(KeyCode::Delete)).unwrap();
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
    tui.input(key(KeyCode::Tab)).unwrap();
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
    tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 39, 3))
        .unwrap();
    assert!(tui.drag.is_some());
    assert!(tui.focus.as_ref().unwrap().same(&original));
    let buffer = draw(&mut tui, 40, 10);
    assert!(text(&buffer).contains("pending 5"));
    assert!(!text(&buffer).contains("pending 0"));
    assert_eq!(buffer[(39, 3)].symbol(), "┃");
    tui.input(mouse(MouseEventKind::Drag(MouseButton::Left), 0, 0))
        .unwrap();
    assert!(text(&draw(&mut tui, 40, 10)).contains("pending 0"));
    tui.input(mouse(MouseEventKind::Drag(MouseButton::Left), 0, 9))
        .unwrap();
    assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
    tui.input(mouse(MouseEventKind::Up(MouseButton::Left), 0, 9))
        .unwrap();
    assert!(tui.drag.is_none());
    assert!(tui.focus.as_ref().unwrap().same(&original));
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 0);
    // The selected row was not transferred to the row underneath the track.
    tui.input(key(KeyCode::Tab)).unwrap();
    tui.input(key(KeyCode::Up)).unwrap();
    assert!(text(&draw(&mut tui, 40, 10)).contains("pending 0"));
    tui.input(key(KeyCode::Delete)).unwrap();
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
        tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 39, 1))
            .unwrap();
        let area = Rect::new(0, 0, 40, 10);
        let mut buffer = Buffer::empty(area);
        let held = tui.paint(area, &mut buffer).unwrap();
        tui.input(mouse(MouseEventKind::Drag(MouseButton::Left), 0, 9))
            .unwrap();
        tui.present(held);
        assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
        tui.input(interruption).unwrap();
        assert!(tui.drag.is_none());
        tui.input(mouse(MouseEventKind::Drag(MouseButton::Left), 0, 0))
            .unwrap();
        assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
        tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 39, 3))
            .unwrap();
        modal.set(true);
        refresh(&mut tui);
        draw(&mut tui, 40, 10);
        assert!(tui.drag.is_none());
        modal.set(false);
        refresh(&mut tui);
        draw(&mut tui, 40, 10);
        tui.input(mouse(MouseEventKind::Drag(MouseButton::Left), 39, 1))
            .unwrap();
        assert!(text(&draw(&mut tui, 40, 10)).contains("pending 5"));
        close(tui);
    }
}
