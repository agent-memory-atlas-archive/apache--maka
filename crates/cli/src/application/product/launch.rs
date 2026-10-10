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

//! One native launcher record, original source commits, and the existing UI/Host owners.
use super::{Product, Sources, author, packages};
use crate::application::{HostError, read_json, run_tui};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::Args;
use maka_application_bridge::{Configuration, HostLocation};
use maka_event_log::{
    EventLog,
    applications::launch::Launch as Record,
    root::{RootNamespaces, RootOwner},
};
use maka_plugin_core::{
    composition::Input,
    deployment::{DeploymentRef, RemoteObservation},
};
use maka_plugins::package::Package;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use uuid::Uuid;

#[derive(Args)]
pub(crate) struct Launch {
    /// UI State Root; also selects the saved product on subsequent launches.
    #[arg(long)]
    root: PathBuf,
    /// Required only for first initialization; explicit workspaces, Host Root and grants.
    #[arg(long, value_name = "FILE")]
    configuration: Option<PathBuf>,
    /// Product for first initialization. Reopen uses the persisted application.
    #[arg(long, value_enum, requires = "configuration")]
    product: Option<Product>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preparation {
    sources: Sources,
    // Exact immutable files, independent of whatever binary resumes this work.
    packages: Vec<BTreeMap<String, String>>,
}

impl Launch {
    pub(in crate::application) async fn run(self) -> Result<(), HostError> {
        eprintln!("Opening product…");
        let store = Arc::new(open(self.root, self.configuration.is_some(), None).await?);
        let result = prepare(
            &store,
            self.configuration,
            self.product.unwrap_or(Product::Agent),
        )
        .await;
        let (deployment, configuration) = match result {
            Ok(value) => value,
            Err(error) => {
                store.drain().await;
                return Err(error);
            }
        };
        run_tui(store, deployment, configuration).await
    }
}
async fn open(
    root: PathBuf,
    initialize: bool,
    expected_domain: Option<String>,
) -> Result<EventLog, HostError> {
    let owner = tokio::task::spawn_blocking(move || {
        let namespaces = RootNamespaces::for_current_account()?;
        if initialize {
            maka_event_log::root::initialize(&root, &namespaces)?;
        }
        let owner = RootOwner::open(&root, &namespaces)?;
        if expected_domain
            .as_deref()
            .is_some_and(|domain| owner.root_id() != domain)
        {
            return Err(std::io::Error::other("original product Host Root changed"));
        }
        Ok(owner)
    })
    .await??;
    Ok(maka_application_bridge::open_store(Arc::new(owner)).await?)
}
fn targets(configuration: &Configuration) -> Result<(PathBuf, String, Uuid, Uuid), HostError> {
    configuration.validate().map_err(HostError::from)?;
    let mut process = None;
    let mut management = None;
    for host in &configuration.hosts {
        match &host.location {
            HostLocation::Process { root, state_domain } if process.is_none() => {
                process = Some((
                    root.clone(),
                    state_domain
                        .clone()
                        .ok_or("product Host Root identity missing")?,
                    host.deployment,
                ))
            }
            HostLocation::Cohost {} if management.is_none() => management = Some(host.deployment),
            _ => return Err("product launch requires one process Host and one cohost".into()),
        }
    }
    let (path, domain, host) = process.ok_or("product Host is missing")?;
    Ok((
        path,
        domain,
        host,
        management.ok_or("product management Host is missing")?,
    ))
}
async fn prepare(
    store: &EventLog,
    configuration: Option<PathBuf>,
    product: Product,
) -> Result<(Uuid, Configuration), HostError> {
    let record = match store.application_launch().await? {
        Some(record) => {
            if configuration.is_some() {
                return Err("product already selected; reopen with --root only (initialization will resume if needed)".into());
            }
            record
        }
        None => {
            let configuration = configuration.ok_or("first launch requires --configuration")?;
            begin(store, read_json(&configuration)?, product).await?;
            store
                .application_launch()
                .await?
                .ok_or("launcher record missing after commit")?
        }
    };
    let connections: Configuration = serde_json::from_value(record.configuration.clone())?;
    let (root, domain, host, management) = targets(&connections)?;
    if let Some(bytes) = record.pending {
        let preparation: Preparation = serde_json::from_slice(&bytes)?;
        if preparation.sources.connections != record.configuration {
            return Err("original launcher connection input changed".into());
        }
        let packages = decode_packages(preparation.packages)?;
        let host_store = open(root, false, Some(domain)).await?;
        let result = async {
            ensure(&host_store, host, &preparation.sources.host, &[], &packages).await?;
            ensure(
                store,
                management,
                &preparation.sources.management_host,
                &[],
                &packages,
            )
            .await?;
            ensure(
                store,
                record.deployment,
                &preparation.sources.ui,
                &preparation.sources.remote_exports,
                &packages,
            )
            .await?;
            store.complete_application_launch(record.deployment).await?;
            Ok::<_, HostError>(())
        }
        .await;
        host_store.drain().await;
        result?;
    } else if maka_event_log::root::resolve(&root)?.root_id() != domain {
        return Err("original product Host Root changed".into());
    }
    Ok((record.deployment, connections))
}
async fn begin(
    store: &EventLog,
    mut configuration: Value,
    product: Product,
) -> Result<(), HostError> {
    let values = configuration
        .as_object_mut()
        .ok_or("installation configuration must be an object")?;
    match values.get("presentation") {
        None => {
            values.insert("presentation".into(), Value::String("tui".into()));
        }
        Some(Value::String(value)) if value == "tui" => {}
        _ => return Err("product launch requires tui presentation".into()),
    }
    let root = PathBuf::from(
        values
            .get("hostRoot")
            .and_then(Value::as_str)
            .ok_or("hostRoot is required")?,
    );
    let root = std::path::absolute(root)?;
    // Acquire the actual independent Host Root before recording its identity.
    let host_store = open(root, true, None).await?;
    let result = async {
        let owner = host_store.owned_root()?;
        configuration["hostRoot"] = serde_json::to_value(owner.canonical_path())?;
        let packages = tokio::task::spawn_blocking(packages).await??;
        let (mut sources, _) = author(product, configuration, &packages).await?;
        for host in sources.connections["hosts"].as_array_mut().ok_or("product connections missing")? {
            if host["location"]["kind"] == "process" {
                host["location"]["stateDomain"] = Value::String(owner.root_id().into());
            }
        }
        let connections: Configuration = serde_json::from_value(sources.connections.clone())?;
        let (_, _, host, management) = targets(&connections)?;
        // Explicitly supplied identities must also represent a new installation.
        if host_store.read_application_metadata(host).await?.is_some()
            || store.read_application_metadata(management).await?.is_some() {
            return Err("installation deployment already exists; use its original launcher or application run".into());
        }
        let selection = sources.connections.clone();
        let preparation = Preparation {
            sources,
            packages: packages.values().map(|package| package.files().iter().map(|(path, bytes)| (path.clone(), STANDARD.encode(bytes))).collect()).collect(),
        };
        store.begin_application_launch(Record {
            deployment: Uuid::new_v4(), configuration: selection,
            pending: Some(serde_json::to_vec(&preparation)?),
        }).await?;
        Ok::<_, HostError>(())
    }.await;
    host_store.drain().await;
    result
}
fn decode_packages(files: Vec<BTreeMap<String, String>>) -> Result<Vec<Package>, HostError> {
    if files.len() > 128 {
        return Err("too many prepared packages".into());
    }
    let mut total = 0usize;
    files
        .into_iter()
        .map(|files| {
            let files = files
                .into_iter()
                .map(|(name, encoded)| {
                    let bytes = STANDARD.decode(encoded)?;
                    total = total
                        .checked_add(bytes.len())
                        .filter(|size| *size <= 128 * 1024 * 1024)
                        .ok_or("prepared packages exceed 128 MiB")?;
                    Ok::<_, HostError>((name, bytes))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            Ok(Package::new(files)?)
        })
        .collect()
}
async fn ensure(
    store: &EventLog,
    deployment: Uuid,
    input: &Input,
    observations: &[RemoteObservation],
    packages: &[Package],
) -> Result<(), HostError> {
    let reference = DeploymentRef {
        state_domain: store.owned_root()?.root_id().into(),
        deployment,
        revision: 1,
    };
    if let Some(original) = store.load_application_at(&reference).await? {
        let mut actual: Vec<_> = original
            .packages
            .iter()
            .map(Package::application_ref)
            .collect();
        let mut expected: Vec<_> = packages.iter().map(Package::application_ref).collect();
        actual.sort();
        expected.sort();
        if serde_json::to_value(&original.source.input)? != serde_json::to_value(input)?
            || serde_json::to_value(&original.source.remote_exports)?
                != serde_json::to_value(observations)?
            || actual != expected
        {
            return Err(
                "original product initialization source conflicts with stored revision 1".into(),
            );
        }
        // Reading validated every original byte against its immutable revision.
        // A newer user-authored head is deliberately preserved.
    } else {
        store
            .commit_application(
                deployment,
                0,
                input.clone(),
                observations.to_vec(),
                packages.to_vec(),
            )
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
