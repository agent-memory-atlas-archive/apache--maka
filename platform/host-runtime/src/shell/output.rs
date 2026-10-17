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

use super::{Result, ShellError};
use maka_agent_facts::terminal::TerminalSize;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tokio::sync::Notify;

const DATA_BYTES: usize = 16 * 1024;
const EVENT_COUNT: usize = 8;
const MAX_SEQUENCE: u64 = 9_007_199_254_740_991;

/// Ephemeral native observation order, independent of durable ShellRun revisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PtyStreamEvent {
    Data {
        sequence: u64,
        size: TerminalSize,
        bytes: Arc<[u8]>,
    },
    Resize {
        sequence: u64,
        size: TerminalSize,
    },
    /// No cursor advance: resumption after a gap is an explicit caller choice.
    Gap {
        after: u64,
        first_available: u64,
    },
    Pending {
        after: u64,
        size: TerminalSize,
    },
    /// Byte-source closure, not process success or confirmed native cleanup.
    Closed {
        after: u64,
        size: TerminalSize,
    },
}

struct State {
    sequence: u64,
    size: TerminalSize,
    // Only consecutive Data/Resize events enter this bounded ring.
    events: VecDeque<PtyStreamEvent>,
    closed: bool,
}
impl State {
    fn capacity(&self, count: u64) -> Result<()> {
        if self.closed || count > MAX_SEQUENCE - self.sequence {
            return Err(ShellError::Rejected(
                "PTY output sequence exhausted or closed",
            ));
        }
        Ok(())
    }
    fn push(&mut self, event: PtyStreamEvent) {
        if self.events.len() == EVENT_COUNT {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
}

#[derive(Clone)]
pub(super) struct Output(Arc<Shared>);
struct Shared {
    state: Mutex<State>,
    changed: Notify,
    host_changes: tokio::sync::watch::Sender<()>,
}

/// Independent bounded observation cursor. Dropping it never stops the process.
pub struct PtyStream {
    output: Output,
    after: u64,
}
impl Output {
    pub fn new(size: TerminalSize, host_changes: tokio::sync::watch::Sender<()>) -> Self {
        Self(Arc::new(Shared {
            state: Mutex::new(State {
                sequence: 0,
                size,
                events: VecDeque::new(),
                closed: false,
            }),
            changed: Notify::new(),
            host_changes,
        }))
    }

    pub fn stream(&self, after: u64) -> Result<PtyStream> {
        if after > self.0.state.lock().unwrap().sequence {
            return Err(ShellError::Rejected("PTY output cursor is in the future"));
        }
        Ok(PtyStream {
            output: self.clone(),
            after,
        })
    }

    /// Publish original native bytes before any UTF-8 or screen interpretation.
    pub fn publish(&self, bytes: &[u8]) -> Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        let count = u64::try_from(bytes.len().div_ceil(DATA_BYTES))
            .map_err(|_| ShellError::Rejected("PTY output sequence exhausted"))?;
        let mut state = self.0.state.lock().unwrap();
        state.capacity(count)?;
        for bytes in bytes.chunks(DATA_BYTES) {
            state.sequence += 1;
            let event = PtyStreamEvent::Data {
                sequence: state.sequence,
                size: state.size,
                bytes: Arc::from(bytes),
            };
            state.push(event);
        }
        drop(state);
        self.notify();
        Ok(())
    }

    /// The sole worker checks counter capacity before performing native resize.
    /// This is a preflight, not a second reservation or resize authority.
    pub fn check_resize(&self, size: TerminalSize) -> Result<()> {
        let state = self.0.state.lock().unwrap();
        state.capacity(u64::from(size != state.size))
    }

    /// Called immediately after native resize succeeds, before parser/SQL work.
    pub fn resize(&self, size: TerminalSize) -> Result<()> {
        let mut state = self.0.state.lock().unwrap();
        state.capacity(u64::from(size != state.size))?;
        if size == state.size {
            return Ok(());
        }
        state.size = size;
        state.sequence += 1;
        let event = PtyStreamEvent::Resize {
            sequence: state.sequence,
            size,
        };
        state.push(event);
        drop(state);
        self.notify();
        Ok(())
    }

    pub fn close(&self) {
        self.0.state.lock().unwrap().closed = true;
        self.notify();
    }
    fn notify(&self) {
        self.0.changed.notify_waiters();
        self.0.host_changes.send_replace(());
    }
}
impl PtyStream {
    /// Immediate observation; only delivered Data/Resize advances this cursor.
    pub fn read(&mut self) -> PtyStreamEvent {
        let state = self.output.0.state.lock().unwrap();
        if self.after < state.sequence {
            let first_available = state.sequence - state.events.len() as u64 + 1;
            if self.after + 1 < first_available {
                return PtyStreamEvent::Gap {
                    after: self.after,
                    first_available,
                };
            }
            let index = (self.after + 1 - first_available) as usize;
            let event = state.events[index].clone();
            self.after += 1;
            return event;
        }
        if state.closed {
            PtyStreamEvent::Closed {
                after: self.after,
                size: state.size,
            }
        } else {
            PtyStreamEvent::Pending {
                after: self.after,
                size: state.size,
            }
        }
    }

    pub async fn next(&mut self) -> PtyStreamEvent {
        let output = self.output.clone();
        loop {
            // Register before inspection, so publication and EOF cannot be lost.
            let changed = output.0.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let event = self.read();
            if !matches!(event, PtyStreamEvent::Pending { .. }) {
                return event;
            }
            changed.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn output() -> Output {
        Output::new(
            TerminalSize::new(80, 24).unwrap(),
            tokio::sync::watch::channel(()).0,
        )
    }

    #[test]
    fn ordered_bytes_and_resize_keep_exact_cuts_and_retained_closed_data() {
        let output = output();
        let initial = TerminalSize::new(80, 24).unwrap();
        let resized = TerminalSize::new(100, 40).unwrap();
        let mut stream = output.stream(0).unwrap();
        assert_eq!(
            stream.read(),
            PtyStreamEvent::Pending {
                after: 0,
                size: initial
            }
        );
        output.publish(b"\xff\0\x1b[31m").unwrap();
        output.check_resize(resized).unwrap();
        output.resize(resized).unwrap();
        output.resize(resized).unwrap(); // same-size acceptance is not a new cut
        output.publish(&[0xe4, 0xb8, 0xad]).unwrap();
        output.close();
        assert_eq!(
            stream.read(),
            PtyStreamEvent::Data {
                sequence: 1,
                size: initial,
                bytes: Arc::from(&b"\xff\0\x1b[31m"[..])
            }
        );
        assert_eq!(
            stream.read(),
            PtyStreamEvent::Resize {
                sequence: 2,
                size: resized
            }
        );
        assert_eq!(
            stream.read(),
            PtyStreamEvent::Data {
                sequence: 3,
                size: resized,
                bytes: Arc::from(&[0xe4, 0xb8, 0xad][..])
            }
        );
        assert_eq!(
            stream.read(),
            PtyStreamEvent::Closed {
                after: 3,
                size: resized
            }
        );
        assert_eq!(
            stream.read(),
            PtyStreamEvent::Closed {
                after: 3,
                size: resized
            }
        );
        assert!(output.stream(4).is_err());
    }

    #[test]
    fn bounded_byte_events_gap_without_advancing_and_explicit_resumption() {
        let output = output();
        let mut lagged = output.stream(0).unwrap();
        let bytes: Vec<u8> = (0..DATA_BYTES * 9 + 7).map(|index| index as u8).collect();
        output.publish(&bytes).unwrap();
        let gap = PtyStreamEvent::Gap {
            after: 0,
            first_available: 3,
        };
        assert_eq!(lagged.read(), gap);
        output.close();
        assert_eq!(
            lagged.read(),
            gap,
            "closure must not silently skip an omitted event"
        );
        let mut resumed = output.stream(2).unwrap();
        let mut actual = Vec::new();
        for expected in 3..=10 {
            let PtyStreamEvent::Data {
                sequence, bytes, ..
            } = resumed.read()
            else {
                panic!("expected retained bytes")
            };
            assert_eq!(sequence, expected);
            assert!(bytes.len() <= DATA_BYTES);
            actual.extend_from_slice(&bytes);
        }
        assert_eq!(actual, bytes[DATA_BYTES * 2..]);
        assert!(matches!(
            resumed.read(),
            PtyStreamEvent::Closed { after: 10, .. }
        ));
    }

    #[tokio::test]
    async fn resize_only_wakes_a_waiter_and_closure_wakes_without_cursor_advance() {
        let output = output();
        let size = TerminalSize::new(100, 40).unwrap();
        let mut stream = output.stream(0).unwrap();
        {
            let next = stream.next();
            tokio::pin!(next);
            assert!(futures_util::poll!(next.as_mut()).is_pending());
            output.resize(size).unwrap();
            assert_eq!(next.await, PtyStreamEvent::Resize { sequence: 1, size });
        }
        {
            let next = stream.next();
            tokio::pin!(next);
            assert!(futures_util::poll!(next.as_mut()).is_pending());
            output.close();
            assert_eq!(next.await, PtyStreamEvent::Closed { after: 1, size });
        }
    }

    #[test]
    fn sequence_capacity_rejects_before_resize_and_never_wraps() {
        let output = output();
        output.0.state.lock().unwrap().sequence = MAX_SEQUENCE;
        assert!(
            output
                .check_resize(TerminalSize::new(81, 24).unwrap())
                .is_err()
        );
        assert!(output.publish(b"x").is_err());
        assert!(output.stream(MAX_SEQUENCE + 1).is_err());
    }
}
