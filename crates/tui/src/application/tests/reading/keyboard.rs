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

fn key(presenter: &mut Presenter, code: KeyCode) {
    presenter
        .input(Event::Key(KeyEvent::new(code, KeyModifiers::SHIFT)))
        .unwrap();
}
fn settle(presenter: &mut Presenter, width: u16, height: u16) {
    // The first confirmed frame resolves the head's row; the next paints the
    // resulting viewport. Both use the real original native presenter.
    draw(presenter, width, height);
    draw(presenter, width, height);
    presenter.sync_native().unwrap();
}
#[test]
fn alt_click_extends_original_range_across_sources_and_preserves_exact_unicode() {
    let (_runtime, mut presenter, _) =
        native(vec![source("a", "one 中文"), source("b", "😀 tail")]);
    draw(&mut presenter, 20, 4);
    mouse(
        &mut presenter,
        MouseEventKind::Down(MouseButton::Left),
        4,
        0,
    );
    mouse(&mut presenter, MouseEventKind::Up(MouseButton::Left), 4, 0);
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: 1,
            modifiers: KeyModifiers::ALT,
        }))
        .unwrap();
    assert_eq!(copied(&mut presenter).unwrap(), "中文\n😀".as_bytes());
    assert_eq!(observation(&presenter)["selectedChildKey"], "b");
    close(&mut presenter);
}
#[test]
fn vertical_range_retains_column_across_short_lines_and_clipped_original_documents() {
    let (_runtime, mut presenter, _) = native(vec![
        source("a", "abcdef\nx"),
        source("b", "中文abcd\nlast"),
    ]);
    draw(&mut presenter, 20, 2);
    mouse(
        &mut presenter,
        MouseEventKind::Down(MouseButton::Left),
        4,
        0,
    );
    mouse(&mut presenter, MouseEventKind::Up(MouseButton::Left), 4, 0);
    key(&mut presenter, KeyCode::Down);
    assert_eq!(copied(&mut presenter).unwrap(), b"ef\nx");
    key(&mut presenter, KeyCode::Down);
    settle(&mut presenter, 20, 2);
    assert_eq!(copied(&mut presenter).unwrap(), "ef\nx\n中文".as_bytes());
    assert_eq!(observation(&presenter)["selectedChildKey"], "b");
    assert_eq!(state(&presenter).scrolls["reading"].offset, 1);
    key(&mut presenter, KeyCode::Down);
    settle(&mut presenter, 20, 2);
    assert_eq!(
        copied(&mut presenter).unwrap(),
        "ef\nx\n中文abcd\nlast".as_bytes()
    );
    key(&mut presenter, KeyCode::Up);
    settle(&mut presenter, 20, 2);
    assert_eq!(copied(&mut presenter).unwrap(), "ef\nx\n中文".as_bytes());
    key(&mut presenter, KeyCode::Up);
    key(&mut presenter, KeyCode::Up);
    settle(&mut presenter, 20, 2);
    assert!(copied(&mut presenter).is_none());
    assert_eq!(state(&presenter).scrolls["reading"].offset, 0);
    close(&mut presenter);
}
#[test]
fn page_range_crosses_documents_reveals_head_and_keeps_keyboard_copy_after_old_source_leaves_view()
{
    let children = (0..9)
        .map(|i| source(&format!("m{i}"), &format!("row-{i}")))
        .collect();
    let (_runtime, mut presenter, _) = native(children);
    draw(&mut presenter, 12, 3);
    key(&mut presenter, KeyCode::PageDown);
    settle(&mut presenter, 12, 3);
    assert_eq!(copied(&mut presenter).unwrap(), b"row-0\nrow-1\n");
    key(&mut presenter, KeyCode::PageDown);
    settle(&mut presenter, 12, 3);
    assert_eq!(observation(&presenter)["selectedChildKey"], "m4");
    assert_eq!(state(&presenter).scrolls["reading"].offset, 2);
    key(&mut presenter, KeyCode::Right);
    settle(&mut presenter, 12, 3);
    assert_eq!(
        copied(&mut presenter).unwrap(),
        b"row-0\nrow-1\nrow-2\nrow-3\nr"
    );
    key(&mut presenter, KeyCode::PageUp);
    settle(&mut presenter, 12, 3);
    assert_eq!(copied(&mut presenter).unwrap(), b"row-0\nrow-1\nr");
    close(&mut presenter);
}
#[test]
fn soft_wrap_end_preserves_visual_affinity_and_home_returns_to_same_row() {
    let (_runtime, mut presenter, _) = native(vec![source("a", "abcdefghij")]);
    draw(&mut presenter, 5, 3);
    key(&mut presenter, KeyCode::End);
    assert_eq!(copied(&mut presenter).unwrap(), b"abcde");
    key(&mut presenter, KeyCode::Home);
    assert!(copied(&mut presenter).is_none());
    key(&mut presenter, KeyCode::End);
    key(&mut presenter, KeyCode::Down);
    assert_eq!(copied(&mut presenter).unwrap(), b"abcdefghij");
    close(&mut presenter);
}
#[test]
fn failed_flush_and_new_pointer_intent_cannot_apply_keyboard_reveal() {
    let (_runtime, mut presenter, _) = native(vec![source("a", "0\n1\n2\n3\n4\n5")]);
    draw(&mut presenter, 12, 2);
    key(&mut presenter, KeyCode::PageDown);
    key(&mut presenter, KeyCode::PageDown);
    let area = Rect::new(0, 0, 12, 2);
    let mut buffer = Buffer::empty(area);
    let old = presenter.paint(area, &mut buffer).unwrap();
    assert_eq!(state(&presenter).scrolls["reading"].offset, 0);
    mouse(
        &mut presenter,
        MouseEventKind::Down(MouseButton::Left),
        0,
        0,
    );
    presenter.present(old);
    settle(&mut presenter, 12, 2);
    assert_eq!(state(&presenter).scrolls["reading"].offset, 0);
    assert!(copied(&mut presenter).is_none());
    close(&mut presenter);
}

#[test]
fn tab_cancels_pending_reveal_and_settled_range_never_steals_new_focus() {
    let (_runtime, mut presenter, _) = native(vec![
        source("a", "aaa"),
        source("b", "bbb"),
        source("c", "ccc"),
        source("d", "ddd"),
    ]);
    draw(&mut presenter, 12, 2);
    key(&mut presenter, KeyCode::Down);
    key(&mut presenter, KeyCode::Down);
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
        .unwrap();
    let chosen = presenter.focus.as_ref().unwrap().mount.id();
    settle(&mut presenter, 12, 2);
    assert_eq!(state(&presenter).scrolls["reading"].offset, 0);
    assert_eq!(presenter.focus.as_ref().unwrap().mount.id(), chosen);
    // Complete a new movement, then select another ordinary native tab stop.
    key(&mut presenter, KeyCode::Up);
    settle(&mut presenter, 12, 4);
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
        .unwrap();
    let chosen = presenter.focus.as_ref().unwrap().mount.id();
    settle(&mut presenter, 12, 4);
    assert_eq!(presenter.focus.as_ref().unwrap().mount.id(), chosen);
    close(&mut presenter);
}
#[test]
fn source_replacement_withdraws_pending_keyboard_range_before_new_geometry() {
    let (_runtime, mut presenter, props) =
        native(vec![source("a", "a\nb\nc"), source("b", "original")]);
    draw(&mut presenter, 12, 2);
    key(&mut presenter, KeyCode::PageDown);
    key(&mut presenter, KeyCode::PageDown);
    props.borrow_mut()["content"]["children"][0]["props"]["source"] = json!("replacement");
    refresh(&mut presenter);
    settle(&mut presenter, 12, 2);
    assert!(copied(&mut presenter).is_none());
    assert!(observation(&presenter)["selectedChildKey"].is_null());
    close(&mut presenter);
}

#[test]
fn hide_and_reveal_retains_the_range_without_replaying_pending_keyboard_scroll() {
    let (_runtime, mut presenter, props) = native(vec![source("a", "0\n1\n2\n3\n4")]);
    draw(&mut presenter, 12, 2);
    key(&mut presenter, KeyCode::Down);
    key(&mut presenter, KeyCode::Down);
    let selected = copied(&mut presenter).unwrap();
    props.borrow_mut()["content"]["visible"] = json!(false);
    refresh(&mut presenter);
    draw(&mut presenter, 12, 2);
    assert!(copied(&mut presenter).is_none());
    props.borrow_mut()["content"]["visible"] = json!(true);
    refresh(&mut presenter);
    settle(&mut presenter, 12, 2);
    assert_eq!(state(&presenter).scrolls["reading"].offset, 0);
    assert_eq!(copied(&mut presenter).unwrap(), selected);
    close(&mut presenter);
}
