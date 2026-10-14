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

use super::dialog::{column, root_count};
use super::*;

fn menu() -> Value {
    json!({"kind":"context_menu","key":"message-menu","label":"Message", "width":24,"height":8,"padding":0,"background_color":0x17171a,"border_color":0x888888,
        "background":source("body", "Original message 中\nsecond line"),
        "content":column("actions", vec![
            json!({"kind":"button","key":"copy-message","label":"Copy","copy_text":"Original message 中\nsecond line"}),
            json!({"kind":"button","key":"details","label":"Details"})])})
}
#[test]
fn context_menu_physical_open_copy_close_and_focus_return() {
    let (_runtime, mut p, _) = native(vec![menu()]);
    draw(&mut p, 40, 12);
    mouse(&mut p, MouseEventKind::Down(MouseButton::Left), 1, 0);
    draw(&mut p, 40, 12);
    let focus = p.focus.clone().unwrap();
    mouse(&mut p, MouseEventKind::Down(MouseButton::Right), 1, 0);
    let before = root_count(&p);
    assert!(!p.physical_admitted());
    p.input_settled(key(KeyCode::Enter)).unwrap();
    assert_eq!(root_count(&p), before);
    draw(&mut p, 40, 12);
    assert_eq!(p.physical_path.len(), 1);
    let action = p
        .scene
        .iter()
        .find(|t| t.key == "copy-message")
        .unwrap()
        .area
        .visible();
    mouse(
        &mut p,
        MouseEventKind::Down(MouseButton::Left),
        action.x,
        action.y,
    );
    assert!(state(&p).dialogs.is_empty());
    let mut bytes = Vec::new();
    assert!(p.flush_clipboard(&mut bytes).unwrap());
    assert_eq!(
        STANDARD.decode(&bytes[7..bytes.len() - 1]).unwrap(),
        "Original message 中\nsecond line".as_bytes()
    );
    draw(&mut p, 40, 12);
    assert!(p.focus.as_ref().is_some_and(|current| current.same(&focus)));
    assert!(p.physical_path.is_empty());
    close(&mut p);
}
#[test]
fn context_menu_visible_trigger_keyboard_escape_and_narrow_bounds() {
    let (_runtime, mut p, _) = native(vec![menu()]);
    draw(&mut p, 40, 12);
    let trigger = p
        .scene
        .iter()
        .find(|t| {
            matches!(
                t.kind,
                super::super::super::paint::TargetKind::ContextMenu { button: true }
            )
        })
        .unwrap()
        .clone();
    mouse(
        &mut p,
        MouseEventKind::Down(MouseButton::Left),
        trigger.area.visible().x,
        trigger.area.visible().y,
    );
    draw(&mut p, 40, 12);
    assert_eq!(p.physical_path.len(), 1);
    draw(&mut p, 15, 6);
    let menu = p
        .scene
        .iter()
        .find_map(|t| {
            if let super::super::super::paint::TargetKind::Backdrop { body } = t.kind {
                Some(body.visible())
            } else {
                None
            }
        })
        .unwrap();
    assert!(menu.right() <= 15 && menu.bottom() <= 6);
    p.input_settled(key(KeyCode::Esc)).unwrap();
    draw(&mut p, 15, 6);
    assert!(p.physical_path.is_empty());
    mouse(&mut p, MouseEventKind::Down(MouseButton::Left), 1, 0);
    draw(&mut p, 15, 6);
    p.input_settled(Event::Key(KeyEvent::new(
        KeyCode::F(10),
        KeyModifiers::SHIFT,
    )))
    .unwrap();
    draw(&mut p, 15, 6);
    assert_eq!(p.physical_path.len(), 1);
    mouse(&mut p, MouseEventKind::Down(MouseButton::Left), 14, 5);
    draw(&mut p, 15, 6);
    assert!(p.physical_path.is_empty());
    close(&mut p);
}

#[test]
fn context_menu_copies_original_selection_without_business_event() {
    let mut node = menu();
    node["copy_selection_label"] = json!("Copy selection");
    let (_runtime, mut p, _) = native(vec![node]);
    draw(&mut p, 40, 12);
    mouse(&mut p, MouseEventKind::Down(MouseButton::Left), 1, 0);
    all(&mut p);
    draw(&mut p, 40, 12);
    mouse(&mut p, MouseEventKind::Down(MouseButton::Right), 1, 0);
    draw(&mut p, 40, 12);
    assert!(matches!(
        p.focus.as_ref().unwrap().kind,
        super::super::super::paint::TargetKind::SelectionCopy { .. }
    ));
    p.input_settled(key(KeyCode::Down)).unwrap();
    assert_eq!(p.focus.as_ref().unwrap().key, "copy-message");
    p.input_settled(key(KeyCode::Up)).unwrap();
    // Repainting must retain the copy action, not the same node's backdrop.
    draw(&mut p, 40, 12);
    p.input_settled(key(KeyCode::Enter)).unwrap();
    let mut bytes = Vec::new();
    assert!(p.flush_clipboard(&mut bytes).unwrap());
    assert_eq!(
        STANDARD.decode(&bytes[7..bytes.len() - 1]).unwrap(),
        "Original message 中\nsecond line".as_bytes()
    );
    draw(&mut p, 40, 12);
    assert!(p.physical_path.is_empty());
    close(&mut p);
}

#[test]
fn context_action_replacing_menu_with_dialog_keeps_original_focus_return() {
    let (_runtime, mut p, props) = native(vec![
        json!({"kind":"button","key":"before","label":"Before"}),
        super::dialog::dialog(
            "confirmation",
            false,
            menu(),
            json!({"kind":"button","key":"cancel","label":"Cancel"}),
        ),
    ]);
    draw(&mut p, 40, 12);
    mouse(&mut p, MouseEventKind::Down(MouseButton::Left), 1, 1);
    draw(&mut p, 40, 12);
    let original = p.focus.clone().unwrap();
    mouse(&mut p, MouseEventKind::Down(MouseButton::Right), 1, 1);
    draw(&mut p, 40, 12);
    p.input_settled(key(KeyCode::Down)).unwrap();
    draw(&mut p, 40, 12);
    assert_eq!(p.focus.as_ref().unwrap().key, "details");
    props.borrow_mut()["content"]["children"][1]["open"] = json!(true);
    p.input_settled(key(KeyCode::Enter)).unwrap();
    draw(&mut p, 40, 12);
    assert_eq!(p.physical_path.len(), 1);
    assert_eq!(p.focus.as_ref().unwrap().key, "cancel");
    assert!(!state(&p).dialogs.contains_key("message-menu"));
    props.borrow_mut()["content"]["children"][1]["open"] = json!(false);
    refresh(&mut p);
    draw(&mut p, 40, 12);
    assert!(p.focus.as_ref().unwrap().same(&original));
    close(&mut p);
}
