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

fn select(presenter: &mut maka_tui::application::Presenter, title: &str) {
    draw(presenter);
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    await_screen(presenter, "[ Close commands ]");
    presenter
        .input(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    presenter.input(Event::Paste(title.into())).unwrap();
    click(presenter, &format!("[ › Conversation · {title} ]"));
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let rendered = screen(&draw(presenter));
        if rendered.contains(title)
            && !rendered.contains("[ Close commands ]")
            && !rendered.contains("Restoring draft…")
        {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "{rendered}");
        std::thread::yield_now();
    }
    if !screen(&draw(presenter)).contains("32 drafts are open.") {
        wait_for_target(presenter, "[ Attachments ]", 0);
    }
}
fn key(presenter: &mut maka_tui::application::Presenter, code: KeyCode, modifiers: KeyModifiers) {
    presenter
        .input(Event::Key(KeyEvent::new(code, modifiers)))
        .unwrap();
    draw(presenter);
}
pub(super) fn exercise(
    presenter: &mut maka_tui::application::Presenter,
    fixture: &Fixture,
    runtime: &tokio::runtime::Runtime,
) {
    let original = runtime
        .block_on(
            fixture
                .store
                .list_sessions::<SessionConfiguration>(None, None, 32),
        )
        .unwrap()
        .sessions
        .pop()
        .unwrap();
    runtime.block_on(fixture.owner.metadata(serde_json::from_value(json!({"sessionId":original.id,"expectedRevision":original.revision,"title":"Draft-00","labels":[]})).unwrap())).unwrap();
    // Seed canonical rows in the original store. UI discovery/selection still
    // uses the authenticated remote Agent binding, not a relaxed peer policy.
    let create = |title: &str| {
        let mut configuration = original.configuration.clone();
        configuration.title = title.into();
        let id = format!("{}{}", fixture.owner.prefix, Uuid::new_v4().simple());
        runtime
            .block_on(
                fixture
                    .store
                    .create_session(&id, &id, &configuration, now()),
            )
            .unwrap();
    };
    await_screen(presenter, "Draft-00");
    wait_for_target(presenter, "[ Attachments ]", 0);
    presenter.input(Event::Paste("first draft".into())).unwrap();
    await_screen(presenter, "first draft");
    key(presenter, KeyCode::Home, KeyModifiers::CONTROL);
    for _ in 0..5 {
        key(presenter, KeyCode::Right, KeyModifiers::SHIFT);
    }
    create("Draft-01");
    select(presenter, "Draft-01");
    await_screen(presenter, "What would you like to work on?");
    assert!(!screen(&draw(presenter)).contains("first draft"));
    presenter
        .input(Event::Paste("second draft".into()))
        .unwrap();
    await_screen(presenter, "second draft");
    select(presenter, "Draft-00");
    await_screen(presenter, "first draft");
    presenter.input(Event::Paste("changed".into())).unwrap();
    await_screen(presenter, "changed draft");
    key(presenter, KeyCode::Char('z'), KeyModifiers::CONTROL);
    await_screen(presenter, "first draft");
    let wrapped = "Question background with enough words to wrap the native draft";
    presenter.input(Event::Paste(wrapped.into())).unwrap();
    let narrow = screen(&draw_at(presenter, Rect::new(0, 0, 40, 16)));
    let body = narrow
        .lines()
        .map(|line| line.replace("[ Send ]", ""))
        .map(|line| {
            line.chars()
                .filter(|c| !c.is_whitespace() && *c != '│')
                .collect::<String>()
        })
        .collect::<String>();
    let expected = format!("{wrapped} draft")
        .split_whitespace()
        .collect::<String>();
    assert!(body.contains(&expected), "{narrow}");
    key(presenter, KeyCode::Char('z'), KeyModifiers::CONTROL);
    await_screen(presenter, "first draft");
    if let Some(directory) = std::env::var_os("MAKA_DRAFT_EVIDENCE") {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let buffer = draw_at(presenter, Rect::new(0, 0, 40, 16));
        std::fs::write(directory.join("native-restored-draft.txt"), screen(&buffer)).unwrap();
        let cells: Vec<_> = (0..buffer.area.height).flat_map(|y| {
            let buffer = &buffer;
            (0..buffer.area.width).map(move |x| {
                let cell = &buffer[(x, y)];
                json!({"x":x,"y":y,"text":cell.symbol(),"fg":format!("{:?}",cell.fg),"bg":format!("{:?}",cell.bg),"bold":cell.modifier.contains(ratatui::style::Modifier::BOLD)})
            })
        }).collect();
        std::fs::write(
            directory.join("native-restored-draft.json"),
            serde_json::to_vec_pretty(&cells).unwrap(),
        )
        .unwrap();
    }
    for index in 2..32 {
        let title = format!("Draft-{index:02}");
        create(&title);
        select(presenter, &title);
        await_screen(presenter, "What would you like to work on?");
        wait_for_target(presenter, "[ Attachments ]", 0);
        let body = format!("draft body {index:02}");
        presenter.input(Event::Paste(body.clone())).unwrap();
        await_screen(presenter, &body);
    }
    create("Draft-32");
    select(presenter, "Draft-32");
    await_screen(presenter, "32 drafts are open.");
    assert!(!screen(&draw(presenter)).contains("draft body 31"));
    select(presenter, "Draft-00");
    await_screen(presenter, "first draft");
    key(presenter, KeyCode::Char('a'), KeyModifiers::CONTROL);
    key(presenter, KeyCode::Backspace, KeyModifiers::NONE);
    await_screen(presenter, "What would you like to work on?");
    select(presenter, "Draft-32");
    await_screen(presenter, "What would you like to work on?");
    wait_for_target(presenter, "[ Attachments ]", 0);
    presenter
        .input(Event::Paste("new draft at capacity".into()))
        .unwrap();
    await_screen(presenter, "new draft at capacity");
    select(presenter, "Draft-01");
    await_screen(presenter, "second draft");
    presenter.input(Event::Paste("!".into())).unwrap();
    await_screen(presenter, "second draft!");
    select(presenter, "Draft-31");
    await_screen(presenter, "draft body 31");
    key(presenter, KeyCode::Char('a'), KeyModifiers::CONTROL);
    presenter.input(Event::Paste("甲👩‍💻z".into())).unwrap();
    key(presenter, KeyCode::End, KeyModifiers::CONTROL);
    key(presenter, KeyCode::Left, KeyModifiers::NONE);
    key(presenter, KeyCode::Left, KeyModifiers::SHIFT);
}

pub(super) fn verify_restored(presenter: &mut maka_tui::application::Presenter) {
    use base64::Engine as _;
    await_screen(presenter, "[ Toggle sidebar ]");
    select(presenter, "Draft-31");
    await_screen(presenter, "👩‍💻");
    let mut copied = Vec::new();
    assert!(presenter.copy(&mut copied).unwrap());
    assert_eq!(
        copied,
        format!(
            "\x1b]52;c;{}\x07",
            base64::engine::general_purpose::STANDARD.encode("👩‍💻")
        )
        .as_bytes()
    );
    presenter.input(Event::Paste("R".into())).unwrap();
    await_screen(presenter, "Rz");
    key(presenter, KeyCode::Char('a'), KeyModifiers::CONTROL);
    copied.clear();
    assert!(presenter.copy(&mut copied).unwrap());
    assert_eq!(
        copied,
        format!(
            "\x1b]52;c;{}\x07",
            base64::engine::general_purpose::STANDARD.encode("甲Rz")
        )
        .as_bytes()
    );
    select(presenter, "Draft-01");
    await_screen(presenter, "second draft!");
}
