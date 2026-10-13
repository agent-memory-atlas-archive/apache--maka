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

//! Paths a reply names, in the window: on a Host on this machine a click
//! on one goes to the Files face, which previews a Markdown file there and
//! shows itself, or only shows a script in Finder, even through a link the
//! agent wrote with the client's own scheme. The face's desk is stubbed:
//! nothing opens or shows on the machine running the tests. On a Host
//! elsewhere the same reply is text.

// Tests write their scratch files, and the changes' read checks the task
// folder on the blocking pool.
#![allow(clippy::disallowed_methods)]

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use files::Desk;
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{App, ElementId, TestAppContext, point, px};
use host_protocol::{AccessCredential, RemoteTransport};
use workspace::{RemoteHost, RemoteHostProfile, WindowHost};

use crate::chrome_tests::row;
use crate::find_tests::open_with_reply;
use crate::tests::{Harness, ScriptedHost, session, settle};
use crate::workbar_tests::NoGit;

/// A task folder holding `notes.md` and `run.command`, removed when
/// dropped.
struct Folder(PathBuf);

impl Folder {
    fn new() -> Self {
        let folder =
            std::env::temp_dir().join(format!("maka-app-reply-paths-{}", uuid_like())).join("demo");
        std::fs::create_dir_all(&folder).expect("folder");
        let folder = std::fs::canonicalize(folder).expect("canonical");
        std::fs::write(folder.join("notes.md"), "# Notes\n\nReady.").expect("notes");
        std::fs::write(folder.join("run.command"), "#!/bin/sh\n").expect("command");
        Self(folder)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        if let Some(parent) = self.0.parent() {
            std::fs::remove_dir_all(parent).ok();
        }
    }
}

/// A name no other run uses.
fn uuid_like() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    format!("{}-{nanos}", std::process::id())
}

/// What the Files face handed to the default app and to Finder.
#[derive(Default)]
struct DeskLog {
    opened: RefCell<Vec<PathBuf>>,
    revealed: RefCell<Vec<PathBuf>>,
}

/// The window on `window_host` with task `s1`, run in `folder`, selected;
/// its one reply is `reply`.
fn window_with_reply(
    window_host: WindowHost,
    folder: &Path,
    reply: &str,
    cx: &mut TestAppContext,
) -> (Harness, Rc<DeskLog>) {
    let path = folder.to_str().expect("utf-8");
    let transport = ScriptedHost::new(vec![session("s1", "Alpha", path, "active")]);
    transport.reply("subscription.open", Ok(open_with_reply(reply)));
    cx.executor().allow_parking();
    let harness = Harness::on_host(transport, window_host, cx);
    let log = Rc::new(DeskLog::default());
    let desk = {
        let (opened, revealed) = (log.clone(), log.clone());
        Desk::new(
            Rc::new(|_: &str, cx: &mut App| cx.spawn(async |_| None)),
            Rc::new(move |path: &Path, _: &mut App| opened.opened.borrow_mut().push(path.into())),
            None,
        )
        .with_reveal(Rc::new(move |path: &Path, _: &mut App| {
            revealed.revealed.borrow_mut().push(path.into());
        }))
    };
    harness.workbench.update(cx, |workbench, cx| {
        workbench.review_panel().update(cx, |panel, _| panel.set_git_runner(Arc::new(NoGit)));
        workbench.files_view().update(cx, |files, _| files.set_desk(desk));
    });
    harness.with_window(cx, |window, cx| window.click(row("s1"), cx));
    frames(&harness, cx);
    (harness, log)
}

/// Lets the reply's check, its text view's parse and what a click set off
/// land.
fn frames(harness: &Harness, cx: &mut TestAppContext) {
    for _ in 0..4 {
        harness.with_window(cx, |_, _| {});
        settle(cx);
    }
}

fn reply_id() -> ElementId {
    conversation::item_element_id("t1", &transcript_model::ItemKey::Text("t1-a".into()))
}

/// Clicks the start of the reply's first line.
fn click_reply(harness: &Harness, cx: &mut TestAppContext) {
    harness.with_window(cx, |window, cx| window.click_at(reply_id(), point(px(16.), px(10.)), cx));
    frames(harness, cx);
}

/// The file on this machine the Files face previews.
fn previewed(harness: &Harness, cx: &mut TestAppContext) -> Option<PathBuf> {
    harness.workbench.read_with(cx, |workbench, cx| {
        let files = workbench.files_view().read(cx);
        files
            .preview()
            .and_then(|preview| preview.read(cx).source().local().map(|file| file.path.clone()))
    })
}

/// Whether the panel shows, on the Files face.
fn files_shown(harness: &Harness, cx: &mut TestAppContext) -> bool {
    harness.workbench.read_with(cx, |workbench, cx| {
        workbench.workbar_shown(cx)
            && workbench.strip_tabs(cx).iter().any(|(key, _, shown)| key == "files" && *shown)
    })
}

fn remote_host() -> WindowHost {
    let transport = RemoteTransport::tls("wss://box.example.com/runtime-host").expect("tls");
    let profile =
        RemoteHostProfile::new("box", "Box", &"a".repeat(64), transport).expect("profile");
    let credential = AccessCredential::new("mrha_x").expect("credential");
    WindowHost::Remote(RemoteHost::new(profile, credential))
}

/// A Markdown path in the task folder, relative as the agent wrote it,
/// opens the Files face on a preview of the file there.
#[gpui_kit::test]
fn a_markdown_path_in_a_reply_previews_the_file_in_the_files_face(cx: &mut TestAppContext) {
    let folder = Folder::new();
    let (harness, log) =
        window_with_reply(WindowHost::Local, folder.path(), "`notes.md` is ready.", cx);
    assert!(!files_shown(&harness, cx));
    click_reply(&harness, cx);
    assert_eq!(previewed(&harness, cx), Some(folder.path().join("notes.md")));
    assert!(files_shown(&harness, cx), "the face shows the preview");
    let name = harness.with_window(cx, |window, _| {
        window.try_find("files-preview-name").and_then(|name| name.label().map(str::to_owned))
    });
    assert_eq!(name.as_deref(), Some("notes.md"));
    assert!(log.opened.borrow().is_empty() && log.revealed.borrow().is_empty());
}

/// A link the agent wrote with the client's own scheme to a script only
/// shows the script in Finder: nothing runs, and the face stays as it was.
#[gpui_kit::test]
fn an_agent_written_link_to_a_script_only_shows_it_in_finder(cx: &mut TestAppContext) {
    let folder = Folder::new();
    let (harness, log) = window_with_reply(
        WindowHost::Local,
        folder.path(),
        "[Run the setup script now](maka-file:run.command)",
        cx,
    );
    click_reply(&harness, cx);
    assert_eq!(*log.revealed.borrow(), [folder.path().join("run.command")]);
    assert!(log.opened.borrow().is_empty(), "nothing is opened or launched");
    assert_eq!(previewed(&harness, cx), None);
    assert!(!files_shown(&harness, cx));
    assert_eq!(cx.opened_url(), None);
}

/// On a Host elsewhere a path in a reply is text, even one that exists on
/// this machine: a click does nothing.
#[gpui_kit::test]
fn a_remote_hosts_reply_names_no_path(cx: &mut TestAppContext) {
    let folder = Folder::new();
    let notes = folder.path().join("notes.md");
    let reply = format!("`{}` is ready.", notes.display());
    let (harness, log) = window_with_reply(remote_host(), folder.path(), &reply, cx);
    click_reply(&harness, cx);
    assert_eq!(previewed(&harness, cx), None);
    assert!(!files_shown(&harness, cx));
    assert!(log.opened.borrow().is_empty() && log.revealed.borrow().is_empty());
}
