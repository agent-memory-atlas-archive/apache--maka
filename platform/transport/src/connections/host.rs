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

//! Public session attachment and locally owned cohost execution.
use super::*;
pub(super) struct ConnectedHost {
    pub(super) owner: HostLink,
    pub(super) connection: Option<Arc<native::Connection>>,
    pub(super) ready: bool,
    // Publication is distinct from connection readiness: an unused controller
    // must keep custody without obstructing healthy UI-source retention.
    pub(super) published: bool,
    // Last approved endpoint observations, never executable registrations.
    pub(super) epochs: BTreeMap<String, Option<Uuid>>,
    pub(super) configuration: Host,
    pub(super) peers: Arc<crate::transport::Peers>,
}
pub(super) enum HostLink {
    Public {
        client: Option<Box<maka_host_runtime::application::host::Client>>,
    },
    Cohost {
        application: Option<maka_host_runtime::application::Application>,
        server: Option<Arc<crate::transport::Server>>,
        captured: Option<crate::transport::Endpoints>,
    },
}
impl ConnectedHost {
    pub(super) async fn registrations_changed(&mut self) -> Result<bool> {
        let current = match &mut self.owner {
            HostLink::Public { client } => {
                client
                    .as_mut()
                    .ok_or("public Host session is detached")?
                    .epochs()
                    .await?
            }
            HostLink::Cohost { application, .. } => crate::transport::Endpoints::capture(
                &application
                    .as_ref()
                    .ok_or("cohost is not initialized")?
                    .client(),
                &self.configuration.endpoints,
            )
            .await?
            .epochs(),
        };
        if !current.keys().eq(self.configuration.endpoints.keys()) {
            return Err("Host observed endpoints differ from configured selections".into());
        }
        Ok(current != self.epochs)
    }
    pub(super) async fn capture(&mut self) -> Result<crate::transport::Catalogue> {
        match &mut self.owner {
            HostLink::Public { client } => {
                client
                    .as_mut()
                    .ok_or("public Host session is detached")?
                    .capture()
                    .await
            }
            HostLink::Cohost {
                application,
                captured,
                ..
            } => {
                let endpoints = crate::transport::Endpoints::capture(
                    &application
                        .as_ref()
                        .ok_or("cohost is not initialized")?
                        .client(),
                    &self.configuration.endpoints,
                )
                .await?;
                let catalogue = crate::transport::Catalogue::captured(&endpoints);
                *captured = Some(endpoints);
                Ok(catalogue)
            }
        }
    }
    pub(super) fn check_approval(
        &self,
        source: &DeploymentRef,
        consumers: &BTreeSet<InstanceId>,
        retain: &BTreeSet<u64>,
        catalogue: &Option<crate::transport::Catalogue>,
    ) -> Result<()> {
        match &self.owner {
            HostLink::Public { client } => client
                .as_ref()
                .ok_or("public Host session is detached")?
                .check_approval(source, consumers, retain, catalogue),
            HostLink::Cohost {
                server, captured, ..
            } => {
                if server.is_none() || consumers.is_empty() != catalogue.is_none() {
                    return Err("cohost is not ready for approval".into());
                }
                if let Some(expected) = catalogue {
                    let actual = captured
                        .as_ref()
                        .ok_or("cohost endpoints were not captured")?;
                    if &crate::transport::Catalogue::captured(actual) != expected {
                        return Err("cohost captured endpoints differ".into());
                    }
                }
                Ok(())
            }
        }
    }
    pub(super) async fn approve(
        &mut self,
        source: DeploymentRef,
        consumers: BTreeSet<InstanceId>,
        retain: BTreeSet<u64>,
        catalogue: Option<crate::transport::Catalogue>,
    ) -> Result<()> {
        self.check_approval(&source, &consumers, &retain, &catalogue)?;
        match &mut self.owner {
            HostLink::Public { client } => {
                client
                    .as_mut()
                    .ok_or("public Host session is detached")?
                    .approve(source, consumers, retain, catalogue)
                    .await
            }
            HostLink::Cohost {
                server, captured, ..
            } => {
                if catalogue.is_some() {
                    server
                        .as_ref()
                        .unwrap()
                        .replace_endpoints(captured.take().unwrap())?;
                }
                Ok(())
            }
        }
    }
    pub(super) async fn close_owner(
        &mut self,
        mode: crate::ExitMode,
        namespaces: &RootNamespaces,
    ) -> Result<()> {
        match &mut self.owner {
            HostLink::Public { client } => {
                if mode == crate::ExitMode::Quit
                    && let Some(client) = client.as_mut()
                    && let Err(error) = client.quit().await
                {
                    let HostLocation::Public { root, .. } = &self.configuration.location else {
                        unreachable!();
                    };
                    if !released(root, namespaces, client).await {
                        return Err(error);
                    }
                }
                // The caller has already confirmed all original physical Calls.
                // Keeping the Host only detaches this anchor. Normal quit instead
                // receives the original session's last-client decision above.
                drop(client.take());
                Ok(())
            }
            HostLink::Cohost {
                server,
                application,
                ..
            } => {
                let closed = match server {
                    Some(server) => server.close().await,
                    None => Ok(()),
                };
                let stopped = match application {
                    Some(application) => application.shutdown().await,
                    None => Ok(()),
                };
                closed.and(stopped)
            }
        }
    }
}
/// An externally stopped Host cannot answer quit. Prove the original owner is
/// gone without opening its database or treating a disconnected socket as cleanup.
async fn released(root: &Path, namespaces: &RootNamespaces, client: &Client) -> bool {
    let domain = &client.source().state_domain;
    if let Ok(owner) = maka_event_log::root::RootOwner::open(root, namespaces) {
        return owner.root_id() == domain;
    }
    // A replacement may already hold the Root; never send it the old UI's quit.
    let Ok(found) = discovery::read_with_namespaces(root, namespaces) else {
        return false;
    };
    if found.root_id != *domain || found.host_epoch == client.epoch() {
        return false;
    }
    let Ok(status) = Client::status(&found.endpoint, Some(found.host_epoch)).await else {
        return false;
    };
    found.verify(&status).is_ok()
}
pub(super) struct CohostConnector {
    pub(super) server: Arc<crate::transport::Server>,
    pub(super) peers: Arc<crate::transport::Peers>,
}
impl native::Connector for CohostConnector {
    fn connect(
        &self,
        stop: tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'static, Result<native::Stream>> {
        let server = self.server.clone();
        let peers = self.peers.clone();
        async move {
            if stop.is_cancelled() {
                return Err("cohost opening cancelled".into());
            }
            let (client, host) = tokio::io::duplex(64 * 1024);
            let (reader, writer) = tokio::io::split(host);
            // Server publishes actual tracked custody before this observer is
            // dropped. This is the same wire/Scope adapter as process Hosts.
            drop(server.serve(reader, writer, peers)?);
            Ok(Box::new(client) as native::Stream)
        }
        .boxed()
    }
}
