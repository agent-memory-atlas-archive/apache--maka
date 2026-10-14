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

use super::{PaintedScene, Presenter, registry, sanitize};
use crate::terminal;
use crossterm::event::{self, Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures_util::{FutureExt, StreamExt};
use serde_json::json;
use std::time::{Duration, Instant};
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

#[derive(Default)]
struct TurnWake(tokio::sync::Notify);
impl Wake for TurnWake {
    fn wake(self: Arc<Self>) {
        self.0.notify_one();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.notify_one();
    }
}
async fn deadline(at: Option<Instant>) {
    match at {
        Some(at) => tokio::time::sleep_until(at.into()).await,
        None => std::future::pending().await,
    }
}

struct PendingOutput {
    sequence: u64,
    generation: uuid::Uuid,
    surface: maka_plugin_core::runtime::Mount,
    receipt: tokio::sync::oneshot::Receiver<std::io::Result<()>>,
    scene: Option<PaintedScene>,
}
// Input readiness can win the same select as a completed write. Publish that
// original receipt before resolving coordinates or keyboard focus, including
// the first visible frame. Pending writes never make the foreground wait.
fn poll_output(
    pending: &mut Option<PendingOutput>,
    written_sequence: &mut u64,
    screen: Option<&terminal::Screen>,
    presenter: &mut Presenter,
) -> Result<(), String> {
    let Some(output) = pending.as_mut() else {
        return Ok(());
    };
    let destination = presenter.application.waker();
    let mut cx = Context::from_waker(&destination);
    let Poll::Ready(result) = Pin::new(&mut output.receipt).poll(&mut cx) else {
        return Ok(());
    };
    let output = pending.take().expect("original output receipt");
    result
        .map_err(|error| sanitize(&error.to_string()))?
        .map_err(|error| sanitize(&error.to_string()))?;
    if output.sequence != *written_sequence + 1 {
        return Err("terminal output receipt sequence mismatch".into());
    }
    *written_sequence = output.sequence;
    if screen.is_some_and(|screen| screen.generation == output.generation)
        && presenter.surface() == output.surface
        && let Some(scene) = output.scene
    {
        presenter.present_written(scene);
    }
    Ok(())
}

fn retain_clipboard(queue: &mut VecDeque<Vec<u8>>, bytes: Vec<u8>) -> Result<(), String> {
    if bytes.is_empty() {
        return Ok(());
    }
    if queue.len() >= 16 || queue.iter().map(Vec::len).sum::<usize>() + bytes.len() > 1024 * 1024 {
        return Err("terminal clipboard output queue full".into());
    }
    queue.push_back(bytes);
    Ok(())
}

/// Run on the foreground thread inside a multithreaded Tokio runtime.
/// Accepted service workers remain owned after the terminal surface closes.
/// 在前台线程运行；关闭界面后仍等待真实服务执行与资源回执。
#[allow(clippy::too_many_arguments)]
pub fn run(
    store: std::sync::Arc<maka_event_log::EventLog>,
    source: maka_plugin_core::deployment::DeploymentRef,
    registrations: Vec<maka_ui_runtime::actions::RemoteService>,
    scheduler: tokio::runtime::Handle,
    mut updates: maka_transport::Updates,
    execution: maka_ui_runtime::ExecutionOptions,
    startup: Option<crate::startup::Startup>,
    mut startup_errors: Vec<String>,
) -> Result<(), String> {
    let mut presenter = scheduler
        .block_on(Presenter::start_from_store(
            store,
            source,
            scheduler.clone(),
            registry(),
            json!({}),
            registrations,
            execution,
        ))
        .map_err(|e| sanitize(&e.to_string()))?;
    let (mut guard, previous_screen) = match startup {
        Some(startup) => (startup.guard, Some(startup.screen)),
        None => (terminal::Guard::new(), None),
    };
    let mut failure = None;
    let mut screen = match previous_screen
        .map_or_else(|| guard.setup("an interactive terminal is required"), Ok)
    {
        Ok(screen) => Some(screen),
        Err(error) => {
            failure = Some(sanitize(&error.to_string()));
            None
        }
    };
    if startup_errors.is_empty()
        && let Some(error) = presenter.deployment_status().error
    {
        startup_errors.push(error);
    }
    if !startup_errors.is_empty()
        && let Some(screen) = screen.as_mut()
    {
        let diagnostic = startup_errors.join("\n\n");
        match crate::startup::issues(screen, &diagnostic) {
            Ok(true) => {}
            Ok(false) => failure = Some(diagnostic),
            Err(error) => failure = Some(format!("{diagnostic}\n{error}")),
        }
    }
    let wake = Arc::new(TurnWake::default());
    let waker = Waker::from(wake.clone());
    presenter.application.set_waker(waker.clone());
    updates.set_waker(waker);
    let mut events = EventStream::new();
    let output = terminal::Output::new().map_err(|e| e.to_string())?;
    let mut pending_output: Option<PendingOutput> = None;
    let mut clipboard_output = VecDeque::new();
    let mut issued_sequence = 0u64;
    let mut written_sequence = 0u64;
    let mut exit_key = ExitKey::default();
    let mut queued_input = None;
    loop {
        if let Err(error) = poll_output(
            &mut pending_output,
            &mut written_sequence,
            screen.as_ref(),
            &mut presenter,
        ) {
            failure.get_or_insert(error);
        }
        if pending_output.is_none()
            && let Some(bytes) = clipboard_output.pop_front()
        {
            issued_sequence += 1;
            match output.submit(bytes) {
                Ok(receipt) => {
                    pending_output = Some(PendingOutput {
                        sequence: issued_sequence,
                        generation: screen.as_ref().expect("original terminal").generation,
                        surface: presenter.surface(),
                        receipt,
                        scene: None,
                    });
                    wake.0.notify_one();
                }
                Err(error) => {
                    failure.get_or_insert(sanitize(&error.to_string()));
                }
            }
        }
        if failure.is_some()
            || ((!presenter.surface.is_live()
                || presenter.application.is_retiring(&presenter.root))
                && !presenter.requires_surface_replacement())
        {
            updates.stop();
            if let Err(error) = presenter.retire() {
                presenter.report(&error);
            }
        }
        match updates.poll(&presenter.application) {
            Ok(Some(update)) => {
                if let Err(error) = presenter.adopt_prepared(update.prepared, update.registrations)
                {
                    presenter.report(&error);
                }
            }
            Ok(None) => {}
            Err(error) => presenter.report(&error),
        }
        if let Err(error) = presenter.pump() {
            let error = sanitize(&error.to_string());
            presenter.report(&error);
            failure.get_or_insert(error);
        }
        let mut bytes = Vec::new();
        if let Err(error) = presenter.flush_clipboard(&mut bytes) {
            presenter.report(&error);
        }
        if let Err(error) = retain_clipboard(&mut clipboard_output, bytes) {
            presenter.report(&error);
        }
        // A blocked write already owns its completion wakeup. Queued clipboard
        // bytes become runnable only after it finishes; waking ourselves while
        // it is pending would spin the foreground under output backpressure.
        if pending_output.is_none() && !clipboard_output.is_empty() {
            wake.0.notify_one();
        }
        if failure.is_some()
            || !presenter.surface.is_live()
            || presenter.application.is_retiring(&presenter.root)
        {
            if (failure.is_some() || !presenter.requires_surface_replacement())
                && let Err(error) = presenter.retire()
            {
                presenter.report(&error);
            }
            if let super::SurfaceState::Retiring { cleanup: None, .. } = &presenter.surface {
                let original = presenter.surface();
                // Backend destruction and Guard restoration belong to this exact
                // original root. A late receipt cannot close a replacement.
                if pending_output.is_some() || !clipboard_output.is_empty() {
                    scheduler.block_on(async {
                        tokio::select! {
                            _ = wake.0.notified() => {},
                            _ = events.next() => {},
                        }
                    });
                    continue;
                }
                screen.take();
                exit_key = ExitKey::default();
                queued_input = None;
                let cleanup = guard.finish().map_err(|e| sanitize(&e.to_string()));
                if let Err(error) = presenter.surface_closed(&original, cleanup) {
                    presenter.report(&error);
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
                Ok(false) => scheduler.block_on(async {
                    tokio::select! {
                        _ = wake.0.notified() => {},
                        _ = deadline(updates.next_deadline()) => {},
                    }
                }),
                Err(error) => {
                    presenter.report(&error);
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
            if pending_output.is_none()
                && (presenter.resources.redraw
                    || presenter.next_frame.is_some_and(|at| Instant::now() >= at))
            {
                let mut scene = None;
                let mut paint_error = None;
                let bytes = terminal::render(screen, |frame| {
                    match presenter.paint(frame.area(), frame.buffer_mut()) {
                        Ok(painted) => scene = Some(painted),
                        Err(error) => paint_error = Some(error.to_string()),
                    }
                })
                .map_err(|e| e.to_string())?;
                if let Some(error) = paint_error {
                    return Err(error);
                }
                issued_sequence += 1;
                pending_output = Some(PendingOutput {
                    sequence: issued_sequence,
                    generation: screen.generation,
                    surface: presenter.surface(),
                    receipt: output.submit(bytes).map_err(|e| e.to_string())?,
                    scene,
                });
                wake.0.notify_one();
                presenter.resources.redraw = false;
                presenter.next_frame = None;
            }
            let first_frame_pending = written_sequence == 0 && pending_output.is_some();
            let input = if queued_input.is_some() && !first_frame_pending {
                queued_input.take()
            } else {
                let at = [
                    presenter.next_frame.filter(|_| pending_output.is_none()),
                    exit_key.0.map(|(_, at)| at),
                    updates.next_deadline(),
                ]
                .into_iter()
                .flatten()
                .min();
                scheduler
                    .block_on(async {
                        tokio::select! {
                            input = events.next(), if queued_input.is_none() => input.transpose(),
                            _ = wake.0.notified() => Ok(None),
                            _ = deadline(at) => Ok(None),
                        }
                    })
                    .map_err(|e| e.to_string())?
            };
            if let Some(input) = input {
                poll_output(
                    &mut pending_output,
                    &mut written_sequence,
                    Some(screen),
                    &mut presenter,
                )?;
                if exit_key.ready(Some(&input), Instant::now()) {
                    return Ok(true);
                }
                // A terminal can expose the first frame before the writer sends
                // its receipt. Retain input until that frame admits its keyboard
                // and hit targets; later frames use the already presented scene.
                if written_sequence == 0 && pending_output.is_some() && !is_exit(&input) {
                    queued_input = Some(input);
                    return Ok(false);
                }
                if is_copy(&input) {
                    let mut bytes = Vec::new();
                    if let Err(error) = presenter.copy(&mut bytes) {
                        presenter.report(&error);
                    }
                    if let Err(error) = retain_clipboard(&mut clipboard_output, bytes) {
                        presenter.report(&error);
                    }
                    wake.0.notify_one();
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
                let scrolling = is_wheel(&input);
                if scrolling {
                    presenter.wheel(input)
                } else {
                    presenter.input(input)
                }
                .map_err(|e| e.to_string())?;
                // A trackpad gesture arrives as many small wheel events. Apply
                // the queued gesture before another live update and repaint,
                // so streaming cannot repeatedly move the tail ahead of it.
                // Keep other gestures for the next presented frame.
                let until = Instant::now() + Duration::from_millis(8);
                while scrolling && Instant::now() < until {
                    let Some(Some(input)) = events.next().now_or_never() else {
                        break;
                    };
                    let input = input.map_err(|e| e.to_string())?;
                    if !is_wheel(&input) {
                        queued_input = Some(input);
                        break;
                    }
                    poll_output(
                        &mut pending_output,
                        &mut written_sequence,
                        Some(screen),
                        &mut presenter,
                    )?;
                    presenter.wheel(input).map_err(|e| e.to_string())?;
                }
                if scrolling {
                    presenter.sync_native().map_err(|e| e.to_string())?;
                }
                let mut bytes = Vec::new();
                if let Err(error) = presenter.flush_clipboard(&mut bytes) {
                    presenter.report(&error);
                }
                if let Err(error) = retain_clipboard(&mut clipboard_output, bytes) {
                    presenter.report(&error);
                }
                if pending_output.is_none() && !clipboard_output.is_empty() {
                    wake.0.notify_one();
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
            presenter.report(&error);
        }
        if presenter.can_shutdown() {
            match presenter.shutdown() {
                Ok(()) => {
                    for diagnostic in &presenter.reported {
                        if failure.as_ref() != Some(diagnostic) {
                            eprintln!("{diagnostic}");
                        }
                    }
                    return failure.map_or(Ok(()), Err);
                }
                Err(maka_ui_runtime::Error::Core(maka_plugin_core::Error::CleanupPending)) => {}
                Err(error) => presenter.report(&error),
            }
        }
        scheduler.block_on(wake.0.notified());
    }
}
fn is_wheel(event: &Event) -> bool {
    matches!(event, Event::Mouse(mouse) if matches!(mouse.kind,
        crossterm::event::MouseEventKind::ScrollUp | crossterm::event::MouseEventKind::ScrollDown))
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
