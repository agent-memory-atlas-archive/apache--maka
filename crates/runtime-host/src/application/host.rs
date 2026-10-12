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

//! Authenticated public Host. One Application owns every accepted execution.
mod access;
mod client;
pub mod discovery;
mod lifecycle;
pub mod websocket;
pub use access::Pairing;
pub mod protocol;
pub use client::{Client, LocalCall};
pub use discovery::Registration;
pub use protocol::{LifecycleMode, Status, VERSION};

use super::transport::{
    Catalogue, Endpoints, HostEndpoint, NativeGrant, Peer, PeerPolicy, Peers, Server, wire,
    with_gateway,
};
use super::{Application, NativeRegistry, Result};
use crate::ipc::{LocalListener, LocalStream};
use access::{Authority, Principal};
use maka_event_log::EventLog;
use maka_plugin_core::{composition::InstanceId, deployment::DeploymentRef};
use protocol::{Connected, Opening, Request, Response, SessionControl};
use sha2::Digest;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Semaphore;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use uuid::Uuid;
use websocket::WebSocketListener;

const SESSIONS: usize = 8;
const CONNECTIONS: usize = 144;

#[derive(Clone)]
pub struct Options {
    pub generation: Option<String>,
    pub lifecycle_mode: LifecycleMode,
    pub handshake_timeout: Duration,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            generation: None,
            lifecycle_mode: LifecycleMode::Service,
            handshake_timeout: Duration::from_secs(10),
        }
    }
}

/// Opened only from a checked, persisted source and trusted endpoint grants.
/// The caller retains this owner after any failed cleanup acknowledgement.
pub struct Host {
    root: Arc<maka_event_log::root::RootOwner>,
    options: Options,
    application: Application,
    store: Arc<EventLog>,
    access: Arc<maka_config::ConfigurationStore>,
    access_grant: String,
    access_gate: tokio::sync::Mutex<()>,
    epoch: Uuid,
    source: DeploymentRef,
    policy: Arc<PeerPolicy>,
    endpoints: Endpoints,
    consumers: BTreeMap<String, InstanceId>,
    sessions: Mutex<BTreeMap<Uuid, Arc<Session>>>,
    stop: CancellationToken,
    residency: Mutex<lifecycle::Residency>,
    capacity: Arc<Semaphore>,
}
struct Session {
    principal: Principal,
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
        Self::open_with_options(
            store,
            deployment,
            native,
            selections,
            grants,
            Options::default(),
        )
        .await
    }
    pub async fn open_with_options(
        store: Arc<EventLog>,
        deployment: Uuid,
        native: NativeRegistry,
        selections: BTreeMap<String, HostEndpoint>,
        grants: Vec<NativeGrant>,
        options: Options,
    ) -> Result<Arc<Self>> {
        if options.handshake_timeout.is_zero()
            || options.handshake_timeout > Duration::from_secs(300)
            || options
                .generation
                .as_ref()
                .is_some_and(|value| value.is_empty() || value.len() > 512)
        {
            return Err("invalid Host options".into());
        }
        let root = store.owned_root().map_err(|error| error.to_string())?;
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
        let access = match maka_config::ConfigurationStore::for_root(root.clone()).await {
            Ok(access) => Arc::new(access),
            Err(error) => {
                application.shutdown().await?;
                return Err(error.to_string());
            }
        };
        let access_grant = format!(
            "application.owner:{:x}",
            sha2::Sha256::digest(
                serde_json::to_vec(&(application.source(), &selections))
                    .map_err(|error| error.to_string())?
            )
        );
        Ok(Arc::new(Self {
            root,
            options,
            source: application.source().expect("persisted Host").clone(),
            application,
            store,
            access,
            access_grant,
            access_gate: tokio::sync::Mutex::new(()),
            policy,
            endpoints,
            epoch: Uuid::new_v4(),
            consumers: selections
                .into_iter()
                .map(|(name, endpoint)| (name, endpoint.consumer))
                .collect(),
            sessions: Mutex::default(),
            stop: CancellationToken::new(),
            residency: Mutex::new(lifecycle::Residency::default()),
            capacity: Arc::new(Semaphore::new(CONNECTIONS)),
        }))
    }
    pub fn root_id(&self) -> &str {
        self.root.root_id()
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
            root_id: self.root_id().to_owned(),
            version: VERSION,
            pid: std::num::NonZeroU32::new(std::process::id()).expect("process id"),
            generation: self.options.generation.clone(),
            lifecycle_mode: self.options.lifecycle_mode,
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
        let stop = self.stop.clone();
        let mut updates = self.application.client().updates;
        async move {
            loop {
                if updates.borrow().closing {
                    return;
                }
                tokio::select! {
                    () = stop.cancelled() => return,
                    result = updates.changed() => if result.is_err() { return; },
                }
            }
        }
    }
    /// Current-account IPC is enforced by LocalListener's private directory/ACL
    /// and the held Unix peer credentials. No request chooses its own principal.
    pub async fn serve_local(
        self: &Arc<Self>,
        listener: LocalListener,
        cancel: CancellationToken,
    ) -> Result<()> {
        self.serve(listener, None, cancel).await
    }
    pub async fn serve(
        self: &Arc<Self>,
        mut listener: LocalListener,
        websocket: Option<WebSocketListener>,
        cancel: CancellationToken,
    ) -> Result<()> {
        enum Incoming {
            Local(LocalStream),
            Remote(tokio::net::TcpStream),
        }
        let tasks = TaskTracker::new();
        let accepted = loop {
            let stream = tokio::select! {
                biased;
                () = self.stop.cancelled() => break Ok(()),
                () = cancel.cancelled() => break Ok(()),
                value = listener.accept() => match value {
                    Ok(stream) => Incoming::Local(stream),
                    Err(error) => break Err(error.to_string()),
                },
                value = async {
                    match &websocket {
                        Some(listener) => listener.listener.accept().await,
                        None => std::future::pending().await,
                    }
                } => match value {
                    Ok((stream, _)) => Incoming::Remote(stream),
                    Err(error) => break Err(error.to_string()),
                },
            };
            let connection = {
                let mut residency = self.residency.lock().unwrap();
                if residency.paused || self.stop.is_cancelled() {
                    continue;
                }
                let Ok(permit) = self.capacity.clone().try_acquire_owned() else {
                    continue;
                };
                residency.last_activity = tokio::time::Instant::now();
                lifecycle::Connection {
                    host: self.clone(),
                    permit: Some(permit),
                }
            };
            let host = self.clone();
            let origins = websocket
                .as_ref()
                .map(|listener| listener.allowed_origins.clone());
            tasks.spawn(async move {
                let _connection = connection;
                // A malformed or disconnected peer fails only its own channel.
                match stream {
                    Incoming::Local(stream) => {
                        let _ = host.connection(stream).await;
                    }
                    Incoming::Remote(stream) => {
                        let _ = websocket::connection(
                            stream,
                            host,
                            origins.expect("WebSocket listener"),
                        )
                        .await;
                    }
                }
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
        self.channel(stream, Authority::Local).await
    }
    async fn channel<S>(self: &Arc<Self>, stream: S, authority: Authority) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let (input, mut output) = tokio::io::split(stream);
        let mut input = wire::Reader::new(input);
        let opening: Opening = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err("Host is closing".into()),
            value = tokio::time::timeout(self.options.handshake_timeout, input.read_bounded(4096)) => value.map_err(|_| "Host handshake timed out")??,
        };
        if opening.version != VERSION || opening.epoch.is_some_and(|epoch| epoch != self.epoch) {
            return Err("Host protocol or incarnation changed".into());
        }
        match opening.request {
            Request::Status => {
                let gate = self.access_gate.lock().await;
                self.identify(&authority, opening.client_instance.as_deref())
                    .await?;
                drop(gate);
                wire::write(&mut output, &Response::Status(self.status())).await
            }
            Request::PrepareAccess { principal } => {
                authority.local()?;
                wire::write(
                    &mut output,
                    &Response::Pairing(self.prepare_access(principal).await),
                )
                .await
            }
            Request::RevokeAccess { credential } => {
                authority.local()?;
                wire::write(
                    &mut output,
                    &Response::Revoked(self.revoke_access(credential).await),
                )
                .await
            }
            Request::FinalizeAccess => {
                wire::write(
                    &mut output,
                    &Response::Finalized(
                        self.finalize_access(&authority, opening.client_instance)
                            .await,
                    ),
                )
                .await
            }
            Request::Retire {
                interrupt,
                allow_idle_connections,
            } => {
                authority.local()?;
                let result = self.retire(interrupt, allow_idle_connections).await;
                wire::write(&mut output, &Response::Retired(result)).await
            }
            Request::Connect => {
                let gate = self.access_gate.lock().await;
                let principal = self
                    .identify(&authority, opening.client_instance.as_deref())
                    .await?;
                let id = Uuid::new_v4();
                let session = {
                    let mut sessions = self.sessions.lock().unwrap();
                    if self.stop.is_cancelled() || sessions.len() >= SESSIONS {
                        return Err("Host session capacity exhausted or Host is closing".into());
                    }
                    let session = Arc::new(Session {
                        principal,
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
                    let mut residency = self.residency.lock().unwrap();
                    if residency.paused {
                        return Err("Host admission is paused".into());
                    }
                    residency.admitted = true;
                    sessions.insert(id, session.clone());
                    session
                };
                drop(gate);
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
                        () = session.transport.wait_for_close() => Ok(()),
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
                let gate = self.access_gate.lock().await;
                if self.stop.is_cancelled() || self.residency.lock().unwrap().paused {
                    return Err("Host admission is closed".into());
                }
                let principal = self
                    .identify(&authority, opening.client_instance.as_deref())
                    .await?;
                let session = self
                    .sessions
                    .lock()
                    .unwrap()
                    .get(&session)
                    .cloned()
                    .ok_or("Host session is unavailable")?;
                if session.principal != principal {
                    return Err("Host session belongs to another authenticated principal".into());
                }
                drop(gate);
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
        self.access
            .shutdown()
            .await
            .map_err(|error| error.to_string())?;
        self.store.drain().await;
        self.sessions.lock().unwrap().clear();
        Ok(())
    }
}
