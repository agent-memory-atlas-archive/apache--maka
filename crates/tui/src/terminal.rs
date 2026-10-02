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
    SynchronizedUpdate,
    cursor::Show,
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture,
    },
    execute,
    terminal::{
        EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
// Native Windows already reports modifiers and key event kinds through WinAPI.
// Crossterm's keyboard enhancement commands are unsupported on that backend.
#[cfg(unix)]
use crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, IsTerminal, Stdout};

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

/// Destructive edits require a confirmed native copy, never an OSC 52 request.
/// Run off the input thread because clipboard owners can take time to respond.
#[cfg(any(target_os = "macos", windows))]
pub fn copy_native(text: String) -> Result<(), &'static str> {
    use clipboard_rs::{Clipboard, ClipboardContext};
    if text.is_empty() || text.len() > MAX_CLIPBOARD_BYTES {
        return Err("chat-copy-failed");
    }
    let clipboard = ClipboardContext::new().map_err(|_| "chat-copy-failed")?;
    clipboard
        .set_text(text.clone())
        .map_err(|_| "chat-copy-failed")?;
    if clipboard.get_text().map_err(|_| "chat-copy-failed")? == text {
        Ok(())
    } else {
        Err("chat-copy-failed")
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn copy_native(_: String) -> Result<(), &'static str> {
    Err("chat-copy-failed")
}

pub type Screen = Terminal<CrosstermBackend<Stdout>>;

/// Publish a whole frame together on terminals with synchronized-output support.
/// Others ignore the mode and retain the usual incremental rendering. End the
/// update even when drawing fails; the terminal guard also ends it after panic.
pub fn draw(screen: &mut Screen, render: impl FnOnce(&mut ratatui::Frame<'_>)) -> io::Result<()> {
    io::stdout().sync_update(|_| screen.draw(render).map(|_| ()))?
}

/// Installed before the first terminal mutation, including partial setup errors.
pub struct Guard {
    active: bool,
}
impl Guard {
    pub fn new() -> Self {
        Self { active: false }
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
        execute!(
            io::stdout(),
            crossterm::style::Print("\x1b[>1s"),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        )?;
        Terminal::new(CrosstermBackend::new(io::stdout()))
    }
    pub fn enter(i18n: &crate::i18n::I18n) -> io::Result<(Self, Screen)> {
        let mut guard = Self::new();
        let screen = guard.setup(&i18n.text("terminal-required"))?;
        Ok((guard, screen))
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
pub fn restore() {
    let _ = restore_checked(&mut io::stdout(), disable_raw_mode);
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
