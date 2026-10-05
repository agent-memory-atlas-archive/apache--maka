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

//! Private invocation evidence from a trusted launcher or authenticated acceptor.
use super::wire::Source;
use crate::{Grant, Result};
use maka_event_log::applications::intents::CallerIntent;
use maka_plugin_core::{
    composition::InstanceId,
    contract::{Interface, Transport},
    deployment::DeploymentRef,
};
use maka_plugins::call::{Identity, Issuer, Scope};
use maka_runtime_host::application::{Binding, Invocation, Policy, RecoveryInvocation};
use std::{collections::BTreeSet, sync::Arc};
use tokio_util::sync::CancellationToken;

/// Trusted embedding input. Reading the same fields from a request is not approval.
#[derive(Clone)]
pub struct Peer {
    source: DeploymentRef,
    consumers: BTreeSet<InstanceId>,
}
impl Peer {
    pub fn new(source: DeploymentRef, consumers: BTreeSet<InstanceId>) -> Result<Self> {
        source.validate()?;
        if consumers.is_empty()
            || consumers.len() > 128
            || consumers
                .iter()
                .any(|id| id.0.is_empty() || id.0.len() > 64 || id.0.iter().any(uuid::Uuid::is_nil))
        {
            return Err("invalid approved UI consumers".into());
        }
        Ok(Self { source, consumers })
    }
}
#[derive(Clone)]
pub struct Endpoint {
    pub(super) binding: Binding,
    pub(super) grant: Grant,
}
impl Endpoint {
    pub fn new(binding: Binding, grant: Grant) -> Result<Self> {
        let Interface::Service { methods } = &binding.contract().interface else {
            return Err("Application endpoint requires a service".into());
        };
        if binding.contract().transport != Transport::Remote
            || grant.methods.is_empty()
            || grant.methods.len() > 64
            || grant.methods.iter().any(|name| !methods.contains_key(name))
        {
            return Err("invalid Application endpoint grant".into());
        }
        Ok(Self { binding, grant })
    }
}
struct Evidence {
    source: Source,
    endpoint: Endpoint,
}
#[derive(Default)]
pub struct PeerPolicy {
    issuer: Issuer,
}
impl PeerPolicy {
    pub fn issuer(&self) -> Issuer {
        self.issuer.clone()
    }
    pub(super) fn scope(
        &self,
        peer: &Peer,
        source: Source,
        endpoint: Endpoint,
        cancel: CancellationToken,
    ) -> Result<Scope> {
        if source.deployment != peer.source
            || !peer.consumers.contains(&source.consumer)
            || source.request.is_nil()
        {
            return Err("caller observations differ from the approved peer".into());
        }
        self.issuer
            .issue_with(
                Identity::Remote {
                    request_id: source.request,
                },
                Evidence { source, endpoint },
                cancel,
            )
            .map_err(|e| e.to_string())
    }
    fn evidence<'a>(&self, scope: &'a Scope) -> Result<&'a Evidence> {
        let evidence = self
            .issuer
            .evidence::<Evidence>(scope)
            .ok_or("Application peer evidence mismatch")?;
        if scope.identity
            != (Identity::Remote {
                request_id: evidence.source.request,
            })
        {
            return Err("Application request identity mismatch".into());
        }
        Ok(evidence)
    }
}
impl Policy for PeerPolicy {
    fn admit(&self, invocation: Invocation<'_>) -> Result<()> {
        let e = self.evidence(invocation.scope)?;
        if invocation.consumer != e.endpoint.binding.consumer()
            || invocation.provider != e.endpoint.binding.provider()
            || invocation.export != e.endpoint.binding.export()
            || invocation.contract != e.endpoint.binding.contract()
            || !e.endpoint.grant.methods.contains(invocation.method)
        {
            return Err("Application invocation is not granted".into());
        }
        Ok(())
    }
    fn confirm_peer_mutation(
        &self,
        invocation: Invocation<'_>,
        intent: &CallerIntent,
    ) -> Result<()> {
        let e = self.evidence(invocation.scope)?;
        if intent.caller != e.source.deployment
            || intent.consumer != e.source.consumer
            || intent.reference.operation != e.source.request
        {
            return Err("caller confirmation differs from the original peer request".into());
        }
        Ok(())
    }
    fn recover(&self, invocation: RecoveryInvocation<'_>) -> Result<()> {
        let e = self.evidence(invocation.scope)?;
        if !e.endpoint.grant.recovery || invocation.consumer != e.endpoint.binding.provider() {
            return Err("original receipt recovery is not granted".into());
        }
        Ok(())
    }
}
// Policy ownership is shared with the original Application, not a second kernel.
pub(super) type PolicyOwner = Arc<PeerPolicy>;
