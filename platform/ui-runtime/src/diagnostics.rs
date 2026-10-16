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

//! Opt-in UI diagnostics, enabled with `MAKA_UI_TRACE=/absolute/path.jsonl`.
//! Payloads contain scheduling metadata only, never service inputs or outputs.

use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Trace {
    sender: mpsc::SyncSender<Message>,
    dropped: AtomicU64,
}
enum Message {
    Event(Value),
    Flush(mpsc::SyncSender<()>),
}
fn trace() -> Option<&'static Trace> {
    static TRACE: OnceLock<Option<Trace>> = OnceLock::new();
    TRACE
        .get_or_init(|| {
            let path = std::env::var_os("MAKA_UI_TRACE")?;
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .ok()?;
            let (sender, receiver) = mpsc::sync_channel(4096);
            std::thread::Builder::new()
                .name("maka-ui-trace".into())
                .spawn(move || {
                    let header = json!({
                        "event": "trace_started",
                        "pid": std::process::id(),
                        "time_us": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_micros(),
                        "fields": {
                            "debug_build": cfg!(debug_assertions),
                            "version": env!("CARGO_PKG_VERSION"),
                        },
                    });
                    if serde_json::to_writer(&mut file, &header).is_err()
                        || file.write_all(b"\n").is_err()
                    {
                        return;
                    }
                    while let Ok(message) = receiver.recv() {
                        match message {
                            Message::Event(value) => {
                                if serde_json::to_writer(&mut file, &value).is_err()
                                    || file.write_all(b"\n").is_err()
                                {
                                    break;
                                }
                            }
                            Message::Flush(done) => {
                                let _ = file.flush();
                                let _ = done.try_send(());
                            }
                        }
                    }
                })
                .ok()?;
            Some(Trace {
                sender,
                dropped: AtomicU64::new(0),
            })
        })
        .as_ref()
}
/// Record lazily without blocking the foreground on filesystem writes.
/// A saturated diagnostic queue drops records and reports the count next time.
pub fn record(event: &'static str, fields: impl FnOnce() -> Value) {
    let Some(trace) = trace() else { return };
    let dropped = trace.dropped.swap(0, Ordering::Relaxed);
    let value = json!({
        "time_us": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_micros(),
        "pid": std::process::id(), "event": event, "dropped": dropped, "fields": fields(),
    });
    if trace.sender.try_send(Message::Event(value)).is_err() {
        trace.dropped.fetch_add(dropped + 1, Ordering::Relaxed);
    }
}
/// Give fatal diagnostics a bounded opportunity to reach disk before exit.
pub fn flush() {
    if let Some(trace) = trace() {
        let (send, receive) = mpsc::sync_channel(1);
        if trace.sender.try_send(Message::Flush(send)).is_ok() {
            let _ = receive.recv_timeout(Duration::from_millis(250));
        }
    }
}
/// Start a latency measurement only when diagnostics are enabled.
pub fn started() -> Option<Instant> {
    trace().map(|_| Instant::now())
}
/// Log synchronous work exceeding one 60 Hz frame, including error returns.
/// Do not wrap waits for input: idle time is not UI work.
pub struct Span {
    event: &'static str,
    start: Option<Instant>,
}
impl Span {
    pub fn new(event: &'static str) -> Self {
        Self {
            event,
            start: trace().map(|_| Instant::now()),
        }
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            let elapsed = start.elapsed();
            if elapsed >= Duration::from_millis(16) {
                record(self.event, || json!({"elapsed_us": elapsed.as_micros()}));
            }
        }
    }
}
