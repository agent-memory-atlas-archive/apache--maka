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
//! A public Host opens the same persisted product source across process restarts.
use super::{HostError, Input, Product, author, installation, launch, packages, selected};
use base64::{Engine, engine::general_purpose::STANDARD};
use maka_event_log::{EventLog, applications::launch::Launch, root::RootOwner};
use maka_runtime_host::{
    DirectoryRootSpec,
    application::{
        host::{Host, Options},
        transport::{HostEndpoint, NativeGrant},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use uuid::Uuid;

/// Managed deployment directories are authoritative; standalone reopen preserves source edits.
pub(crate) enum Roots {
    Saved,
    Configured(Option<Vec<DirectoryRootSpec>>),
}
fn directories(roots: Option<Vec<DirectoryRootSpec>>) -> Result<Vec<DirectoryRootSpec>, HostError> {
    DirectoryRootSpec::normalize(match roots {
        Some(roots) => roots,
        None => vec![DirectoryRootSpec {
            label: "~".into(),
            path: crate::serve::home_directory()?
                .ok_or("public Host needs a home directory or explicit project directories")?,
        }],
    })
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Selection {
    PublicHost {
        endpoints: BTreeMap<String, HostEndpoint>,
        native: Vec<NativeGrant>,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preparation {
    input: Input,
    packages: Vec<BTreeMap<String, String>>,
}

pub(crate) async fn open_host(
    owner: RootOwner,
    options: Options,
    roots: Roots,
) -> Result<Arc<Host>, HostError> {
    let store = Arc::new(maka_application_bridge::open_store(Arc::new(owner)).await?);
    let result = async {
        let (deployment, Selection::PublicHost { endpoints, native }) =
            prepare(&store, roots).await?;
        Ok(Host::open_with_options(
            store.clone(),
            deployment,
            crate::application::serve::native()?,
            endpoints,
            native,
            options,
        )
        .await?)
    }
    .await;
    if result.is_err() {
        store.drain().await;
    }
    result
}
async fn prepare(store: &EventLog, roots: Roots) -> Result<(Uuid, Selection), HostError> {
    if store.application_launch().await?.is_none() {
        let initial = match &roots {
            Roots::Saved => None,
            Roots::Configured(roots) => roots.clone(),
        };
        begin(store, initial).await?;
    }
    let record = store
        .application_launch()
        .await?
        .ok_or("public Host selection missing")?;
    // A terminal launcher cannot accidentally reinterpret this Root as its Host.
    let selection: Selection = serde_json::from_value(record.configuration)?;
    if let Some(pending) = record.pending {
        let preparation: Preparation = serde_json::from_slice(&pending)?;
        let packages = launch::decode_packages(preparation.packages)?;
        launch::ensure(store, record.deployment, &preparation.input, &[], &packages).await?;
        store.complete_application_launch(record.deployment).await?;
    }
    if let Roots::Configured(roots) = roots {
        apply_directories(store, record.deployment, directories(roots)?).await?;
    }
    Ok((record.deployment, selection))
}
async fn begin(store: &EventLog, roots: Option<Vec<DirectoryRootSpec>>) -> Result<(), HostError> {
    let home = crate::serve::home_directory()?;
    let roots = directories(roots)?;
    let root = store.owned_root()?;
    let mut configuration = installation(json!({
        "workspace": roots.first().map(|root| root.path.as_path()).unwrap_or(root.canonical_path()), "hostRoot": root.canonical_path(),
        "helper": std::env::current_exe()?, "themeFile": root.canonical_path().join("appearance/theme.json"),
        "home": home,
    })).await?;
    configuration["agent"]["workspaceRoots"] = serde_json::to_value(&roots)?;
    configuration["agent"]["permissions"]["access"] = json!("read_only");
    configuration["skills"]["projectRoots"] = serde_json::to_value(&roots)?;
    configuration["processes"]["filesystem"]["rules"] = Value::Array(
        roots
            .iter()
            .map(|root| json!({"path": root.path, "scope": "subtree", "access": "write"}))
            .collect(),
    );
    let packages = tokio::task::spawn_blocking(packages).await??;
    let (sources, _) = author(Product::Agent, configuration, &packages).await?;
    let connections: maka_application_bridge::Configuration =
        serde_json::from_value(sources.connections)?;
    let connection = connections
        .hosts
        .into_iter()
        .find(|host| host.name == "local")
        .ok_or("product public Host missing")?;
    let packages = selected(
        &sources.host,
        &[],
        &packages.into_values().collect::<Vec<_>>(),
    )?;
    let selection = Selection::PublicHost {
        endpoints: connection.endpoints,
        native: connection.native,
    };
    let preparation = Preparation {
        input: sources.host,
        packages: packages
            .iter()
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
        .begin_application_launch(Launch {
            deployment: connection.deployment,
            configuration: serde_json::to_value(selection)?,
            pending: Some(serde_json::to_vec(&preparation)?),
        })
        .await?;
    Ok(())
}

async fn apply_directories(
    store: &EventLog,
    deployment: Uuid,
    roots: Vec<DirectoryRootSpec>,
) -> Result<(), HostError> {
    let metadata = store
        .read_application_metadata(deployment)
        .await?
        .ok_or("public Host source missing")?;
    let roots = serde_json::to_value(roots)?;
    let current = metadata
        .source
        .input
        .instances
        .get("agent")
        .and_then(|node| node.configuration.values.get("workspaceRoots"));
    if current == Some(&roots) {
        return Ok(());
    }
    // Read payloads only for an actual configuration change, never on ordinary reopen.
    let mut loaded = store
        .load_application_at(&metadata.source.reference)
        .await?
        .ok_or("public Host source missing")?;
    let input = &mut loaded.source.input;
    configuration_values(
        input,
        &loaded.packages,
        "agent",
        maka_agent_service::IMPLEMENTATION,
    )?
    .insert("workspaceRoots".into(), roots.clone());
    if input.instances.contains_key("skills") {
        configuration_values(
            input,
            &loaded.packages,
            "skills",
            maka_agent_service::skills::IMPLEMENTATION,
        )?
        .insert("projectRoots".into(), roots.clone());
    }
    if input.instances.contains_key("workspace-processes") {
        let filesystem = configuration_values(
            input,
            &loaded.packages,
            "workspace-processes",
            maka_agent_service::processes::IMPLEMENTATION,
        )?
        .get_mut("filesystem")
        .and_then(Value::as_object_mut)
        .ok_or("public Host filesystem ceiling missing")?;
        let previous = current
            .and_then(Value::as_array)
            .ok_or("saved published directories missing")?;
        let rules = filesystem
            .get_mut("rules")
            .and_then(Value::as_array_mut)
            .ok_or("saved filesystem rules missing")?;
        // Replace the published roots' write ceiling, preserving narrower
        // user-authored read/deny rules and unrelated filesystem restrictions.
        rules.retain(|rule| {
            !(rule["scope"] == "subtree"
                && rule["access"] == "write"
                && previous.iter().any(|root| root["path"] == rule["path"]))
        });
        for root in roots.as_array().expect("directory list") {
            if !rules.iter().any(|rule| rule["path"] == root["path"]) {
                rules.push(json!({"path": root["path"], "scope": "subtree", "access": "write"}));
            }
        }
    }
    store
        .commit_application(
            deployment,
            metadata.source.reference.revision,
            loaded.source.input,
            loaded.source.remote_exports,
            loaded.packages,
        )
        .await?;
    Ok(())
}
fn configuration_values<'a>(
    input: &'a mut Input,
    packages: &[super::Package],
    name: &str,
    expected: &str,
) -> Result<&'a mut BTreeMap<String, Value>, HostError> {
    use maka_plugin_core::{
        composition::PackageUse,
        package::{EntryTarget, Implementation, PackageSelection},
    };
    let node = input
        .instances
        .get_mut(name)
        .ok_or("managed public Host component was removed")?;
    let PackageSelection::Pinned { package } = &node.package else {
        return Err("managed public Host component must be pinned".into());
    };
    let native = packages
        .iter()
        .find(|bytes| bytes.application_ref() == *package)
        .and_then(|bytes| bytes.manifest().application.as_ref())
        .and_then(|declaration| declaration.entries.get(&EntryTarget::Host));
    if node.use_package != PackageUse::Entry
        || !node.configuration.parameters.is_empty()
        || !matches!(native, Some(entry) if matches!(&entry.implementation, Implementation::Native { implementation } if implementation == expected))
    {
        return Err("managed directories no longer target the saved native Host components".into());
    }
    Ok(&mut node.configuration.values)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn public_host_resumes_original_source_and_starts_with_read_only_permissions() {
        let directory = tempfile::tempdir().unwrap();
        let namespaces = maka_event_log::root::RootNamespaces {
            ownership: directory.path().join("owners"),
            control: directory.path().join("control"),
        };
        let root = directory.path().join("root");
        let workspace = directory.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();
        let store = EventLog::for_root(Arc::new(RootOwner::create(&root, &namespaces).unwrap()))
            .await
            .unwrap();
        let roots = vec![DirectoryRootSpec {
            label: "Workspace".into(),
            path: workspace.clone(),
        }];
        begin(&store, Some(roots)).await.unwrap();
        let record = store.application_launch().await.unwrap().unwrap();
        assert!(record.pending.is_some());
        let selected_id = record.deployment;
        let (deployment, Selection::PublicHost { endpoints, .. }) =
            prepare(&store, Roots::Saved).await.unwrap();
        assert_eq!(deployment, selected_id);
        assert!(endpoints.contains_key("agent"));
        assert!(
            store
                .application_launch()
                .await
                .unwrap()
                .unwrap()
                .pending
                .is_none()
        );
        let original = store.load_application(deployment).await.unwrap().unwrap();
        let agent = original.source.input.instances.get("agent").unwrap();
        let config = serde_json::to_value(agent).unwrap();
        assert_eq!(
            config["configuration"]["values"]["permissions"]["access"],
            "read_only"
        );
        assert_eq!(
            config["configuration"]["values"]["workspaceRoots"][0]["path"],
            serde_json::to_value(workspace.canonicalize().unwrap()).unwrap()
        );
        assert!(original.packages.iter().all(|package| {
            package
                .manifest()
                .application
                .as_ref()
                .unwrap()
                .entries
                .contains_key(&maka_plugin_core::package::EntryTarget::Host)
        }));
        let revision = original.source.reference.revision;
        let (reopened, _) = prepare(&store, Roots::Saved).await.unwrap();
        assert_eq!(reopened, deployment);
        assert_eq!(
            store
                .load_application(reopened)
                .await
                .unwrap()
                .unwrap()
                .source
                .reference
                .revision,
            revision
        );
        let mut customized = original.source.input.clone();
        let restriction =
            json!({"path": workspace.join("private"), "scope": "subtree", "access": "deny"});
        customized
            .instances
            .get_mut("workspace-processes")
            .unwrap()
            .configuration
            .values
            .get_mut("filesystem")
            .unwrap()["rules"]
            .as_array_mut()
            .unwrap()
            .push(restriction.clone());
        store
            .commit_application(
                deployment,
                revision,
                customized,
                original.source.remote_exports.clone(),
                original.packages.clone(),
            )
            .await
            .unwrap();
        prepare(&store, Roots::Configured(Some(vec![])))
            .await
            .unwrap();
        let changed = store.load_application(deployment).await.unwrap().unwrap();
        assert_eq!(changed.source.reference.revision, revision + 2);
        assert_eq!(
            changed.source.input.instances["agent"].configuration.values["workspaceRoots"],
            json!([])
        );
        assert_eq!(
            changed.source.input.instances["skills"]
                .configuration
                .values["projectRoots"],
            json!([])
        );
        assert_eq!(
            changed.source.input.instances["workspace-processes"]
                .configuration
                .values["filesystem"]["rules"],
            json!([restriction])
        );
        assert_eq!(
            changed.source.input.instances["agent"].configuration.values["permissions"]["access"],
            "read_only"
        );
        prepare(&store, Roots::Configured(Some(vec![])))
            .await
            .unwrap();
        prepare(&store, Roots::Saved).await.unwrap();
        assert_eq!(
            store
                .read_application_metadata(deployment)
                .await
                .unwrap()
                .unwrap()
                .source
                .reference,
            changed.source.reference
        );
        store.drain().await;
    }
}
