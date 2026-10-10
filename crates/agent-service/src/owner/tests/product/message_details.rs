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
use base64::Engine as _;
use maka_runtime::{
    attachment::{AttachmentKind, AttachmentRef, StorageRef},
    event::{EventWrite, Fact, Invocation, InvocationInput, InvocationOutcome, RuntimeEvent},
    input::{DirectoryReference, MessageInput},
};

pub(super) fn run(
    presenter: &mut maka_tui::application::Presenter,
    runtime: &tokio::runtime::Runtime,
    fixture: &Fixture,
    session: &str,
    capture: &impl Fn(&mut maka_tui::application::Presenter, &str),
) {
    // Canonical historical facts enter through the real store. No invented
    // UI/source response, file read, upload or model invocation is introduced.
    runtime.block_on(async {
        let mut content: MessageInput = "hidden model material".into();
        content.display_text = Some("Visible material prompt".into());
        content.attachments = Some(vec![AttachmentRef {
            kind: AttachmentKind::Other,
            name: "notes.txt".into(),
            mime_type: "text/plain".into(),
            bytes: 2048,
            storage_ref: StorageRef::WorkspaceFile {
                relative_path: "not-opened.txt".into(),
            },
        }]);
        content.directory_references = Some(vec![DirectoryReference {
            host_id: "original-host".into(),
            path: "src/中文".into(),
        }]);
        let invocation = Invocation {
            session_id: session.into(),
            turn_id: "material-turn".into(),
            run_id: "material-run".into(),
            invocation_id: "material-invocation".into(),
        };
        for fact in [
            Fact::InvocationOpened {
                configuration: None,
                input: InvocationInput::Message {
                    content,
                    source_messages: Vec::new(),
                    request_fingerprint: None,
                },
            },
            Fact::InvocationEnded {
                outcome: InvocationOutcome::Completed,
            },
        ] {
            let mut event = RuntimeEvent::new(invocation.clone(), fact);
            event.recorded_at = std::time::UNIX_EPOCH + Duration::from_millis(946_771_200_000);
            fixture
                .store
                .append(&EventWrite::plain(event).unwrap())
                .await
                .unwrap();
        }
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let visible = screen(&draw(presenter));
        if visible.contains("Visible material prompt") {
            break;
        }
        if let Some((row, column)) = visible
            .lines()
            .enumerate()
            .find_map(|(row, line)| line.find("[ Latest messages ]").map(|column| (row, column)))
        {
            // Latest is a read. Use this painted frame: the automatic tail
            // may finish and remove the control before a separate click wait.
            presenter
                .input(Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: column as u16,
                    row: row as u16,
                    modifiers: KeyModifiers::NONE,
                }))
                .unwrap();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "canonical material body did not arrive: {visible}"
        );
        std::thread::yield_now();
    }
    await_screen(presenter, "↳ notes.txt · 2.0 KiB");
    directory_label(presenter);
    await_screen(presenter, "You · 2000-");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    await_screen(presenter, "Find…");
    presenter
        .input(Event::Paste("Visible material".into()))
        .unwrap();
    await_screen(presenter, "1/1");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('f'),
            KeyModifiers::ALT,
        )))
        .unwrap();
    await_screen(presenter, "[ › 1. Visible material prompt ]");
    await_screen(presenter, "↳ notes.txt · 2.0 KiB");
    directory_label(presenter);
    await_screen(presenter, "You · 2000-");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(presenter, "File background draft");
    // Enter browsing through the ordinary palette after closing native Find.
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    wait_for_target(presenter, "[ Close commands ]", 0);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste("Browse messages".into()))
        .unwrap();
    wait_for_target(presenter, "[ › Browse messages", 0);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(presenter, "messages ·");
    draw(presenter);
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)))
        .unwrap();
    let (column, row) = wait_for_target(presenter, "You · 2000-", 0);
    let buffer = draw(presenter);
    assert!(
        buffer[(column, row)]
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED)
    );
    capture(presenter, "native-message-browse.txt");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)))
        .unwrap();
    draw(presenter);
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)))
        .unwrap();
    let (column, row) = wait_for_target(presenter, "You · 2000-", 0);
    assert!(
        draw(presenter)[(column, row)]
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED)
    );
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::SHIFT)))
        .unwrap();
    draw(presenter);
    let mut selection = Vec::new();
    assert!(presenter.copy(&mut selection).unwrap());
    let expected = b"Visible material prompt";
    assert_eq!(
        selection,
        format!(
            "\x1b]52;c;{}\x07",
            base64::engine::general_purpose::STANDARD.encode(expected)
        )
        .as_bytes()
    );
    capture(presenter, "native-header-selection.txt");
    // The first Escape clears the original native range; the second leaves Browse.
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    draw(presenter);
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(presenter, "File background draft");
    presenter
        .input(Event::Paste(" browse returned".into()))
        .unwrap();
    await_screen(presenter, "File background draft browse returned");
    capture(presenter, "native-browse-return.txt");
    assert!(!fixture.workspace.join("not-opened.txt").exists());
}

fn directory_label(presenter: &mut maka_tui::application::Presenter) {
    let (column, row) = wait_for_target(presenter, "▱ src/", 0);
    let buffer = draw(presenter);
    assert_eq!(buffer[(column + 6, row)].symbol(), "中");
    assert_eq!(buffer[(column + 8, row)].symbol(), "文");
}

pub(super) fn tool_space(
    presenter: &mut maka_tui::application::Presenter,
    capture: &impl Fn(&mut maka_tui::application::Presenter, &str),
) {
    click(presenter, "native file contents");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    wait_for_target(presenter, "[ Close commands ]", 0);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter
        .input(Event::Paste("Browse messages".into()))
        .unwrap();
    wait_for_target(presenter, "[ › Browse messages", 0);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(presenter, "messages ·");
    draw(presenter);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(presenter, "[ Show details ]");
    assert!(!screen(&draw(presenter)).contains("Requested content"));
    capture(presenter, "native-space-collapsed.txt");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(presenter, "Requested content");
    await_screen(presenter, "native file contents");
    capture(presenter, "native-space-expanded.txt");
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(presenter, "File background draft");
}
