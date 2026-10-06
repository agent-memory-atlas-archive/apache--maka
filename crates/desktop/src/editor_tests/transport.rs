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

//! Tests wire the existing physical transport, not another service/session adapter.
use futures_util::future::BoxFuture;
use maka_application_bridge::{
    Grant,
    transport::{
        Endpoint, Peer, PeerPolicy, Peers, Server,
        native::{Connection, Connector, Service, Stream},
    },
};
use maka_plugin_core::{composition::InstanceId, deployment::DeploymentRef};
use maka_runtime_host::application::{Binding, Client};
use maka_ui::actions::{
    NativeMutation, NativeService, ServiceFuture, ServiceRequest, ServiceSource, StreamFuture,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};
use tokio_util::sync::CancellationToken;

struct Physical {
    server: Arc<Server>,
    peer: Arc<Peers>,
}
impl Connector for Physical {
    fn connect(&self, stop: CancellationToken) -> BoxFuture<'static, Result<Stream, String>> {
        let server = self.server.clone();
        let peer = self.peer.clone();
        Box::pin(async move {
            if stop.is_cancelled() {
                return Err("physical opening cancelled before admission".into());
            }
            let (channel, accepted) = tokio::io::duplex(64 * 1024);
            let (reader, writer) = tokio::io::split(accepted);
            // Server owns accepted work before the observer is discarded.
            drop(server.serve(reader, writer, peer)?);
            Ok(Box::new(channel) as Stream)
        })
    }
}
struct Capture {
    inner: Arc<dyn NativeService>,
    observed: Arc<Mutex<Option<ServiceSource>>>,
}
impl NativeService for Capture {
    fn call(&self, request: ServiceRequest) -> ServiceFuture {
        *self.observed.lock().unwrap() = Some(request.source.clone());
        self.inner.call(request)
    }
    fn mutation(&self, request: ServiceRequest) -> Result<Arc<dyn NativeMutation>, String> {
        *self.observed.lock().unwrap() = Some(request.source.clone());
        self.inner.mutation(request)
    }
    fn subscribe(&self, request: ServiceRequest, stop: CancellationToken) -> StreamFuture {
        *self.observed.lock().unwrap() = Some(request.source.clone());
        self.inner.subscribe(request, stop)
    }
}
pub(super) struct Remote {
    pub service: Arc<dyn NativeService>,
    pub observed: Arc<Mutex<Option<ServiceSource>>>,
    connection: Arc<Connection>,
    server: Arc<Server>,
}
impl Remote {
    pub fn new(
        client: Client,
        binding: Binding,
        policy: Arc<PeerPolicy>,
        source: DeploymentRef,
        consumer: InstanceId,
        grant: Grant,
    ) -> Self {
        let server = Server::new(
            client,
            policy,
            BTreeMap::from([("reader".into(), Endpoint::new(binding, grant).unwrap())]),
        )
        .unwrap();
        let peers = Peers::new(Peer::new(source, BTreeSet::from([consumer])).unwrap());
        let connection = Connection::new(Arc::new(Physical {
            server: server.clone(),
            peer: peers.clone(),
        }));
        let observed = Arc::new(Mutex::new(None));
        let service = Arc::new(
            Service::new(
                connection.clone(),
                server.catalogue().select("reader").unwrap(),
                peers,
            )
            .unwrap(),
        );
        Self {
            service: Arc::new(Capture {
                inner: service,
                observed: observed.clone(),
            }),
            observed,
            connection,
            server,
        }
    }
    pub async fn close(&self) -> Result<(), String> {
        let (physical, server) = tokio::join!(self.connection.close(), self.server.close());
        physical.and(server)
    }
}
