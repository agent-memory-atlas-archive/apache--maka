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

mod schema;
mod serve;

use std::{
    io::Read,
    path::{Path, PathBuf},
};

use clap::{Args, Subcommand};
use maka_plugin_core::{
    composition::{Input, PresentationTarget},
    deployment::RemoteObservation,
    package::Packages,
    plan::{Plan, RemoteExports},
};
use maka_plugins::package::Package;
use maka_runtime_host::server::HostError;
use serde::Serialize;

#[derive(Subcommand)]
pub(super) enum Command {
    /// Check exact package revisions, template overrides, variants and capability bindings.
    Check(Check),
    /// Emit authoring schemas from the actual runtime wire types.
    Schema,
    /// Validate immutable package bytes and print their exact pinned reference.
    InspectPackage(InspectPackage),
    /// Run a plugin composition with native terminal presentation.
    Run(Run),
    /// Atomically commit desired deployment input and exact immutable bytes.
    Commit(Commit),
    /// Read and validate the persisted source and every selected package.
    Read(ReadSource),
    /// Own a shared Host with inherited bootstrap/control and private IPC calls.
    Serve(serve::Serve),
}

#[derive(Args)]
pub(super) struct InspectPackage {
    #[arg(long, value_name = "PATH")]
    package: PathBuf,
}

#[derive(Args)]
pub(super) struct Check {
    /// Deployment composition input in JSON.
    #[arg(long, value_name = "FILE")]
    input: PathBuf,
    /// Immutable application package directory or bundle; repeat for each revision.
    #[arg(long = "package", value_name = "PATH")]
    packages: Vec<PathBuf>,
    /// Observed remote declarations in JSON; compatibility evidence, never authorization.
    #[arg(long, value_name = "FILE")]
    remote_exports: Option<PathBuf>,
}

#[derive(Args)]
pub(super) struct Run {
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    deployment: uuid::Uuid,
    /// Trusted local Host connections and explicit method/recovery grants.
    #[arg(long, value_name = "FILE")]
    connections: Option<PathBuf>,
}

#[derive(Args)]
pub(super) struct Commit {
    /// Existing owned State Root.
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    deployment: uuid::Uuid,
    /// Zero creates; replacement requires the currently observed revision.
    #[arg(long)]
    expected_revision: u64,
    #[command(flatten)]
    source: Check,
}

#[derive(Args)]
pub(super) struct ReadSource {
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    deployment: uuid::Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report<'a> {
    instances: Vec<&'a maka_plugin_core::plan::Instance>,
    launch: Option<&'a maka_plugin_core::plan::Binding>,
}

impl Command {
    pub(super) async fn run(self) -> Result<(), HostError> {
        self.run_inner().await.map_err(|error| -> HostError {
            maka_tui::application::sanitize(&error.to_string()).into()
        })
    }

    async fn run_inner(self) -> Result<(), HostError> {
        match self {
            Self::Serve(args) => serve::run(args).await?,
            Self::Schema => println!("{}", serde_json::to_string_pretty(&schema::catalog())?),
            Self::InspectPackage(args) => {
                let reference = tokio::task::spawn_blocking(move || {
                    Package::read_from(&std::path::absolute(args.package)?)
                        .map(|package| package.application_ref())
                })
                .await??;
                println!("{}", serde_json::to_string_pretty(&reference)?);
            }
            Self::Check(args) => {
                let report = tokio::task::spawn_blocking(move || args.check()).await??;
                println!("{report}");
            }
            Self::Commit(args) => {
                let (input, observations, packages) =
                    tokio::task::spawn_blocking(move || args.source.read()).await??;
                let store = open_store(args.root).await?;
                let result = store
                    .commit_application(
                        args.deployment,
                        args.expected_revision,
                        input,
                        observations,
                        packages,
                    )
                    .await;
                let closed = store.close().await;
                let source = result?;
                closed?;
                println!("{}", serde_json::to_string_pretty(&source)?);
            }
            Self::Read(args) => {
                let store = open_store(args.root).await?;
                let result = store.load_application(args.deployment).await;
                let closed = store.close().await;
                let loaded = result?.ok_or("deployment not found")?;
                closed?;
                println!("{}", serde_json::to_string_pretty(&loaded.source)?);
            }
            Self::Run(args) => {
                // Keep the foreground VM on one dedicated blocking owner, with
                // no deeply nested CLI async stack under V8 initialization.
                let scheduler = tokio::runtime::Handle::current();
                let configuration = args
                    .connections
                    .as_deref()
                    .map(maka_application_bridge::Configuration::read)
                    .transpose()?
                    .unwrap_or_default();
                let namespaces = maka_event_log::root::RootNamespaces::for_current_account()?;
                let executable = maka_application_bridge::HostExecutable::new(
                    std::env::current_exe()?,
                    vec!["application".into(), "serve".into()],
                )?;
                let cohost_native = serve::native()?;
                let store = std::sync::Arc::new(open_store(args.root).await?);
                let mut connections = match maka_application_bridge::Connections::prepare(
                    store.clone(),
                    args.deployment,
                    PresentationTarget::Tui,
                    configuration,
                    namespaces,
                    executable,
                    cohost_native,
                )
                .await
                {
                    Ok(connections) => connections,
                    Err(error) => {
                        maka_application_bridge::drain_store(&store).await;
                        return Err(error.into());
                    }
                };
                if let Err(error) = connections.start().await {
                    connections.drain().await;
                    maka_application_bridge::drain_store(&store).await;
                    return Err(error.into());
                }
                let source = connections.source().clone();
                let registrations = connections.registrations();
                let foreground_store = store.clone();
                let result = tokio::task::spawn_blocking(move || -> Result<(), HostError> {
                    maka_tui::application::run(foreground_store, source, registrations, scheduler)
                        .map_err(Into::into)
                })
                .await;
                // A panic is not a terminal/module cleanup receipt. Preserve
                // both actual owners if the foreground execution was lost.
                if let Err(error) = &result {
                    eprintln!("terminal execution cleanup unconfirmed: {error}");
                    std::future::pending::<()>().await;
                }
                connections.drain().await;
                maka_application_bridge::drain_store(&store).await;
                result??;
            }
        }
        Ok(())
    }
}

impl Check {
    fn read(self) -> Result<(Input, Vec<RemoteObservation>, Vec<Package>), HostError> {
        let input = read_json(&self.input)?;
        if self.packages.len() > 128 {
            return Err("application package count exceeds 128".into());
        }
        let mut size = 0usize;
        let mut packages = Vec::new();
        for source in self.packages {
            let package = Package::read_from(&std::path::absolute(source)?)?;
            size += package.files().values().map(Vec::len).sum::<usize>();
            if size > 128 * 1024 * 1024 {
                return Err("application packages exceed 128 MiB".into());
            }
            packages.push(package);
        }
        let observations = self
            .remote_exports
            .map(|source| read_json(&source))
            .transpose()?
            .unwrap_or_default();
        Ok((input, observations, packages))
    }
    fn check(self) -> Result<String, HostError> {
        let (input, observations, bytes) = self.read()?;
        let mut packages = Packages::default();
        for package in bytes {
            let declaration = package
                .manifest()
                .application
                .as_ref()
                .ok_or("package has no application declaration")?;
            packages.insert(package.application_ref(), declaration.clone())?;
        }
        let mut remote = RemoteExports::new();
        for observation in observations {
            if remote
                .insert(observation.capability, observation.declaration)
                .is_some()
            {
                return Err("duplicate remote capability observation".into());
            }
        }
        let plan = Plan::prepare(&input, &packages, &remote)?;
        Ok(serde_json::to_string_pretty(&Report {
            instances: plan.instances().values().collect(),
            launch: plan.launch(),
        })?)
    }
}

fn read_json<T: serde::de::DeserializeOwned>(source: &Path) -> Result<T, HostError> {
    let mut bytes = Vec::new();
    std::fs::File::open(source)?
        .take(512 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 512 * 1024 {
        return Err("application input exceeds 512 KiB".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

async fn open_store(root: PathBuf) -> Result<maka_event_log::EventLog, HostError> {
    let owner = tokio::task::spawn_blocking(move || {
        maka_event_log::root::RootOwner::open(
            &root,
            &maka_event_log::root::RootNamespaces::for_current_account()?,
        )
    })
    .await??;
    Ok(maka_application_bridge::open_store(std::sync::Arc::new(owner)).await?)
}
