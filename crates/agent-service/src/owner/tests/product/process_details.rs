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
