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

//! Original native request custody on an authenticated physical connector.
use super::{Call, Reply, Selection, Source};
use crate::Result;
use futures_util::{FutureExt, future::BoxFuture};
use maka_event_log::applications::intents::RecordedIntent;
use maka_plugin_core::{
    contract::{Interface, MethodMode},
    deployment::DeploymentRef,
    operation::{OperationReply, Receipt},
};
use maka_ui::actions::*;
use std::{
    panic::AssertUnwindSafe,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf},
    sync::{Notify, OwnedSemaphorePermit, Semaphore},
};
use tokio_util::sync::CancellationToken;

pub trait Channel: AsyncRead + AsyncWrite + Unpin + Send + 'static {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + 'static> Channel for T {}
pub type Stream = Box<dyn Channel>;
/// Trusted physical admission only. A fresh channel belongs to the same
/// authenticated Host incarnation that published the frozen Selection.
/// Cancellation requests settlement of this same opening. Its future must
/// complete before custody is released; an error confirms that no execution or
/// physical custody remains outside the embedding. Dropping or panicking an
/// opening observer does not establish that confirmation.
pub trait Connector: Send + Sync + 'static {
    fn connect(&self, stop: CancellationToken) -> BoxFuture<'static, Result<Stream>>;
}
type Invocation = Call<ReadHalf<Stream>, WriteHalf<Stream>>;
const CONNECTIONS: u32 = 16;
struct Failures {
    reason: Option<String>,
    custody: Vec<Failed>,
}
struct Failed {
    _call: Option<Invocation>,
    _request: Option<ServiceRequest>,
    _recorded: Option<RecordedIntent>,
    _permit: OwnedSemaphorePermit,
}
/// Physical lifecycle custody. This is not a routing, replay or execution map.
/// Failed original calls remain owned and fenced; historical receipts cannot
/// settle them. Drain UI accepted tasks before awaiting this connection close.
pub struct Connection {
    connector: Arc<dyn Connector>,
    slots: Arc<Semaphore>,
    stop: CancellationToken,
    closed: Mutex<bool>,
    failed: Mutex<Failures>,
    changed: Notify,
}
impl Connection {
    pub fn new(connector: Arc<dyn Connector>) -> Arc<Self> {
        Arc::new(Self {
            connector,
            slots: Arc::new(Semaphore::new(CONNECTIONS as usize)),
            stop: CancellationToken::new(),
            closed: Mutex::new(false),
            failed: Mutex::new(Failures {
                reason: None,
                custody: Vec::new(),
            }),
            changed: Notify::new(),
        })
    }
    fn session(
        self: &Arc<Self>,
        selection: Selection,
        request: ServiceRequest,
    ) -> Result<Arc<Session>> {
        let gate = self.closed.lock().unwrap();
        if *gate {
            return Err("physical connection is closing".into());
        }
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| "physical connection capacity exhausted")?;
        Ok(Arc::new(Session {
            connection: self.clone(),
            selection,
            stop: self.stop.child_token(),
            state: tokio::sync::Mutex::new(State {
                request: Some(request),
                call: None,
                opening: false,
                physical_failure: None,
            }),
            recorded: Mutex::new(None),
            permit: Some(permit),
        }))
    }
    fn failure(&self, reason: String) {
        self.failed.lock().unwrap().reason.get_or_insert(reason);
        self.changed.notify_waiters();
    }
    /// Synchronous revocation precedes polling the close observer. Active owners
    /// receive cancellation independently of their per-session async mutex.
    pub fn close(self: &Arc<Self>) -> BoxFuture<'static, Result<()>> {
        *self.closed.lock().unwrap() = true;
        self.stop.cancel();
        let own = self.clone();
        Box::pin(async move {
            loop {
                let changed = own.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if own.failed.lock().unwrap().reason.is_some() {
                    return Err("original physical call cleanup remains unconfirmed".into());
                }
                if own.slots.available_permits() == CONNECTIONS as usize {
                    return Ok(());
                }
                changed.await;
            }
        })
    }
}
pub struct Service {
    connection: Arc<Connection>,
    selection: Selection,
    caller: DeploymentRef,
}
impl Service {
    pub fn new(
        connection: Arc<Connection>,
        selection: Selection,
        caller: DeploymentRef,
    ) -> Result<Self> {
        caller.validate()?;
        let Interface::Service { .. } = &selection.description().contract.interface else {
            return Err("remote native endpoint requires a service".into());
        };
        Ok(Self {
            connection,
            selection,
            caller,
        })
    }
    fn session(&self, request: ServiceRequest, mode: MethodMode) -> Result<Arc<Session>> {
        if request.source.deployment() != Some(&self.caller) {
            return Err("UI source differs from the frozen connection approval".into());
        }
        let Interface::Service { methods } = &self.selection.description().contract.interface
        else {
            unreachable!()
        };
        if methods
            .get(&request.method)
            .is_none_or(|method| method.mode != mode)
        {
            return Err("method differs from the captured native service mode".into());
        }
        self.connection.session(self.selection.clone(), request)
    }
}
impl NativeService for Service {
    fn call(&self, request: ServiceRequest) -> ServiceFuture {
        let session = self.session(request, MethodMode::Query);
        Box::pin(async move {
            let session = match session {
                Ok(session) => session,
                Err(e) => return completion(Err(e), Ok(())),
            };
            let mut state = session.state.lock().await;
            let result = async {
                session.begin(&mut state).await?;
                let call = state.call.as_mut().unwrap();
                let reply = observe(call.receive(), &session.stop).await?;
                match reply {
                    Reply::Finished { result, .. } => result,
                    Reply::Aborted { reason, .. } => Err(reason),
                    _ => Err("unexpected native query reply".into()),
                }
            }
            .await;
            let settlement = session.finish(&mut state).await;
            let result = match settlement.terminal {
                Some(Reply::Finished { result, .. }) => result,
                _ => result,
            };
            completion(result, settlement.cleanup)
        })
    }
    fn subscribe(&self, request: ServiceRequest, stop: CancellationToken) -> StreamFuture {
        let session = self.session(request, MethodMode::Subscription);
        Box::pin(async move {
            let session = match session {
                Ok(session) => session,
                Err(e) => return StreamCompletion::Finished(completion(Err(e), Ok(()))),
            };
            let mut state = session.state.lock().await;
            let admitted = async {
                session.begin_with_stop(&mut state, &stop).await?;
                let reply =
                    observe_two(state.call.as_mut().unwrap().receive(), &session.stop, &stop)
                        .await?;
                match reply {
                    Reply::Subscribed => Ok(()),
                    Reply::Aborted { reason, .. } => Err(reason),
                    _ => Err("unexpected native subscription admission".into()),
                }
            }
            .await;
            if let Err(e) = admitted {
                let settlement = session.finish(&mut state).await;
                return StreamCompletion::Finished(completion(Err(e), settlement.cleanup));
            }
            drop(state);
            next(session, stop).await
        })
    }
    fn mutation(&self, request: ServiceRequest) -> Result<Arc<dyn NativeMutation>> {
        Ok(self.session(request, MethodMode::Mutation)?)
    }
}
struct State {
    request: Option<ServiceRequest>,
    call: Option<Invocation>,
    opening: bool,
    physical_failure: Option<String>,
}
struct Session {
    connection: Arc<Connection>,
    selection: Selection,
    stop: CancellationToken,
    state: tokio::sync::Mutex<State>,
    recorded: Mutex<Option<RecordedIntent>>,
    permit: Option<OwnedSemaphorePermit>,
}
impl Session {
    async fn begin(&self, state: &mut State) -> Result<()> {
        self.begin_with_stop(state, &self.stop).await
    }
    async fn begin_with_stop(&self, state: &mut State, stop: &CancellationToken) -> Result<()> {
        let source = Source::try_from(
            &state
                .request
                .as_ref()
                .ok_or("native request already started")?
                .source,
        )?;
        // Publish original opening custody before constructing the embedding's
        // future. Cancellation awaits that same future, never infers cleanup
        // from dropping it or from the absence of a published channel.
        state.opening = true;
        let opening_stop = self.stop.child_token();
        let mut opening = match std::panic::catch_unwind(AssertUnwindSafe(|| {
            self.connection.connector.connect(opening_stop.clone())
        })) {
            Ok(opening) => opening,
            Err(_) => return Err("native channel opening constructor panicked".into()),
        };
        // Keep the original future outside the poll unwind boundary: its own
        // destructor may panic independently. A returned Stream is likewise
        // retained outside the subsequent future-drop boundary.
        let result = AssertUnwindSafe(async {
            tokio::select! {
                biased;
                () = self.stop.cancelled() => {
                    opening_stop.cancel();
                    (&mut opening).await
                }
                () = stop.cancelled() => {
                    opening_stop.cancel();
                    (&mut opening).await
                }
                result = &mut opening => result,
            }
        })
        .catch_unwind()
        .await;
        // Even a completed embedding future may own physical custody in its
        // destructor. Publish neither a settled error nor a channel until that
        // destructor has completed under the opening fence.
        let dropped = std::panic::catch_unwind(AssertUnwindSafe(|| drop(opening)));
        let failed = result.is_err() || dropped.is_err();
        let stream = match result {
            Ok(Ok(stream)) => stream,
            Ok(Err(reason)) if !failed => {
                state.opening = false;
                return Err(reason);
            }
            _ => return Err("native channel opening cleanup panicked".into()),
        };
        let request = state.request.take().expect("original opening request");
        let (reader, writer) = tokio::io::split(stream);
        state.call = Some(Call::new(
            reader,
            writer,
            self.selection.clone(),
            source,
            request.method,
            request.input,
            request.resources,
        ));
        if failed {
            let reason = "native channel opening cleanup panicked".to_string();
            state.physical_failure = Some(reason.clone());
            self.connection.failure(reason.clone());
            // The returned physical stream now belongs to the original Call;
            // each half is destroyed under its independent unwind boundary.
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                state.call.as_mut().unwrap().disconnect();
            }));
            return Err(reason);
        }
        state.opening = false;
        observe_two(state.call.as_mut().unwrap().start(), &self.stop, stop).await
    }
    async fn finish(&self, state: &mut State) -> Settlement {
        if let Some(reason) = &state.physical_failure {
            return Settlement {
                terminal: None,
                cleanup: Err(reason.clone()),
            };
        }
        let Some(call) = state.call.as_mut() else {
            if state.opening {
                let reason = "original native channel opening remains unconfirmed".to_string();
                self.connection.failure(reason.clone());
                return Settlement {
                    terminal: None,
                    cleanup: Err(reason),
                };
            }
            let cleanup = self.release_inert(state);
            if let Err(reason) = &cleanup {
                state.physical_failure = Some(reason.clone());
                self.connection.failure(reason.clone());
            }
            return Settlement {
                terminal: None,
                cleanup,
            };
        };
        let (terminal, cleanup) = match call.drain().await {
            Ok(Some(reply)) => {
                let cleanup = reply.cleanup().expect("terminal reply").clone();
                (Some(reply), cleanup)
            }
            Ok(None) => (None, Ok(())),
            Err(reason) => (None, Err(reason)),
        };
        if std::panic::catch_unwind(AssertUnwindSafe(|| call.disconnect())).is_err() {
            let reason = "physical native channel cleanup panicked".to_string();
            state.physical_failure = Some(reason.clone());
            self.connection.failure(reason.clone());
            return Settlement {
                terminal,
                cleanup: Err(reason),
            };
        }
        if let Err(reason) = &cleanup {
            self.connection.failure(reason.clone());
        } else if let Err(reason) = self.release_inert(state) {
            state.physical_failure = Some(reason.clone());
            self.connection.failure(reason.clone());
            return Settlement {
                terminal,
                cleanup: Err(reason),
            };
        }
        Settlement { terminal, cleanup }
    }
    fn release_inert(&self, state: &mut State) -> Result<()> {
        let request = state.request.take();
        let recorded = self.recorded.lock().unwrap().take();
        let inputs = std::panic::catch_unwind(AssertUnwindSafe(|| drop(request)));
        let token = std::panic::catch_unwind(AssertUnwindSafe(|| drop(recorded)));
        if inputs.is_err() || token.is_err() {
            Err("native request custody cleanup panicked".into())
        } else {
            Ok(())
        }
    }
}
impl NativeMutation for Session {
    fn prepare(self: Arc<Self>) -> ReferenceFuture {
        Box::pin(async move {
            let mut state = self.state.lock().await;
            let result = async {
                self.begin(&mut state).await?;
                match observe(state.call.as_mut().unwrap().receive(), &self.stop).await? {
                    Reply::Prepared { reference } => Ok(*reference),
                    Reply::Aborted { reason, .. } => Err(reason),
                    _ => Err("unexpected native preparation reply".into()),
                }
            }
            .await;
            if result.is_err() {
                let _ = self.finish(&mut state).await;
            }
            result
        })
    }
    fn dispatch(self: Arc<Self>, recorded: RecordedIntent) -> MutationFuture {
        let reference = recorded.intent().reference.clone();
        let stored = {
            let mut slot = self.recorded.lock().unwrap();
            if slot.is_some() {
                false
            } else {
                *slot = Some(recorded);
                true
            }
        };
        Box::pin(async move {
            let mut state = self.state.lock().await;
            let result = async {
                if !stored {
                    return Err("native mutation already confirmed".into());
                }
                let call = state.call.as_mut().ok_or("native mutation not prepared")?;
                let recorded = self
                    .recorded
                    .lock()
                    .unwrap()
                    .take()
                    .ok_or("native preparation already closed")?;
                // Call takes the actual token synchronously before this observer
                // can be selected away by concurrent native close.
                match observe(call.commit(recorded), &self.stop).await? {
                    Reply::Mutation { reply, .. } => Ok(*reply),
                    Reply::Aborted { reason, .. } => Err(reason),
                    _ => Err("unexpected native mutation reply".into()),
                }
            }
            .await;
            let settlement = self.finish(&mut state).await;
            let result = match settlement.terminal {
                Some(Reply::Mutation { reply, .. }) => Ok(*reply),
                _ => result,
            };
            MutationCompletion {
                reply: result.unwrap_or_else(|diagnostic| OperationReply {
                    reference,
                    receipt: Receipt::Unavailable { diagnostic },
                }),
                cleanup: settlement.cleanup,
            }
        })
    }
    fn close(self: Arc<Self>) -> CleanupFuture {
        self.stop.cancel();
        Box::pin(async move {
            let mut state = self.state.lock().await;
            self.finish(&mut state).await.cleanup
        })
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let state = self.state.get_mut();
        let mut call = state.call.take();
        let physical = std::panic::catch_unwind(AssertUnwindSafe(|| {
            if let Some(call) = call.as_mut() {
                call.disconnect();
            }
        }));
        let mut failed = state.opening
            || state.physical_failure.is_some()
            || physical.is_err()
            || call.as_ref().is_some_and(|call| !call.cleanup_confirmed());
        if !failed {
            // Destructors own real resources and SQL leases. Complete them
            // before publishing the final permit; panic keeps the close fence.
            let inputs = std::panic::catch_unwind(AssertUnwindSafe(|| drop(state.request.take())));
            let token = std::panic::catch_unwind(AssertUnwindSafe(|| {
                drop(self.recorded.get_mut().unwrap().take());
            }));
            let original = std::panic::catch_unwind(AssertUnwindSafe(|| drop(call.take())));
            failed = inputs.is_err() || token.is_err() || original.is_err();
        }
        if failed {
            self.connection
                .failure("original native invocation was dropped without cleanup".into());
            self.connection.failed.lock().unwrap().custody.push(Failed {
                _call: call,
                _request: state.request.take(),
                _recorded: self.recorded.get_mut().unwrap().take(),
                _permit: self.permit.take().unwrap(),
            });
        }
        drop(self.permit.take());
        self.connection.changed.notify_waiters();
    }
}
fn completion(result: Result<serde_json::Value>, cleanup: Result<()>) -> ServiceCompletion {
    ServiceCompletion { result, cleanup }
}
async fn observe<T>(
    future: impl Future<Output = Result<T>>,
    stop: &CancellationToken,
) -> Result<T> {
    tokio::select! { biased; () = stop.cancelled() => Err("native call cancelled".into()), result = future => result }
}
fn next(session: Arc<Session>, stop: CancellationToken) -> StreamFuture {
    Box::pin(async move {
        let mut state = session.state.lock().await;
        let reply = if session.stop.is_cancelled() || stop.is_cancelled() {
            Err("native subscription cancelled".into())
        } else {
            observe_two(state.call.as_mut().unwrap().pull(), &session.stop, &stop).await
        };
        match reply {
            Ok(Reply::Item { value }) => {
                drop(state);
                StreamCompletion::Item {
                    value,
                    next: next(session, stop),
                }
            }
            result => {
                let result = match result {
                    Ok(Reply::Finished { result, .. }) => result,
                    Ok(Reply::Aborted { reason, .. }) => Err(reason),
                    Ok(_) => Err("unexpected native subscription reply".into()),
                    Err(e) => Err(e),
                };
                let settlement = session.finish(&mut state).await;
                let result = match settlement.terminal {
                    Some(Reply::Finished { result, .. }) => result,
                    _ => result,
                };
                StreamCompletion::Finished(completion(result, settlement.cleanup))
            }
        }
    })
}

struct Settlement {
    terminal: Option<Reply>,
    cleanup: Result<()>,
}
async fn observe_two<T>(
    future: impl Future<Output = Result<T>>,
    own: &CancellationToken,
    stop: &CancellationToken,
) -> Result<T> {
    tokio::select! { biased; () = own.cancelled() => Err("native call cancelled".into()), () = stop.cancelled() => Err("native subscription cancelled".into()), result = future => result }
}
