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
use std::{collections::BTreeSet, time::Duration};

/// Run on the foreground thread inside a multithreaded Tokio runtime.
/// Accepted service workers remain owned after the terminal surface closes.
/// 在前台线程运行；关闭界面后仍等待真实服务执行与资源回执。
pub fn run(
    store: std::sync::Arc<maka_event_log::EventLog>,
    source: maka_plugin_core::deployment::DeploymentRef,
    registrations: Vec<maka_ui::actions::RemoteService>,
    scheduler: tokio::runtime::Handle,
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
    loop {
        if let Err(error) = presenter.pump() {
            let error = sanitize(&error.to_string());
            if reported.insert(error.clone()) {
                eprintln!("{error}");
            }
            failure.get_or_insert(error);
        }
        if failure.is_some() || presenter.application.is_retiring(&presenter.root) {
            if let Err(error) = presenter.retire() {
                report(&mut reported, &error.to_string());
            }
            // Drop the backend before acknowledging restoration; no paint/input
            // references or terminal writes remain after this point.
            screen.take();
            let cleanup = guard.finish().map_err(|e| sanitize(&e.to_string()));
            if let Err(error) = presenter.surface_closed(cleanup) {
                report(&mut reported, &error.to_string());
            }
            break;
        }
        let result = (|| -> Result<bool, String> {
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
                if is_copy(&input) {
                    if let Err(error) = presenter.copy(&mut std::io::stdout().lock()) {
                        report(&mut reported, &error.to_string());
                    }
                    return Ok(false);
                }
                if is_exit(&input) {
                    return Ok(true);
                }
                presenter.input(input).map_err(|e| e.to_string())?;
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
    matches!(event,Event::Key(key) if key.kind!=KeyEventKind::Release && key.code==KeyCode::Char('c') && key.modifiers==KeyModifiers::CONTROL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
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
        let mut released = KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        released.kind = KeyEventKind::Release;
        assert!(!is_copy(&Event::Key(released)));
        assert!(!is_exit(&Event::Key(released)));
    }
}
