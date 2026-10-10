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

pub(super) fn run(
    presenter: &mut maka_tui::application::Presenter,
    capture: &impl Fn(&mut maka_tui::application::Presenter, &str),
) {
    await_screen(presenter, "Shell command");
    // The original reading viewport is shorter than the command, cwd and failure.
    // Reveal the output with real wheel input, leaving the native owner in charge.
    for _ in 0..3 {
        presenter
            .input(Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 10,
                row: 8,
                modifiers: KeyModifiers::NONE,
            }))
            .unwrap();
        draw(presenter);
    }
    let (column, row) = wait_for_target(presenter, "Standard output", 0);
    await_screen(presenter, "process stdout");
    presenter
        .input(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row: row + 1,
            modifiers: KeyModifiers::NONE,
        }))
        .unwrap();
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::SHIFT)))
        .unwrap();
    let mut copied = Vec::new();
    assert!(presenter.copy(&mut copied).unwrap());
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(&copied[7..copied.len() - 1])
            .unwrap(),
        b"process stdout"
    );
    capture(presenter, "native-process-output.txt");
    for label in ["Copy message", "Copy raw tool result"] {
        super::message_copy::command(presenter, label);
        capture(
            presenter,
            if label == "Copy message" {
                "native-process-copy-message.txt"
            } else {
                "native-process-copy-source.txt"
            },
        );
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();
        let mut copied = Vec::new();
        assert!(presenter.flush_clipboard(&mut copied).unwrap());
        let value = String::from_utf8(
            base64::engine::general_purpose::STANDARD
                .decode(&copied[7..copied.len() - 1])
                .unwrap(),
        )
        .unwrap();
        if label == "Copy message" {
            assert!(
                value.starts_with("Tool output · Run command\nReturned\n\nTool arguments\n"),
                "{value}"
            );
            for original in [
                "\"command\": printf 'native file contents'",
                "\"stdout\": process stdout",
                "\"stderr\": process stderr",
                "\"exitCode\": 7",
                "\"status\": failed",
                "Tool result\n",
            ] {
                assert!(value.contains(original), "missing {original}: {value}");
            }
        } else {
            let result: serde_json::Value = serde_json::from_str(&value).unwrap();
            assert_eq!(result["kind"], "completed");
            assert_eq!(result["value"]["exitCode"], 7);
            assert_eq!(result["value"]["output"]["stdout"], "process stdout");
            assert_eq!(result["value"]["output"]["stderr"], "process stderr");
        }
        assert!(!presenter.flush_clipboard(&mut Vec::new()).unwrap());
        // A new physical shortcut waits for the closed modal's successful draw.
        assert!(!screen(&draw(presenter)).contains("[ Close commands ]"));
    }
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    for _ in 0..20 {
        let visible = screen(&draw(presenter));
        if visible.contains("[ Tool arguments ]") {
            return;
        }
        presenter
            .input(Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 10,
                row: 8,
                modifiers: KeyModifiers::NONE,
            }))
            .unwrap();
    }
    panic!(
        "process result controls did not become visible: {}",
        screen(&draw(presenter))
    );
}

pub(super) fn header(presenter: &mut maka_tui::application::Presenter) {
    for _ in 0..20 {
        if screen(&draw(presenter)).contains("[ Hide details ]") {
            return;
        }
        presenter
            .input(Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 10,
                row: 8,
                modifiers: KeyModifiers::NONE,
            }))
            .unwrap();
    }
    panic!(
        "process header did not become visible: {}",
        screen(&draw(presenter))
    );
}
