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

use super::dialog::column;
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
    let (_runtime, mut p, _) = plain();
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
    p.input(key(KeyCode::Enter)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(observation(&p), json!({"kind":"submit"}));
    p.input(key(KeyCode::Home)).unwrap();
    draw(&mut p, 24, 6);
    assert_eq!(p.focus.as_ref().unwrap().key, "item-0");
    p.input(key(KeyCode::Tab)).unwrap();
    assert_eq!(p.focus.as_ref().unwrap().key, "after");
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
fn grouped_items_pointer_hover_does_not_select_and_second_line_activates() {
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
    assert_eq!(observation(&p), json!({"kind":"activate"}));
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
    assert_eq!(observation(&p), json!({"kind":"activate"}));
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
