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
//! Foreground-only JavaScript execution. A callback returns staged JSON intent;
//! the adapter validates and commits it after this method succeeds. No ambient
//! system capabilities or async Host bridge are installed in this execution domain.

use std::{
    collections::BTreeMap,
    rc::Rc,
    sync::{Arc, Condvar, Mutex, OnceLock, atomic::AtomicBool},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use deno_core::JsRuntime;
use serde_json::Value;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use super::{Bootstrap, Health, Limits, Result, bridge, engine, failed};

/// An opaque module reference, valid only in its originating foreground realm.
#[derive(Clone)]
pub struct ForegroundModule {
    domain: Rc<()>,
    id: u64,
}

/// Create and use on the UI owner thread. The watchdog only interrupts V8; it
/// never executes callbacks or owns native UI objects. The embedding supplies a
/// background Tokio timer handle for Deno's native V8 scheduling and keeps that
/// scheduler alive until this realm is destroyed. No ambient Tokio context is needed.
/// Shared modules share a failure domain. A script exception, promise result or
/// budget violation closes the domain before any returned intent can commit.
pub struct Foreground {
    // Module globals must be destroyed before their isolate.
    modules: BTreeMap<u64, engine::Module>,
    domain: Rc<()>,
    next_module: u64,
    budget: Budget,
    scheduler: tokio::runtime::Handle,
    runtime: JsRuntime,
}

impl Foreground {
    pub fn new(limits: Limits, scheduler: tokio::runtime::Handle) -> Result<Self> {
        if limits.heap_bytes < 16 * 1024 * 1024 || limits.synchronous_slice.is_zero() {
            return Err(failed("invalid foreground VM limits"));
        }
        crate::initialize_platform();
        let health = Arc::new(Health {
            isolate: OnceLock::new(),
            failure: Mutex::default(),
            stopped: CancellationToken::new(),
            closed: CancellationToken::new(),
            initializing: AtomicBool::new(false),
            polling: watch::channel(None).0,
        });
        let runtime = {
            let _entered = scheduler.enter();
            engine::foreground_runtime(&health, &limits)?
        };
        let budget = Budget::new(health, limits.synchronous_slice)?;
        let mut foreground = Self {
            modules: BTreeMap::new(),
            domain: Rc::new(()),
            next_module: 0,
            budget,
            scheduler,
            runtime,
        };
        // OwnedIsolate enters on creation. Foreground realms coexist and may
        // be destroyed in any order, so operations enter their own isolate.
        unsafe {
            foreground.runtime.v8_isolate().exit();
        }
        Ok(foreground)
    }

    pub fn load(&mut self, name: &str, source: &str) -> Result<ForegroundModule> {
        self.ready()?;
        if name.is_empty() || name.len() > 1024 || source.len() > 8 * 1024 * 1024 {
            return Err(failed("foreground module exceeds name/source limits"));
        }
        let id = self
            .next_module
            .checked_add(1)
            .ok_or_else(|| failed("module identity exhausted"))?;
        self.budget.enter();
        let _entered = self.scheduler.enter();
        let outcome = entered(&mut self.runtime, |runtime| {
            engine::load(runtime, name, source, Bootstrap::Plain, id).and_then(|module| {
                engine::poll_ready_jobs(runtime)?;
                Ok(module)
            })
        });
        let outcome = self.budget.finish(outcome)?;
        self.next_module = id;
        self.modules.insert(id, outcome);
        Ok(ForegroundModule {
            domain: self.domain.clone(),
            id,
        })
    }

    /// Return an inert JSON batch. Native resources, service calls and event
    /// decisions must only be applied by the adapter after validating this batch.
    /// Ready JavaScript jobs are drained under the same budget, without awaiting
    /// I/O. Their return values never publish intent. Promise callback results are rejected.
    pub fn call(
        &mut self,
        module: &ForegroundModule,
        path: &[String],
        args: &[Value],
    ) -> Result<Value> {
        self.ready()?;
        if !Rc::ptr_eq(&module.domain, &self.domain) || !self.modules.contains_key(&module.id) {
            return Err(failed("foreground module is retired or from another realm"));
        }
        if path.is_empty()
            || path.len() > 32
            || path.iter().any(|part| part.is_empty() || part.len() > 128)
        {
            return Err(failed("invalid foreground export path"));
        }
        bridge::encoded_size(args, 1024 * 1024).map_err(failed)?;
        self.budget.enter();
        let _entered = self.scheduler.enter();
        let outcome = entered(&mut self.runtime, |runtime| {
            engine::call_synchronous(runtime, &self.modules[&module.id], path, args).and_then(
                |value| {
                    engine::poll_ready_jobs(runtime)?;
                    Ok(value)
                },
            )
        });
        self.budget.finish(outcome)
    }

    /// Revocation is synchronous and does not dispatch dispose code. Cleanup of
    /// resources created by a committed batch belongs to the embedding owner.
    pub fn close(&mut self, module: &ForegroundModule) -> Result<()> {
        if !Rc::ptr_eq(&module.domain, &self.domain) || !self.modules.contains_key(&module.id) {
            return Err(failed("foreground module is retired or from another realm"));
        }
        entered(&mut self.runtime, |_| {
            self.modules.remove(&module.id);
        });
        Ok(())
    }

    pub fn failure(&self) -> Option<super::Error> {
        self.budget.health.failure.lock().unwrap().clone()
    }

    fn ready(&self) -> Result<()> {
        if self.budget.health.stopped.is_cancelled() {
            Err(self.budget.health.error())
        } else {
            Ok(())
        }
    }
}

impl Drop for Foreground {
    fn drop(&mut self) {
        // Globals and JsRuntime drop with their isolate current; OwnedIsolate's
        // destructor balances this entry before disposal.
        unsafe {
            self.runtime.v8_isolate().enter();
        }
    }
}

fn entered<T>(runtime: &mut JsRuntime, action: impl FnOnce(&mut JsRuntime) -> T) -> T {
    struct Exit(std::ptr::NonNull<deno_core::v8::Isolate>);
    impl Drop for Exit {
        fn drop(&mut self) {
            // Private actions never replace/destroy the borrowed runtime. V8
            // scopes unwind before this guard restores the previous isolate.
            unsafe {
                self.0.as_ref().exit();
            }
        }
    }
    let isolate = std::ptr::NonNull::from(&mut **runtime.v8_isolate());
    unsafe {
        isolate.as_ref().enter();
    }
    let _exit = Exit(isolate);
    action(runtime)
}

struct Clock {
    started: Option<Instant>,
    closed: bool,
}
struct Budget {
    health: Arc<Health>,
    clock: Arc<(Mutex<Clock>, Condvar)>,
    slice: Duration,
    watcher: Option<JoinHandle<()>>,
}

impl Budget {
    fn new(health: Arc<Health>, slice: Duration) -> Result<Self> {
        let clock = Arc::new((
            Mutex::new(Clock {
                started: None,
                closed: false,
            }),
            Condvar::new(),
        ));
        let observing = clock.clone();
        let observed_health = health.clone();
        let watcher = std::thread::Builder::new()
            .name("maka-foreground-js-budget".into())
            .spawn(move || {
                let (lock, changed) = &*observing;
                let mut clock = lock.lock().unwrap();
                loop {
                    if clock.closed {
                        break;
                    }
                    match clock.started {
                        None => clock = changed.wait(clock).unwrap(),
                        Some(started) => {
                            if let Some(remaining) = slice.checked_sub(started.elapsed()) {
                                clock = changed.wait_timeout(clock, remaining).unwrap().0;
                            } else {
                                observed_health
                                    .fail(failed("foreground synchronous budget exceeded"));
                                clock.started = None;
                            }
                        }
                    }
                }
            })
            .map_err(failed)?;
        Ok(Self {
            health,
            clock,
            slice,
            watcher: Some(watcher),
        })
    }

    fn enter(&self) {
        self.clock.0.lock().unwrap().started = Some(Instant::now());
        self.clock.1.notify_one();
    }

    fn finish<T>(&self, outcome: Result<T>) -> Result<T> {
        let started = self.clock.0.lock().unwrap().started.take();
        self.clock.1.notify_one();
        // Even a delayed watchdog cannot let an over-budget callback commit.
        if started.is_some_and(|started| started.elapsed() >= self.slice) {
            self.health
                .fail(failed("foreground synchronous budget exceeded"));
        }
        if let Err(error) = &outcome {
            self.health.fail(error.clone());
        }
        if self.health.stopped.is_cancelled() {
            Err(self.health.error())
        } else {
            outcome
        }
    }
}

impl Drop for Budget {
    fn drop(&mut self) {
        self.clock.0.lock().unwrap().closed = true;
        self.clock.1.notify_one();
        if let Some(watcher) = self.watcher.take() {
            let _ = watcher.join();
        }
    }
}
