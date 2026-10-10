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
use maka_runtime::{
    event::{
        EventWrite, Fact, Invocation, InvocationInput, InvocationOutcome, LogScope, RuntimeEvent,
    },
    model::{ModelEvent, ModelFinishReason, ModelPart, ModelStep, ModelUsage, TextKind},
};

pub(super) fn run(
    presenter: &mut maka_tui::application::Presenter,
    runtime: &tokio::runtime::Runtime,
    fixture: &Fixture,
    session: &str,
    capture: &impl Fn(&mut maka_tui::application::Presenter, &str),
) {
    let source = format!(
        "**Copy origin** {}**Copy ending**",
        "完整 café 😀 ".repeat(1500)
    );
    let expected = maka_tui::markdown::prepared::Document::markdown(
        &source,
        false,
        maka_tui::markdown::Palette::default(),
    )
    .unwrap()
    .text()
    .to_owned();
    assert!(source.len() > 8192 && source.len() < 65536);
    assert!(expected.starts_with("Copy origin") && expected.contains("Copy ending"));
    assert!(!expected.contains("**"));
    runtime.block_on(async {
        let invocation = Invocation {
            session_id: session.into(),
            turn_id: "copy-turn".into(),
            run_id: "copy-run".into(),
            invocation_id: "copy-invocation".into(),
        };
        for fact in [
            Fact::InvocationOpened {
                configuration: None,
                input: InvocationInput::Message {
                    content: "Whole copy fixture".into(),
                    source_messages: Vec::new(),
                    request_fingerprint: None,
                },
            },
            Fact::ModelRequested {
                item_acceptance: false,
                purpose: maka_runtime::context::ModelPurpose::Main,
                context: None,
                checkpoint_event_id: None,
                step_id: "copy-step".into(),
                model_id: "fixture-model".into(),
                source_scope: LogScope::Root,
                source_high_water: 0,
                source_digest: "fixture".into(),
                effective_source_digest: None,
                input_digest: "fixture".into(),
                route_identity: "fixture".into(),
            },
            Fact::ModelObserved {
                step_id: "copy-step".into(),
                event: ModelEvent::PartStarted {
                    id: "copy-part".into(),
                    text_kind: TextKind::Text,
                    provider_options: None,
                },
            },
            Fact::ModelObserved {
                step_id: "copy-step".into(),
                event: ModelEvent::PartDelta {
                    id: "copy-part".into(),
                    text: source.clone(),
                    provider_options: None,
                },
            },
            Fact::ModelObserved {
                step_id: "copy-step".into(),
                event: ModelEvent::PartFinished {
                    id: "copy-part".into(),
                    provider_options: None,
                },
            },
            Fact::ModelCompleted {
                step_id: "copy-step".into(),
                output: ModelStep {
                    parts: vec![ModelPart::Text {
                        text_kind: TextKind::Text,
                        text: source.clone(),
                        provider_options: None,
                    }],
                    finish_reason: ModelFinishReason::Stop,
                    usage: ModelUsage::default(),
                    provider_options: None,
                    response_id: None,
                    model: None,
                    timestamp: None,
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
    // The native viewport holds at most the ordinary 8 KiB history window.
    // Advance through the real Latest control if the user-owned position held it.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let visible = screen(&draw(presenter));
        if visible.contains("café") {
            assert!(
                !visible.contains("Copy origin"),
                "the whole-message prefix must be outside the preview"
            );
            capture(presenter, "native-copy-excerpt.txt");
            break;
        }
        if let Some((row, column)) = visible
            .lines()
            .enumerate()
            .find_map(|(row, line)| line.find("[ Latest messages ]").map(|column| (row, column)))
        {
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
            "full copy history did not arrive: {visible}"
        );
        std::thread::yield_now();
    }
    command(presenter, "Browse messages");
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
    wait_for_target(presenter, "Maka · 2000-", 0);
    for (label, expected, frame) in [
        (
            "Copy message source",
            source.as_str(),
            "native-copy-source.txt",
        ),
        ("Copy message", expected.as_str(), "native-copy-message.txt"),
    ] {
        command(presenter, label);
        capture(presenter, frame);
        presenter
            .input(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();
        let mut copied = Vec::new();
        assert!(presenter.flush_clipboard(&mut copied).unwrap());
        use base64::Engine as _;
        assert_eq!(
            copied,
            format!(
                "\x1b]52;c;{}\x07",
                base64::engine::general_purpose::STANDARD.encode(expected)
            )
            .as_bytes()
        );
        assert!(!presenter.flush_clipboard(&mut Vec::new()).unwrap());
        assert!(!screen(&draw(presenter)).contains("[ Close commands ]"));
    }
    presenter
        .input(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
        .unwrap();
    await_screen(presenter, "File background draft browse returned");
}

pub(super) fn command(presenter: &mut maka_tui::application::Presenter, label: &str) {
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
    presenter.input(Event::Paste(label.into())).unwrap();
    wait_for_target(presenter, &format!("[ › {label} ]"), 0);
}
