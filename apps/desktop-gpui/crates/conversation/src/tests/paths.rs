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

//! Paths a reply names open on this machine (`crate::paths`): once the
//! reply has settled, a code span or a link naming a path that exists is a
//! link with the client's own scheme, and a click on it asks the owner to
//! open the path again resolved. A Host elsewhere, a user message, a reply
//! still streaming and a span that names nothing there stay text. A find
//! still matches inside a path and paints it in place.

#![allow(clippy::disallowed_methods)] // Tests write their scratch files.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::{point, px};

use super::find::{count, fills, find, open_find, open_tail, page_of, reply, turn};
use super::*;
use crate::{ConversationViewEvent, LocalPaths};

/// A task folder with `notes.md`, `src/` and `run.command`, removed when
/// dropped.
struct Scratch {
    folder: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let folder = std::env::temp_dir()
            .join(format!("maka-reply-paths-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(folder.join("src")).expect("folder");
        std::fs::write(folder.join("notes.md"), b"# Notes").expect("notes");
        std::fs::write(folder.join("run.command"), b"#!/bin/sh\n").expect("command");
        Self { folder }
    }

    fn paths(&self) -> LocalPaths {
        LocalPaths::new(None, Some(self.folder.clone()))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.folder).ok();
    }
}

/// The session open on turn `t1`, whose reply is `text`, with what its
/// view asks of its owner recorded.
fn open_reply(
    text: &str,
    cx: &mut TestAppContext,
) -> (Harness, Rc<RefCell<Vec<ConversationViewEvent>>>) {
    let rows = turn("t1", 1, "Where is it?", None, text);
    let harness = open_tail(page_of(&rows, None), false, cx);
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    cx.update(|cx| {
        cx.subscribe(&harness.view, move |_, event: &ConversationViewEvent, _| {
            recorded.borrow_mut().push(event.clone());
        })
        .detach();
    });
    (harness, events)
}

/// Hands the view where paths lie (`None`: a Host elsewhere) and lets the
/// check and the text view's parse land.
fn set_paths(harness: &Harness, paths: Option<LocalPaths>, cx: &mut TestAppContext) {
    harness.view.update(cx, |view, cx| view.set_local_paths(paths, cx));
    for _ in 0..3 {
        settle(cx);
        harness.with_window(cx, |_, _| {});
    }
}

/// Clicks the start of the first line of `row`.
fn click_start(harness: &Harness, row: &RowKey, cx: &mut TestAppContext) {
    let id = row.element_id();
    harness.with_window(cx, |window, cx| window.click_at(id, point(px(16.), px(10.)), cx));
    settle(cx);
}

fn opened(events: &Rc<RefCell<Vec<ConversationViewEvent>>>) -> Vec<PathBuf> {
    events
        .borrow()
        .iter()
        .filter_map(|event| match event {
            ConversationViewEvent::OpenPath { path } => Some(path.clone()),
            _ => None,
        })
        .collect()
}

/// A path in a settled reply opens on a click, as the task folder
/// resolves it; on a Host elsewhere the same reply is text again.
#[gpui_kit::test]
fn a_settled_reply_s_path_opens_on_a_click(cx: &mut TestAppContext) {
    let scratch = Scratch::new();
    let (harness, events) = open_reply("`notes.md` is ready.", cx);
    let row = reply("t1");
    click_start(&harness, &row, cx);
    assert!(opened(&events).is_empty(), "nothing is a link before the view knows the paths");

    set_paths(&harness, Some(scratch.paths()), cx);
    assert!(harness.view.read_with(cx, |view, _| view.path_checked(&row)));
    click_start(&harness, &row, cx);
    assert_eq!(opened(&events), [scratch.folder.join("notes.md")]);
    assert_eq!(cx.opened_url(), None, "the platform is handed nothing");

    // A Host elsewhere: the path is text again, and a click does nothing.
    set_paths(&harness, None, cx);
    click_start(&harness, &row, cx);
    assert_eq!(opened(&events).len(), 1);
}

/// Only paths that exist are links: a command, a missing file and a URL
/// in code stay text.
#[gpui_kit::test]
fn a_span_naming_nothing_there_stays_text(cx: &mut TestAppContext) {
    let scratch = Scratch::new();
    for text in ["`ls ~/Downloads` lists it.", "`gone.md` is gone.", "`https://example.com`"] {
        let (harness, events) = open_reply(text, cx);
        set_paths(&harness, Some(scratch.paths()), cx);
        let row = reply("t1");
        let links = harness.view.read_with(cx, |view, _| {
            let text = view.rows().iter().find_map(|found| match &found.body {
                RowBody::Text { text, .. } if found.key == row => Some(text.clone()),
                _ => None,
            });
            text.and_then(|text| view.path_links(&row, &text))
        });
        assert!(links.is_none(), "{text}");
        click_start(&harness, &row, cx);
        assert!(opened(&events).is_empty(), "{text}");
    }
}

/// A link the agent wrote with the client's own scheme goes the same way
/// as the client's: the owner is handed the path, resolved again, and
/// decides what opens. A Host elsewhere is handed nothing, nor is the
/// platform.
#[gpui_kit::test]
fn an_agent_written_path_link_goes_to_the_owner_only_here(cx: &mut TestAppContext) {
    let scratch = Scratch::new();
    let (harness, events) = open_reply("[Run the setup script now](maka-file:run.command)", cx);
    let row = reply("t1");
    click_start(&harness, &row, cx);
    assert!(opened(&events).is_empty(), "a Host elsewhere");
    assert_eq!(cx.opened_url(), None);

    set_paths(&harness, Some(scratch.paths()), cx);
    click_start(&harness, &row, cx);
    assert_eq!(opened(&events), [scratch.folder.join("run.command")]);
    assert_eq!(cx.opened_url(), None);
}

/// A user message is never checked, and a reply only once it has
/// finished streaming.
#[gpui_kit::test]
fn a_user_message_and_a_streaming_reply_are_not_checked(cx: &mut TestAppContext) {
    let scratch = Scratch::new();
    let (harness, mut frames) = start_turn(cx);
    harness.view.update(cx, |view, cx| view.set_local_paths(Some(scratch.paths()), cx));
    harness.push(frames.delta("m1", 0, "Wrote `notes.md` and"), cx);
    settle(cx);
    harness.with_window(cx, |_, _| {});
    let (streaming, user) = harness.view.read_with(cx, |view, _| {
        let key = |pick: fn(&RowBody) -> bool| {
            view.rows().iter().find(|row| pick(&row.body)).map(|row| row.key.clone())
        };
        (
            key(|body| matches!(body, RowBody::Text { streaming: true, .. })),
            key(|body| matches!(body, RowBody::User { .. })),
        )
    });
    let (streaming, user) = (streaming.expect("a streaming reply"), user.expect("the prompt"));
    harness.view.read_with(cx, |view, _| {
        assert!(!view.path_checked(&streaming), "not while it streams");
        assert!(!view.path_checked(&user), "never a user message");
    });
}

/// A find matches inside a path that is a link, and paints the match on
/// the text it draws.
#[gpui_kit::test]
fn find_matches_inside_a_path_link(cx: &mut TestAppContext) {
    let scratch = Scratch::new();
    let (harness, _) = open_reply("Open `notes.md` and `src` now.", cx);
    set_paths(&harness, Some(scratch.paths()), cx);
    let row = reply("t1");
    assert!(harness.view.read_with(cx, |view, _| view.path_checked(&row)));
    open_find(&harness, cx);
    find(&harness, "notes", cx);
    assert_eq!(count(&harness, cx).as_deref(), Some("1/1"));
    let (_, active_fill) = fills(cx);
    let highlights = harness.view.read_with(cx, |view, _| view.text_views().highlights(&row));
    // "Open notes.md and src now.": `notes`, in the active fill.
    assert_eq!(highlights, [(5..10, active_fill)]);
    let shown = harness.view.read_with(cx, |view, cx| {
        let state = view.text_views().get(&row).expect("laid out");
        state.read(cx).rendered_text().as_str().to_owned()
    });
    assert_eq!(shown.trim_end(), "Open notes.md and src now.", "the text drawn is the reply's");
}
