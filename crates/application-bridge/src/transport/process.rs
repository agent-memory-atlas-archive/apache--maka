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

//! Shared Host process bootstrap and actual lifecycle acknowledgements.
use super::{Catalogue, Endpoint, Peer, PeerPolicy, Server, wire};
use crate::{Grant, Result};
use maka_event_log::{
    EventLog,
    root::{RootNamespaces, RootOwner},
};
use maka_plugin_core::{composition::InstanceId, deployment::DeploymentRef};
use maka_runtime_host::{
    application::{Application, NativeRegistry},
    ipc::LocalListener,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};
use tokio::io::{AsyncRead, AsyncWrite};
use uuid::Uuid;
const BOOTSTRAP_LIMIT: usize = 512 * 1024;

/// Trusted parent input on its selected child's inherited stdin, never an IPC
/// request or plugin-created grant. The child owns exactly one original Host.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub root: PathBuf,
    pub deployment: Uuid,
    pub namespaces: RootNamespaces,
    pub source: DeploymentRef,
    pub consumers: BTreeSet<InstanceId>,
    pub consumer: InstanceId,
    pub imports: BTreeMap<String, Grant>,
    pub address: PathBuf,
    pub nonce: Uuid,
}
impl Bootstrap {
    pub fn validate(&self) -> Result<()> {
        Peer::new(self.source.clone(), self.consumers.clone())?;
        if self.deployment.is_nil()
            || self.nonce.is_nil()
            || self.imports.is_empty()
            || self.imports.len() > 128
            || self.consumer.0.is_empty()
            || self.consumer.0.len() > 64
            || self.consumer.0.iter().any(Uuid::is_nil)
            || self.imports.iter().any(|(name, g)| {
                name.is_empty()
                    || name.len() > 256
                    || g.methods.is_empty()
                    || g.methods.len() > 64
                    || g.methods.iter().any(|m| m.is_empty() || m.len() > 256)
            })
        {
            return Err("invalid Host bootstrap".into());
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Event {
    Ready { nonce: Uuid, catalogue: Catalogue },
    Closed { nonce: Uuid },
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Close { nonce: Uuid },
}
#[derive(PartialEq)]
enum Phase {
    Inert,
    Started,
    Closing,
    Closed,
    Lost,
}
/// Own alongside the actual Child handle. Closed only proves this same child's
/// aggregate cleanup; normal process exit is a separate required observation.
/// Neither observation settles an old Lost Call or its actual caller token.
pub struct Control<R, W> {
    reader: wire::Reader<R>,
    writer: Option<W>,
    nonce: Uuid,
    phase: Phase,
}
impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Control<R, W> {
    pub fn new(reader: R, writer: W, nonce: Uuid) -> Result<Self> {
        if nonce.is_nil() {
            return Err("Host control nonce must be nonnil".into());
        }
        Ok(Self {
            reader: wire::Reader::new(reader),
            writer: Some(writer),
            nonce,
            phase: Phase::Inert,
        })
    }
    pub async fn start(&mut self, bootstrap: &Bootstrap) -> Result<Catalogue> {
        if self.phase != Phase::Inert || bootstrap.nonce != self.nonce {
            return Err("Host control cannot bootstrap in this phase".into());
        }
        bootstrap.validate()?;
        self.phase = Phase::Lost;
        wire::write_bounded(self.writer.as_mut().unwrap(), bootstrap, BOOTSTRAP_LIMIT).await?;
        match self.reader.read().await? {
            Event::Ready { nonce, catalogue } if nonce == self.nonce => {
                catalogue.validate()?;
                self.phase = Phase::Started;
                Ok(catalogue)
            }
            Event::Closed { nonce } if nonce == self.nonce => {
                self.phase = Phase::Closed;
                Err("Host closed before publishing its original registrations".into())
            }
            _ => Err("Host control did not publish its original registrations".into()),
        }
    }
    pub async fn close(&mut self) -> Result<()> {
        if self.phase == Phase::Closed {
            return Ok(());
        }
        if self.phase == Phase::Started {
            self.phase = Phase::Lost;
            if wire::write(
                self.writer.as_mut().unwrap(),
                &Command::Close { nonce: self.nonce },
            )
            .await
            .is_ok()
            {
                self.phase = Phase::Closing;
            } else {
                // Preserve this same child's possible buffered acknowledgement.
                // Never append a command after an uncertain prefix.
                drop(self.writer.take());
            }
        } else if self.phase != Phase::Closing {
            // A partial control write cannot append another command. EOF asks
            // the same child to drain; retain its stdout to observe real Closed.
            drop(self.writer.take());
        }
        loop {
            match self.reader.read().await? {
                Event::Closed { nonce } if nonce == self.nonce => {
                    self.phase = Phase::Closed;
                    return Ok(());
                }
                Event::Ready { nonce, .. } if nonce == self.nonce && self.phase == Phase::Lost => {}
                _ => {
                    self.phase = Phase::Lost;
                    return Err(
                        "Host lifecycle acknowledgement changed its original incarnation".into(),
                    );
                }
            }
        }
    }
}
/// Process entrypoint. Availability is supplied by this product's compiled
/// binary; persisted Host composition selects all executed instances.
pub async fn serve(native: NativeRegistry) -> Result<()> {
    let native = crate::with_gateway(native)?;
    let crate::stdio::Stdio {
        input,
        mut output,
        output_done,
    } = crate::stdio::Stdio::open().map_err(|e| e.to_string())?;
    let mut input = wire::Reader::new(input);
    let bootstrap: Bootstrap = input.read_bounded(BOOTSTRAP_LIMIT).await?;
    bootstrap.validate()?;
    let peer = Peer::new(bootstrap.source.clone(), bootstrap.consumers.clone())?;
    let mut listener = LocalListener::bind(&bootstrap.address).map_err(|e| e.to_string())?;
    let owner = Arc::new(
        RootOwner::open(&bootstrap.root, &bootstrap.namespaces).map_err(|e| e.to_string())?,
    );
    let store = Arc::new(EventLog::for_root(owner).await.map_err(|e| e.to_string())?);
    let policy = Arc::new(PeerPolicy::default());
    let application = Application::start_from_store(
        store.clone(),
        bootstrap.deployment,
        native,
        policy.issuer(),
        policy.clone(),
        Default::default(),
    )
    .await;
    let result = match application {
        Ok(application) => {
            let result = async {
                application.ready().await?;
                let client = application.client();
                let consumer = client.activation(&bootstrap.consumer)?;
                let mut endpoints = BTreeMap::new();
                for (name, grant) in bootstrap.imports {
                    let mut bindings = client.bind(consumer.clone(), name.clone()).await?;
                    if bindings.len() != 1 {
                        return Err("Host import requires one explicitly selected provider".to_string());
                    }
                    endpoints.insert(name, Endpoint::new(bindings.pop().unwrap(), grant)?);
                }
                let server = Server::new(client, policy, endpoints)?;
                let ready = Event::Ready {
                    nonce: bootstrap.nonce,
                    catalogue: server.catalogue(),
                };
                let published = wire::write(&mut output, &ready).await;
                let mut control_error = None;
                if published.is_ok() {
                    loop {
                        tokio::select! {
                            biased;
                            command = input.read_bounded::<Command>(512) => {
                                // Only this private supervisor stream controls lifecycle.
                                // EOF or malformed/foreign control asks actual drain;
                                // neither alone is a cleanup acknowledgement.
                                if matches!(command, Ok(Command::Close { nonce }) if nonce != bootstrap.nonce) {
                                    control_error = Some("Host close named a different incarnation".to_string());
                                }
                                break;
                            }
                            accepted = listener.accept() => {
                                let stream = match accepted {
                                    Ok(stream) => stream,
                                    Err(reason) => {
                                        control_error = Some(format!("Host IPC accept failed: {:?}", reason.kind()));
                                        break;
                                    }
                                };
                                let (reader, writer) = tokio::io::split(stream);
                                // Hello belongs to original wire capacity/tracked custody.
                                // Dropping this observer never drops the actual work.
                                if let Ok(observer) = server.serve_private(reader, writer, peer.clone(), bootstrap.nonce) {
                                    drop(observer);
                                }
                            }
                        }
                    }
                }
                // Revoke acceptance before waiting for actual invocation cleanup.
                drop(listener);
                drain_server(&server).await;
                drop(server);
                published?;
                control_error.map_or(Ok(()), Err)
            }.await;
            drain_application(&application).await;
            drop(application);
            result
        }
        Err(reason) => {
            // No Application owner was returned. Revoke physical acceptance and
            // still observe the original SQL close before publishing Closed.
            drop(listener);
            Err(reason)
        }
    };
    crate::drain_store(&store).await;
    drop(store);
    // Root, SQL and all accepted scopes have actually closed before this frame.
    wire::write(
        &mut output,
        &Event::Closed {
            nonce: bootstrap.nonce,
        },
    )
    .await?;
    drop(output);
    tokio::time::timeout(std::time::Duration::from_secs(5), output_done)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    result
}
async fn drain_server(server: &Server) {
    let mut reported = BTreeSet::new();
    loop {
        match server.close().await {
            Ok(()) => return,
            Err(reason) => {
                if reported.insert(reason.clone()) {
                    eprintln!("Host physical cleanup unconfirmed: {reason:?}");
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
async fn drain_application(application: &Application) {
    let mut reported = BTreeSet::new();
    loop {
        match application.shutdown().await {
            Ok(()) => return,
            Err(reason) => {
                if reported.insert(reason.clone()) {
                    eprintln!("Host Application cleanup unconfirmed: {reason:?}");
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
