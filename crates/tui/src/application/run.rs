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

use super::{Presenter, registry, sanitize};
use crate::terminal;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use serde_json::json;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

/// Run on the foreground thread inside a multithreaded Tokio runtime.
/// Accepted service workers remain owned after the terminal surface closes.
/// 在前台线程运行；关闭界面后仍等待真实服务执行与资源回执。
pub fn run(
    store: std::sync::Arc<maka_event_log::EventLog>,
    source: maka_plugin_core::deployment::DeploymentRef,
    registrations: Vec<maka_ui::actions::RemoteService>,
    scheduler: tokio::runtime::Handle,
    mut updates: maka_application_bridge::Updates,
) -> Result<(), String> {
    let mut presenter = scheduler
        .block_on(Presenter::start_from_store(
            store,
            source,
            scheduler.clone(),
            registry(),
            json!({}),
            registrations,
        ))
        .map_err(|e| sanitize(&e.to_string()))?;
    let mut guard = terminal::Guard::new();
    let mut failure = None;
    let mut screen = match guard.setup("an interactive terminal is required") {
        Ok(screen) => Some(screen),
        Err(error) => {
            failure = Some(sanitize(&error.to_string()));
            None
        }
    };
    let mut reported = BTreeSet::new();
    let mut exit_key = ExitKey::default();
    loop {
        if failure.is_some()
            || ((!presenter.surface.is_live()
                || presenter.application.is_retiring(&presenter.root))
                && !presenter.requires_surface_replacement())
        {
            updates.stop();
            if let Err(error) = presenter.retire() {
                report(&mut reported, &error.to_string());
            }
        }
        match updates.poll(&presenter.application) {
            Ok(Some(update)) => {
                if let Err(error) = presenter.adopt_prepared(update.prepared, update.registrations)
                {
                    report(&mut reported, &error.to_string());
                }
            }
            Ok(None) => {}
            Err(error) => report(&mut reported, &error),
        }
        if let Err(error) = presenter.pump() {
            let error = sanitize(&error.to_string());
            if reported.insert(error.clone()) {
                eprintln!("{error}");
            }
            failure.get_or_insert(error);
        }
        if failure.is_some()
            || !presenter.surface.is_live()
            || presenter.application.is_retiring(&presenter.root)
        {
            if (failure.is_some() || !presenter.requires_surface_replacement())
                && let Err(error) = presenter.retire()
            {
                report(&mut reported, &error.to_string());
            }
            if let super::SurfaceState::Retiring { cleanup: None, .. } = &presenter.surface {
                let original = presenter.surface();
                // Backend destruction and Guard restoration belong to this exact
                // original root. A late receipt cannot close a replacement.
                screen.take();
                exit_key = ExitKey::default();
                let cleanup = guard.finish().map_err(|e| sanitize(&e.to_string()));
                if let Err(error) = presenter.surface_closed(&original, cleanup) {
                    report(&mut reported, &error.to_string());
                    failure.get_or_insert(sanitize(&error.to_string()));
                }
            }
            if failure.is_some() || !presenter.requires_surface_replacement() {
                break;
            }
            match presenter.try_replace_surface(json!({})) {
                Ok(true) => match guard.setup("an interactive terminal is required") {
                    Ok(replacement) => screen = Some(replacement),
                    Err(error) => {
                        failure = Some(sanitize(&error.to_string()));
                    }
                },
                Ok(false) => std::thread::sleep(Duration::from_millis(10)),
                Err(error) => {
                    report(&mut reported, &error.to_string());
                    failure = Some(sanitize(&error.to_string()));
                }
            }
            continue;
        }
        let result = (|| -> Result<bool, String> {
            if exit_key.ready(None, Instant::now()) {
                return Ok(true);
            }
            let screen = screen.as_mut().expect("active terminal");
            let mut scene = None;
            let mut paint_error = None;
            terminal::draw(screen, |frame| {
                match presenter.paint(frame.area(), frame.buffer_mut()) {
                    Ok(painted) => scene = Some(painted),
                    Err(error) => paint_error = Some(error.to_string()),
                }
            })
            .map_err(|e| e.to_string())?;
            if let Some(error) = paint_error {
                return Err(error);
            }
            presenter.present(scene.expect("successful paint"));
            if event::poll(Duration::from_millis(30)).map_err(|e| e.to_string())? {
                let input = event::read().map_err(|e| e.to_string())?;
                if exit_key.ready(Some(&input), Instant::now()) {
                    return Ok(true);
                }
                if is_copy(&input) {
                    if let Err(error) = presenter.copy(&mut std::io::stdout().lock()) {
                        report(&mut reported, &error.to_string());
                    }
                    return Ok(false);
                }
                if is_exit(&input) && !presenter.captures_keyboard().map_err(|e| e.to_string())? {
                    let handled = presenter
                        .dispatch_focused_shortcut(&input)
                        .map_err(|error| error.to_string())?;
                    let Event::Key(key) = input else {
                        unreachable!()
                    };
                    return Ok(!handled
                        && exit_key.request(
                            key.code,
                            guard.reports_key_releases(),
                            Instant::now(),
                        ));
                }
                presenter.input(input).map_err(|e| e.to_string())?;
                if let Err(error) = presenter.flush_clipboard(&mut std::io::stdout().lock()) {
                    report(&mut reported, &error.to_string());
                }
            }
            Ok(false)
        })();
        match result {
            Ok(false) => {}
            Ok(true) => {
                if let Err(error) = presenter.retire() {
                    failure = Some(sanitize(&error.to_string()));
                }
            }
            Err(error) => {
                failure = Some(sanitize(&error));
            }
        }
    }
    updates.stop();
    scheduler.block_on(updates.drain());
    // Failures remain inspectable in this owner; dropping it cannot substitute
    // for a service/terminal cleanup receipt. Hard process exit is not recovery.
    loop {
        if let Err(error) = presenter.pump() {
            report(&mut reported, &error.to_string());
        }
        if presenter.can_shutdown() {
            match presenter.shutdown() {
                Ok(()) => return failure.map_or(Ok(()), Err),
                Err(error) => report(&mut reported, &error.to_string()),
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn report(reported: &mut BTreeSet<String>, message: &str) {
    let message = sanitize(message);
    if reported.insert(message.clone()) {
        eprintln!("{message}");
    }
}

fn is_copy(event: &Event) -> bool {
    matches!(event,Event::Key(key) if key.kind!=KeyEventKind::Release && matches!(key.code,KeyCode::Char('c'|'C')) && key.modifiers==KeyModifiers::CONTROL|KeyModifiers::SHIFT)
}
fn is_exit(event: &Event) -> bool {
    matches!(event,Event::Key(key) if key.kind==KeyEventKind::Press && matches!(key.code,KeyCode::Char('c' | 'q')) && key.modifiers==KeyModifiers::CONTROL)
}

/// Keep a negotiated physical quit gesture inside its original raw terminal.
/// A lost release must not strand shutdown; another physical action cancels it.
#[derive(Default)]
struct ExitKey(Option<(KeyCode, Instant)>);
impl ExitKey {
    fn request(&mut self, key: KeyCode, releases: bool, now: Instant) -> bool {
        if !releases {
            return true;
        }
        self.0 = Some((key, now + Duration::from_secs(2)));
        false
    }
    fn ready(&mut self, event: Option<&Event>, now: Instant) -> bool {
        let Some((code, deadline)) = self.0 else {
            return false;
        };
        if now >= deadline
            || matches!(event, Some(Event::Key(key)) if key.kind == KeyEventKind::Release && key.code == code)
        {
            self.0 = None;
            return true;
        }
        if matches!(event, Some(Event::Key(key)) if key.kind == KeyEventKind::Press)
            || matches!(event, Some(Event::Mouse(mouse)) if matches!(mouse.kind, event::MouseEventKind::Down(_)))
        {
            self.0 = None;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    #[test]
    fn quit_release_keeps_original_key_and_cancels_on_a_new_gesture() {
        let now = Instant::now();
        let mut exit = ExitKey::default();
        assert!(exit.request(KeyCode::Char('q'), false, now));
        assert!(!exit.request(KeyCode::Char('q'), true, now));
        let released = |code| {
            Event::Key(KeyEvent::new_with_kind(
                code,
                KeyModifiers::NONE,
                KeyEventKind::Release,
            ))
        };
        assert!(!exit.ready(Some(&released(KeyCode::Char('c'))), now));
        assert!(exit.ready(Some(&released(KeyCode::Char('q'))), now));
        assert!(!exit.ready(Some(&released(KeyCode::Char('q'))), now));
        assert!(!exit.request(KeyCode::Char('c'), true, now));
        let repeat = Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            KeyEventKind::Repeat,
        ));
        assert!(!exit.ready(Some(&repeat), now));
        let other = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(!exit.ready(Some(&other), now));
        assert!(!exit.ready(Some(&released(KeyCode::Char('c'))), now));
        assert!(!exit.ready(None, now + Duration::from_secs(3)));
        assert!(!exit.request(KeyCode::Char('q'), true, now));
        assert!(!exit.ready(None, now + Duration::from_secs(1)));
        assert!(exit.ready(None, now + Duration::from_secs(2)));
    }
    #[test]
    fn source_copy_does_not_overlap_exit_or_release() {
        let copy = Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));
        assert!(is_copy(&copy));
        assert!(!is_exit(&copy));
        let uppercase = Event::Key(KeyEvent::new(
            KeyCode::Char('C'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));
        assert!(is_copy(&uppercase));
        assert!(!is_exit(&uppercase));
        let exit = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(is_exit(&exit));
        assert!(!is_copy(&exit));
        let quit = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
        assert!(is_exit(&quit));
        assert!(!is_copy(&quit));
        let mut held = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        held.kind = KeyEventKind::Repeat;
        assert!(!is_exit(&Event::Key(held)));
        let mut released = KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        released.kind = KeyEventKind::Release;
        assert!(!is_copy(&Event::Key(released)));
        assert!(!is_exit(&Event::Key(released)));
    }
}
