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

//! HTTP authentication precedes the ordinary bounded application protocol.
mod bytes;
mod upgrade;
use super::{Host, Result, access::Authority};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::{TcpListener, TcpStream};

pub struct WebSocketListener {
    pub(super) listener: TcpListener,
    pub(super) allowed_origins: Arc<[String]>,
}
impl WebSocketListener {
    pub async fn bind(address: SocketAddr, allowed_origins: Vec<String>) -> Result<Self> {
        if !address.ip().is_loopback() {
            return Err("Plain WebSocket listener requires a loopback address".into());
        }
        if allowed_origins
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != allowed_origins.len()
        {
            return Err("WebSocket Origin allowlist contains duplicates".into());
        }
        Ok(Self {
            listener: TcpListener::bind(address)
                .await
                .map_err(|error| error.to_string())?,
            allowed_origins: allowed_origins.into(),
        })
    }
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}
pub(super) async fn connection(
    socket: TcpStream,
    host: Arc<Host>,
    allowed_origins: Arc<[String]>,
) -> Result<()> {
    let upgraded = tokio::select! {
        biased;
        () = host.stop.cancelled() => return Ok(()),
        result = tokio::time::timeout(Duration::from_secs(5), upgrade::accept(socket, host.clone(), allowed_origins)) => result.map_err(|_| "HTTP upgrade timed out")??,
    };
    let Some((socket, hash)) = upgraded else {
        return Ok(());
    };
    host.channel(bytes::Channel::new(socket), Authority::Remote { hash })
        .await
}
