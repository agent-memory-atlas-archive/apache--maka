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

fn item(index: usize) -> Value {
    json!({"kind":"item","key":format!("item-{index}"),"current":index==2,"follow_focus":true,"submit":true,
        "content":column(&format!("row-{index}"),vec![
            json!({"kind":"text","key":format!("title-{index}"),"text":format!("Model {index:02}"),"color":0xffffff,"truncate":true}),
            json!({"kind":"text","key":format!("detail-{index}"),"text":format!("  Provider {index:02}"),"color":0x888888,"truncate":true})])})
}
fn scene() -> Value {
    let mut group = column("list", (0..12).map(item).collect());
    group["focus_group"] = json!(true);
    group["scroll"] = json!("manual");
    group["layout"] = json!({"basis":4});
    column(
        "root",
        vec![
            json!({"kind":"button","key":"before","label":"Before"}),
            group,
            json!({"kind":"button","key":"after","label":"After"}),
        ],
    )
}
fn plain() -> (tokio::runtime::Runtime, Presenter, Rc<RefCell<Value>>) {
    let source = Rc::new(RefCell::new(scene()));
    let (runtime, presenter) = layout::native(
        layout::View::InteractiveScene(source.clone()),
        Rc::new(Cell::new(true)),
    );
    (runtime, presenter, source)
}
#[test]
fn grouped_items_reuse_two_line_content_and_reveal_offscreen_keyboard_targets() {
    let (_runtime, mut p, source) = plain();
    source.borrow_mut()["shortcuts"] = json!([
        {"key":"up","modifiers":[],"input":"before","enabled":true},
        {"key":"down","modifiers":[],"input":"after","enabled":true}
    ]);
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "before");
    assert_eq!(p.tab_stops().len(), 3);
    assert!(
        p.scene
            .iter()
            .find(|t| t.key == "item-11")
            .unwrap()
            .area
            .is_empty()
    );
    p.input(key(KeyCode::Tab)).unwrap();
    let buffer = draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-2");
    let target = p.focus.as_ref().unwrap();
    assert_eq!(target.area.visible().height, 2);
    assert_eq!(buffer[(2, target.area.visible().y + 1)].symbol(), "P");
    let style = &buffer[(2, target.area.visible().y)];
    assert!(
        style
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED)
    );
    assert!(style.modifier.contains(ratatui::style::Modifier::REVERSED));
    p.input(key(KeyCode::End)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-11");
    assert_eq!(p.focus.as_ref().unwrap().area.visible().height, 2);
    assert_eq!(observation(&p), json!({"kind":"activate"}));
    let count = root_count(&p);
    p.input(key(KeyCode::Down)).unwrap();
    assert_eq!(root_count(&p), count + 1);
    assert_eq!(
        p.application.frame(&p.root).unwrap().state["last_input"],
        json!({"key":"after","value":{"kind":"click"}})
    );
    assert_eq!(p.focus.as_ref().unwrap().key, "item-11");
    draw(&mut p, 24, 6);
    p.input(key(KeyCode::Enter)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(observation(&p), json!({"kind":"submit"}));
    p.input(key(KeyCode::Home)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    let count = root_count(&p);
    p.input(key(KeyCode::Up)).unwrap();
    assert_eq!(root_count(&p), count + 1);
    assert_eq!(
        p.application.frame(&p.root).unwrap().state["last_input"],
        json!({"key":"before","value":{"kind":"click"}})
    );
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    draw(&mut p, 24, 6);
    p.input(key(KeyCode::Tab)).unwrap();
    assert_eq!(p.focus.as_ref().unwrap().key, "after");
    close(&mut p);
}
#[test]
fn context_wrapped_items_keep_offscreen_navigation_and_visible_only_menu_hits() {
    let mut node = scene();
    for (index, item) in node["children"][1]["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        *item = json!({"kind":"context_menu","key":format!("menu-{index}"),"label":"Actions",
            "width":12,"height":4,"padding":0,"background_color":0,"border_color":0xffffff,
            "background":item.clone(),
            "content":json!({"kind":"button","key":format!("action-{index}"),"label":"Details"})});
    }
    let source = Rc::new(RefCell::new(node));
    let (_runtime, mut p) = layout::native(
        layout::View::InteractiveScene(source),
        Rc::new(Cell::new(true)),
    );
    draw(&mut p, 24, 6);
    assert!(
        p.scene
            .iter()
            .find(|target| target.key == "item-11")
            .unwrap()
            .area
            .is_empty()
    );
    assert!(!p.scene.iter().any(|target| target.key == "menu-11"));
    p.input(key(KeyCode::Tab)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-2");
    p.input(key(KeyCode::End)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-11");
    assert_eq!(p.focus.as_ref().unwrap().area.visible().height, 2);
    p.input(Event::Key(KeyEvent::new(
        KeyCode::F(10),
        KeyModifiers::SHIFT,
    )))
    .unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "action-11");
    p.input(key(KeyCode::Esc)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-11");
    p.input(key(KeyCode::Home)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    close(&mut p);
}

#[test]
fn grouped_items_directory_arrows_reveal_without_activating_a_destination() {
    let mut source = scene()["children"][1].clone();
    source["children"] = json!((0..64).map(item).collect::<Vec<_>>());
    for item in source["children"].as_array_mut().unwrap() {
        item["follow_focus"] = json!(false);
        item["submit"] = json!(false);
    }
    let (_runtime, mut p) = layout::native(
        layout::View::InteractiveScene(Rc::new(RefCell::new(source))),
        Rc::new(Cell::new(true)),
    );
    draw(&mut p, 24, 6);
    let before = p.application.frame(&p.root).unwrap().revision;
    assert_eq!(
        p.scene
            .iter()
            .filter(|target| matches!(
                target.kind,
                super::super::super::paint::TargetKind::Item { .. }
            ))
            .count(),
        64
    );
    p.input(key(KeyCode::End)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-63");
    assert_eq!(p.focus.as_ref().unwrap().area.visible().height, 2);
    assert_eq!(p.application.frame(&p.root).unwrap().revision, before);
    p.input(key(KeyCode::Enter)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(observation(&p), json!({"kind":"activate"}));
    close(&mut p);
}

#[test]
fn grouped_items_pointer_hover_does_not_select_and_second_line_submits() {
    let (_runtime, mut p, _) = plain();
    draw(&mut p, 24, 6);
    let initial = p.application.frame(&p.root).unwrap().revision;
    mouse(&mut p, MouseEventKind::Moved, 4, 2);
    let hovered = draw(&mut p, 24, 6);
    assert!(
        hovered[(4, 2)]
            .modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
    assert!(
        !hovered[(4, 2)]
            .modifier
            .contains(ratatui::style::Modifier::REVERSED)
    );
    assert_eq!(p.application.frame(&p.root).unwrap().revision, initial);
    assert_eq!(p.focus.as_ref().unwrap().key, "before");
    mouse(&mut p, MouseEventKind::Down(MouseButton::Left), 4, 2);
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    assert_eq!(observation(&p), json!({"kind":"submit"}));
    p.input(key(KeyCode::Down)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-1");
    p.input(key(KeyCode::Char(' '))).unwrap();
    assert_eq!(observation(&p), json!({"kind":"activate"}));
    close(&mut p);
}
#[test]
fn grouped_items_reject_navigation_from_unflushed_replacement_and_keep_reading_owner() {
    let (_runtime, mut p, source) = native((0..12).map(item).collect());
    source.borrow_mut()["content"]["focus_group"] = json!(true);
    refresh(&mut p);
    draw(&mut p, 24, 4);
    p.input(key(KeyCode::End)).unwrap();
    draw(&mut p, 24, 4);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-11");
    assert!(state(&p).scrolls["reading"].offset > 0);
    source.borrow_mut()["content"]["children"][0]["disabled"] = json!(true);
    refresh(&mut p);
    let before = p.application.frame(&p.root).unwrap().revision;
    p.input(key(KeyCode::Home)).unwrap();
    p.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(p.application.frame(&p.root).unwrap().revision, before);
    draw(&mut p, 24, 4);
    p.input(key(KeyCode::Home)).unwrap();
    draw(&mut p, 24, 4);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-1");
    close(&mut p);
}

#[test]
fn grouped_items_reveal_through_entirely_clipped_nested_scrollers() {
    let mut source = scene();
    let group = source["children"][1].clone();
    let mut outer = column(
        "outer",
        vec![
            json!({"kind":"text","key":"spacer","text":"one\ntwo\nthree\nfour\nfive\nsix","color":0xffffff}),
            group,
        ],
    );
    outer["scroll"] = json!("manual");
    outer["layout"] = json!({"basis":4});
    source["children"][1] = outer;
    let shared = Rc::new(RefCell::new(source));
    let (_runtime, mut p) = layout::native(
        layout::View::InteractiveScene(shared.clone()),
        Rc::new(Cell::new(true)),
    );
    draw(&mut p, 24, 6);
    assert!(
        p.scene
            .iter()
            .find(|t| t.key == "item-2")
            .unwrap()
            .area
            .is_empty()
    );
    p.input(key(KeyCode::Tab)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-2");
    assert_eq!(p.focus.as_ref().unwrap().area.visible().height, 2);
    p.input(key(KeyCode::End)).unwrap();
    draw(&mut p, 24, 6);
    let point = p.focus.as_ref().unwrap().area.visible();
    assert_eq!(p.focus.as_ref().unwrap().key, "item-11");
    assert_eq!(point.height, 2);
    mouse(
        &mut p,
        MouseEventKind::Down(MouseButton::Left),
        point.x + 2,
        point.y + 1,
    );
    assert_eq!(observation(&p), json!({"kind":"submit"}));
    shared.borrow_mut()["children"][1]["visible"] = json!(false);
    let revision = p.application.frame(&p.root).unwrap().revision;
    p.application
        .dispatch(&p.root, revision, "before", json!({"kind":"click"}))
        .unwrap();
    p.sync_native().unwrap();
    draw(&mut p, 24, 6);
    assert!(!p.scene.iter().any(|t| t.key.starts_with("item-")));
    close(&mut p);
}
#[test]
fn grouped_items_cross_component_slots_and_focus_child_share_original_entry() {
    let children: Vec<_> = (0..12)
        .map(|index| {
            json!({"kind":"component","key":format!("child-{index}"),"binding":"reading","member":0,
        "props":props(vec![item(index)]),"on":{"input":"row-activated"}})
        })
        .collect();
    let mut source = scene();
    source["children"][0] =
        json!({"kind":"button","key":"jump","label":"Jump","focus_child":"child-8"});
    source["children"][1]["children"] = json!(children);
    let shared = Rc::new(RefCell::new(source));
    let (_runtime, mut p) = layout::native(
        layout::View::InteractiveScene(shared.clone()),
        Rc::new(Cell::new(true)),
    );
    draw(&mut p, 24, 6);
    p.input(key(KeyCode::Tab)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-2");
    p.input(key(KeyCode::End)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-11");
    assert_eq!(p.focus.as_ref().unwrap().area.visible().height, 2);
    p.input(key(KeyCode::Tab)).unwrap();
    p.input(key(KeyCode::Tab)).unwrap();
    assert_eq!(p.focus.as_ref().unwrap().key, "jump");
    p.input(key(KeyCode::Enter)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-8");
    assert_eq!(p.focus.as_ref().unwrap().area.visible().height, 2);
    p.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(observation(&p)["event"]["kind"], "submit");
    draw(&mut p, 24, 6);
    let old_last = p
        .scene
        .iter()
        .find(|target| target.key == "item-11")
        .unwrap()
        .mount
        .clone();
    shared.borrow_mut()["children"][1]["children"][11]["key"] = json!("replacement");
    let revision = p.application.frame(&p.root).unwrap().revision;
    p.application
        .dispatch(&p.root, revision, "after", json!({"kind":"click"}))
        .unwrap();
    p.sync_native().unwrap();
    let revision = p.application.frame(&p.root).unwrap().revision;
    p.input(key(KeyCode::End)).unwrap();
    assert_eq!(p.application.frame(&p.root).unwrap().revision, revision);
    assert_ne!(p.focus.as_ref().unwrap().mount, old_last);
    draw(&mut p, 24, 6);
    p.input(key(KeyCode::End)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-11");
    assert_ne!(p.focus.as_ref().unwrap().mount, old_last);
    close(&mut p);
}

#[test]
fn grouped_item_content_preserves_original_modal_opening_and_choice_owner() {
    let choice = json!({"kind":"choice","key":"choice","label":"Level","options":[{"key":"low","label":"Low"},{"key":"high","label":"High"}],"current":"low","foreground":0xffffff,"background":0,"border":0xffffff,"selected":0x224466,"accent":0x4488ff});
    let content = super::dialog::dialog(
        "modal",
        true,
        json!({"kind":"text","key":"background","text":"Background","color":0xffffff}),
        json!({"kind":"item","key":"inner","content":choice}),
    );
    let (_runtime, mut p, _) = native(vec![json!({"kind":"item","key":"outer","content":content})]);
    draw(&mut p, 30, 10);
    draw(&mut p, 30, 10);
    assert_eq!(p.physical_path.len(), 1);
    let target = p
        .scene
        .iter()
        .find(|t| t.key == "choice")
        .unwrap()
        .area
        .visible();
    mouse(
        &mut p,
        MouseEventKind::Down(MouseButton::Left),
        target.x,
        target.y,
    );
    assert!(!p.physical_admitted());
    draw(&mut p, 30, 10);
    assert_eq!(p.physical_path.len(), 2);
    p.input(key(KeyCode::Esc)).unwrap();
    draw(&mut p, 30, 10);
    assert_eq!(p.physical_path.len(), 1);
    close(&mut p);
}

#[test]
fn horizontal_group_uses_its_published_axis_and_preserves_explicit_activation() {
    let mut source = scene();
    let group = &mut source["children"][1];
    group["direction"] = json!("row");
    group["scroll"] = json!("none");
    group["children"] = json!((0..3).map(item).collect::<Vec<_>>());
    for item in group["children"].as_array_mut().unwrap() {
        item["follow_focus"] = json!(false);
        item["submit"] = json!(false);
    }
    group["children"][1]["disabled"] = json!(true);
    let (_runtime, mut p) = layout::native(
        layout::View::InteractiveScene(Rc::new(RefCell::new(source))),
        Rc::new(Cell::new(true)),
    );
    draw(&mut p, 60, 8);
    p.input(key(KeyCode::Tab)).unwrap();
    draw(&mut p, 60, 8);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-2");
    let revision = p.application.frame(&p.root).unwrap().revision;
    p.input(key(KeyCode::Left)).unwrap();
    draw(&mut p, 60, 8);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    p.input(key(KeyCode::Down)).unwrap();
    draw(&mut p, 60, 8);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    p.input(key(KeyCode::Right)).unwrap();
    draw(&mut p, 60, 8);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-2");
    assert_eq!(p.application.frame(&p.root).unwrap().revision, revision);
    p.input(key(KeyCode::Enter)).unwrap();
    draw(&mut p, 60, 8);
    assert_eq!(observation(&p), json!({"kind":"activate"}));
    p.input(key(KeyCode::Tab)).unwrap();
    assert_eq!(p.focus.as_ref().unwrap().key, "after");
    close(&mut p);
}

fn focus_command(p: &mut Presenter, props: &Rc<RefCell<Value>>, sequence: u64, key: &str) {
    props.borrow_mut()["command"] =
        json!({"sequence":sequence,"target":{"kind":"focus_item","key":key}});
    refresh(p);
}
#[test]
fn reading_focus_command_waits_for_actual_reveal_and_only_queues_follow_focus() {
    let (_runtime, mut p, props) = native((0..12).map(item).collect());
    props.borrow_mut()["content"]["focus_group"] = json!(true);
    refresh(&mut p);
    draw(&mut p, 24, 4);
    focus_command(&mut p, &props, 1, "item-10");
    let area = Rect::new(0, 0, 24, 4);
    let mut buffer = Buffer::empty(area);
    let candidate = p.paint(area, &mut buffer).unwrap();
    let before = p.application.frame(&p.root).unwrap().revision;
    assert_ne!(p.focus.as_ref().unwrap().key, "item-10");
    drop(candidate);
    assert_eq!(state(&p).scrolls["reading"].offset, 0);
    draw(&mut p, 24, 4);
    assert!(state(&p).reading.as_ref().unwrap().pending.is_some());
    assert_ne!(p.focus.as_ref().unwrap().key, "item-10");
    // A current flushed item is revealed before the command can own focus.
    let candidate = p.paint(area, &mut buffer).unwrap();
    let revision = p.application.frame(&p.root).unwrap().revision;
    p.present(candidate);
    assert_eq!(p.application.frame(&p.root).unwrap().revision, revision);
    assert!(revision >= before);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-10");
    assert_eq!(p.focus.as_ref().unwrap().area.visible().height, 2);
    assert!(state(&p).reading.as_ref().unwrap().pending.is_none());
    p.sync_native().unwrap();
    assert!(p.application.frame(&p.root).unwrap().revision > revision);
    draw(&mut p, 24, 4);
    p.input(key(KeyCode::Down)).unwrap();
    draw(&mut p, 24, 4);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-11");
    // Replacing the page before the follow-focus notification is delivered must
    // retire the old item input while retaining the ReadingArea itself.
    focus_command(&mut p, &props, 2, "item-0");
    draw(&mut p, 24, 4);
    draw(&mut p, 24, 4);
    assert!(
        p.resources
            .commit(&p.application, &p.root)
            .unwrap()
            .iter()
            .any(|(_, input)| input.key == "item-0")
    );
    props.borrow_mut()["content"]["children"] = json!([]);
    refresh(&mut p);
    assert!(
        !p.resources
            .commit(&p.application, &p.root)
            .unwrap()
            .iter()
            .any(|(_, input)| input.key == "item-0")
    );
    assert!(p.reported.is_empty(), "{:?}", p.reported);
    close(&mut p);
}
#[test]
fn reading_focus_request_rejects_disabled_or_nonitem_keys_and_new_input_cancels_it() {
    let (_runtime, mut p, props) = native((0..12).map(item).collect());
    props.borrow_mut()["content"]["children"][1]["disabled"] = json!(true);
    refresh(&mut p);
    draw(&mut p, 24, 4);
    for (sequence, target) in [(1, "item-1"), (2, "title-0"), (3, "missing")] {
        focus_command(&mut p, &props, sequence, target);
        draw(&mut p, 24, 4);
        p.sync_native().unwrap();
        assert_eq!(
            observation(&p)["receipt"],
            json!({"sequence":sequence,"status":"rejected"})
        );
        assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    }
    focus_command(&mut p, &props, 4, "item-10");
    draw(&mut p, 24, 4);
    p.input(key(KeyCode::Tab)).unwrap();
    for _ in 0..3 {
        draw(&mut p, 24, 4);
    }
    p.sync_native().unwrap();
    assert_ne!(p.focus.as_ref().unwrap().key, "item-10");
    assert!(state(&p).reading.as_ref().unwrap().pending.is_none());
    for (sequence, kind) in [
        (5, MouseEventKind::ScrollRight),
        (6, MouseEventKind::Drag(MouseButton::Left)),
    ] {
        focus_command(&mut p, &props, sequence, "item-10");
        mouse(&mut p, kind, 1, 1);
        for _ in 0..3 {
            draw(&mut p, 24, 4);
        }
        p.sync_native().unwrap();
        assert!(state(&p).reading.as_ref().unwrap().pending.is_none());
        assert_eq!(
            observation(&p)["receipt"],
            json!({"sequence":sequence,"status":"cancelled"})
        );
    }
    close(&mut p);
}
#[test]
fn reading_focus_request_cannot_replay_after_an_unflushed_hide_or_owner_retirement() {
    let (_runtime, mut p, props) = native((0..4).map(item).collect());
    draw(&mut p, 24, 4);
    focus_command(&mut p, &props, 1, "item-3");
    let area = Rect::new(0, 0, 24, 4);
    let mut buffer = Buffer::empty(area);
    let old = p.paint(area, &mut buffer).unwrap();
    props.borrow_mut()["content"]["visible"] = json!(false);
    refresh(&mut p);
    p.present(old);
    draw(&mut p, 24, 4);
    assert!(p.focus.is_none());
    props.borrow_mut()["content"]["visible"] = json!(true);
    refresh(&mut p);
    draw(&mut p, 24, 4);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    assert!(state(&p).reading.as_ref().unwrap().pending.is_none());
    focus_command(&mut p, &props, 2, "item-3");
    let old = p.paint(area, &mut buffer).unwrap();
    close(&mut p);
    p.present(old);
    assert!(p.focus.is_none());
}
#[test]
fn reading_item_arrows_skip_embedded_text_sources_and_buttons() {
    let mut children = vec![item(0), source("body-0", "first source")];
    children.push(json!({"kind":"button","key":"detail","label":"Details"}));
    children.push(item(1));
    children.push(source("body-1", "second source"));
    let (_runtime, mut p, props) = native(children);
    props.borrow_mut()["content"]["focus_group"] = json!(true);
    refresh(&mut p);
    draw(&mut p, 24, 8);
    p.input(key(KeyCode::Down)).unwrap();
    draw(&mut p, 24, 8);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-1");
    p.input(key(KeyCode::Up)).unwrap();
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    close(&mut p);
}

#[test]
fn item_focus_clears_the_original_reading_selection() {
    let (_runtime, mut p, props) = native(vec![item(0), source("body", "selectable"), item(1)]);
    draw(&mut p, 24, 8);
    mouse(&mut p, MouseEventKind::Down(MouseButton::Left), 0, 2);
    mouse(&mut p, MouseEventKind::Drag(MouseButton::Left), 4, 2);
    mouse(&mut p, MouseEventKind::Up(MouseButton::Left), 4, 2);
    assert_eq!(copied(&mut p).unwrap(), b"sele");
    focus_command(&mut p, &props, 1, "item-1");
    draw(&mut p, 24, 8);
    p.sync_native().unwrap();
    assert_eq!(p.focus.as_ref().unwrap().key, "item-1");
    assert!(observation(&p)["selectedChildKey"].is_null());
    assert!(copied(&mut p).is_none());
    close(&mut p);
}

#[test]
fn focus_command_created_under_hidden_parent_is_rejected_before_reveal() {
    let props = Rc::new(RefCell::new(props(vec![item(0), item(1)])));
    let visible = Rc::new(Cell::new(false));
    props.borrow_mut()["command"] =
        json!({"sequence":1,"target":{"kind":"focus_item","key":"item-1"}});
    let (_runtime, mut p) = layout::native(
        layout::View::HiddenReadingSlot(props, visible.clone()),
        Rc::new(Cell::new(true)),
    );
    draw(&mut p, 24, 8);
    let child = p
        .application
        .child_mount(&p.root, "reader")
        .unwrap()
        .clone();
    let state = p.resources.owners[&child.id()]
        .state
        .downcast_ref::<super::super::super::scroll::SceneState>()
        .unwrap();
    assert!(state.reading.as_ref().unwrap().pending.is_none());
    assert_eq!(
        observation(&p)["receipt"],
        json!({"sequence":1,"status":"rejected"})
    );
    visible.set(true);
    let root = p.root.clone();
    let revision = p.application.frame(&root).unwrap().revision;
    p.application
        .dispatch(&root, revision, "back", json!({"kind":"click"}))
        .unwrap();
    p.sync_native().unwrap();
    draw(&mut p, 24, 8);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    close(&mut p);
}

#[test]
fn foreground_item_focus_does_not_clear_background_reading_selection() {
    let mut items = column("choices", vec![item(0), item(1)]);
    items["focus_group"] = json!(true);
    let (_runtime, mut p, props) = native(vec![dialog::dialog(
        "modal",
        false,
        source("bg", "retained selection"),
        items,
    )]);
    draw(&mut p, 40, 12);
    all(&mut p);
    let scope = state(&p).reading.as_ref().unwrap().scope.clone();
    assert_eq!(scope.copy().unwrap().unwrap(), "retained selection");
    props.borrow_mut()["content"]["children"][0]["open"] = json!(true);
    refresh(&mut p);
    draw(&mut p, 40, 12);
    p.input(key(KeyCode::Down)).unwrap();
    draw(&mut p, 40, 12);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-1");
    shift(&mut p, KeyCode::End);
    assert!(
        copied(&mut p).is_none(),
        "modal item cannot copy the background"
    );
    assert_eq!(scope.copy().unwrap().unwrap(), "retained selection");
    focus_command(&mut p, &props, 1, "item-0");
    draw(&mut p, 40, 12);
    p.sync_native().unwrap();
    assert_eq!(p.focus.as_ref().unwrap().key, "item-1");
    assert_eq!(
        observation(&p)["receipt"],
        json!({"sequence":1,"status":"rejected"})
    );
    assert_eq!(scope.copy().unwrap().unwrap(), "retained selection");
    close(&mut p);
}

fn message(index: usize, text: &str) -> Value {
    column(
        &format!("message-{index}"),
        vec![item(index), source(&format!("body-{index}"), text)],
    )
}
fn shift(p: &mut Presenter, code: KeyCode) {
    p.input(Event::Key(KeyEvent::new(code, KeyModifiers::SHIFT)))
        .unwrap();
}
#[test]
fn reading_header_shift_enters_the_original_body_and_extends_its_shared_range() {
    let (_runtime, mut p, _) = native(vec![message(0, "abc\nxyz"), message(1, "next")]);
    draw(&mut p, 24, 8);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    let scope = state(&p).reading.as_ref().unwrap().scope.clone();
    shift(&mut p, KeyCode::Right);
    assert_eq!(
        copied(&mut p),
        Some(b"a".to_vec()),
        "copy works before the next paint"
    );
    assert_eq!(scope.selected_child().as_deref(), Some("message-0"));
    draw(&mut p, 24, 8);
    assert!(matches!(
        p.focus.as_ref().unwrap().kind,
        super::super::super::paint::TargetKind::Native
    ));
    shift(&mut p, KeyCode::Down);
    draw(&mut p, 24, 8);
    assert_eq!(copied(&mut p), Some(b"abc\nx".to_vec()));
    shift(&mut p, KeyCode::End);
    draw(&mut p, 24, 8);
    shift(&mut p, KeyCode::Right);
    shift(&mut p, KeyCode::End);
    draw(&mut p, 24, 8);
    assert_eq!(copied(&mut p), Some(b"abc\nxyz\nnext".to_vec()));
    close(&mut p);
}
#[test]
fn reading_header_shift_reveals_hidden_body_only_after_successful_flush() {
    let (_runtime, mut p, _) = native(vec![message(0, "abc\nxyz")]);
    draw(&mut p, 24, 2);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    shift(&mut p, KeyCode::End);
    assert_eq!(copied(&mut p), Some(b"abc".to_vec()));
    let mut buffer = Buffer::empty(Rect::new(0, 0, 24, 2));
    let scene = p.paint(Rect::new(0, 0, 24, 2), &mut buffer).unwrap();
    drop(scene);
    assert_eq!(state(&p).scrolls["reading"].offset, 0);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    draw(&mut p, 24, 2);
    draw(&mut p, 24, 2);
    assert!(state(&p).scrolls["reading"].offset > 0);
    assert!(matches!(
        p.focus.as_ref().unwrap().kind,
        super::super::super::paint::TargetKind::Native
    ));
    assert_eq!(copied(&mut p), Some(b"abc".to_vec()));
    close(&mut p);
}

#[test]
fn reading_header_shift_rejects_missing_disabled_and_unflushed_bodies() {
    let mut disabled = message(0, "disabled");
    disabled["children"][1]["props"]["disabled"] = json!(true);
    for first in [column("message-0", vec![item(0)]), disabled] {
        let (_runtime, mut p, _) = native(vec![first, message(1, "neighbor")]);
        draw(&mut p, 24, 8);
        shift(&mut p, KeyCode::End);
        assert!(copied(&mut p).is_none());
        assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
        close(&mut p);
    }
    let (_runtime, mut p, props) = native(vec![message(0, "original")]);
    draw(&mut p, 24, 8);
    props.borrow_mut()["content"]["children"][0]["children"][1]["props"]["source"] =
        json!("replacement");
    refresh(&mut p);
    shift(&mut p, KeyCode::End);
    assert!(copied(&mut p).is_none());
    draw(&mut p, 24, 8);
    shift(&mut p, KeyCode::End);
    assert_eq!(copied(&mut p), Some(b"replacement".to_vec()));
    close(&mut p);
}
#[test]
fn reading_header_shift_pending_focus_is_cancelled_by_tab() {
    let (_runtime, mut p, _) = native(vec![message(0, "abc"), item(1)]);
    draw(&mut p, 24, 3);
    shift(&mut p, KeyCode::End);
    assert_eq!(copied(&mut p), Some(b"abc".to_vec()));
    p.input(key(KeyCode::Tab)).unwrap();
    let focus = p.focus.clone().unwrap();
    draw(&mut p, 24, 3);
    draw(&mut p, 24, 3);
    assert!(p.focus.as_ref().unwrap().same(&focus));
    close(&mut p);
}

#[test]
fn reading_header_shift_starts_at_the_first_visible_body_row() {
    let (_runtime, mut p, _) = native(vec![message(0, "one\ntwo\nthree\nfour\nfive")]);
    draw(&mut p, 24, 3);
    mouse(&mut p, MouseEventKind::ScrollDown, 1, 1);
    draw(&mut p, 24, 3);
    assert_eq!(state(&p).scrolls["reading"].offset, 3);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    shift(&mut p, KeyCode::End);
    assert_eq!(copied(&mut p), Some(b"two".to_vec()));
    close(&mut p);
}

#[test]
fn item_space_submission_is_explicit_and_focus_remains_activation() {
    let mut first = item(0);
    first["submit_space"] = json!(true);
    first["submit"] = json!(false);
    let (_runtime, mut p) = layout::native(
        layout::View::InteractiveScene(Rc::new(RefCell::new(column("root", vec![first, item(1)])))),
        Rc::new(Cell::new(true)),
    );
    draw(&mut p, 24, 8);
    p.input(key(KeyCode::Char(' '))).unwrap();
    assert_eq!(observation(&p), json!({"kind":"submit"}));
    draw(&mut p, 24, 8);
    let revision = p.application.frame(&p.root).unwrap().revision;
    p.input(Event::Key(KeyEvent::new_with_kind(
        KeyCode::Char(' '),
        KeyModifiers::NONE,
        crossterm::event::KeyEventKind::Repeat,
    )))
    .unwrap();
    assert_eq!(p.application.frame(&p.root).unwrap().revision, revision);
    assert_eq!(observation(&p), json!({"kind":"submit"}));
    p.input(key(KeyCode::Enter)).unwrap();
    assert_eq!(observation(&p), json!({"kind":"activate"}));
    draw(&mut p, 24, 8);
    mouse(&mut p, MouseEventKind::Down(MouseButton::Left), 2, 0);
    assert_eq!(observation(&p), json!({"kind":"activate"}));
    draw(&mut p, 24, 8);
    mouse(&mut p, MouseEventKind::Down(MouseButton::Left), 2, 2);
    draw(&mut p, 24, 8);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-1");
    p.input(key(KeyCode::Char(' '))).unwrap();
    assert_eq!(observation(&p), json!({"kind":"activate"}));
    close(&mut p);
}

#[test]
fn list_scrollbar_paints_tracks_clicks_and_captures_drag_outside() {
    let (_runtime, mut p, _) = native((0..30).map(item).collect());
    let first = draw(&mut p, 24, 8);
    assert_eq!(first[(23, 0)].symbol(), "█");
    assert_eq!(first[(23, 7)].symbol(), "│");
    mouse(&mut p, MouseEventKind::ScrollDown, 2, 2);
    draw(&mut p, 24, 8);
    assert_eq!(state(&p).scrolls["reading"].offset, 3);
    mouse(&mut p, MouseEventKind::Down(MouseButton::Left), 23, 7);
    let end = draw(&mut p, 24, 8);
    let viewport = state(&p).scrolls["reading"].presented.as_ref().unwrap();
    assert_eq!(viewport.offset, viewport.end());
    assert_eq!(end[(23, 7)].symbol(), "█");
    assert!(p.drag.is_some());
    mouse(&mut p, MouseEventKind::Drag(MouseButton::Left), 30, 0);
    draw(&mut p, 24, 8);
    assert_eq!(state(&p).scrolls["reading"].offset, 0);
    mouse(&mut p, MouseEventKind::Up(MouseButton::Left), 30, 0);
    assert!(p.drag.is_none());
    close(&mut p);
}
