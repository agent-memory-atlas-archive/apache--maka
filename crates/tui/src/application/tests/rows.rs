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
        "maxRows":3,"tabIndex":0,"foreground":0xffffffffu32,"muted":0x888888ffu32,"selected":0x222222ffu32,"hovered":0x333333ffu32})
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
    let rev = tui.application.frame(&tui.root).unwrap().revision;
    tui.application
        .dispatch(&tui.root.clone(), rev, "back", json!({"kind":"click"}))
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
    tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 37, 2))
        .unwrap();
    assert_eq!(tui.application.frame(&tui.root).unwrap().state["count"], 0);
    drop(pending);
    draw(&mut tui, 40, 10);
    tui.input(mouse(MouseEventKind::Down(MouseButton::Left), 37, 2))
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
