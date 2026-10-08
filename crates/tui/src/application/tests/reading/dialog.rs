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

fn dialog(key: &str, open: bool, background: Value, content: Value) -> Value {
    json!({"kind":"dialog","key":key,"label":"Commands","open":open,
        "width":24,"height":8,"padding":0,"background_color":0x17171a,"border_color":0x888888,
        "background":background,"content":content})
}
fn column(key: &str, children: Vec<Value>) -> Value {
    json!({"kind":"stack","key":key,"direction":"column","gap":0,"padding":0,"background":0,"children":children})
}
fn open(props: &Rc<RefCell<Value>>, presenter: &mut Presenter, value: bool) {
    props.borrow_mut()["content"]["children"][0]["open"] = json!(value);
    refresh(presenter);
}
fn pending(presenter: &Presenter, width: u16, height: u16) -> PaintedScene {
    let area = Rect::new(0, 0, width, height);
    presenter.paint(area, &mut Buffer::empty(area)).unwrap()
}
fn root_count(presenter: &Presenter) -> u64 {
    presenter.application.frame(&presenter.root).unwrap().state["count"]
        .as_u64()
        .unwrap()
}

#[test]
fn dialog_original_foreground_selection_and_flush_gate_preserve_background_bytes() {
    let bg = "background 中\r\n  ";
    let fg = "foreground 界  ";
    let (_runtime, mut presenter, props) = native(vec![dialog(
        "modal",
        false,
        source("bg", bg),
        source("fg", fg),
    )]);
    draw(&mut presenter, 40, 12);
    all(&mut presenter);
    assert_eq!(copied(&mut presenter).unwrap(), bg.as_bytes());
    let bg_scope = state(&presenter).reading.as_ref().unwrap().scope.clone();
    let original_focus = presenter.focus.clone().unwrap();
    open(&props, &mut presenter, true);
    assert!(presenter.focus.is_none());
    assert!(!presenter.physical_admitted());
    assert!(copied(&mut presenter).is_none());
    presenter.input(key(KeyCode::Tab)).unwrap();
    assert!(presenter.focus.is_none());
    assert_eq!(bg_scope.copy().unwrap().unwrap(), bg);
    drop(pending(&presenter, 40, 12));
    assert!(!presenter.physical_admitted());
    draw(&mut presenter, 40, 12);
    assert!(presenter.physical_admitted());
    assert!(presenter.scene.iter().all(|target| target.scope.is_some()));
    assert!(!presenter.target_admitted(&original_focus));
    all(&mut presenter);
    assert_eq!(copied(&mut presenter).unwrap(), fg.as_bytes());
    assert_eq!(bg_scope.copy().unwrap().unwrap(), bg);
    open(&props, &mut presenter, false);
    assert!(!presenter.physical_admitted());
    assert!(copied(&mut presenter).is_none());
    draw(&mut presenter, 40, 12);
    assert!(presenter.focus.as_ref().unwrap().same(&original_focus));
    assert_eq!(copied(&mut presenter).unwrap(), bg.as_bytes());
    close(&mut presenter);
}

#[test]
fn dialog_held_same_recipe_reopen_rejects_entire_original_geometry_batch() {
    let (_runtime, mut presenter, props) = native(vec![dialog(
        "modal",
        true,
        source("bg", "background"),
        source("fg", "foreground"),
    )]);
    draw(&mut presenter, 40, 12);
    let old = presenter.physical_path.last().cloned().unwrap();
    let held = pending(&presenter, 12, 5);
    let bg_mount = presenter
        .application
        .child_mount(&presenter.root, "bg")
        .unwrap()
        .clone();
    let fg_mount = presenter
        .application
        .child_mount(&presenter.root, "fg")
        .unwrap()
        .clone();
    let geometry = |p: &Presenter, mount: &Mount| {
        super::super::super::source::painted_area(p.resources.owners[&mount.id()].state.as_ref())
            .unwrap()
    };
    let original_bg = geometry(&presenter, &bg_mount);
    let original_fg = geometry(&presenter, &fg_mount);
    open(&props, &mut presenter, false);
    open(&props, &mut presenter, true);
    assert_eq!(old.strong_count(), 0);
    presenter.present(held);
    assert!(!presenter.physical_admitted());
    assert_eq!(geometry(&presenter, &bg_mount), original_bg);
    assert_eq!(geometry(&presenter, &fg_mount), original_fg);
    draw(&mut presenter, 30, 10);
    assert!(presenter.physical_admitted());
    assert!(!old.ptr_eq(presenter.physical_path.last().unwrap()));
    close(&mut presenter);
}

#[test]
fn dialog_requests_dismissal_without_implicit_enter_or_background_shortcuts() {
    let mut background = column(
        "bg",
        vec![json!({"kind":"button","key":"back","label":"Back"})],
    );
    background["shortcuts"] =
        json!([{"key":"k","modifiers":["control"],"input":"back","enabled":true}]);
    let foreground = column(
        "fg",
        vec![json!({"kind":"text","key":"note","text":"Choose","color":0xffffff})],
    );
    let (_runtime, mut presenter, props) =
        native(vec![dialog("modal", true, background, foreground)]);
    draw(&mut presenter, 40, 12);
    assert!(presenter.focus.is_none());
    presenter.sync_native().unwrap();
    draw(&mut presenter, 40, 12);
    let count = root_count(&presenter);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(root_count(&presenter), count);
    presenter.input(key(KeyCode::Esc)).unwrap();
    assert_eq!(root_count(&presenter), count + 1);
    assert_eq!(props.borrow()["content"]["children"][0]["open"], true);
    draw(&mut presenter, 40, 12);
    let body = presenter
        .scene
        .iter()
        .find_map(|target| {
            if let super::super::super::paint::TargetKind::Backdrop { body } = target.kind {
                Some(body.visible())
            } else {
                None
            }
        })
        .unwrap();
    mouse(
        &mut presenter,
        MouseEventKind::Down(MouseButton::Left),
        body.x,
        body.y,
    );
    assert_eq!(root_count(&presenter), count + 1);
    mouse(
        &mut presenter,
        MouseEventKind::Down(MouseButton::Left),
        0,
        0,
    );
    assert_eq!(root_count(&presenter), count + 2);
    close(&mut presenter);
}

#[test]
fn dialog_nested_close_restores_original_parent_then_background_focus() {
    let nested = dialog(
        "inner",
        false,
        source("outer-source", "outer"),
        source("inner-source", "inner"),
    );
    let (_runtime, mut presenter, props) = native(vec![dialog(
        "outer",
        false,
        source("bg", "background"),
        nested,
    )]);
    draw(&mut presenter, 40, 12);
    let bg = presenter.focus.clone().unwrap();
    open(&props, &mut presenter, true);
    draw(&mut presenter, 40, 12);
    let outer = presenter.focus.clone().unwrap();
    let outer_scope = presenter.physical_path.last().cloned().unwrap();
    props.borrow_mut()["content"]["children"][0]["content"]["open"] = json!(true);
    refresh(&mut presenter);
    draw(&mut presenter, 40, 12);
    assert!(!presenter.physical_path.last().unwrap().ptr_eq(&outer_scope));
    assert!(!presenter.target_admitted(&outer));
    props.borrow_mut()["content"]["children"][0]["content"]["open"] = json!(false);
    refresh(&mut presenter);
    draw(&mut presenter, 40, 12);
    assert!(presenter.physical_path.last().unwrap().ptr_eq(&outer_scope));
    assert!(presenter.focus.as_ref().unwrap().same(&outer));
    open(&props, &mut presenter, false);
    draw(&mut presenter, 40, 12);
    assert!(presenter.focus.as_ref().unwrap().same(&bg));
    close(&mut presenter);
}

#[test]
fn dialog_open_interrupts_original_drag_without_erasing_selection() {
    let (_runtime, mut presenter, props) = native(vec![dialog(
        "modal",
        false,
        source("bg", "background selection"),
        source("fg", "foreground"),
    )]);
    draw(&mut presenter, 40, 12);
    mouse(
        &mut presenter,
        MouseEventKind::Down(MouseButton::Left),
        0,
        0,
    );
    mouse(
        &mut presenter,
        MouseEventKind::Drag(MouseButton::Left),
        7,
        0,
    );
    let scope = state(&presenter).reading.as_ref().unwrap().scope.clone();
    assert!(scope.has_pointer_capture());
    let selected = scope.copy().unwrap();
    open(&props, &mut presenter, true);
    assert!(!scope.has_pointer_capture());
    assert!(presenter.drag.is_none());
    assert_eq!(scope.copy().unwrap(), selected);
    mouse(
        &mut presenter,
        MouseEventKind::Drag(MouseButton::Left),
        18,
        0,
    );
    mouse(&mut presenter, MouseEventKind::Up(MouseButton::Left), 18, 0);
    assert_eq!(scope.copy().unwrap(), selected);
    open(&props, &mut presenter, false);
    draw(&mut presenter, 40, 12);
    assert!(!scope.has_pointer_capture());
    assert_eq!(scope.copy().unwrap(), selected);
    close(&mut presenter);
}

#[test]
fn dialog_foreground_scroll_keeps_background_reading_command_and_tail_revision() {
    let mut foreground = column(
        "fg-scroll",
        vec![source(
            "fg",
            "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight",
        )],
    );
    foreground["scroll"] = json!("manual");
    let (_runtime, mut presenter, props) = native(vec![dialog(
        "modal",
        true,
        source("bg", "background"),
        foreground,
    )]);
    draw(&mut presenter, 40, 12);
    let scope = state(&presenter).reading.as_ref().unwrap().scope.clone();
    let revision = scope.revision();
    let intent = scope.intent();
    assert!(state(&presenter).reading.as_ref().unwrap().eligible());
    props.borrow_mut()["command"] = json!({"sequence":1,"target":{"kind":"end"}});
    refresh(&mut presenter);
    assert!(
        state(&presenter)
            .reading
            .as_ref()
            .unwrap()
            .pending
            .is_some()
    );
    let command_revision = scope.revision();
    presenter.input(key(KeyCode::PageDown)).unwrap();
    assert!(
        state(&presenter)
            .reading
            .as_ref()
            .unwrap()
            .pending
            .is_some()
    );
    assert_eq!(scope.revision(), command_revision);
    draw(&mut presenter, 40, 12);
    presenter.sync_native().unwrap();
    assert_eq!(
        observation(&presenter)["receipt"],
        json!({"sequence":1,"status":"applied"})
    );
    let settled_revision = scope.revision();
    let settled_intent = scope.intent();
    presenter.input(key(KeyCode::PageUp)).unwrap();
    assert_eq!(scope.revision(), settled_revision);
    assert_eq!(scope.intent(), settled_intent);
    assert!(state(&presenter).reading.as_ref().unwrap().eligible());
    assert!(revision <= settled_revision && intent <= settled_intent);
    close(&mut presenter);
}

#[test]
fn dialog_foreground_is_bounded_even_in_one_cell_viewport() {
    let (_runtime, mut presenter, _props) = native(vec![dialog(
        "modal",
        true,
        source("bg", "background"),
        source("fg", "foreground"),
    )]);
    for (width, height) in [(40, 12), (8, 4), (1, 1)] {
        draw(&mut presenter, width, height);
        let viewport = Rect::new(0, 0, width, height);
        assert!(presenter.scene.iter().all(|target| target.area.visible().intersection(viewport)==target.area.visible()));
        assert!(presenter.physical_admitted());
    }
    close(&mut presenter);
}

#[test]
fn dialog_choice_button_escape_uses_original_native_first_dismiss_chain() {
    let foreground = column(
        "choices",
        vec![json!({"kind":"button","key":"choice","label":"Select"})],
    );
    let (_runtime, mut presenter, _props) = native(vec![dialog(
        "modal",
        true,
        source("bg", "background"),
        foreground,
    )]);
    draw(&mut presenter, 40, 12);
    assert_eq!(presenter.focus.as_ref().unwrap().key, "choice");
    presenter.sync_native().unwrap();
    draw(&mut presenter, 40, 12);
    let count = root_count(&presenter);
    presenter.input(key(KeyCode::Esc)).unwrap();
    assert_eq!(root_count(&presenter), count + 1);
    close(&mut presenter);
}

#[test]
fn dialog_retained_foreground_wrapped_by_new_outer_keeps_original_second_target() {
    let choices = column(
        "choices",
        vec![
            json!({"kind":"button","key":"one","label":"One"}),
            json!({"kind":"button","key":"two","label":"Two"}),
        ],
    );
    let original = dialog("retained", true, source("bg", "background"), choices);
    let (_runtime, mut presenter, props) = native(vec![original.clone()]);
    draw(&mut presenter, 40, 12);
    presenter.input(key(KeyCode::Tab)).unwrap();
    let second = presenter.focus.clone().unwrap();
    assert_eq!(second.key, "two");
    props.borrow_mut()["content"]["children"][0] = dialog(
        "new-outer",
        true,
        json!({"kind":"text","key":"outer-background","text":"","color":0}),
        original,
    );
    refresh(&mut presenter);
    draw(&mut presenter, 40, 12);
    assert!(presenter.focus.as_ref().unwrap().same(&second));
    props.borrow_mut()["content"]["children"][0]["content"]["open"] = json!(false);
    refresh(&mut presenter);
    draw(&mut presenter, 40, 12);
    assert!(presenter.scene.iter().all(|target| !target.same(&second)));
    close(&mut presenter);
}

#[test]
fn dialog_large_legal_padding_keeps_native_content_target_reachable() {
    let mut modal = dialog(
        "modal",
        true,
        source("bg", "background"),
        json!({"kind":"button","key":"choice","label":"Choice"}),
    );
    modal["width"] = json!(64);
    modal["height"] = json!(16);
    modal["padding"] = json!(32);
    let (_runtime, mut presenter, _props) = native(vec![modal]);
    for (width, height) in [(80, 24), (8, 5)] {
        draw(&mut presenter, width, height);
        assert_eq!(presenter.focus.as_ref().unwrap().key, "choice");
        assert!(!presenter.focus.as_ref().unwrap().area.is_empty());
    }
    let count = root_count(&presenter);
    presenter.input(key(KeyCode::Esc)).unwrap();
    assert_eq!(root_count(&presenter), count + 1);
    close(&mut presenter);
}

#[test]
fn dialog_unplaced_original_slot_acknowledges_flush_and_allows_actual_background() {
    let props = Rc::new(RefCell::new(props(vec![dialog(
        "modal",
        true,
        source("bg", "background"),
        source("fg", "foreground"),
    )])));
    let shown = Rc::new(Cell::new(false));
    let (_runtime, mut presenter) = layout::native(
        layout::View::OmittedReadingSlot(props, shown.clone()),
        Rc::new(Cell::new(true)),
    );
    assert!(!presenter.physical_admitted());
    draw(&mut presenter, 40, 12);
    assert!(presenter.physical_admitted());
    assert!(presenter.physical_path.is_empty());
    assert_eq!(presenter.focus.as_ref().unwrap().key, "back");
    shown.set(true);
    draw(&mut presenter, 40, 12);
    assert!(!presenter.physical_path.is_empty());
    assert!(presenter.scene.iter().all(|target| target.scope.is_some()));
    shown.set(false);
    draw(&mut presenter, 40, 12);
    assert!(presenter.physical_path.is_empty());
    assert_eq!(presenter.focus.as_ref().unwrap().key, "back");
    close(&mut presenter);
}

#[test]
fn dialog_foreground_native_escape_clears_selection_then_requests_dismissal() {
    let (_runtime, mut presenter, _props) = native(vec![dialog(
        "modal",
        true,
        source("bg", "background"),
        source("fg", "foreground"),
    )]);
    draw(&mut presenter, 40, 12);
    all(&mut presenter);
    draw(&mut presenter, 40, 12);
    presenter.sync_native().unwrap();
    draw(&mut presenter, 40, 12);
    let count = root_count(&presenter);
    presenter.input(key(KeyCode::Esc)).unwrap();
    assert_eq!(root_count(&presenter), count);
    assert!(copied(&mut presenter).is_none());
    draw(&mut presenter, 40, 12);
    presenter.input(key(KeyCode::Esc)).unwrap();
    assert_eq!(root_count(&presenter), count + 1);
    close(&mut presenter);
}

#[test]
fn dialog_live_covered_sibling_keeps_second_focus_and_original_background_return() {
    let choices = column(
        "choices",
        vec![
            json!({"kind":"button","key":"one","label":"One"}),
            json!({"kind":"button","key":"two","label":"Two"}),
        ],
    );
    let first = dialog("first", false, source("bg", "background"), choices);
    let second = dialog(
        "second",
        false,
        json!({"kind":"text","key":"spacer","text":"","color":0}),
        json!({"kind":"button","key":"cover-choice","label":"Cover"}),
    );
    let (_runtime, mut presenter, props) = native(vec![first, second]);
    draw(&mut presenter, 40, 12);
    let background = presenter.focus.clone().unwrap();
    open(&props, &mut presenter, true);
    draw(&mut presenter, 40, 12);
    presenter.input(key(KeyCode::Tab)).unwrap();
    let original_second = presenter.focus.clone().unwrap();
    assert_eq!(original_second.key, "two");
    props.borrow_mut()["content"]["children"][1]["open"] = json!(true);
    refresh(&mut presenter);
    draw(&mut presenter, 40, 12);
    assert_eq!(presenter.focus.as_ref().unwrap().key, "cover-choice");
    props.borrow_mut()["content"]["children"][1]["open"] = json!(false);
    refresh(&mut presenter);
    draw(&mut presenter, 40, 12);
    assert!(presenter.focus.as_ref().unwrap().same(&original_second));
    open(&props, &mut presenter, false);
    draw(&mut presenter, 40, 12);
    assert!(presenter.focus.as_ref().unwrap().same(&background));
    close(&mut presenter);
}

#[test]
fn dialog_actual_scope_change_cancels_same_original_slot_drag_after_zero_placement() {
    let modal = dialog(
        "modal",
        true,
        json!({"kind":"text","key":"bg","text":"background","color":0xffffff}),
        source("moving", "foreground selection"),
    );
    let (_runtime, mut presenter, props) = native(vec![modal]);
    draw(&mut presenter, 40, 12);
    let target = presenter.focus.clone().unwrap();
    let point = target.area.visible();
    mouse(
        &mut presenter,
        MouseEventKind::Down(MouseButton::Left),
        point.x,
        point.y,
    );
    mouse(
        &mut presenter,
        MouseEventKind::Drag(MouseButton::Left),
        point.x + 7,
        point.y,
    );
    assert!(presenter.drag.is_some());
    let mut empty = column("empty", vec![]);
    empty["layout"] = json!({"basis":0});
    props.borrow_mut()["content"]["children"][0]["background"] = empty;
    props.borrow_mut()["content"]["children"][0]["content"] =
        json!({"kind":"text","key":"placeholder","text":"","color":0});
    props.borrow_mut()["content"]["children"]
        .as_array_mut()
        .unwrap()
        .push(source("moving", "foreground selection"));
    refresh(&mut presenter);
    assert!(presenter.drag.is_some()); // Opening stays live and already acknowledged.
    draw(&mut presenter, 40, 12);
    assert!(presenter.physical_path.is_empty());
    assert!(presenter.scene.iter().any(|new| new.same(&target)));
    assert!(presenter.drag.is_none());
    assert!(!presenter.resources.pointer_capture(&target).unwrap());
    let intent = state(&presenter).reading.as_ref().unwrap().scope.intent();
    mouse(
        &mut presenter,
        MouseEventKind::Drag(MouseButton::Left),
        18,
        0,
    );
    assert_eq!(
        state(&presenter).reading.as_ref().unwrap().scope.intent(),
        intent
    );
    close(&mut presenter);
}

#[test]
fn dialog_original_ancestor_revoke_interrupts_retained_inner_drag_and_allows_fresh_begin() {
    let inner = dialog(
        "inner",
        true,
        json!({"kind":"text","key":"inner-bg","text":"","color":0}),
        source("moving", "foreground selection"),
    );
    let outer = dialog(
        "outer",
        true,
        json!({"kind":"text","key":"outer-bg","text":"","color":0}),
        inner.clone(),
    );
    let (_runtime, mut presenter, props) = native(vec![outer]);
    draw(&mut presenter, 40, 12);
    assert_eq!(presenter.physical_path.len(), 2);
    let original = presenter.focus.clone().unwrap();
    let point = original.area.visible();
    mouse(
        &mut presenter,
        MouseEventKind::Down(MouseButton::Left),
        point.x,
        point.y,
    );
    mouse(
        &mut presenter,
        MouseEventKind::Drag(MouseButton::Left),
        point.x + 7,
        point.y,
    );
    let selected = copied(&mut presenter).unwrap();
    let inner_scope = presenter.physical_path.last().cloned().unwrap();
    props.borrow_mut()["content"]["children"][0] = inner;
    refresh(&mut presenter);
    assert!(!presenter.physical_admitted());
    assert!(presenter.drag.is_none());
    mouse(
        &mut presenter,
        MouseEventKind::Drag(MouseButton::Left),
        point.x + 17,
        point.y,
    );
    draw(&mut presenter, 40, 12);
    assert_eq!(presenter.physical_path.len(), 1);
    assert!(inner_scope.ptr_eq(presenter.physical_path.last().unwrap()));
    assert!(presenter.focus.as_ref().unwrap().same(&original));
    assert_eq!(copied(&mut presenter).unwrap(), selected);
    let point = presenter.focus.as_ref().unwrap().area.visible();
    mouse(
        &mut presenter,
        MouseEventKind::Down(MouseButton::Left),
        point.x + 9,
        point.y,
    );
    mouse(
        &mut presenter,
        MouseEventKind::Drag(MouseButton::Left),
        point.x + 17,
        point.y,
    );
    assert!(presenter.drag.is_some());
    assert_ne!(copied(&mut presenter).unwrap(), selected);
    close(&mut presenter);
}
