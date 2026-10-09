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
use base64::Engine;
use maka_runtime::shell_result::{ShellMode, ShellOutput, ShellStatus};

pub(super) fn run(
    presenter: &mut maka_tui::application::Presenter,
    runtime: &tokio::runtime::Runtime,
    fixture: &Fixture,
    session: &str,
) {
    click(presenter, "[ Terminals & tasks ]");
    click(presenter, "[ New terminal ]");
    click(presenter, "[ Connect input ]");
    await_screen(presenter, "Input connected");
    for _ in 0..24 {
        if presenter.captures_keyboard().unwrap() {
            break;
        }
        presenter
            .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        draw(presenter);
    }
    assert!(
        presenter.captures_keyboard().unwrap(),
        "{}",
        screen(&draw(presenter))
    );
    presenter
        .input(Event::Paste(
            "printf '\\033[2J\\033[Hterminal-%s\\n' workflow-marker".into(),
        ))
        .unwrap();
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    // This contiguous marker is absent from the echoed command. It is produced
    // by the real PTY and displayed through the canonical screen subscription.
    await_screen(presenter, "terminal-workflow-marker");
    let mut clipboard = Vec::new();
    assert!(presenter.copy(&mut clipboard).unwrap());
    let clipboard = std::str::from_utf8(&clipboard).unwrap();
    let encoded = clipboard
        .strip_prefix("\x1b]52;c;")
        .unwrap()
        .strip_suffix('\x07')
        .unwrap();
    let copied = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    assert!(
        std::str::from_utf8(&copied)
            .unwrap()
            .contains("terminal-workflow-marker")
    );
    // Overflow the actual visible viewport: a clipped twelve-row logical
    // screen must not hide the shell's newest line in an 80x24 dialog.
    presenter
        .input(Event::Paste(
            "i=0; while [ $i -lt 30 ]; do printf 'row %s\\n' $i; i=$((i+1)); done; printf 'terminal-%s\\n' tail-marker".into(),
        ))
        .unwrap();
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(presenter, "terminal-tail-marker");
    let original = runtime
        .block_on(fixture.store.query_shell_resources(session, None, 0))
        .unwrap()
        .resources
        .into_iter()
        .find(|r| r.result.mode == ShellMode::Pty)
        .expect("user terminal persisted in its original Session");
    assert_eq!(original.result.status, ShellStatus::Running);
    let Some(ShellOutput::Pty {
        cols: original_cols,
        ..
    }) = original.result.output
    else {
        panic!("PTY canonical output missing");
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let buffer = draw_at(presenter, Rect::new(0, 0, 64, 32));
        let page = runtime
            .block_on(fixture.store.query_shell_resources(session, None, 0))
            .unwrap();
        if page.resources.iter().any(|r| {
            r.result.resource_ref == original.result.resource_ref
                && matches!(r.result.output, Some(ShellOutput::Pty { cols, rows, .. })
                    if cols > 0 && cols < original_cols && rows > 0 && rows < 32)
        }) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "PTY resize did not reach original owner: {}",
            screen(&buffer)
        );
        std::thread::yield_now();
    }
    await_screen(presenter, "terminal-tail-marker");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char(']'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    assert!(!presenter.captures_keyboard().unwrap());
    click(presenter, "[ Close ]");
    await_screen(presenter, "File background draft");
    let still_running = runtime
        .block_on(fixture.store.query_shell_resources(session, None, 0))
        .unwrap()
        .resources
        .into_iter()
        .find(|r| r.result.resource_ref == original.result.resource_ref)
        .unwrap();
    assert_eq!(still_running.result.status, ShellStatus::Running);
    assert!(
        matches!(&still_running.result.output, Some(ShellOutput::Pty { screen, scrollback, .. })
        if screen.contains("terminal-tail-marker") || scrollback.contains("terminal-tail-marker"))
    );
    click(presenter, "[ Terminals & tasks ]");
    click(presenter, "[ Connect input ]");
    await_screen(presenter, "Input connected");
    for _ in 0..24 {
        if presenter.captures_keyboard().unwrap() {
            break;
        }
        presenter
            .input(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        draw(presenter);
    }
    assert!(presenter.captures_keyboard().unwrap());
    presenter
        .input(Event::Paste(
            "printf '\\033[2J\\033[Hterminal-%s\\n' reconnected-marker".into(),
        ))
        .unwrap();
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )))
        .unwrap();
    await_screen(presenter, "terminal-reconnected-marker");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char(']'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    click(presenter, "[ Stop ]");
    await_screen(presenter, "Stop this task?");
    click(presenter, "[ Stop ]");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let buffer = draw(presenter);
        let page = runtime
            .block_on(fixture.store.query_shell_resources(session, None, 0))
            .unwrap();
        if page.resources.iter().any(|r| {
            r.result.resource_ref == original.result.resource_ref
                && r.result.status == ShellStatus::Cancelled
        }) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "terminal stop did not settle: {}",
            screen(&buffer)
        );
        std::thread::yield_now();
    }
    click(presenter, "[ Close ]");
    await_screen(presenter, "File background draft");
    click(presenter, "[ Terminals & tasks ]");
    click(presenter, "[ Run command ]");
    await_screen(presenter, "Shell command");
    presenter
        .input(Event::Paste("printf 'pipes-%s\\n' workflow-marker".into()))
        .unwrap();
    click(presenter, "[ Run ]");
    await_screen(presenter, "pipes-workflow-marker");
    await_screen(presenter, "completed (0)");
    click(presenter, "[ Copy ]");
    let mut clipboard = Vec::new();
    assert!(presenter.flush_clipboard(&mut clipboard).unwrap());
    let clipboard = std::str::from_utf8(&clipboard).unwrap();
    let encoded = clipboard
        .strip_prefix("\x1b]52;c;")
        .unwrap()
        .strip_suffix('\x07')
        .unwrap();
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap(),
        b"pipes-workflow-marker\n"
    );
    let page = runtime
        .block_on(fixture.store.query_shell_resources(session, None, 0))
        .unwrap();
    assert!(
        page.resources
            .iter()
            .any(|r| r.result.mode == ShellMode::Pipes
                && r.result.status == ShellStatus::Completed
                && r.result.cmd == "printf 'pipes-%s\\n' workflow-marker")
    );
    click(presenter, "[ Close ]");
    await_screen(presenter, "File background draft");
}

pub(super) fn lost_reply(
    presenter: &mut maka_tui::application::Presenter,
    runtime: &tokio::runtime::Runtime,
    fixture: &Fixture,
    session: &str,
    loss: &Arc<super::wire_loss::Loss>,
) -> maka_plugin_core::operation::RecoveryRef {
    let before = runtime
        .block_on(fixture.store.query_shell_resources(session, None, 0))
        .unwrap();
    click(presenter, "[ Terminals & tasks ]");
    loss.arm("start");
    click(presenter, "[ New terminal ]");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        match presenter.pump() {
            Err(maka_ui::Error::Core(maka_plugin_core::Error::CleanupFailed(_))) => break,
            Err(error) => panic!("unexpected completion failure: {error}"),
            Ok(()) => {
                let area = Rect::new(0, 0, 80, 24);
                let mut buffer = Buffer::empty(area);
                let scene = presenter.paint(area, &mut buffer).unwrap();
                presenter.present(scene);
                assert!(
                    std::time::Instant::now() < deadline,
                    "no dropped-reply cleanup failure: {}",
                    screen(&buffer)
                );
                std::thread::yield_now();
            }
        }
    }
    let intercepted = loss
        .take()
        .expect("actual complete Host mutation reply intercepted");
    intercepted
        .cleanup
        .expect("the actual Host channel completed before its reply was lost");
    let lost = intercepted.reply;
    assert_eq!(lost.reference.method, "start");
    assert_eq!(lost.reference.contract.name, "maka.workspace.Terminal");
    let Receipt::Recorded {
        outcome: OperationOutcome::Returned { value },
        cleanup,
    } = &lost.receipt
    else {
        panic!(
            "expected actual Host operation completion: {:?}",
            lost.receipt
        );
    };
    assert_eq!(
        *cleanup,
        maka_plugin_core::operation::OperationCleanup::Confirmed
    );
    assert_eq!(value["kind"], "completed");
    let saved = runtime
        .block_on(fixture.store.operation_observation(&lost.reference))
        .unwrap()
        .unwrap();
    assert_eq!(
        saved.outcome,
        OperationOutcome::Returned {
            value: value.clone()
        }
    );
    assert_eq!(saved.cleanup, *cleanup);
    let after = runtime
        .block_on(fixture.store.query_shell_resources(session, None, 0))
        .unwrap();
    assert_eq!(
        after.total,
        before.total + 1,
        "lost reply must not replay start"
    );
    let task = after
        .resources
        .iter()
        .find(|r| r.result.resource_ref == value["value"]["result"]["ref"])
        .unwrap();
    assert_eq!(task.result.mode, ShellMode::Pty);
    assert_eq!(task.result.status, ShellStatus::Running);
    // The durable business result cannot stand in for the lost physical close
    // receipt. The original owner must remain fenced even after this lookup.
    presenter
        .surface_closed(&presenter.surface(), Ok(()))
        .unwrap();
    assert!(!presenter.can_shutdown());
    lost.reference
}
