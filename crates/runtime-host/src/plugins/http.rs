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

//! Legacy embedding authority for the shared HTTP transport.
use crate::{
    execution::Executions,
    http::{Authority, Worker},
};
use futures_util::future::BoxFuture;
use maka_plugins::{
    call::Scope,
    fiber::Context,
    http::{Client, Error, Request, Response},
};
use maka_runtime::tools::ToolError;
use std::sync::{Arc, Weak};

pub(super) struct Http(crate::http::Http);
impl Http {
    pub fn new(
        host: Weak<Executions>,
        owner: Context,
        permissions: Arc<dyn maka_plugins::permissions::Access>,
    ) -> Self {
        Self(crate::http::Http::new(Arc::new(Host {
            host,
            owner,
            permissions,
        })))
    }
}
impl Client for Http {
    fn request(&self, call: Scope, input: Request) -> BoxFuture<'_, Result<Response, Error>> {
        self.0.request(call, input)
    }
}
struct Host {
    host: Weak<Executions>,
    owner: Context,
    permissions: Arc<dyn maka_plugins::permissions::Access>,
}
impl Authority for Host {
    fn admit(&self, call: &Scope) -> Result<Box<dyn Send>, Error> {
        let lease = self.owner.admit().map_err(|_| Error::Denied)?;
        let host = self.host.upgrade().ok_or(Error::Denied)?;
        if !host.plugin_calls.owns(call) || call.cancellation.is_cancelled() {
            return Err(Error::Denied);
        }
        Ok(Box::new(lease))
    }
    fn policy<'a>(
        &'a self,
        call: &'a Scope,
        destination: &'a maka_sandbox::Destination,
        origin: String,
    ) -> BoxFuture<'a, Result<maka_network::Policy, Error>> {
        Box::pin(async move {
            if call.identity.agent().is_some() {
                let granted = self
                    .permissions
                    .request(
                        call.clone(),
                        maka_plugins::permissions::Request {
                            reason: format!("Allow plugin HTTP access to {origin}"),
                            permissions: maka_plugins::permissions::Permissions {
                                filesystem: Vec::new(),
                                network: maka_sandbox::Network::destination(destination.clone()),
                            },
                        },
                    )
                    .await
                    .map_err(|error| match error {
                        ToolError::Io {
                            kind: std::io::ErrorKind::PermissionDenied,
                            ..
                        } => Error::Denied,
                        other => Error::Failed(other.to_string()),
                    })?;
                if !granted.network.allows(destination) {
                    return Err(Error::Denied);
                }
            }
            self.host
                .upgrade()
                .ok_or(Error::Denied)?
                .plugin_network_policy(call, destination)
                .await
                .map_err(|_| Error::Denied)
        })
    }
    fn check<'a>(
        &'a self,
        call: &'a Scope,
        destination: &'a maka_sandbox::Destination,
    ) -> BoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            self.host
                .upgrade()
                .ok_or(Error::Denied)?
                .check_plugin_network(call, destination)
                .await
                .map_err(|_| Error::Denied)
        })
    }
    fn record(
        &self,
        call: Scope,
        operation: maka_event_log::effects::Operation,
        destination: maka_sandbox::Destination,
        effect: BoxFuture<'static, Result<serde_json::Value, ToolError>>,
    ) -> BoxFuture<'static, Result<serde_json::Value, ToolError>> {
        let host = self.host.clone();
        let owner = self.owner.clone();
        Box::pin(async move {
            host.upgrade()
                .ok_or_else(|| ToolError::Failed("HTTP owner retired".into()))?
                .record_plugin_http(owner, call, operation, destination, effect)
                .await
        })
    }
    fn spawn(&self, worker: Worker) -> Result<(), Error> {
        self.owner
            .spawn_resource("HTTP response", worker)
            .map(|_| ())
            .map_err(|_| Error::Denied)
    }
}
