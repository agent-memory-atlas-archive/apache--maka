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

use super::*;
use maka_event_log::applications::launch::Launch;
use maka_packages::Package;
use serde_json::json;
use uuid::Uuid;

#[test]
fn three_way_merge_preserves_customizations_and_reports_ambiguous_changes() {
    let base = json!({"plugin":{"package":"v1","values":{"theme":"dark","language":"en"}},"removed":1,"list":[1]});
    let local = json!({"plugin":{"package":"v1","values":{"theme":"light","language":"en"}},"removed":1,"list":[1],"custom":true});
    let next = json!({"plugin":{"package":"v2","values":{"theme":"dark","language":"zh-CN"}},"list":[1,2],"added":true});
    let mut outcome = Outcome::default();
    let merged = merge(Some(&base), Some(&local), Some(&next), "", &mut outcome).unwrap();
    assert_eq!(
        merged,
        json!({"plugin":{"package":"v2","values":{"theme":"light","language":"zh-CN"}},"list":[1,2],"custom":true,"added":true})
    );
    assert!(outcome.conflicts.is_empty());
    let mut local = local;
    local["plugin"]["values"]["language"] = json!("zh-TW");
    local["list"] = json!([3]);
    local["removed"] = json!(2);
    let mut outcome = Outcome::default();
    merge(Some(&base), Some(&local), Some(&next), "", &mut outcome);
    assert_eq!(
        outcome.conflicts,
        ["/list", "/plugin/values/language", "/removed"]
    );
    let mut outcome = Outcome::default();
    merge(Some(&base), None, Some(&next), "/deleted", &mut outcome);
    assert_eq!(outcome.conflicts, ["/deleted"]);
}

async fn old_installation(path: &std::path::Path) -> (EventLog, Uuid) {
    let store = launch::open(path.join("ui"), true, None).await.unwrap();
    let host = launch::open(path.join("host"), true, None).await.unwrap();
    let options = json!({"workspace": path, "hostRoot": path.join("host"), "themeFile": path.join("theme.json"), "helper": std::env::current_exe().unwrap(), "locale":"zh-CN"});
    let config = super::super::installation(options).await.unwrap();
    let mut packages = packages().unwrap();
    let package = packages.get("agent-screen").unwrap();
    let mut files = package.files().clone();
    files
        .get_mut("tui.js")
        .unwrap()
        .extend_from_slice(b"\n// previous product release\n");
    packages.insert("agent-screen".into(), Package::new(files).unwrap());
    let (mut sources, installation) = author(Product::Agent, config, &packages).await.unwrap();
    for endpoint in sources.connections["hosts"].as_array_mut().unwrap() {
        if endpoint["location"]["kind"] == "process" {
            endpoint["location"]["stateDomain"] = json!(host.owned_root().unwrap().root_id());
        }
    }
    let ui = Uuid::new_v4();
    let preparation = launch::Preparation {
        product: Some(Baseline {
            product: Product::Agent,
            installation,
            sources: sources.clone(),
        }),
        previous_connections: None,
        restore: BTreeMap::new(),
        sources,
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
        .begin_application_launch(Launch {
            deployment: ui,
            configuration: preparation.sources.connections.clone(),
            pending: Some(serde_json::to_vec(&preparation).unwrap()),
        })
        .await
        .unwrap();
    host.drain().await;
    drop(host);
    launch::prepare(&store, None, Product::Agent).await.unwrap();
    (store, ui)
}

#[tokio::test]
#[ignore = "requires the built bundled product"]
async fn defaults_update_once_and_resume_an_acknowledgement_lost_after_commit() {
    let directory = tempfile::tempdir().unwrap();
    let (store, ui) = old_installation(directory.path()).await;
    let original = store.load_application(ui).await.unwrap().unwrap();
    let preview = synchronize(&store, true, false).await.unwrap();
    assert!(preview.changed && !preview.customized && preview.conflicts.is_empty());
    assert!(
        preview
            .changes
            .iter()
            .any(|path| path.contains("agent-screen"))
    );
    assert!(
        store
            .application_launch()
            .await
            .unwrap()
            .unwrap()
            .pending
            .is_none()
    );
    assert!(begin(&store).await.unwrap());
    let pending = store
        .application_launch()
        .await
        .unwrap()
        .unwrap()
        .pending
        .unwrap();
    let preparation: launch::Preparation = serde_json::from_slice(&pending).unwrap();
    let available = launch::decode_packages(preparation.packages).unwrap();
    let selected = super::super::selected(
        &preparation.sources.ui,
        &preparation.sources.remote_exports,
        &available,
    )
    .unwrap();
    // Simulate a source commit whose acknowledgement was lost before publication.
    store
        .commit_application(
            ui,
            1,
            preparation.sources.ui.clone(),
            preparation.sources.remote_exports.clone(),
            selected,
        )
        .await
        .unwrap();
    launch::prepare(&store, None, Product::Agent).await.unwrap();
    let current = store.load_application(ui).await.unwrap().unwrap();
    assert_eq!(current.source.reference.revision, 2);
    for (name, node) in &original.source.input.instances {
        assert_eq!(node.key, current.source.input.instances[name].key);
    }
    assert!(
        store
            .load_application_at(&original.source.reference)
            .await
            .unwrap()
            .is_some()
    );
    assert!(!begin(&store).await.unwrap());
    assert!(
        store
            .application_launch()
            .await
            .unwrap()
            .unwrap()
            .pending
            .is_none()
    );
    store.drain().await;
}

#[tokio::test]
#[ignore = "requires the built bundled product"]
async fn explicit_sync_keeps_custom_configuration_and_advances_the_default_baseline() {
    let directory = tempfile::tempdir().unwrap();
    let (store, ui) = old_installation(directory.path()).await;
    let current = store.load_application(ui).await.unwrap().unwrap();
    let mut customized = current.source.input;
    customized
        .instances
        .get_mut("appearance")
        .unwrap()
        .configuration
        .values
        .insert("scheme".into(), json!("light"));
    store
        .commit_application(
            ui,
            1,
            customized,
            current.source.remote_exports,
            current.packages,
        )
        .await
        .unwrap();
    assert!(!begin(&store).await.unwrap()); // Normal reopen cannot change a customized product.
    let preview = synchronize(&store, true, false).await.unwrap();
    assert!(preview.customized && preview.changed && preview.conflicts.is_empty());
    assert!(
        store
            .application_launch()
            .await
            .unwrap()
            .unwrap()
            .pending
            .is_none()
    );
    synchronize(&store, true, true).await.unwrap();
    launch::prepare(&store, None, Product::Agent).await.unwrap();
    let current = store.load_application(ui).await.unwrap().unwrap();
    assert_eq!(
        current.source.input.instances["appearance"]
            .configuration
            .values["scheme"],
        json!("light")
    );
    assert_eq!(current.source.reference.revision, 3);
    let preview = synchronize(&store, true, false).await.unwrap();
    assert!(preview.customized && !preview.changed);
    assert!(!begin(&store).await.unwrap());
    store.drain().await;
}

#[tokio::test]
#[ignore = "requires the built bundled product"]
async fn conflicting_custom_plugin_does_not_prepare_or_change_any_head() {
    let directory = tempfile::tempdir().unwrap();
    let (store, ui) = old_installation(directory.path()).await;
    let current = store.load_application(ui).await.unwrap().unwrap();
    let old_ref = match &current.source.input.instances["agent-screen"].package {
        maka_plugin_core::package::PackageSelection::Pinned { package } => package.clone(),
        _ => unreachable!(),
    };
    let mut packages = current.packages;
    let at = packages
        .iter()
        .position(|p| p.application_ref() == old_ref)
        .unwrap();
    let mut files = packages[at].files().clone();
    files
        .get_mut("tui.js")
        .unwrap()
        .extend_from_slice(b"\n// user customization\n");
    packages[at] = Package::new(files).unwrap();
    let mut input = current.source.input;
    for node in input.instances.values_mut() {
        if node.package == maka_plugin_core::package::PackageSelection::from(old_ref.clone()) {
            node.package = packages[at].application_ref().into();
        }
    }
    store
        .commit_application(ui, 1, input, current.source.remote_exports, packages)
        .await
        .unwrap();
    let baseline = store.application_product().await.unwrap();
    let result = synchronize(&store, true, true).await.unwrap();
    assert!(result.changed && result.customized && !result.conflicts.is_empty());
    assert!(
        result
            .conflicts
            .iter()
            .any(|p| p.contains("/agent-screen/package/"))
    );
    assert!(
        store
            .application_launch()
            .await
            .unwrap()
            .unwrap()
            .pending
            .is_none()
    );
    assert_eq!(store.application_product().await.unwrap(), baseline);
    assert_eq!(
        store
            .load_application(ui)
            .await
            .unwrap()
            .unwrap()
            .source
            .reference
            .revision,
        2
    );
    store.drain().await;
}
