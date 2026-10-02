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

use futures::channel::oneshot;
use std::sync::{Arc, Mutex};

/// A one-shot acknowledgement of native destruction and backend resource release.
/// Failure (including cancellation) does not confirm cleanup. The receiver never
/// owns a platform window. Driver-internal deferred GPU work is outside this receipt.
pub type WindowCleanupReceiver = oneshot::Receiver<Result<(), String>>;

/// Backend-owned cleanup ticket. Keep this as the final field of resource state,
/// so its guard releases only after the renderer, input handlers and callbacks.
pub struct WindowCleanup {
    guard: WindowCleanupGuard,
    receiver: Option<WindowCleanupReceiver>,
}
impl Default for WindowCleanup {
    fn default() -> Self {
        let (sender, receiver) = oneshot::channel();
        Self {
            guard: WindowCleanupGuard(Arc::new(Completion {
                outcome: Mutex::new(Outcome::default()),
                sender: Mutex::new(Some(sender)),
            })),
            receiver: Some(receiver),
        }
    }
}
impl WindowCleanup {
    /// Claims the receiver exactly once, without retaining any backend resource.
    pub fn take_receiver(&mut self) -> anyhow::Result<WindowCleanupReceiver> {
        self.receiver
            .take()
            .ok_or_else(|| anyhow::anyhow!("window cleanup receiver already claimed"))
    }
    /// Extends the release barrier across an OS destructor or asynchronous ack.
    pub fn guard(&self) -> WindowCleanupGuard {
        self.guard.clone()
    }
}

/// Clonable release barrier, independent of the platform window and its state.
/// Success waits for the final clone; known failure is reported immediately.
/// Dropping a clone cannot confirm destruction.
#[derive(Clone)]
pub struct WindowCleanupGuard(Arc<Completion>);
impl WindowCleanupGuard {
    /// Records the backend's actual native destruction acknowledgement.
    /// Does not override failure and does not complete until every guard releases.
    pub fn confirm_native_destroyed(&self) {
        self.0.outcome.lock().unwrap().destroyed = true;
    }
    /// Whether native destruction has been observed (resources may still exist).
    pub fn native_destroyed(&self) -> bool {
        self.0.outcome.lock().unwrap().destroyed
    }
    /// Records a teardown error; subsequent acknowledgements cannot mask it.
    pub fn fail(&self, error: impl Into<String>) {
        let error = {
            let mut outcome = self.0.outcome.lock().unwrap();
            outcome.error.get_or_insert_with(|| error.into()).clone()
        };
        // A known failure may itself keep native resources alive forever. Report
        // it immediately; this is a failed fence, never a release acknowledgement.
        if let Some(sender) = self.0.sender.lock().unwrap().take() {
            let _ = sender.send(Err(error));
        }
    }
}
#[derive(Default)]
struct Outcome {
    destroyed: bool,
    error: Option<String>,
}

struct Completion {
    outcome: Mutex<Outcome>,
    sender: Mutex<Option<oneshot::Sender<Result<(), String>>>>,
}
impl Drop for Completion {
    fn drop(&mut self) {
        let outcome = match self.outcome.get_mut() {
            Ok(Outcome {
                error: Some(error), ..
            }) => Err(error.clone()),
            Ok(Outcome {
                destroyed: true,
                error: None,
            }) => Ok(()),
            Ok(Outcome {
                destroyed: false,
                error: None,
            }) => Err("native window destruction unconfirmed".into()),
            Err(_) => Err("native window cleanup outcome poisoned".into()),
        };
        if let Ok(sender) = self.sender.get_mut()
            && let Some(sender) = sender.take()
        {
            let _ = sender.send(outcome);
        }
    }
}
