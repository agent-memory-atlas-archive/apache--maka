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

fn column(key: &str, children: Vec<Value>) -> Value {
    json!({"kind":"stack","key":key,"direction":"column","gap":0,"padding":0,"background":0,"layout":{"grow":1},"children":children})
}
fn declaration() -> Value {
    let background = column(
        "body",
        vec![
            json!({"kind":"component","key":"editor","binding":"editor","member":0,"on":{},"props":{
                "initialText":"","placeholder":"Draft","autoFocus":true,"terminalRows":{"min":1,"max":3},
                "appearance":support::brand_appearance()["textarea"],"accessibilityLabel":"Draft",
                "readOnly":false,"disabled":false,"softWrap":true,"tabIndex":0
            }}),
            json!({"kind":"button","key":"back","label":"Refresh"}),
        ],
    );
    let mut content = column(
        "directory",
        (0..30)
            .map(|i| json!({"kind":"button","key":format!("row-{i}"),"label":format!("Row {i}")}))
            .collect(),
    );
    content["scroll"] = json!("manual");
    json!({"kind":"adaptive_pane","key":"pane","background":background,"content":content,
        "pane_width":32,"inline_min_width":60,"auto_inline_min_width":90,"motion":false,"command":null,
        "overlay":{"label":"Directory","width":50,"height":12,"padding":0,"background_color":0,"border_color":0xffffff}})
}
fn fixture() -> (tokio::runtime::Runtime, Presenter, Rc<RefCell<Value>>) {
    let value = Rc::new(RefCell::new(declaration()));
    let (runtime, presenter) = layout::native(
        layout::View::InteractiveScene(value.clone()),
        Rc::new(Cell::new(true)),
    );
    (runtime, presenter, value)
}
fn refresh(p: &mut Presenter) {
    let revision = p.application.frame(&p.root).unwrap().revision;
    p.application
        .dispatch(&p.root, revision, "back", json!({}))
        .unwrap();
    p.sync_native().unwrap();
}
fn command(p: &mut Presenter, value: &RefCell<Value>, sequence: u64, op: &str) {
    value.borrow_mut()["command"] = json!({"sequence":sequence,"op":op});
    refresh(p);
}
fn state(p: &Presenter) -> &super::super::scroll::SceneState {
    p.resources.owners[&p.root.id()]
        .state
        .downcast_ref()
        .unwrap()
}
fn inline(p: &Presenter) -> u16 {
    state(p).panes["pane"]
        .geometry
        .as_ref()
        .unwrap()
        .inline_width
}
fn pending(p: &Presenter, width: u16) -> PaintedScene {
    let area = Rect::new(0, 0, width, 20);
    p.paint(area, &mut Buffer::empty(area)).unwrap()
}
fn close(p: &mut Presenter) {
    p.surface_closed(&p.surface(), Ok(())).unwrap();
    p.shutdown().unwrap();
}
#[test]
fn adaptive_original_thresholds_keep_manual_preference_and_independent_drawer() {
    let (_runtime, mut p, value) = fixture();
    for (width, expected) in [(59, 0), (60, 0), (89, 0), (90, 32)] {
        draw(&mut p, width, 20);
        assert_eq!(inline(&p), expected);
        assert!(p.physical_path.is_empty());
    }
    draw(&mut p, 60, 20);
    command(&mut p, &value, 1, "toggle");
    draw(&mut p, 60, 20);
    assert_eq!(inline(&p), 32);
    draw(&mut p, 59, 20);
    assert_eq!(inline(&p), 0);
    assert!(p.physical_path.is_empty(), "resize never opens a drawer");
    command(&mut p, &value, 2, "toggle");
    assert!(!p.physical_admitted());
    draw(&mut p, 59, 20);
    let opening = p.physical_path.last().unwrap().clone();
    draw(&mut p, 100, 20);
    assert!(
        p.physical_path.last().unwrap().ptr_eq(&opening),
        "widening preserves the actual original drawer"
    );
    assert_eq!(inline(&p), 0, "pane is only painted in the foreground");
    command(&mut p, &value, 3, "dismiss_transient");
    assert_eq!(opening.strong_count(), 0);
    draw(&mut p, 100, 20);
    assert_eq!(
        inline(&p),
        32,
        "selection dismissal retains the pinned preference"
    );
    command(&mut p, &value, 4, "toggle");
    draw(&mut p, 100, 20);
    assert_eq!(inline(&p), 0);
    command(&mut p, &value, 4, "toggle");
    draw(&mut p, 100, 20);
    assert_eq!(inline(&p), 0, "same immutable command cannot toggle twice");
    close(&mut p);
}
#[test]
fn adaptive_unflushed_viewport_and_stale_recipes_cannot_steer_toggle_or_focus() {
    let (_runtime, mut p, value) = fixture();
    command(&mut p, &value, 1, "toggle"); // No successful placement yet.
    draw(&mut p, 90, 20);
    assert_eq!(inline(&p), 32);
    let editor = p.focus.clone().unwrap();
    let held = pending(&p, 40);
    assert_eq!(state(&p).panes["pane"].geometry.as_ref().unwrap().width, 90);
    command(&mut p, &value, 2, "toggle");
    assert!(
        state(&p).dialogs.is_empty(),
        "unflushed narrow candidate cannot open drawer"
    );
    p.present(held);
    assert_eq!(state(&p).panes["pane"].geometry.as_ref().unwrap().width, 90);
    draw(&mut p, 40, 20);
    command(&mut p, &value, 3, "toggle");
    let unflushed = pending(&p, 40);
    assert!(!p.physical_admitted());
    command(&mut p, &value, 4, "dismiss_transient");
    p.present(unflushed);
    assert!(p.physical_path.is_empty());
    draw(&mut p, 40, 20);
    assert!(p.physical_admitted());
    assert!(p.focus.as_ref().unwrap().same(&editor));
    close(&mut p);
}
#[test]
fn adaptive_successful_same_recipe_refresh_accepts_the_published_viewport() {
    let (_runtime, mut p, value) = fixture();
    draw(&mut p, 40, 20);
    let held = pending(&p, 90);
    refresh(&mut p);
    p.present(held);
    assert_eq!(state(&p).panes["pane"].geometry.as_ref().unwrap().width, 90);
    command(&mut p, &value, 1, "toggle");
    assert!(state(&p).dialogs.is_empty());
    draw(&mut p, 90, 20);
    assert_eq!(inline(&p), 0, "toggle uses the published wide auto default");
    close(&mut p);
}
#[test]
fn adaptive_editor_undo_and_directory_scroll_survive_inline_and_modal_layouts() {
    let (_runtime, mut p, value) = fixture();
    draw(&mut p, 100, 20);
    let editor_mount = p
        .application
        .child_mount(&p.root, "editor")
        .unwrap()
        .clone();
    let original_owner = p.resources.owners[&editor_mount.id()].identity.clone();
    p.input(Event::Paste("Draft 中🙂".into())).unwrap();
    draw(&mut p, 100, 20);
    let editor_focus = p.focus.clone().unwrap();
    p.input(Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 1,
        row: 3,
        modifiers: KeyModifiers::NONE,
    }))
    .unwrap();
    draw(&mut p, 100, 20);
    let offset = state(&p).scrolls["directory"].offset;
    assert!(offset > 0);
    assert!(p.focus.as_ref().unwrap().same(&editor_focus));
    draw(&mut p, 40, 20);
    assert!(p.focus.as_ref().unwrap().same(&editor_focus));
    command(&mut p, &value, 1, "toggle");
    draw(&mut p, 40, 20);
    assert!(!p.physical_path.is_empty());
    p.input(key(KeyCode::Esc)).unwrap();
    draw(&mut p, 100, 20);
    assert!(p.physical_path.is_empty());
    assert_eq!(inline(&p), 32);
    assert_eq!(state(&p).scrolls["directory"].offset, offset);
    assert!(Rc::ptr_eq(
        &original_owner,
        &p.resources.owners[&editor_mount.id()].identity
    ));
    assert!(
        p.focus.as_ref().unwrap().same(&editor_focus),
        "expected editor {}, got {}",
        editor_focus.key,
        p.focus.as_ref().unwrap().key
    );
    let text = |p: &Presenter| {
        editor_owner(&*p.resources.owners[&editor_mount.id()].state)
            .editor
            .text()
            .to_owned()
    };
    assert_eq!(text(&p), "Draft 中🙂");
    p.input(Event::Key(KeyEvent::new(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(text(&p), "");
    p.input(Event::Key(KeyEvent::new(
        KeyCode::Char('y'),
        KeyModifiers::CONTROL,
    )))
    .unwrap();
    assert_eq!(text(&p), "Draft 中🙂");
    close(&mut p);
}
#[test]
fn adaptive_hidden_nested_dialog_uses_original_omission_and_foreground_publication() {
    let (_runtime, mut p, value) = fixture();
    let content = value.borrow()["content"].clone();
    value.borrow_mut()["content"] = json!({"kind":"dialog","key":"inner","open":true,"label":"Inner",
      "width":20,"height":8,"padding":0,"background_color":0,"border_color":0xffffff,
      "background":content,"content":{"kind":"button","key":"inside","label":"Inside"}});
    refresh(&mut p);
    assert!(!p.physical_admitted());
    draw(&mut p, 59, 20);
    assert!(p.physical_admitted());
    assert!(p.physical_path.is_empty());
    draw(&mut p, 100, 20);
    assert_eq!(p.physical_path.len(), 1);
    assert_eq!(p.focus.as_ref().unwrap().key, "inside");
    draw(&mut p, 59, 20);
    assert!(p.physical_admitted());
    assert!(p.physical_path.is_empty());
    assert_eq!(
        p.focus.as_ref().unwrap().mount,
        p.application
            .child_mount(&p.root, "editor")
            .unwrap()
            .clone()
    );
    close(&mut p);
}
