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

//! Trusted in-process embedding fixture for LocalService and native owner tests.
//! Production launchers use the separate child-process Connections.
use maka_application_bridge::{Configuration, Grant, LocalService, Result, Scopes};
use maka_event_log::{
    EventLog,
    root::{RootNamespaces, RootOwner},
};
use maka_plugin_core::{
    composition::ExecutionLocation,
    contract::{Interface, Transport},
    deployment::DeploymentRef,
    plan::RemoteCapability,
};
use maka_plugins::call::{Identity, Issuer, Scope};
use maka_runtime_host::application::{
    Application, Binding, Invocation, NativeRegistry, Policy, RecoveryInvocation,
};
use maka_ui::actions::{RemoteService, ServiceSource};
use std::{collections::BTreeSet, sync::Arc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct OwnedHost {
    application: Application,
    store: Arc<EventLog>,
}
/// Lifecycle custody only. UI endpoints cannot keep these deployment owners alive.
/// Close the UI and its actual accepted work before draining these connections.
pub struct Connections {
    source: DeploymentRef,
    hosts: Vec<OwnedHost>,
    registrations: Vec<RemoteService>,
}
impl Connections {
    pub async fn start(
        store: &EventLog,
        deployment: Uuid,
        configuration: Configuration,
        namespaces: &RootNamespaces,
        native: NativeRegistry,
    ) -> Result<Self> {
        let loaded = store
            .load_application(deployment)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("UI deployment not found")?;
        if loaded.source.input.environment.location != ExecutionLocation::Ui {
            return Err("UI deployment required".into());
        }
        let mut connections = Self {
            source: loaded.source.reference,
            hosts: Vec::new(),
            registrations: Vec::new(),
        };
        let result = connections
            .connect(
                configuration,
                namespaces,
                &loaded.source.remote_exports,
                native,
            )
            .await;
        if let Err(error) = result {
            // Even failed startup retains every started owner until real cleanup.
            connections.drain().await;
            return Err(error);
        }
        Ok(connections)
    }
    async fn connect(
        &mut self,
        configuration: Configuration,
        namespaces: &RootNamespaces,
        observations: &[maka_plugin_core::deployment::RemoteObservation],
        native: NativeRegistry,
    ) -> Result<()> {
        let mut targets = BTreeSet::new();
        for host in configuration.hosts {
            let owner =
                Arc::new(RootOwner::open(&host.root, namespaces).map_err(|e| e.to_string())?);
            let store = Arc::new(EventLog::for_root(owner).await.map_err(|e| e.to_string())?);
            let issuer = Issuer::default();
            let policy = Arc::new(LocalPolicy {
                issuer: issuer.clone(),
                source: self.source.clone(),
            });
            let application = match Application::start_from_store(
                store.clone(),
                host.deployment,
                maka_application_bridge::with_gateway(native.clone())?,
                issuer.clone(),
                policy,
                Default::default(),
            )
            .await
            {
                Ok(application) => application,
                Err(error) => {
                    maka_application_bridge::drain_store(&store).await;
                    return Err(error);
                }
            };
            self.hosts.push(OwnedHost { application, store });
            let application = &self.hosts.last().unwrap().application;
            application.ready().await?;
            let client = application.client();
            let consumer = client.activation(&host.consumer)?;
            for (import, grant) in host.imports {
                let bindings = client.bind(consumer.clone(), import).await?;
                if bindings.is_empty() {
                    return Err("local service grant has no provider".into());
                }
                for binding in bindings {
                    let contract = binding.contract().clone();
                    if contract.transport != Transport::Remote {
                        return Err("local UI connection requires a remote service contract".into());
                    }
                    let Interface::Service { methods } = &contract.interface else {
                        return Err("local connection requires a service".into());
                    };
                    if grant.methods.iter().any(|m| !methods.contains_key(m)) {
                        return Err("grant names an undeclared service method".into());
                    }
                    let target = RemoteCapability {
                        deployment: host.name.clone(),
                        instance: binding.provider().instance().clone(),
                        export: binding.export().into(),
                    };
                    if !targets.insert(target.clone()) {
                        return Err("duplicate local endpoint grant".into());
                    }
                    if !observations.iter().any(|o| {
                        o.capability == target && o.declaration.contract.accepts(&contract)
                    }) {
                        return Err(
                            "local endpoint differs from the selected UI source observation".into(),
                        );
                    }
                    let scopes = Arc::new(LocalScopes {
                        issuer: issuer.clone(),
                        source: self.source.clone(),
                        binding: binding.clone(),
                        grant: grant.clone(),
                    });
                    self.registrations.push(RemoteService {
                        target,
                        registration_epoch: binding.registration_epoch(),
                        contract,
                        implementation: Arc::new(LocalService::new(
                            client.clone(),
                            binding,
                            scopes,
                        )),
                    });
                }
            }
        }
        Ok(())
    }
    pub fn source(&self) -> &DeploymentRef {
        &self.source
    }
    pub fn take_registrations(&mut self) -> Vec<RemoteService> {
        std::mem::take(&mut self.registrations)
    }
    /// Err retains every application/store lease. It never confirms cleanup.
    pub async fn close(&mut self) -> Result<()> {
        let mut error = None;
        for host in &self.hosts {
            if let Err(failure) = host.application.shutdown().await {
                error.get_or_insert(failure);
            }
        }
        if let Some(error) = error {
            return Err(error);
        }
        for host in &self.hosts {
            host.store.shutdown().await.map_err(|e| e.to_string())?;
        }
        self.registrations.clear();
        self.hosts.clear();
        Ok(())
    }
    /// Keep the owner and runtime alive through unconfirmed cleanup. Diagnostics
    /// are emitted once per distinct failure; process termination is not a receipt.
    pub async fn drain(&mut self) {
        let mut reported = BTreeSet::new();
        loop {
            match self.close().await {
                Ok(()) => return,
                Err(error) => {
                    if reported.insert(error.clone()) {
                        eprintln!("local Host cleanup unconfirmed: {error:?}");
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
}
struct Evidence {
    source: ServiceSource,
    binding: Binding,
    grant: Grant,
}
struct LocalScopes {
    issuer: Issuer,
    source: DeploymentRef,
    binding: Binding,
    grant: Grant,
}
impl Scopes for LocalScopes {
    fn scope(&self, source: &ServiceSource) -> Result<Scope> {
        if source.deployment() != Some(&self.source) {
            return Err("UI source differs from the approved connection".into());
        }
        self.issuer
            .issue_with(
                Identity::Remote {
                    request_id: source.request_id(),
                },
                Evidence {
                    source: source.clone(),
                    binding: self.binding.clone(),
                    grant: self.grant.clone(),
                },
                CancellationToken::new(),
            )
            .map_err(|e| e.to_string())
    }
}
struct LocalPolicy {
    issuer: Issuer,
    source: DeploymentRef,
}
impl LocalPolicy {
    fn evidence<'a>(&self, scope: &'a Scope) -> Result<&'a Evidence> {
        let evidence = self
            .issuer
            .evidence::<Evidence>(scope)
            .ok_or("local connection issuer or evidence mismatch")?;
        if evidence.source.deployment() != Some(&self.source)
            || scope.identity
                != (Identity::Remote {
                    request_id: evidence.source.request_id(),
                })
        {
            return Err("local connection source mismatch".into());
        }
        Ok(evidence)
    }
}
impl Policy for LocalPolicy {
    fn admit(&self, invocation: Invocation<'_>) -> Result<()> {
        let evidence = self.evidence(invocation.scope)?;
        if invocation.consumer != evidence.binding.consumer()
            || invocation.provider != evidence.binding.provider()
            || invocation.export != evidence.binding.export()
            || invocation.contract != evidence.binding.contract()
            || !evidence.grant.methods.contains(invocation.method)
        {
            return Err("local service invocation not granted".into());
        }
        Ok(())
    }
    fn recover(&self, invocation: RecoveryInvocation<'_>) -> Result<()> {
        let evidence = self.evidence(invocation.scope)?;
        if !evidence.grant.recovery || invocation.consumer != evidence.binding.provider() {
            return Err("original receipt recovery not granted".into());
        }
        Ok(())
    }
}
