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

//! Original process/cohost execution and physical-call custody.
use super::*;
use tokio::process::{Child, ChildStdin, ChildStdout};

pub(super) struct Address {
    pub(super) path: PathBuf,
    // The parent retains the private directory until this same child has closed
    // and actually exited. Failed cleanup never recycles an old address.
    #[cfg(unix)]
    _directory: tempfile::TempDir,
}
impl Address {
    pub(super) fn create(nonce: Uuid) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Short account-private paths also fit Darwin's sockaddr_un limit.
            let directory = tempfile::Builder::new()
                .prefix("maka-")
                .permissions(std::fs::Permissions::from_mode(0o700))
                .tempdir_in("/tmp")
                .map_err(|e| e.to_string())?;
            let _ = nonce;
            Ok(Self {
                path: directory.path().join("host"),
                _directory: directory,
            })
        }
        #[cfg(windows)]
        {
            Ok(Self {
                path: PathBuf::from(format!(r"\\.\pipe\maka-application-{nonce}")),
            })
        }
    }
}
pub(super) struct OwnedHost {
    pub(super) owner: HostOwner,
    pub(super) connection: Option<Arc<native::Connection>>,
    pub(super) ready: bool,
    // Publication is distinct from bootstrap readiness: an unused controller
    // must keep custody without obstructing healthy UI-source retention.
    pub(super) published: bool,
    pub(super) configuration: Host,
    pub(super) peers: Arc<crate::transport::Peers>,
}
pub(super) enum HostOwner {
    Process {
        child: Child,
        control: Box<Control<ChildStdout, ChildStdin>>,
        _address: Address,
    },
    Cohost {
        application: Option<maka_runtime_host::application::Application>,
        server: Option<Arc<crate::transport::Server>>,
        captured: Option<crate::transport::Endpoints>,
    },
}
impl OwnedHost {
    pub(super) async fn capture(&mut self) -> Result<crate::transport::Catalogue> {
        match &mut self.owner {
            HostOwner::Process { control, .. } => control.capture().await,
            HostOwner::Cohost {
                application,
                captured,
                ..
            } => {
                let endpoints = crate::transport::process::capture_endpoints(
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
            HostOwner::Process { control, .. } => {
                control.check_approval(source, consumers, retain, catalogue)
            }
            HostOwner::Cohost {
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
            HostOwner::Process { control, .. } => {
                control.approve(source, consumers, retain, catalogue).await
            }
            HostOwner::Cohost {
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
    pub(super) async fn close_owner(&mut self) -> Result<()> {
        match &mut self.owner {
            HostOwner::Process { child, control, .. } => {
                let closed = control.close().await;
                let exited = child.wait().await.map_err(|e| e.to_string());
                closed?;
                let status = exited?;
                if self.ready && !status.success() {
                    return Err(format!("original Host exited unsuccessfully: {status}"));
                }
                Ok(())
            }
            HostOwner::Cohost {
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
