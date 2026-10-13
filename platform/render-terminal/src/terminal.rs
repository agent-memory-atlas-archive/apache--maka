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

use base64::{Engine, engine::general_purpose::STANDARD};
use crossterm::{
    cursor::Show,
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture,
    },
    execute, queue,
    terminal::{
        BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen,
        disable_raw_mode, enable_raw_mode,
    },
};
// Native Windows already reports modifiers and key event kinds through WinAPI.
// Crossterm's keyboard enhancement commands are unsupported on that backend.
#[cfg(unix)]
use crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    cell::RefCell,
    io::{self, IsTerminal, Write},
    rc::Rc,
    sync::mpsc,
};

pub const MAX_CLIPBOARD_BYTES: usize = 64 * 1024;

/// OSC 52 is a write request, not confirmation that the terminal accepted it.
/// Never queries clipboard contents or invokes an external clipboard process.
pub fn copy(writer: &mut impl io::Write, text: &str) -> io::Result<()> {
    if text.is_empty() || text.len() > MAX_CLIPBOARD_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid clipboard payload size",
        ));
    }
    write!(writer, "\x1b]52;c;{}\x07", STANDARD.encode(text))?;
    writer.flush()
}

const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
#[derive(Clone, Default)]
struct FrameWriter(Rc<RefCell<Vec<u8>>>);
impl Write for FrameWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut output = self.0.borrow_mut();
        if output
            .len()
            .checked_add(bytes.len())
            .is_none_or(|size| size > MAX_OUTPUT_BYTES)
        {
            return Err(io::Error::other("terminal output buffer byte limit"));
        }
        output.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub struct Screen {
    terminal: Terminal<CrosstermBackend<FrameWriter>>,
    output: FrameWriter,
    pub(crate) generation: uuid::Uuid,
}
/// Encode one complete diff. The caller must finish writing it before encoding
/// another: ratatui's baseline advances here, while hit geometry advances only
/// at the original output completion receipt. No partial ANSI can be replaced.
pub(crate) fn render(
    screen: &mut Screen,
    render: impl FnOnce(&mut ratatui::Frame<'_>),
) -> io::Result<Vec<u8>> {
    queue!(&mut screen.output, BeginSynchronizedUpdate)?;
    let result = screen.terminal.draw(render).map(|_| ());
    let end = queue!(&mut screen.output, EndSynchronizedUpdate);
    result?;
    end?;
    Ok(std::mem::take(&mut *screen.output.0.borrow_mut()))
}
/// Startup and controlled synchronous callers have no interactive owner yet.
pub fn draw(screen: &mut Screen, paint: impl FnOnce(&mut ratatui::Frame<'_>)) -> io::Result<()> {
    let bytes = render(screen, paint)?;
    let mut output = io::stdout().lock();
    output.write_all(&bytes)?;
    output.flush()
}
struct WriteRequest {
    bytes: Vec<u8>,
    receipt: tokio::sync::oneshot::Sender<io::Result<()>>,
}
/// Sole interactive stdout owner. The foreground retains one original receipt
/// and never awaits blocking stdout. Dropping observation never interrupts an
/// already-started buffer or fabricates its completion.
pub(crate) struct Output(mpsc::SyncSender<WriteRequest>);
impl Output {
    pub fn new() -> io::Result<Self> {
        Self::with_writer(io::stdout())
    }
    fn with_writer(mut writer: impl Write + Send + 'static) -> io::Result<Self> {
        let (send, receive) = mpsc::sync_channel::<WriteRequest>(1);
        std::thread::Builder::new()
            .name("maka-terminal-output".into())
            .spawn(move || {
                while let Ok(request) = receive.recv() {
                    let result = writer
                        .write_all(&request.bytes)
                        .and_then(|_| writer.flush());
                    let failed = result.is_err();
                    let _ = request.receipt.send(result);
                    if failed {
                        break;
                    }
                }
            })?;
        Ok(Self(send))
    }
    pub fn submit(
        &self,
        bytes: Vec<u8>,
    ) -> io::Result<tokio::sync::oneshot::Receiver<io::Result<()>>> {
        if bytes.len() > MAX_OUTPUT_BYTES {
            return Err(io::Error::other("terminal output buffer byte limit"));
        }
        let (receipt, completion) = tokio::sync::oneshot::channel();
        self.0
            .try_send(WriteRequest { bytes, receipt })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => {
                    io::Error::other("terminal output owner already occupied")
                }
                mpsc::TrySendError::Disconnected(_) => {
                    io::Error::other("terminal output owner stopped")
                }
            })?;
        Ok(completion)
    }
}

/// Installed before the first terminal mutation, including partial setup errors.
pub struct Guard {
    active: bool,
    key_releases: bool,
}
impl Guard {
    pub fn new() -> Self {
        Self {
            active: false,
            key_releases: false,
        }
    }
    pub fn reports_key_releases(&self) -> bool {
        self.key_releases
    }
    /// The caller owns the guard even if setup fails after a partial mutation.
    pub fn setup(&mut self, required: &str) -> io::Result<Screen> {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::other(required.to_owned()));
        }
        self.active = true;
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange,
            EnableBracketedPaste
        )?;
        #[cfg(unix)]
        {
            self.key_releases =
                crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false);
            let mut flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES;
            if self.key_releases {
                flags |= KeyboardEnhancementFlags::REPORT_EVENT_TYPES;
            }
            execute!(
                io::stdout(),
                crossterm::style::Print("\x1b[>1s"),
                PushKeyboardEnhancementFlags(flags)
            )?;
        }
        #[cfg(windows)]
        {
            self.key_releases = true;
        }
        let output = FrameWriter::default();
        let terminal = Terminal::new(CrosstermBackend::new(output.clone()))?;
        Ok(Screen {
            terminal,
            output,
            generation: uuid::Uuid::new_v4(),
        })
    }
    /// Restore every independent native effect, even if a prior write failed.
    /// Only complete success acknowledges actual terminal cleanup.
    pub fn finish(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        restore_checked(&mut io::stdout(), disable_raw_mode)?;
        self.active = false;
        Ok(())
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}
fn restore_checked(
    writer: &mut impl io::Write,
    raw: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let mut failures = Vec::new();
    macro_rules! restore_command {
        ($command:expr) => {
            if let Err(error) = execute!(writer, $command) {
                failures.push(error.to_string());
            }
        };
    }
    #[cfg(unix)]
    {
        restore_command!(crossterm::style::Print("\x1b[>0s"));
        restore_command!(PopKeyboardEnhancementFlags);
    }
    restore_command!(EndSynchronizedUpdate);
    restore_command!(DisableBracketedPaste);
    restore_command!(DisableFocusChange);
    restore_command!(DisableMouseCapture);
    restore_command!(LeaveAlternateScreen);
    restore_command!(Show);
    if let Err(error) = writer.flush() {
        failures.push(error.to_string());
    }
    if let Err(error) = raw() {
        failures.push(error.to_string());
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(failures.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restoration_attempts_raw_cleanup_after_writer_failure_and_checks_both_receipts() {
        let mut raw_attempted = false;
        let mut failing = &mut [0u8; 0][..];
        assert!(
            restore_checked(&mut failing, || {
                raw_attempted = true;
                Ok(())
            })
            .is_err()
        );
        assert!(raw_attempted);
        let mut output = Vec::new();
        assert!(
            restore_checked(&mut output, || Err(io::Error::other("raw cleanup failure"))).is_err()
        );
        assert!(output.ends_with(b"\x1b[?25h"));
        assert!(restore_checked(&mut Vec::new(), || Ok(())).is_ok());
    }
    #[test]
    fn clipboard_only_writes_bounded_encoded_payloads_and_propagates_writer_failure() {
        let input = "中文 e\u{301} 👩‍💻\n\x1b]52;c;hostile\x07";
        let mut output = vec![];
        copy(&mut output, input).unwrap();
        let encoded = output
            .strip_prefix(b"\x1b]52;c;")
            .unwrap()
            .strip_suffix(b"\x07")
            .unwrap();
        assert_eq!(STANDARD.decode(encoded).unwrap(), input.as_bytes());
        assert!(!encoded.contains(&b'\x1b'));
        for input in [String::new(), "x".repeat(MAX_CLIPBOARD_BYTES + 1)] {
            let mut output = vec![];
            assert!(copy(&mut output, &input).is_err());
            assert!(output.is_empty());
        }
        assert!(copy(&mut io::sink(), &"x".repeat(MAX_CLIPBOARD_BYTES)).is_ok());
        assert!(copy(&mut &mut [0u8; 1][..], "text").is_err());
    }
}
