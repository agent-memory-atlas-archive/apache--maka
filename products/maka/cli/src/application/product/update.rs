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

//! Default upgrades and explicit three-way synchronization of customized products.
use super::{HostError, Product, Sources, author, launch, packages};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::Args;
use maka_event_log::EventLog;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Baseline {
    pub(super) product: Product,
    pub(super) installation: Value,
    /// Standard launcher inputs regenerate defaults; explicit configurations stay explicit.
    pub(super) default_options: Option<Value>,
    pub(super) sources: Sources,
}

#[derive(Args)]
pub(crate) struct Update {
    /// UI State Root (default: ~/.maka/application). Close its UI and local Host first.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Apply the previewed merge only when there are no conflicts.
    #[arg(long)]
    apply: bool,
}
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Outcome {
    customized: bool,
    changed: bool,
    applied: bool,
    changes: Vec<String>,
    conflicts: Vec<String>,
}
impl Update {
    pub(crate) async fn run(self) -> Result<(), HostError> {
        let store = launch::open(super::root_path(self.root)?, false, None).await?;
        let result = async {
            let mut outcome = synchronize(&store, true, self.apply).await?;
            if self.apply && outcome.changed && outcome.conflicts.is_empty() {
                launch::prepare(&store, None, Product::Agent).await?;
                outcome.applied = true;
            }
            println!("{}", serde_json::to_string(&outcome)?);
            if self.apply && !outcome.conflicts.is_empty() {
                return Err("default update has conflicts; no update was written".into());
            }
            Ok(())
        }
        .await;
        store.drain().await;
        result
    }
}

pub(super) async fn begin(store: &EventLog) -> Result<bool, HostError> {
    let outcome = synchronize(store, false, true).await?;
    Ok(outcome.changed && !outcome.customized)
}

async fn synchronize(store: &EventLog, custom: bool, apply: bool) -> Result<Outcome, HostError> {
    let record = store
        .application_launch()
        .await?
        .ok_or("product selection missing")?;
    if record.pending.is_some() {
        return Err("finish the prepared product operation before checking for updates".into());
    }
    let baseline: Baseline = serde_json::from_value(store.application_product().await?
        .ok_or("product storage predates automatic updates; remove this development root and initialize it again")?)?;
    let connections = serde_json::from_value(record.configuration.clone())?;
    let (path, domain, host_id, management) = launch::targets(&connections)?;
    let host = launch::open(path, false, Some(domain.clone())).await?;
    let result = async {
        let mut revisions = BTreeMap::new();
        let mut current = Vec::new();
        let mut available = BTreeMap::new();
        for (target, deployment) in [
            (&host, host_id),
            (store, management),
            (store, record.deployment),
        ] {
            let head = target
                .load_application(deployment)
                .await?
                .ok_or("product source missing")?;
            revisions.insert(deployment, head.source.reference.revision);
            for package in &head.packages {
                available.insert(package.application_ref(), package.clone());
            }
            current.push(head);
        }
        let local = Sources {
            host: current[0].source.input.clone(),
            management_host: current[1].source.input.clone(),
            ui: current[2].source.input.clone(),
            remote_exports: current[2].source.remote_exports.clone(),
            connections: record.configuration.clone(),
        };
        let base = serde_json::to_value(&baseline.sources)?;
        let local_value = serde_json::to_value(&local)?;
        let mut outcome = Outcome {
            customized: local_value != base,
            ..Default::default()
        };
        if outcome.customized && !custom {
            return Ok(outcome);
        }
        let packages = tokio::task::spawn_blocking(packages).await??;
        let configuration = match &baseline.default_options {
            Some(options) => {
                let mut configuration = super::installation(options.clone()).await?;
                // Product identities survive changes to the default feature set.
                // The author allocates identities only for newly introduced instances.
                for key in ["keys", "hostDeployment", "managementDeployment"] {
                    configuration[key] = baseline.installation[key].clone();
                }
                configuration
            }
            None => baseline.installation,
        };
        let (mut next, installation) = author(baseline.product, configuration, &packages).await?;
        for endpoint in next.connections["hosts"]
            .as_array_mut()
            .ok_or("product connections missing")?
        {
            if endpoint["location"]["kind"] == "process" {
                endpoint["location"]["stateDomain"] = Value::String(domain.clone());
            }
        }
        let next_value = serde_json::to_value(&next)?;
        if next_value == base {
            return Ok(outcome);
        }
        outcome.changed = true;
        let merged = merge(
            Some(&base),
            Some(&local_value),
            Some(&next_value),
            "",
            &mut outcome,
        )
        .ok_or("product merge removed its root")?;
        if !outcome.conflicts.is_empty() {
            return Ok(outcome);
        }
        let sources: Sources = serde_json::from_value(merged)?;
        // Neither automatic nor explicit synchronization relocates installation data.
        if launch::targets(&serde_json::from_value(sources.connections.clone())?)?
            != launch::targets(&connections)?
        {
            return Err("default update changed installation identity".into());
        }
        for package in packages.values() {
            available.insert(package.application_ref(), package.clone());
        }
        let available: Vec<_> = available.into_values().collect();
        let mut selected = BTreeMap::new();
        // Validate the complete merge and its actual package bytes before writing preparation.
        for (input, observations) in [
            (&sources.host, &[][..]),
            (&sources.management_host, &[][..]),
            (&sources.ui, sources.remote_exports.as_slice()),
        ] {
            let required = super::selected(input, observations, &available)?;
            maka_event_log::applications::validate_application(input, observations, &required)?;
            for package in required {
                selected.insert(package.application_ref(), package);
            }
        }
        if selected.len() > 128 {
            return Err("updated product exceeds the prepared package budget".into());
        }
        if apply {
            let preparation = launch::Preparation {
                product: Some(Baseline {
                    product: baseline.product,
                    installation,
                    default_options: baseline.default_options,
                    sources: next,
                }),
                previous_connections: Some(record.configuration),
                restore: revisions,
                sources,
                packages: selected
                    .values()
                    .map(|package| {
                        package
                            .files()
                            .iter()
                            .map(|(path, bytes)| (path.clone(), STANDARD.encode(bytes)))
                            .collect()
                    })
                    .collect(),
            };
            store
                .begin_application_restore(record.deployment, serde_json::to_vec(&preparation)?)
                .await?;
        }
        Ok::<_, HostError>(outcome)
    }
    .await;
    host.drain().await;
    result
}

/// Objects merge by field. Ordered lists and scalar values remain atomic; an
/// ambiguous edit is reported rather than silently changing user intent.
fn merge(
    base: Option<&Value>,
    local: Option<&Value>,
    next: Option<&Value>,
    path: &str,
    outcome: &mut Outcome,
) -> Option<Value> {
    if local == next || next == base {
        return local.cloned();
    }
    if let (Some(Value::Object(base)), Some(Value::Object(local)), Some(Value::Object(next))) =
        (base, local, next)
    {
        let keys: BTreeSet<_> = base.keys().chain(local.keys()).chain(next.keys()).collect();
        let mut result = serde_json::Map::new();
        for key in keys {
            let escaped = key.replace('~', "~0").replace('/', "~1");
            if let Some(value) = merge(
                base.get(key),
                local.get(key),
                next.get(key),
                &format!("{path}/{escaped}"),
                outcome,
            ) {
                result.insert(key.clone(), value);
            }
        }
        Some(Value::Object(result))
    } else if local == base {
        outcome.changes.push(path.into());
        next.cloned()
    } else {
        outcome.conflicts.push(path.into());
        local.cloned()
    }
}

#[cfg(test)]
mod tests;
