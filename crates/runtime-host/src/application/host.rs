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

//! Public current-account Host. One Application owns every accepted execution.
mod client;
mod protocol;
pub use client::{Client, LocalCall};
pub use protocol::{Status, VERSION};

use super::transport::{
    Catalogue, Endpoints, HostEndpoint, NativeGrant, Peer, PeerPolicy, Peers, Server, wire,
    with_gateway,
};
use super::{Application, NativeRegistry, Result};
use crate::ipc::{LocalListener, LocalStream};
use maka_event_log::EventLog;
use maka_plugin_core::{composition::InstanceId, deployment::DeploymentRef};
use protocol::{Connected, Opening, Request, Response, SessionControl};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Semaphore;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use uuid::Uuid;

const SESSIONS: usize = 8;
const CONNECTIONS: usize = 144;

/// Opened only from a checked, persisted source and trusted endpoint grants.
/// The caller retains this owner after any failed cleanup acknowledgement.
pub struct Host {
    application: Application,
    store: Arc<EventLog>,
    epoch: Uuid,
    source: DeploymentRef,
    policy: Arc<PeerPolicy>,
    endpoints: Endpoints,
    consumers: BTreeMap<String, InstanceId>,
    sessions: Mutex<BTreeMap<Uuid, Arc<Session>>>,
    stop: CancellationToken,
}
struct Session {
    peer: Arc<Peers>,
    transport: Arc<Server>,
}
impl Host {
    pub async fn open(
        store: Arc<EventLog>,
        deployment: Uuid,
        native: NativeRegistry,
        selections: BTreeMap<String, HostEndpoint>,
        grants: Vec<NativeGrant>,
    ) -> Result<Arc<Self>> {
        let policy = Arc::new(PeerPolicy::with_native_grants(grants)?);
        let application = Application::start_from_store(
            store.clone(),
            deployment,
            with_gateway(native)?,
            policy.issuer(),
            policy.clone(),
            Default::default(),
        )
        .await?;
        let endpoints = match Endpoints::start(&application.client(), &selections).await {
            Ok(endpoints) => endpoints,
            Err(error) => {
                application.shutdown().await?;
                return Err(error);
            }
        };
        Ok(Arc::new(Self {
            source: application.source().expect("persisted Host").clone(),
            application,
            store,
            policy,
            endpoints,
            epoch: Uuid::new_v4(),
            consumers: selections
                .into_iter()
                .map(|(name, endpoint)| (name, endpoint.consumer))
                .collect(),
            sessions: Mutex::default(),
            stop: CancellationToken::new(),
        }))
    }
    pub fn epoch(&self) -> Uuid {
        self.epoch
    }
    pub fn source(&self) -> &DeploymentRef {
        &self.source
    }
    pub fn status(&self) -> Status {
        let snapshot = self.application.client().snapshot();
        Status {
            epoch: self.epoch,
            source: self.source.clone(),
            closing: self.stop.is_cancelled() || snapshot.closing,
            closed: snapshot.closed,
            cleanup_error: snapshot.cleanup_error,
            sessions: self.sessions.lock().unwrap().len(),
            active_calls: snapshot
                .instances
                .iter()
                .map(|instance| instance.calls)
                .sum(),
        }
    }
    pub fn wait_for_drain(&self) -> impl Future<Output = ()> + Send + 'static {
        self.stop.clone().cancelled_owned()
    }
    /// Current-account IPC is enforced by LocalListener's private directory/ACL
    /// and the held Unix peer credentials. No request chooses its own principal.
    pub async fn serve_local(
        self: &Arc<Self>,
        mut listener: LocalListener,
        cancel: CancellationToken,
    ) -> Result<()> {
        let tasks = TaskTracker::new();
        let capacity = Arc::new(Semaphore::new(CONNECTIONS));
        let accepted = loop {
            let stream = tokio::select! {
                biased;
                () = self.stop.cancelled() => break Ok(()),
                () = cancel.cancelled() => break Ok(()),
                value = listener.accept() => match value {
                    Ok(stream) => stream,
                    Err(error) => break Err(error.to_string()),
                },
            };
            let Ok(permit) = capacity.clone().try_acquire_owned() else {
                drop(stream);
                continue;
            };
            let host = self.clone();
            tasks.spawn(async move {
                let _permit = permit;
                // A malformed or disconnected peer fails only its own channel.
                let _ = host.connection(stream).await;
            });
        };
        self.stop.cancel();
        tasks.close();
        tasks.wait().await;
        self.shutdown().await?;
        accepted
    }
    async fn connection(self: &Arc<Self>, stream: LocalStream) -> Result<()> {
        #[cfg(unix)]
        if stream.peer_cred().map_err(|error| error.to_string())?.uid()
            != unsafe { libc::geteuid() }
        {
            return Err("Host peer must belong to the current account".into());
        }
        let (input, mut output) = tokio::io::split(stream);
        let mut input = wire::Reader::new(input);
        let opening: Opening = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err("Host is closing".into()),
            value = tokio::time::timeout(Duration::from_secs(10), input.read_bounded(4096)) => value.map_err(|_| "Host handshake timed out")??,
        };
        if opening.version != VERSION || opening.epoch.is_some_and(|epoch| epoch != self.epoch) {
            return Err("Host protocol or incarnation changed".into());
        }
        match opening.request {
            Request::Status => wire::write(&mut output, &Response::Status(self.status())).await,
            Request::Retire => {
                // No connection count or RPC completion substitutes for native
                // producer fences and actual accepted background work.
                let result = self.application.shutdown_if_idle().await;
                if matches!(result, Ok(true)) {
                    self.stop.cancel();
                }
                wire::write(&mut output, &Response::Retired(result)).await
            }
            Request::Connect => {
                let id = Uuid::new_v4();
                let session = {
                    let mut sessions = self.sessions.lock().unwrap();
                    if self.stop.is_cancelled() || sessions.len() >= SESSIONS {
                        return Err("Host session capacity exhausted or Host is closing".into());
                    }
                    let session = Arc::new(Session {
                        peer: Peers::new(Peer::new(
                            self.source.clone(),
                            self.consumers.values().cloned().collect(),
                        )?),
                        transport: Server::new(
                            self.application.client(),
                            self.policy.clone(),
                            self.endpoints.clone(),
                        )?,
                    });
                    sessions.insert(id, session.clone());
                    session
                };
                let result = async {
                    let connected = Connected {
                        version: VERSION, epoch: self.epoch, session: id, source: self.source.clone(),
                        consumers: self.consumers.clone(), catalogue: Catalogue::captured(&self.endpoints),
                    };
                    let response = Response::Connected(connected);
                    tokio::select! {
                        biased;
                        () = self.stop.cancelled() => return Err("Host is closing".into()),
                        result = wire::write(&mut output, &response) => result?,
                    }
                    tokio::select! {
                        biased;
                        () = self.stop.cancelled() => Ok(()),
                        result = input.read::<SessionControl>() => result.map(|SessionControl::Close| ()),
                    }
                }.await;
                // Revocation fences late admissions, then the same transport
                // drains all original calls before releasing the session.
                session.peer.replace(vec![])?;
                session.transport.close().await?;
                self.sessions.lock().unwrap().remove(&id);
                if result.is_ok() {
                    wire::write(&mut output, &Response::Closed).await?;
                }
                result
            }
            Request::Call { session } => {
                let session = self
                    .sessions
                    .lock()
                    .unwrap()
                    .get(&session)
                    .cloned()
                    .ok_or("Host session is unavailable")?;
                wire::write(&mut output, &Response::Accepted).await?;
                // Carry buffered bytes into the existing bounded transport.
                session
                    .transport
                    .serve_owned(input.inner, output, session.peer.clone())?
                    .await
            }
        }
    }
    /// Physical transport cleanup precedes Application cleanup and SQL drain.
    /// Errors retain this Host and its original Root for the embedding to hold.
    pub async fn shutdown(&self) -> Result<()> {
        self.stop.cancel();
        let sessions: Vec<_> = self.sessions.lock().unwrap().values().cloned().collect();
        for session in sessions {
            session.peer.replace(vec![])?;
            session.transport.close().await?;
        }
        self.application.shutdown().await?;
        self.store.drain().await;
        self.sessions.lock().unwrap().clear();
        Ok(())
    }
}
