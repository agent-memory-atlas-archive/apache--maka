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

//! Trusted private-channel bootstrap, independent of the legacy Host server.
use super::{open_store, read_json};
use clap::Args;
use maka_application_bridge::{
    Grant,
    transport::{Endpoint, Peer, PeerPolicy, Server},
};
use maka_plugin_core::{composition::InstanceId, deployment::DeploymentRef};
use maka_runtime_host::{
    application::{Application, NativeRegistry},
    server::HostError,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};
use uuid::Uuid;
#[derive(Args)]
pub(crate) struct Serve {
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    deployment: Uuid,
    /// Trusted launcher approval file, never an incoming channel message.
    #[arg(long, value_name = "FILE")]
    peer: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Approval {
    source: DeploymentRef,
    consumers: BTreeSet<InstanceId>,
    consumer: InstanceId,
    imports: BTreeMap<String, Grant>,
}
pub(super) async fn run(args: Serve) -> Result<(), HostError> {
    let approval: Approval = read_json(&args.peer)?;
    let peer = Peer::new(approval.source, approval.consumers)?;
    if approval.imports.is_empty() || approval.imports.len() > 128 {
        return Err("invalid private channel imports".into());
    }
    let mut native = NativeRegistry::default();
    maka_agent_service::provide(&mut native)?;
    maka_settings_service::provide(&mut native)?;
    let native = maka_application_bridge::with_gateway(native)?;
    let policy = Arc::new(PeerPolicy::default());
    let crate::stdio::Stdio {
        input,
        output,
        output_done,
    } = crate::stdio::Stdio::open()?;
    let store = Arc::new(open_store(args.root).await?);
    let application = match Application::start_from_store(
        store.clone(),
        args.deployment,
        native,
        policy.issuer(),
        policy.clone(),
        Default::default(),
    )
    .await
    {
        Ok(application) => application,
        Err(e) => {
            maka_application_bridge::drain_store(&store).await;
            return Err(e.into());
        }
    };
    let result = async {
        application.ready().await?;
        let client = application.client();
        let consumer = client.activation(&approval.consumer)?;
        let mut endpoints = BTreeMap::new();
        for (name, grant) in approval.imports {
            let mut bindings = client.bind(consumer.clone(), name.clone()).await?;
            if bindings.len() != 1 {
                return Err(
                    "private channel import requires one explicitly selected provider".to_string(),
                );
            }
            endpoints.insert(name, Endpoint::new(bindings.pop().unwrap(), grant)?);
        }
        let server = Server::new(client, policy, endpoints)?;
        let observer = server.serve(input, output, peer)?;
        let result = observer.await;
        // Preserve all failed physical custody. EOF is not a cleanup receipt.
        let mut reported = BTreeSet::new();
        loop {
            match server.close().await {
                Ok(()) => break,
                Err(e) => {
                    if reported.insert(e.clone()) {
                        eprintln!(
                            "private connection cleanup unconfirmed: {}",
                            maka_tui::application::sanitize(&e)
                        );
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        result
    }
    .await;
    let mut reported = BTreeSet::new();
    loop {
        match application.shutdown().await {
            Ok(()) => break,
            Err(e) => {
                if reported.insert(e.clone()) {
                    eprintln!(
                        "Application cleanup unconfirmed: {}",
                        maka_tui::application::sanitize(&e)
                    );
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    maka_application_bridge::drain_store(&store).await;
    drop(application);
    drop(store);
    // This deadline bounds delivery only, after real Root/SQL cleanup.
    let flushed = tokio::time::timeout(std::time::Duration::from_secs(5), output_done).await;
    result?;
    flushed???;
    Ok(())
}
