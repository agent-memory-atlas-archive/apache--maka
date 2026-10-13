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

//! Bounded native observations, owned by the original renderer resource.
//! Queue admission transfers custody; only the original logical commit confirms
//! publication. Candidate rejection retains the exact observation and its ACK.
use crate::{
    Error, Result,
    backend::Job,
    budget::{Kind, NativeBudget, Permit},
};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Wake, Waker},
};
use tokio::sync::oneshot;

const MAX_PENDING: usize = 128;
const MAX_VALUE_BYTES: usize = 1024 * 1024;
struct Observation {
    value: Value,
    replaceable: bool,
    _bytes: Permit,
    acknowledgement: oneshot::Sender<Result<()>>,
}
#[derive(Default)]
struct Channel {
    pending: VecDeque<Observation>,
    reserved: VecDeque<Permit>,
    publishing: Option<((u64, u64), Job<()>)>,
    rejected: Option<(u64, u64)>,
}
struct State {
    channels: BTreeMap<String, Channel>,
    closed: bool,
}
/// Capacity prepared by a trusted immutable recipe before its native commands.
/// Dropping a failed candidate releases it without touching the committed queue.
pub struct Capacity(Vec<(String, Permit)>);
struct Notification(std::sync::Arc<dyn Fn() + Send + Sync>);
impl Wake for Notification {
    fn wake(self: std::sync::Arc<Self>) {
        (self.0)();
    }
    fn wake_by_ref(self: &std::sync::Arc<Self>) {
        (self.0)();
    }
}
#[derive(Clone)]
pub struct Observations {
    state: Rc<RefCell<State>>,
    budget: NativeBudget,
    wake: Waker,
}
impl Observations {
    pub fn new(budget: NativeBudget, notify: impl Fn() + Send + Sync + 'static) -> Self {
        let wake = Waker::from(std::sync::Arc::new(Notification(std::sync::Arc::new(
            notify,
        ))));
        Self {
            state: Rc::new(RefCell::new(State {
                channels: BTreeMap::new(),
                closed: false,
            })),
            budget,
            wake,
        }
    }
    pub fn prepare(
        &self,
        requirements: impl IntoIterator<Item = (&'static str, usize)>,
    ) -> Result<Capacity> {
        let state = self.state.borrow();
        if state.closed {
            return Err(Error::Invalid(
                "original native publication owner closed".into(),
            ));
        }
        let mut required = BTreeMap::<&str, (usize, usize)>::new();
        for (key, bytes) in requirements {
            if bytes > MAX_VALUE_BYTES {
                return Err(Error::Invalid(
                    "native publication bound exceeds wire capacity".into(),
                ));
            }
            let capacity = required.entry(key).or_default();
            capacity.0 += 1;
            capacity.1 = capacity.1.max(bytes);
        }
        let mut reserved = Vec::new();
        for (key, (slots, bytes)) in required {
            let channel = state.channels.get(key);
            let queued = channel.map_or(0, |channel| channel.pending.len());
            let retained = channel.map_or(0, |channel| channel.reserved.len());
            if queued + slots.max(retained) > MAX_PENDING {
                return Err(Error::Invalid(
                    "native publication capacity exhausted before command".into(),
                ));
            }
            for _ in retained..slots {
                reserved.push((
                    key.to_owned(),
                    self.budget
                        .reserve(Kind::Encoded, bytes)
                        .map_err(|e| Error::Invalid(e.to_string()))?,
                ));
            }
        }
        Ok(Capacity(reserved))
    }
    pub fn adopt(&self, capacity: Capacity) {
        let mut state = self.state.borrow_mut();
        assert!(!state.closed, "original native publication owner");
        for (key, permit) in capacity.0 {
            state
                .channels
                .entry(key)
                .or_default()
                .reserved
                .push_back(permit);
        }
    }
    /// Capture at the native effect boundary, before a later command can change
    /// its receipt. Identical evidence already in custody needs no second ACK.
    pub fn capture(&self, key: &str, value: Value, replaceable: bool) -> Result<Option<Job<()>>> {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(Error::Invalid(
                "original native publication owner closed".into(),
            ));
        }
        let channel = state.channels.entry(key.to_owned()).or_default();
        if channel
            .pending
            .back()
            .is_some_and(|entry| entry.value == value)
        {
            return Ok(None);
        }
        let bytes = serde_json::to_vec(&value)
            .map_err(|e| Error::Invalid(e.to_string()))?
            .len();
        if bytes > MAX_VALUE_BYTES {
            return Err(Error::Invalid(
                "native publication exceeds wire capacity".into(),
            ));
        }
        let mut permit = if !replaceable && !channel.reserved.is_empty() {
            channel.reserved.pop_front().unwrap()
        } else {
            self.budget
                .reserve(Kind::Encoded, bytes)
                .map_err(|e| Error::Invalid(e.to_string()))?
        };
        permit.shrink_to(bytes);
        if replaceable
            && channel
                .pending
                .back()
                .is_some_and(|entry| entry.replaceable)
            && (channel.publishing.is_none() || channel.pending.len() > 1)
        {
            if let Some(entry) = channel.pending.pop_back() {
                let _ = entry.acknowledgement.send(Err(Error::Invalid(
                    "replaceable native observation superseded".into(),
                )));
            }
        } else if channel.pending.len() + channel.reserved.len() >= MAX_PENDING {
            return Err(Error::Invalid(
                "native publication capacity exhausted".into(),
            ));
        }
        let (acknowledgement, result) = oneshot::channel();
        channel.pending.push_back(Observation {
            value,
            replaceable,
            _bytes: permit,
            acknowledgement,
        });
        channel.rejected = None;
        drop(state);
        self.wake.wake_by_ref();
        Ok(Some(Job::new(async move {
            result.await.unwrap_or_else(|_| {
                Err(Error::Invalid(
                    "original native publication owner retired".into(),
                ))
            })
        })))
    }
    /// Poll one original publication per channel. A failed candidate stays at
    /// the head and retries only after a real business/source revision changes.
    pub fn advance<C>(
        &self,
        revision: (u64, u64),
        context: &mut C,
        mut send: impl FnMut(&str, Value, &mut C) -> Result<Job<()>>,
        mut committed: impl FnMut(&str, &Value, &mut C),
    ) -> Vec<Error> {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Vec::new();
        }
        let mut cx = Context::from_waker(&self.wake);
        let mut errors = Vec::new();
        for (key, channel) in &mut state.channels {
            if let Some((basis, mut job)) = channel.publishing.take() {
                match Pin::new(&mut job).poll(&mut cx) {
                    Poll::Pending => channel.publishing = Some((basis, job)),
                    Poll::Ready(Ok(())) => {
                        let entry = channel
                            .pending
                            .pop_front()
                            .expect("original publishing observation");
                        committed(key, &entry.value, context);
                        let _ = entry.acknowledgement.send(Ok(()));
                        channel.rejected = None;
                    }
                    Poll::Ready(Err(error)) => {
                        channel.rejected = Some(basis);
                        errors.push(error);
                    }
                }
            }
            if channel.publishing.is_none()
                && channel.rejected != Some(revision)
                && let Some(entry) = channel.pending.front()
            {
                match send(key, entry.value.clone(), context) {
                    Ok(mut job) => match Pin::new(&mut job).poll(&mut cx) {
                        Poll::Pending => channel.publishing = Some((revision, job)),
                        Poll::Ready(Ok(())) => {
                            let entry = channel
                                .pending
                                .pop_front()
                                .expect("original publishing observation");
                            committed(key, &entry.value, context);
                            let _ = entry.acknowledgement.send(Ok(()));
                            channel.rejected = None;
                        }
                        Poll::Ready(Err(error)) => {
                            channel.rejected = Some(revision);
                            errors.push(error);
                        }
                    },
                    Err(error) => {
                        channel.rejected = Some(revision);
                        errors.push(error);
                    }
                }
            }
        }
        errors
    }
    /// A removed declaration cannot reinterpret the accepted input on a new
    /// control. Resolve its original custody with retirement, never fake success.
    pub fn retain(&self, mut declared: impl FnMut(&str) -> bool) {
        self.state
            .borrow_mut()
            .channels
            .retain(|key, _| declared(key));
    }
    pub fn close(&self) {
        let mut state = self.state.borrow_mut();
        state.closed = true;
        state.channels.clear();
    }
    pub fn contains(&self, key: &str) -> bool {
        self.state
            .borrow()
            .channels
            .get(key)
            .is_some_and(|channel| !channel.pending.is_empty())
    }
    pub fn pending(&self) -> bool {
        self.state
            .borrow()
            .channels
            .values()
            .any(|channel| !channel.pending.is_empty())
    }
}
