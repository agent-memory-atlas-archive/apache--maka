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
//! Ordinary product authoring from immutable build assets, without Node or a checkout.
use super::{HostError, Input, Package, RemoteObservation, read_json};
use clap::{Args, ValueEnum};
use maka_js_runtime::plugin::Vm;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, io::Write, path::PathBuf};

struct Bundled {
    name: &'static str,
    files: &'static [(&'static str, &'static [u8])],
}
include!(concat!(env!("OUT_DIR"), "/product-assets.rs"));

#[derive(Clone, Copy, ValueEnum)]
pub(super) enum Product {
    Agent,
    Studio,
}

#[derive(Args)]
pub(crate) struct Compose {
    /// Installation configuration and explicit grants; omitted identities are generated once.
    #[arg(long, value_name = "FILE")]
    configuration: PathBuf,
    /// New output directory for checked sources, connection input and exact packages.
    #[arg(long, value_name = "DIRECTORY")]
    output: PathBuf,
    #[arg(long, value_enum, default_value = "agent")]
    product: Product,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Sources {
    host: Input,
    management_host: Input,
    ui: Input,
    remote_exports: Vec<RemoteObservation>,
    connections: Value,
}

impl Compose {
    pub(super) async fn run(self) -> Result<(), HostError> {
        let configuration = read_json(&self.configuration)?;
        let packages = tokio::task::spawn_blocking(packages).await??;
        let (sources, installation) = author(self.product, configuration, &packages).await?;
        tokio::task::spawn_blocking(move || export(self.output, sources, installation, packages))
            .await??;
        println!("Checked product sources and immutable packages written");
        Ok(())
    }
}

fn packages() -> Result<BTreeMap<String, Package>, HostError> {
    BUNDLED
        .iter()
        .map(|bundled| {
            let files = bundled
                .files
                .iter()
                .map(|(name, bytes)| ((*name).into(), bytes.to_vec()))
                .collect();
            Ok((bundled.name.into(), Package::new(files)?))
        })
        .collect()
}

async fn author(
    product: Product,
    mut configuration: Value,
    packages: &BTreeMap<String, Package>,
) -> Result<(Sources, Value), HostError> {
    let values = configuration
        .as_object_mut()
        .ok_or("installation configuration must be an object")?;
    let references: BTreeMap<_, _> = packages
        .iter()
        .map(|(name, package)| (name, package.application_ref()))
        .collect();
    // Caller-supplied package locators cannot substitute for actual build bytes.
    values.insert("packages".into(), serde_json::to_value(references)?);
    for key in ["hostDeployment", "managementDeployment"] {
        values
            .entry(key)
            .or_insert_with(|| Value::String(uuid::Uuid::new_v4().to_string()));
    }
    let vm = Vm::pure(Default::default())?;
    let result: Result<Value, HostError> =
        match vm.load("maka-product-authoring".into(), AUTHORING.into()) {
            Ok(module) => {
                let result = async {
                    if !values.contains_key("keys") {
                        // Read the original exported array through its ordinary slice
                        // method; no second instance-name list lives in the launcher.
                        let names: Vec<String> = serde_json::from_value(
                            module
                                .call(vec!["instanceNames".into(), "slice".into()], vec![])
                                .await?,
                        )?;
                        let keys: BTreeMap<_, _> = names
                            .into_iter()
                            .map(|name| (name, uuid::Uuid::new_v4().to_string()))
                            .collect();
                        values.insert("keys".into(), serde_json::to_value(keys)?);
                    }
                    let method = match product {
                        Product::Agent => "composition",
                        Product::Studio => "studioComposition",
                    };
                    Ok(module
                        .call(vec![method.into()], vec![Value::Object(values.clone())])
                        .await?)
                }
                .await;
                let closed = module.close().await.map_err(HostError::from);
                result.and_then(|value| closed.map(|()| value))
            }
            Err(error) => Err(error.into()),
        };
    vm.shutdown().await;
    let sources: Sources = serde_json::from_value(result?)?;
    let checked: Vec<_> = packages.values().cloned().collect();
    for (input, remote) in [
        (&sources.host, &[][..]),
        (&sources.management_host, &[][..]),
        (&sources.ui, sources.remote_exports.as_slice()),
    ] {
        maka_event_log::applications::validate_application(input, remote, &checked)?;
    }
    let connections: maka_application_bridge::Configuration =
        serde_json::from_value(sources.connections.clone())?;
    connections.validate().map_err(HostError::from)?;
    values.remove("packages");
    Ok((sources, configuration))
}

fn export(
    output: PathBuf,
    sources: Sources,
    installation: Value,
    packages: BTreeMap<String, Package>,
) -> Result<(), HostError> {
    let output = std::path::absolute(output)?;
    // All source validation precedes filesystem effects. Existing output,
    // including an empty directory or a symlink, is never replaced.
    std::fs::create_dir(&output)?;
    maka_event_log::root::private_directory(&output)?;
    let package_directory = output.join("packages");
    std::fs::create_dir(&package_directory)?;
    let mut references = BTreeMap::new();
    for (name, package) in packages {
        package.export_to(&package_directory.join(format!("{name}.maka-plugin")))?;
        references.insert(name, package.application_ref());
    }
    for (name, value) in [
        ("host", serde_json::to_value(sources.host)?),
        (
            "management-host",
            serde_json::to_value(sources.management_host)?,
        ),
        ("ui", serde_json::to_value(sources.ui)?),
        (
            "remote-exports",
            serde_json::to_value(sources.remote_exports)?,
        ),
        ("connections", sources.connections),
        ("packages", serde_json::to_value(references)?),
        ("installation", installation),
    ] {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(format!("{name}.json")))?;
        serde_json::to_writer(&mut file, &value)?;
        file.write_all(b"\n")?;
    }
    Ok(())
}
