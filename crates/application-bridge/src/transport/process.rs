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
use super::{Catalogue, Endpoints, Peer, PeerPolicy, Server, wire};
use crate::{HostEndpoint, Result};
use maka_event_log::{
    EventLog,
    root::{RootNamespaces, RootOwner},
};
use maka_plugin_core::{composition::InstanceId, deployment::DeploymentRef};
use maka_runtime_host::{
    application::{Application, Client, NativeRegistry},
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
    #[serde(default)]
    pub state_domain: Option<String>,
    pub deployment: Uuid,
    pub namespaces: RootNamespaces,
    pub source: DeploymentRef,
    pub consumers: BTreeSet<InstanceId>,
    pub endpoints: BTreeMap<String, HostEndpoint>,
    pub native: Vec<super::NativeGrant>,
    pub address: PathBuf,
    pub nonce: Uuid,
}
impl Bootstrap {
    pub fn validate(&self) -> Result<()> {
        Peer::new(self.source.clone(), self.consumers.clone())?;
        if self
            .state_domain
            .as_deref()
            .is_some_and(|domain| !crate::connections::valid_domain(domain))
        {
            return Err("invalid Host State Root identity".into());
        }
        if self.deployment.is_nil()
            || self.nonce.is_nil()
            || self.endpoints.is_empty()
            || self.endpoints.len() > 128
            || self
                .endpoints
                .keys()
                .any(|name| name.is_empty() || name.len() > 256)
        {
            return Err("invalid Host bootstrap".into());
        }
        for endpoint in self.endpoints.values() {
            endpoint.validate()?;
        }
        if self.native.len() > 128 {
            return Err("maximum 128 native service grants".into());
        }
        for grant in &self.native {
            grant.validate()?;
        }
        // The parent checks the actual wire bound before spawning any child.
        // An oversized bootstrap must not leave a child waiting on partial IO.
        wire::encode_bounded(self, BOOTSTRAP_LIMIT)?;
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Event {
    Ready {
        nonce: Uuid,
        catalogue: Catalogue,
    },
    Closed {
        nonce: Uuid,
    },
    Captured {
        nonce: Uuid,
        result: Result<Catalogue>,
    },
    Approved {
        nonce: Uuid,
        source: DeploymentRef,
        result: Result<()>,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Close {
        nonce: Uuid,
    },
    Capture {
        nonce: Uuid,
    },
    Approve {
        nonce: Uuid,
        source: DeploymentRef,
        consumers: BTreeSet<InstanceId>,
        retain: BTreeSet<u64>,
        captured: Option<Catalogue>,
    },
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
    /// Capture actual registrations without publishing endpoints or approvals.
    pub async fn capture(&mut self) -> Result<Catalogue> {
        if self.phase != Phase::Started {
            return Err("Host control is not ready for capture".into());
        }
        self.phase = Phase::Lost;
        wire::write(
            self.writer.as_mut().unwrap(),
            &Command::Capture { nonce: self.nonce },
        )
        .await?;
        match self.reader.read().await? {
            Event::Captured { nonce, result } if nonce == self.nonce => {
                self.phase = Phase::Started;
                let catalogue = result?;
                catalogue.validate()?;
                Ok(catalogue)
            }
            _ => Err("Host capture acknowledgement differs from its original incarnation".into()),
        }
    }
    pub(crate) fn check_approval(
        &self,
        source: &DeploymentRef,
        consumers: &BTreeSet<InstanceId>,
        retain: &BTreeSet<u64>,
        captured: &Option<Catalogue>,
    ) -> Result<()> {
        if self.phase != Phase::Started {
            return Err("Host control is not ready for approval".into());
        }
        source.validate()?;
        if !consumers.is_empty() {
            Peer::new(source.clone(), consumers.clone())?;
        }
        if consumers.is_empty() != captured.is_none() {
            return Err("approval requires its captured endpoints".into());
        }
        if retain.len() > 256 {
            return Err("maximum 256 retained source approvals".into());
        }
        wire::encode_bounded(
            &Command::Approve {
                nonce: self.nonce,
                source: source.clone(),
                consumers: consumers.clone(),
                retain: retain.clone(),
                captured: captured.clone(),
            },
            BOOTSTRAP_LIMIT,
        )?;
        Ok(())
    }
    /// Publish the previously captured endpoints and exact caller approval.
    /// An empty consumer set performs only retention, without rebinding endpoints.
    pub async fn approve(
        &mut self,
        source: DeploymentRef,
        consumers: BTreeSet<InstanceId>,
        retain: BTreeSet<u64>,
        captured: Option<Catalogue>,
    ) -> Result<()> {
        self.check_approval(&source, &consumers, &retain, &captured)?;
        let command = Command::Approve {
            nonce: self.nonce,
            source: source.clone(),
            consumers,
            retain,
            captured,
        };
        wire::encode_bounded(&command, BOOTSTRAP_LIMIT)?;
        // An abandoned/partial exchange cannot append another control frame.
        self.phase = Phase::Lost;
        wire::write_bounded(self.writer.as_mut().unwrap(), &command, BOOTSTRAP_LIMIT).await?;
        match self.reader.read().await? {
            Event::Approved {
                nonce,
                source: approved,
                result,
            } if nonce == self.nonce && approved == source => {
                self.phase = Phase::Started;
                result
            }
            _ => Err("Host approval acknowledgement differs from its original request".into()),
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
            let event = match self.reader.read().await {
                Ok(event) => event,
                Err(error) => {
                    self.phase = Phase::Lost;
                    drop(self.writer.take());
                    return Err(error);
                }
            };
            match event {
                Event::Closed { nonce } if nonce == self.nonce => {
                    self.phase = Phase::Closed;
                    return Ok(());
                }
                Event::Ready { nonce, .. }
                | Event::Captured { nonce, .. }
                | Event::Approved { nonce, .. }
                    if nonce == self.nonce && self.phase == Phase::Lost => {}
                _ => {
                    self.phase = Phase::Lost;
                    drop(self.writer.take());
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
    let peer = super::Peers::new(Peer::new(
        bootstrap.source.clone(),
        bootstrap.consumers.clone(),
    )?);
    let initialized = async {
        let listener = LocalListener::bind(&bootstrap.address).map_err(|e| e.to_string())?;
        let owner = Arc::new(
            RootOwner::open(&bootstrap.root, &bootstrap.namespaces).map_err(|e| e.to_string())?,
        );
        if bootstrap
            .state_domain
            .as_deref()
            .is_some_and(|domain| owner.root_id() != domain)
        {
            return Err("original Host State Root identity changed".into());
        }
        let store = match EventLog::for_root(owner).await {
            Ok(store) => Arc::new(store),
            Err(error @ maka_event_log::StoreError::StartupCleanupFailed { .. }) => {
                // The original SQL worker still holds its actual authority.
                // Neither process exit nor an aggregate Closed can settle it.
                eprintln!("Host SQL startup cleanup unconfirmed: {error}");
                std::future::pending::<()>().await;
                unreachable!();
            }
            Err(error) => return Err(error.to_string()),
        };
        maka_runtime_host::recovery::recover(&store)
            .await
            .map_err(|error| error.to_string())?;
        Ok::<_, String>((listener, store))
    }
    .await;
    let (mut listener, store) = match initialized {
        Ok(initialized) => initialized,
        Err(error) => {
            // No Application was admitted. Every successfully acquired SQL
            // owner has confirmed its real close before reaching this branch.
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
            return Err(error);
        }
    };
    let policy = Arc::new(PeerPolicy::with_native_grants(bootstrap.native)?);
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
                let client = application.client();
                let endpoints = start_endpoints(&client, &bootstrap.endpoints).await?;
                let server = Server::new(client.clone(), policy, endpoints)?;
                let ready = Event::Ready {
                    nonce: bootstrap.nonce,
                    catalogue: server.catalogue(),
                };
                let published = wire::write(&mut output, &ready).await;
                let mut control_error = None;
                let mut captured = None;
                if published.is_ok() {
                    loop {
                        tokio::select! {
                            biased;
                            command = input.read_bounded::<Command>(BOOTSTRAP_LIMIT) => {
                                match command {
                                    Ok(Command::Capture { nonce }) if nonce == bootstrap.nonce => {
                                        let result = capture_endpoints(&client, &bootstrap.endpoints).await;
                                        let result = match result {
                                            Ok(endpoints) => {
                                                let catalogue = Catalogue::captured(&endpoints);
                                                captured = Some(endpoints);
                                                Ok(catalogue)
                                            }
                                            Err(error) => { captured = None; Err(error) }
                                        };
                                        let reply = capture_reply(nonce, result);
                                        if matches!(&reply, Event::Captured { result: Err(_), .. }) { captured = None; }
                                        if let Err(error) = wire::write(&mut output, &reply).await {
                                            control_error = Some(error); break;
                                        }
                                    }
                                    Ok(Command::Approve { nonce, source, consumers, retain, captured: expected }) if nonce == bootstrap.nonce => {
                                        let result = (|| {
                                            source.validate()?;
                                            if source.state_domain != bootstrap.source.state_domain || source.deployment != bootstrap.source.deployment {
                                                return Err("foreign approval deployment".into());
                                            }
                                            let approved = if consumers.is_empty() { None } else { Some(Peer::new(source.clone(), consumers)?) };
                                            if approved.is_none() != expected.is_none() { return Err("approval requires its captured endpoints".into()); }
                                            peer.check_update(approved.clone(), &retain)?;
                                            if let Some(expected) = expected {
                                                let endpoints = captured.take().ok_or("Host endpoints were not captured")?;
                                                if Catalogue::captured(&endpoints) != expected { return Err("Host captured endpoints differ".into()); }
                                                // Publish original Bindings, never rebind an alias after preflight.
                                                server.replace_endpoints(endpoints)?;
                                            }
                                            peer.update(approved, &retain)
                                        })();
                                        if let Err(error) = wire::write(&mut output, &Event::Approved { nonce, source, result }).await {
                                            control_error = Some(error); break;
                                        }
                                    }
                                    Ok(Command::Close { nonce }) if nonce == bootstrap.nonce => break,
                                    _ => { control_error = Some("Host control ended or named a foreign incarnation".into()); break; }
                                }
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
    store.drain().await;
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
fn capture_reply(nonce: Uuid, result: Result<Catalogue>) -> Event {
    let event = Event::Captured { nonce, result };
    match wire::encode_bounded(&event, wire::FRAME_LIMIT) {
        Ok(_) => event,
        // Known serialization bounds fail before IO; they must not close the Host.
        Err(_) => Event::Captured {
            nonce,
            result: Err("Host captured catalogue exceeds control frame limit".into()),
        },
    }
}
pub(crate) async fn start_endpoints(
    client: &Client,
    selections: &BTreeMap<String, HostEndpoint>,
) -> Result<Endpoints> {
    // Reject malformed selectors and grants before waiting for initialization.
    // These observations own no jobs; all initializers belong to Application.
    drop(capture_endpoints(client, selections).await?);
    for consumer in selections
        .values()
        .map(|endpoint| &endpoint.consumer)
        .collect::<BTreeSet<_>>()
    {
        // A failed consumer can yield only a declaration in the final capture.
        // Closing or invalid configuration still fails that exact owner request.
        let _ = client.ready_instance(consumer).await;
    }
    capture_endpoints(client, selections).await
}
pub(crate) async fn capture_endpoints(
    client: &Client,
    selections: &BTreeMap<String, HostEndpoint>,
) -> Result<Endpoints> {
    let mut endpoints = Endpoints::empty();
    for (name, selection) in selections {
        let captured = client
            .capture_import(selection.consumer.clone(), selection.import.clone())
            .await?;
        endpoints.insert(name.clone(), captured, selection.grant.clone())?;
    }
    endpoints.validate()?;
    Ok(endpoints)
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_capture_is_a_bounded_error_acknowledgement() {
        let contract = serde_json::json!({
            "name":"large.Service", "version":1, "transport":"remote",
            "interface":{"kind":"service","methods":{"read":{"mode":"query","input":{"description":"x".repeat(wire::FRAME_LIMIT)},"output":{},"resources":{}}}}
        });
        // Read-only descriptors cannot manufacture any Binding or grant.
        let catalogue: Catalogue = serde_json::from_value(serde_json::json!({"endpoints":{"large":{
            "provider":[Uuid::from_u128(1)],"export":"Large","contract":contract,"registration_epoch":Uuid::from_u128(2)
        }}})).unwrap();
        let nonce = Uuid::from_u128(3);
        let reply = capture_reply(nonce, Ok(catalogue));
        assert!(
            matches!(&reply, Event::Captured { nonce: actual, result: Err(_) } if *actual == nonce)
        );
        wire::encode_bounded(&reply, wire::FRAME_LIMIT).unwrap();
    }
}
