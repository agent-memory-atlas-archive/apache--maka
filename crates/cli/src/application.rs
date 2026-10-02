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

use std::{
    io::Read,
    path::{Path, PathBuf},
};

use clap::{Args, Subcommand};
use maka_plugin_core::{
    composition::Input,
    contract::Export,
    package::Packages,
    plan::{Plan, RemoteCapability, RemoteExports},
};
use maka_plugins::package::Package;
use maka_runtime_host::server::HostError;
use serde::{Deserialize, Serialize};

#[derive(Subcommand)]
pub(super) enum Command {
    /// Check exact package revisions, template overrides, variants and capability bindings.
    Check(Check),
    /// Emit authoring schemas from the actual runtime wire types.
    Schema,
    /// Run a plugin composition with native terminal presentation.
    Run(Run),
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
    #[arg(long, value_name = "FILE")]
    input: PathBuf,
    #[arg(long = "package", value_name = "PATH", required = true)]
    packages: Vec<PathBuf>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoteObservation {
    capability: RemoteCapability,
    declaration: Export,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report<'a> {
    instances: Vec<&'a maka_plugin_core::plan::Instance>,
    launch: Option<&'a maka_plugin_core::plan::Binding>,
}

impl Command {
    pub(super) async fn run(self) -> Result<(), HostError> {
        match self {
            Self::Schema => println!("{}", serde_json::to_string_pretty(&schema::catalog())?),
            Self::Check(args) => {
                let report = tokio::task::spawn_blocking(move || args.check()).await??;
                println!("{report}");
            }
            Self::Run(args) => {
                // Keep the foreground VM on one dedicated blocking owner, with
                // no deeply nested CLI async stack under V8 initialization.
                let scheduler = tokio::runtime::Handle::current();
                let result = tokio::task::spawn_blocking(move || -> Result<(), HostError> {
                    let input: Input = read_json(&args.input)?;
                    let packages = args
                        .packages
                        .into_iter()
                        .map(|source| -> Result<_, HostError> {
                            Ok(Package::read_from(&std::path::absolute(source)?)?)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    maka_tui::application::run(&input, packages, scheduler).map_err(Into::into)
                })
                .await
                .map_err(|error| -> HostError {
                    maka_tui::application::sanitize(&error.to_string()).into()
                })?;
                // All errors from this terminal-facing command are untrusted.
                result.map_err(|error| -> HostError {
                    maka_tui::application::sanitize(&error.to_string()).into()
                })?;
            }
        }
        Ok(())
    }
}

impl Check {
    fn check(self) -> Result<String, HostError> {
        let input: Input = read_json(&self.input)?;
        let mut packages = Packages::default();
        for source in self.packages {
            let source = std::path::absolute(source)?;
            let package = Package::read_from(&source)?;
            let declaration = package
                .manifest()
                .application
                .as_ref()
                .ok_or("package has no application declaration")?;
            packages.insert(package.application_ref(), declaration.clone())?;
        }
        let mut remote = RemoteExports::new();
        if let Some(source) = self.remote_exports {
            let observations: Vec<RemoteObservation> = read_json(&source)?;
            for observation in observations {
                if remote
                    .insert(observation.capability, observation.declaration)
                    .is_some()
                {
                    return Err("duplicate remote capability observation".into());
                }
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
