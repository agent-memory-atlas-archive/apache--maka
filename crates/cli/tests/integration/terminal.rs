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

#![cfg(unix)]

mod application;
mod frames;

use maka_process::terminal::Screen;
use maka_runtime::terminal::TerminalSize;
use std::{
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    },
    process::{Child, Command},
    time::{Duration, Instant},
};

struct Pty {
    child: Child,
    master: Option<File>,
    // Keep one slave descriptor to read the final termios and avoid EIO while draining.
    slave: Option<File>,
    screen: Screen,
    pending: Vec<u8>,
    output: Vec<u8>,
    frames: frames::Frames,
    read_size: usize,
}

impl Pty {
    fn spawn(args: &[&str]) -> Self {
        let mut master = -1;
        let mut slave = -1;
        let mut size = libc::winsize {
            ws_row: 40,
            ws_col: 120,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &raw mut size,
                )
            },
            0
        );
        let master = unsafe { File::from_raw_fd(master) };
        let slave = unsafe { File::from_raw_fd(slave) };
        for fd in [master.as_raw_fd(), slave.as_raw_fd()] {
            assert_ne!(
                unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
                -1
            );
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_maka"));
        command
            .args(args)
            .env("TERM", "xterm-256color")
            .env("LC_ALL", "en_US.UTF-8")
            .stdin(slave.try_clone().unwrap())
            .stdout(slave.try_clone().unwrap())
            .stderr(slave.try_clone().unwrap());
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn().unwrap();
        Self {
            child,
            master: Some(master),
            slave: Some(slave),
            screen: Screen::new(TerminalSize::new(120, 40).unwrap()),
            pending: Vec::new(),
            output: Vec::new(),
            frames: frames::Frames::default(),
            read_size: 8192,
        }
    }
    fn send(&mut self, bytes: &[u8]) {
        self.master.as_mut().unwrap().write_all(bytes).unwrap();
    }
    fn read(&mut self) {
        let mut poll = libc::pollfd {
            fd: self.master.as_ref().unwrap().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let available = unsafe { libc::poll(&mut poll, 1, 50) };
        assert!(available >= 0);
        if available == 0 {
            return;
        }
        let mut buffer = [0; 8192];
        let length = self
            .master
            .as_mut()
            .unwrap()
            .read(&mut buffer[..self.read_size])
            .unwrap();
        self.frames.feed(&buffer[..length]);
        self.output.extend_from_slice(&buffer[..length]);
        if self.output.len() > 1024 * 1024 {
            self.output.drain(..self.output.len() - 1024 * 1024);
        }
        self.pending.extend_from_slice(&buffer[..length]);
        let end = match std::str::from_utf8(&self.pending) {
            Ok(_) => self.pending.len(),
            Err(error) if error.error_len().is_none() => error.valid_up_to(),
            Err(error) => panic!("Invalid terminal UTF-8: {error}"),
        };
        let text = std::str::from_utf8(&self.pending[..end]).unwrap();
        let reply = self.screen.write(text).unwrap();
        self.pending.drain(..end);
        if !reply.is_empty() {
            self.send(reply.as_bytes());
        }
    }
    #[track_caller]
    fn wait_for(&mut self, text: &str) {
        self.wait_until(|screen| screen.contains(text));
    }
    #[track_caller]
    fn wait_until(&mut self, predicate: impl Fn(&str) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.read();
            let snapshot = self.screen.snapshot().unwrap();
            if self.frames.ready() && predicate(&snapshot.screen) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "screen condition timed out (pending tty bytes: {}):\n{}",
                {
                    let mut pending = 0;
                    assert_eq!(
                        unsafe {
                            libc::ioctl(
                                self.slave.as_ref().unwrap().as_raw_fd(),
                                libc::FIONREAD,
                                &mut pending,
                            )
                        },
                        0
                    );
                    pending
                },
                snapshot.screen
            );
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "TUI exited:\n{}",
                snapshot.screen
            );
        }
    }
    fn finish(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.read();
            if let Some(status) = self.child.try_wait().unwrap() {
                self.read();
                assert!(status.success());
                return;
            }
            assert!(Instant::now() < deadline, "TUI did not exit");
        }
    }
    fn terminate(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.kill()?;
        // Session teardown may need the last parent-held PTY handles released.
        self.master.take();
        self.slave.take();
        self.child.wait()
    }
}
impl Drop for Pty {
    fn drop(&mut self) {
        if std::thread::panicking()
            && let Ok(snapshot) = self.screen.snapshot()
        {
            eprintln!("Final terminal frame:\n{}", snapshot.screen);
        }
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.terminate();
        }
    }
}
