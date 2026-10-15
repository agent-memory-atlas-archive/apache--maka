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
use super::{Product, Sources, packages};
use crate::application::{HostError, read_json, run_tui};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::Args;
use maka_event_log::{
    EventLog,
    applications::launch::Launch as Record,
    root::{RootNamespaces, RootOwner},
};
use maka_packages::Package;
use maka_plugin_core::{
    composition::Input,
    deployment::{DeploymentRef, RemoteObservation},
};
use maka_transport::{Configuration, HostLocation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, io::IsTerminal, path::PathBuf, sync::Arc};
use uuid::Uuid;

pub(super) enum Installation {
    Standard(Value),
    Configured(Value),
}

#[derive(Args, Default)]
pub(crate) struct Launch {
    /// UI State Root (default: ~/.maka/application); reopen its saved product.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Advanced first-install configuration; otherwise use the standard product.
    #[arg(long, value_name = "FILE", conflicts_with_all = ["workspace", "locale", "light", "trust_workspace"])]
    configuration: Option<PathBuf>,
    /// Product for first initialization. Reopen uses the persisted application.
    #[arg(long, value_enum)]
    product: Option<Product>,
    /// First-install workspace (default: current directory).
    #[arg(long, value_name = "DIRECTORY")]
    workspace: Option<PathBuf>,
    /// Initial language; omitted follows the system, settings can override it.
    #[arg(long, value_parser = ["auto", "en", "zh-CN", "zh-TW"])]
    locale: Option<String>,
    /// Start the first installation with the light theme.
    #[arg(long)]
    light: bool,
    /// Trust the first-install workspace for file changes and sandboxed commands.
    #[arg(long)]
    trust_workspace: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Preparation {
    #[serde(default)]
    pub(super) product: Option<super::update::Baseline>,
    #[serde(default)]
    pub(super) previous_connections: Option<Value>,
    #[serde(default)]
    pub(super) restore: BTreeMap<Uuid, u64>,
    pub(super) sources: Sources,
    // Exact immutable files, independent of whatever binary resumes this work.
    pub(super) packages: Vec<BTreeMap<String, String>>,
}

impl Launch {
    pub(crate) async fn run(self) -> Result<(), HostError> {
        if self.configuration.is_none()
            && (!std::io::stdin().is_terminal() || !std::io::stdout().is_terminal())
        {
            return Err(
                "Maka needs an interactive terminal. Use --help for automation commands.".into(),
            );
        }
        eprintln!("Opening product…");
        let root = super::root_path(self.root.clone())?;
        let store = Arc::new(open(root.clone(), true, None).await?);

        let mut startup = None;
        let result = async {
            let existing = store.application_launch().await?.is_some();
            if existing && (self.workspace.is_some() || self.locale.is_some() || self.light
                || self.product.is_some() || self.trust_workspace) {
                return Err("first-install options cannot change a saved product; reopen with --root only and use Settings or Plugins".into());
            }
            let configuration = match self.configuration {
                Some(path) => Some(Installation::Configured(read_json(&path)?)),
                None if existing => None,
                None => match standard_configuration(root, self.workspace, self.locale, self.light, self.trust_workspace, &mut startup).await? {
                    Some(configuration) => Some(configuration),
                    None => return Ok(None),
                },
            };
            let selected = prepare(&store, configuration, self.product.unwrap_or(Product::Agent)).await?;

            if existing && super::update::begin(&store).await? {
                prepare(&store, None, Product::Agent).await.map(Some)
            } else {
                Ok(Some(selected))
            }
        }.await;
        let (deployment, configuration, anchors) = match result {
            Ok(Some(value)) => value,
            Ok(None) => {
                store.drain().await;
                return Ok(());
            }
            Err(error) => {
                store.drain().await;
                return Err(error);
            }
        };
        run_tui(store, deployment, configuration, startup, anchors).await
    }
}
async fn standard_configuration(
    root: PathBuf,
    workspace: Option<PathBuf>,
    locale: Option<String>,
    light: bool,
    trusted: bool,
    startup: &mut Option<maka_render_terminal::startup::Startup>,
) -> Result<Option<Installation>, HostError> {
    let workspace = std::fs::canonicalize(workspace.unwrap_or(std::env::current_dir()?))?;
    if !workspace.is_dir() {
        return Err("workspace must be a directory".into());
    }
    let home = crate::serve::home_directory()?;
    if !trusted {
        let shown = workspace.clone();
        let language = locale
            .as_deref()
            .filter(|value| *value != "auto")
            .map(str::to_owned)
            .unwrap_or_else(maka_ui_runtime::locale::system);
        *startup = tokio::task::spawn_blocking(move || {
            maka_render_terminal::startup::Startup::confirm(&shown, &language, light)
        })
        .await??;
        if startup.is_none() {
            return Ok(None);
        }
    }
    let options = serde_json::json!({
        "workspace": workspace, "hostRoot": root.join("host"),
        "themeFile": root.join("appearance/theme.json"),
        "helper": std::env::current_exe()?, "home": home,
        "locale": locale.unwrap_or_else(|| "auto".into()),
        "scheme": if light { "light" } else { "dark" },
    });
    Ok(Some(Installation::Standard(options)))
}

pub(super) async fn open(
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
    Ok(maka_transport::open_store(Arc::new(owner)).await?)
}
pub(super) fn targets(
    configuration: &Configuration,
) -> Result<(PathBuf, String, Uuid, Uuid), HostError> {
    configuration.validate().map_err(HostError::from)?;
    let mut public = None;
    let mut management = None;
    for host in &configuration.hosts {
        match &host.location {
            HostLocation::Public { root, state_domain } if public.is_none() => {
                public = Some((
                    root.clone(),
                    state_domain
                        .clone()
                        .ok_or("product Host Root identity missing")?,
                    host.deployment,
                ))
            }
            HostLocation::Cohost {} if management.is_none() => management = Some(host.deployment),
            _ => return Err("product launch requires one public Host and one cohost".into()),
        }
    }
    let (path, domain, host) = public.ok_or("product Host is missing")?;
    Ok((
        path,
        domain,
        host,
        management.ok_or("product management Host is missing")?,
    ))
}
pub(super) async fn prepare(
    store: &EventLog,
    configuration: Option<Installation>,
    product: Product,
) -> Result<
    (
        Uuid,
        Configuration,
        Vec<maka_host_runtime::application::host::Client>,
    ),
    HostError,
> {
    let record = match store.application_launch().await? {
        Some(record) => {
            if configuration.is_some() {
                return Err("product already selected; reopen with --root only (initialization will resume if needed)".into());
            }
            record
        }
        None => {
            let configuration = configuration.ok_or("first launch requires --configuration")?;
            begin(store, configuration, product).await?;
            store
                .application_launch()
                .await?
                .ok_or("launcher record missing after commit")?
        }
    };
    let mut connections: Configuration = serde_json::from_value(record.configuration.clone())?;
    let (root, domain, host, management) = targets(&connections)?;
    if let Some(bytes) = record.pending {
        let preparation: Preparation = serde_json::from_slice(&bytes)?;
        if preparation
            .previous_connections
            .as_ref()
            .unwrap_or(&preparation.sources.connections)
            != &record.configuration
        {
            return Err("original launcher connection input changed".into());
        }
        let packages = decode_packages(preparation.packages)?;
        let host_packages = super::selected(&preparation.sources.host, &[], &packages)?;
        let management_packages =
            super::selected(&preparation.sources.management_host, &[], &packages)?;
        let ui_packages = super::selected(
            &preparation.sources.ui,
            &preparation.sources.remote_exports,
            &packages,
        )?;
        if preparation.restore.is_empty() {
            bootstrap_host(
                &root,
                &domain,
                host,
                &connections,
                &preparation.sources.host,
                &host_packages,
            )
            .await?;
        }
        let public = super::public::Host::connect(root.clone(), domain.clone(), host).await?;
        let result = async {
            if preparation.restore.is_empty() {
                public
                    .ensure_initial(&preparation.sources.host, &[], &host_packages)
                    .await?;
            } else {
                let previous = host_configuration(&connections)?;
                let desired = host_configuration(&serde_json::from_value(
                    preparation.sources.connections.clone(),
                )?)?;
                let configuration =
                    if serde_json::to_value(&previous)? == serde_json::to_value(&desired)? {
                        None
                    } else {
                        Some((previous, desired))
                    };
                public
                    .ensure_restore(
                        *preparation
                            .restore
                            .get(&host)
                            .ok_or("restore revision missing")?,
                        &preparation.sources.host,
                        &[],
                        &host_packages,
                        configuration,
                    )
                    .await?;
            }
            for (deployment, input, observations, packages) in [
                (
                    management,
                    &preparation.sources.management_host,
                    &[][..],
                    management_packages.as_slice(),
                ),
                (
                    record.deployment,
                    &preparation.sources.ui,
                    preparation.sources.remote_exports.as_slice(),
                    ui_packages.as_slice(),
                ),
            ] {
                if preparation.restore.is_empty() {
                    ensure(store, deployment, input, observations, packages).await?;
                } else {
                    super::restore::ensure(
                        store,
                        deployment,
                        *preparation
                            .restore
                            .get(&deployment)
                            .ok_or("restore revision missing")?,
                        input,
                        observations,
                        packages,
                    )
                    .await?;
                }
            }
            if let Some(baseline) = preparation.product {
                store
                    .complete_application_product(
                        record.deployment,
                        preparation.sources.connections.clone(),
                        serde_json::to_value(baseline)?,
                    )
                    .await?;
                connections = serde_json::from_value(preparation.sources.connections.clone())?;
            } else {
                store.complete_application_launch(record.deployment).await?;
            }
            Ok::<_, HostError>(())
        }
        .await;
        result?;
        return Ok((record.deployment, connections, vec![public.into_client()]));
    }
    let public = super::public::Host::connect(root, domain, host).await?;
    Ok((record.deployment, connections, vec![public.into_client()]))
}
fn host_configuration(
    configuration: &Configuration,
) -> Result<maka_host_runtime::application::host::Configuration, HostError> {
    let connection = configuration
        .hosts
        .iter()
        .find(|connection| matches!(&connection.location, HostLocation::Public { .. }))
        .ok_or("product public Host missing")?;
    let configuration = maka_host_runtime::application::host::Configuration::PublicHost {
        endpoints: connection.endpoints.clone(),
        native: connection.native.clone(),
    };
    configuration.validate()?;
    Ok(configuration)
}

async fn bootstrap_host(
    root: &std::path::Path,
    domain: &str,
    deployment: Uuid,
    connections: &Configuration,
    input: &Input,
    packages: &[Package],
) -> Result<(), HostError> {
    let host_store = match open(root.into(), false, Some(domain.into())).await {
        Ok(store) => store,
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::WouldBlock) =>
        {
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    let connection = connections
        .hosts
        .iter()
        .find(|connection| matches!(&connection.location, HostLocation::Public { .. }))
        .ok_or("product public Host missing")?;
    let result = super::headless::save_product(
        &host_store,
        deployment,
        input,
        packages,
        connection.endpoints.clone(),
        connection.native.clone(),
    )
    .await;
    // Public startup can acquire Root only after bootstrap has actually drained.
    host_store.drain().await;
    drop(host_store);
    result
}

async fn begin(store: &EventLog, input: Installation, product: Product) -> Result<(), HostError> {
    let version = super::bundle_fingerprint()?;
    let authoring = super::authoring()?;
    let (mut configuration, mut default_options) = match input {
        Installation::Standard(options) => (
            super::installation_with_source(options.clone(), &authoring).await?,
            Some(options),
        ),
        Installation::Configured(configuration) => (configuration, None),
    };
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
        if let Some(options) = &mut default_options {
            options["hostRoot"] = configuration["hostRoot"].clone();
        }
        let packages = tokio::task::spawn_blocking(packages).await??;
        let version = super::verified_fingerprint(&authoring, &packages, &version)?;
        let (mut sources, installation, _) = super::author_with_source(product, configuration, &packages, &authoring).await?;
        for host in sources.connections["hosts"].as_array_mut().ok_or("product connections missing")? {
            if host["location"]["kind"] == "public" {
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
            product: Some(super::update::Baseline { product, installation, default_options, sources: sources.clone(), version: Some(version) }),
            previous_connections: None,
            restore: BTreeMap::new(),
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
pub(super) fn decode_packages(
    files: Vec<BTreeMap<String, String>>,
) -> Result<Vec<Package>, HostError> {
    files
        .into_iter()
        .map(|files| {
            let files = files
                .into_iter()
                .map(|(name, encoded)| {
                    let bytes = STANDARD.decode(encoded)?;
                    Ok::<_, HostError>((name, bytes))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            Ok(Package::new(files)?)
        })
        .collect()
}
pub(super) async fn ensure(
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
            .packages()
            .iter()
            .map(Package::application_ref)
            .collect();
        let mut expected: Vec<_> = packages.iter().map(Package::application_ref).collect();
        actual.sort();
        expected.sort();
        if serde_json::to_value(&original.source().input)? != serde_json::to_value(input)?
            || serde_json::to_value(&original.source().remote_exports)?
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
