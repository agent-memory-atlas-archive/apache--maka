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

//! Restore the installation baseline with the existing durable launcher preparation.
use super::{HostError, Input, Package, Product, RemoteObservation, Sources, launch};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::Args;
use maka_event_log::EventLog;
use maka_plugin_core::deployment::DeploymentRef;
use std::{collections::BTreeMap, path::PathBuf};
use uuid::Uuid;

#[derive(Args)]
pub(crate) struct Restore {
    /// UI State Root (default: ~/.maka/application). Close its UI and local Host first.
    #[arg(long)]
    root: Option<PathBuf>,
}
impl Restore {
    pub(crate) async fn run(self) -> Result<(), HostError> {
        let root = super::root_path(self.root)?;
        let store = launch::open(root, false, None).await?;
        let result = restore(&store).await;
        store.drain().await;
        println!("{}", serde_json::to_string(&result?)?);
        Ok(())
    }
}
async fn restore(store: &EventLog) -> Result<DeploymentRef, HostError> {
    let record = store
        .application_launch()
        .await?
        .ok_or("root has no saved product")?;
    // An interrupted initialization/restoration must retain its exact preparation.
    if let Some(pending) = &record.pending {
        let preparation: launch::Preparation = serde_json::from_slice(pending)?;
        let restoring = !preparation.restore.is_empty();
        launch::prepare(store, None, Product::Agent).await?;
        if restoring {
            return Ok(store
                .read_application_metadata(record.deployment)
                .await?
                .ok_or("UI source missing")?
                .source
                .reference);
        }
        // Finishing initialization preserves later heads. The requested restore
        // must still replace those heads with the installation baseline below.
    }
    let connections = serde_json::from_value(record.configuration.clone())?;
    let (path, domain, host_id, management) = launch::targets(&connections)?;
    let host = launch::open(path, false, Some(domain)).await?;
    let result = async {
        let mut revisions = BTreeMap::new();
        let mut originals = Vec::new();
        let mut packages = BTreeMap::new();
        for (target, deployment) in [
            (&host, host_id),
            (store, management),
            (store, record.deployment),
        ] {
            let current = target
                .read_application_metadata(deployment)
                .await?
                .ok_or("product source missing")?;
            let mut baseline = current.source.reference.clone();
            baseline.revision = 1;
            let original = target
                .load_application_at(&baseline)
                .await?
                .ok_or("installation baseline missing")?;
            revisions.insert(deployment, current.source.reference.revision);
            for package in &original.packages {
                packages.insert(package.application_ref(), package.clone());
            }
            originals.push(original);
        }
        let ui = originals.pop().unwrap();
        let management = originals.pop().unwrap();
        let host = originals.pop().unwrap();
        let preparation = launch::Preparation {
            product: None,
            previous_connections: None,
            restore: revisions,
            sources: Sources {
                host: host.source.input,
                management_host: management.source.input,
                ui: ui.source.input,
                remote_exports: ui.source.remote_exports,
                connections: record.configuration,
            },
            packages: packages
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
        Ok::<_, HostError>(())
    }
    .await;
    host.drain().await;
    drop(host);
    result?;
    launch::prepare(store, None, Product::Agent).await?;
    Ok(store
        .read_application_metadata(record.deployment)
        .await?
        .ok_or("UI source missing")?
        .source
        .reference)
}

pub(super) async fn ensure(
    store: &EventLog,
    deployment: Uuid,
    expected: u64,
    input: &Input,
    observations: &[RemoteObservation],
    packages: &[Package],
) -> Result<(), HostError> {
    let current = store
        .load_application(deployment)
        .await?
        .ok_or("restore source missing")?;
    let mut actual: Vec<_> = current
        .packages
        .iter()
        .map(Package::application_ref)
        .collect();
    let mut desired: Vec<_> = packages.iter().map(Package::application_ref).collect();
    actual.sort();
    desired.sort();
    let matches = actual == desired
        && serde_json::to_value(&current.source.input)? == serde_json::to_value(input)?
        && serde_json::to_value(&current.source.remote_exports)?
            == serde_json::to_value(observations)?;
    if matches
        && (current.source.reference.revision == expected
            || current.source.reference.revision == expected + 1)
    {
        return Ok(());
    }
    if current.source.reference.revision != expected {
        return Err("restoration conflicts with a later source revision".into());
    }
    store
        .commit_application(
            deployment,
            expected,
            input.clone(),
            observations.to_vec(),
            packages.to_vec(),
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use maka_event_log::applications::launch::Launch;
    use maka_plugin_core::composition::{Environment, ExecutionLocation};
    use serde_json::json;

    fn input() -> Input {
        Input {
            environment: Environment {
                location: ExecutionLocation::Host,
                presentation: None,
            },
            instances: BTreeMap::new(),
            launch: None,
            recovery: None,
        }
    }
    #[tokio::test]
    async fn restore_preserves_original_identity_history_and_resumes_without_replay() {
        let directory = tempfile::tempdir().unwrap();
        let store = launch::open(directory.path().join("ui"), true, None)
            .await
            .unwrap();
        let host_path = directory.path().join("host");
        let host = launch::open(host_path.clone(), true, None).await.unwrap();
        let ui = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let manager = Uuid::new_v4();
        let owner = host.owned_root().unwrap();
        let endpoint = json!({"service":{"consumer":[Uuid::new_v4()],"import":"service","grant":{"methods":["read"],"recovery":true}}});
        let configuration = json!({"hosts":[
            {"name":"local","location":{"kind":"process","root":owner.canonical_path(),"stateDomain":owner.root_id()},"deployment":host_id,"endpoints":endpoint},
            {"name":"manager","location":{"kind":"cohost"},"deployment":manager,"endpoints":endpoint}
        ]});
        drop(owner);
        for (target, id) in [(&host, host_id), (&store, manager), (&store, ui)] {
            target
                .commit_application(id, 0, input(), vec![], vec![])
                .await
                .unwrap();
            // A different valid environment is sufficient to distinguish desired source.
            let mut changed = input();
            changed.environment.location = ExecutionLocation::Ui;
            changed.environment.presentation =
                Some(maka_plugin_core::composition::PresentationTarget::Tui);
            target
                .commit_application(id, 1, changed, vec![], vec![])
                .await
                .unwrap();
        }
        store
            .begin_application_launch(Launch {
                deployment: ui,
                configuration: configuration.clone(),
                pending: Some(
                    serde_json::to_vec(&launch::Preparation {
                        product: None,
                        previous_connections: None,
                        restore: BTreeMap::new(),
                        sources: Sources {
                            host: input(),
                            management_host: input(),
                            ui: input(),
                            remote_exports: vec![],
                            connections: configuration.clone(),
                        },
                        packages: vec![],
                    })
                    .unwrap(),
                ),
            })
            .await
            .unwrap();
        host.drain().await;
        drop(host);
        let restored = restore(&store).await.unwrap();
        assert_eq!(restored.deployment, ui);
        assert_eq!(restored.revision, 3);
        let record = store.application_launch().await.unwrap().unwrap();
        assert_eq!(record.configuration, configuration);
        assert!(record.pending.is_none());
        let old = store
            .load_application_at(&DeploymentRef {
                revision: 2,
                ..restored.clone()
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(old.source.input.environment.location, ExecutionLocation::Ui);
        assert_eq!(restore(&store).await.unwrap(), restored); // Already restored: no new revision.

        let host = launch::open(host_path.clone(), false, None).await.unwrap();
        let preparation = launch::Preparation {
            product: None,
            previous_connections: None,
            restore: BTreeMap::from([(host_id, 3), (manager, 3), (ui, 3)]),
            sources: Sources {
                host: input(),
                management_host: input(),
                ui: input(),
                remote_exports: vec![],
                connections: configuration,
            },
            packages: vec![],
        };
        store
            .begin_application_restore(ui, serde_json::to_vec(&preparation).unwrap())
            .await
            .unwrap();
        // Simulate a committed first target whose acknowledgement was lost.
        host.commit_application(host_id, 3, input(), vec![], vec![])
            .await
            .unwrap();
        host.drain().await;
        drop(host);
        assert_eq!(restore(&store).await.unwrap(), restored);
        let host = launch::open(host_path, false, None).await.unwrap();
        assert_eq!(
            host.read_application_metadata(host_id)
                .await
                .unwrap()
                .unwrap()
                .source
                .reference
                .revision,
            4
        );
        assert!(ensure(&host, host_id, 2, &input(), &[], &[]).await.is_err());
        host.drain().await;
        store.drain().await;
    }
}
