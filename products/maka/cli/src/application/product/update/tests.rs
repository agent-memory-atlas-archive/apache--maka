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

async fn old_installation(path: &std::path::Path, standard: bool) -> (EventLog, Uuid) {
    let store = launch::open(path.join("ui"), true, None).await.unwrap();
    let host = launch::open(path.join("host"), true, None).await.unwrap();
    let options = json!({"workspace": path, "hostRoot": path.join("host"), "themeFile": path.join("theme.json"), "helper": std::env::current_exe().unwrap(), "locale":"zh-CN"});
    let mut config = super::super::installation(options.clone()).await.unwrap();
    // Previous ceilings blocked network and external-write approval requests.
    for processes in ["processes", "coordinatorProcesses"] {
        config[processes]["network"] = json!("denied");
        config[processes]["filesystem"] = json!({"default":"read", "rules":[
            {"path":path,"scope":"subtree","access":"write"}
        ],"denyGlobs":[]});
    }
    // Previous defaults had no Computer Use service or model-visible tools.
    config.as_object_mut().unwrap().remove("computer");
    for agent in ["agent", "coordinator"] {
        config[agent]["tools"]
            .as_array_mut()
            .unwrap()
            .retain(|tool| tool["import"] != "computer");
    }
    let mut packages = packages().unwrap();
    let package = packages.get("agent-screen").unwrap();
    let mut files = package.files().clone();
    files
        .get_mut("tui.js")
        .unwrap()
        .extend_from_slice(b"\n// previous product release\n");
    packages.insert("agent-screen".into(), Package::new(files).unwrap());
    let (mut sources, installation) = super::super::author_with_source(
        Product::Agent,
        config,
        &packages,
        &super::super::authoring().unwrap(),
    )
    .await
    .unwrap();
    for endpoint in sources.connections["hosts"].as_array_mut().unwrap() {
        if endpoint["location"]["kind"] == "process" {
            endpoint["location"]["stateDomain"] = json!(host.owned_root().unwrap().root_id());
        }
    }
    let ui = Uuid::new_v4();
    let preparation = launch::Preparation {
        product: Some(Baseline {
            product: Product::Agent,
            version: None,
            installation,
            default_options: standard.then_some(options),
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
async fn explicit_installation_keeps_its_selected_tools_when_packages_upgrade() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = old_installation(directory.path(), false).await;
    assert!(begin(&store).await.unwrap());
    let record = store.application_launch().await.unwrap().unwrap();
    let preparation: launch::Preparation =
        serde_json::from_slice(record.pending.as_ref().unwrap()).unwrap();
    for processes in ["workspace-processes", "coordinator-workspace-processes"] {
        assert_eq!(
            preparation.sources.host.instances[processes]
                .configuration
                .values["network"],
            "denied"
        );
        assert_eq!(
            preparation.sources.host.instances[processes]
                .configuration
                .values["filesystem"]["default"],
            "read"
        );
    }
    assert!(
        !preparation
            .sources
            .host
            .instances
            .contains_key("workspace-computer")
    );
    let tools = preparation.sources.host.instances["agent"]
        .configuration
        .values["tools"]
        .as_array()
        .unwrap();
    assert!(!tools.iter().any(|tool| tool["import"] == "computer"));
    launch::prepare(&store, None, Product::Agent).await.unwrap();
    assert!(!begin(&store).await.unwrap());
    store.drain().await;
}

#[tokio::test]
#[ignore = "requires the built bundled product"]
async fn defaults_update_once_and_resume_an_acknowledgement_lost_after_commit() {
    let directory = tempfile::tempdir().unwrap();
    let (store, ui) = old_installation(directory.path(), true).await;
    let original = store.load_application(ui).await.unwrap().unwrap();
    let (original_source, _, _, _) = original.into_parts();
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
    for processes in ["workspace-processes", "coordinator-workspace-processes"] {
        assert_eq!(
            preparation.sources.host.instances[processes]
                .configuration
                .values["network"],
            "allowed"
        );
        assert_eq!(
            preparation.sources.host.instances[processes]
                .configuration
                .values["filesystem"]["default"],
            "write"
        );
    }
    assert!(
        preparation
            .sources
            .host
            .instances
            .contains_key("workspace-computer")
    );
    let tools = preparation.sources.host.instances["agent"]
        .configuration
        .values["tools"]
        .as_array()
        .unwrap();
    for name in ["cua_repl", "cua_reset"] {
        assert!(tools.iter().any(|tool| tool["name"] == name));
    }
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
    let (current_source, _, _, _) = current.into_parts();
    assert_eq!(current_source.reference.revision, 2);
    for (name, node) in &original_source.input.instances {
        assert_eq!(node.key, current_source.input.instances[name].key);
    }
    assert!(
        store
            .load_application_at(&original_source.reference)
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
    let (store, ui) = old_installation(directory.path(), true).await;
    let current = store.load_application(ui).await.unwrap().unwrap();
    let (current_source, current_packages, _, _) = current.into_parts();
    let mut customized = current_source.input;
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
            current_source.remote_exports,
            current_packages,
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
    let (current_source, _, _, _) = current.into_parts();
    assert_eq!(
        current_source.input.instances["appearance"]
            .configuration
            .values["scheme"],
        json!("light")
    );
    assert_eq!(current_source.reference.revision, 3);
    let preview = synchronize(&store, true, false).await.unwrap();
    assert!(preview.customized && !preview.changed);
    assert!(!begin(&store).await.unwrap());
    store.drain().await;
}

#[tokio::test]
#[ignore = "requires the built bundled product"]
async fn conflicting_custom_plugin_does_not_prepare_or_change_any_head() {
    let directory = tempfile::tempdir().unwrap();
    let (store, ui) = old_installation(directory.path(), true).await;
    let current = store.load_application(ui).await.unwrap().unwrap();
    let (current_source, current_packages, _, _) = current.into_parts();
    let old_ref = match &current_source.input.instances["agent-screen"].package {
        maka_plugin_core::package::PackageSelection::Pinned { package } => package.clone(),
        _ => unreachable!(),
    };
    let mut packages = current_packages;
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
    let mut input = current_source.input;
    for node in input.instances.values_mut() {
        if node.package == maka_plugin_core::package::PackageSelection::from(old_ref.clone()) {
            node.package = packages[at].application_ref().into();
        }
    }
    store
        .commit_application(ui, 1, input, current_source.remote_exports, packages)
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
            .source()
            .reference
            .revision,
        2
    );
    store.drain().await;
}

#[test]
fn product_version_covers_authoring_and_each_named_package() {
    let version = |source, packages: &[(&str, &str)]| {
        super::super::fingerprint(
            source,
            packages
                .iter()
                .map(|(name, digest)| (name.to_string(), digest.to_string())),
        )
    };
    let original = version("author", &[("a", "one"), ("b", "two")]);
    assert_eq!(original, version("author", &[("b", "two"), ("a", "one")]));
    assert_ne!(original, version("changed", &[("a", "one"), ("b", "two")]));
    assert_ne!(
        original,
        version("author", &[("a", "changed"), ("b", "two")])
    );
    assert_ne!(
        original,
        version("author", &[("renamed", "one"), ("b", "two")])
    );
    assert_ne!(original, version("author", &[("a", "one")]));
}

#[tokio::test]
#[ignore = "requires the built bundled product"]
async fn unchanged_product_records_version_without_rewriting_deployments() {
    let directory = tempfile::tempdir().unwrap();
    let (store, ui) = old_installation(directory.path(), true).await;
    assert!(begin(&store).await.unwrap());
    launch::prepare(&store, None, Product::Agent).await.unwrap();
    let original = store.application_product().await.unwrap().unwrap();
    let mut uncached = original.clone();
    uncached.as_object_mut().unwrap().remove("version");
    assert!(
        store
            .compare_and_set_application_product(original, uncached)
            .await
            .unwrap()
    );
    let revision = store
        .read_application_metadata(ui)
        .await
        .unwrap()
        .unwrap()
        .source
        .reference;
    assert!(!begin(&store).await.unwrap());
    let saved: Baseline =
        serde_json::from_value(store.application_product().await.unwrap().unwrap()).unwrap();
    assert_eq!(
        saved.version,
        Some(super::super::bundle_fingerprint().unwrap())
    );
    assert!(!begin(&store).await.unwrap());
    assert_eq!(
        store
            .read_application_metadata(ui)
            .await
            .unwrap()
            .unwrap()
            .source
            .reference,
        revision
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
    store.drain().await;
}

#[test]
fn generated_product_version_matches_canonical_unicode_and_binary_vectors() {
    let files = [
        ("z", &[0, 255][..]),
        ("😀.js", "你好".as_bytes()),
        ("\u{e000}.js", b"x".as_slice()),
    ];
    let digest = maka_packages::content_digest(files);
    assert_eq!(
        digest,
        "sha256-1be108087645221ec647c9ce43ad7d10de9d6433c533fb2b67e1bb93609d302c"
    );
    assert_eq!(
        super::super::fingerprint(
            "author 😀\n",
            [("b".into(), "two".into()), ("a".into(), digest)]
        ),
        "sha256-41298191ef2403397572e03ff6aa1b82f8d5f839f32260bfa02925628406d86e"
    );
}

#[test]
fn generated_version_reads_no_packages_and_rejects_invalid_metadata() {
    let directory = tempfile::tempdir().unwrap();
    assert!(super::super::read_bundle_fingerprint(directory.path()).is_err());
    let version = super::super::fingerprint("author", []);
    // There is deliberately no authoring or packages directory to scan.
    std::fs::write(
        directory.path().join("product-version.txt"),
        format!("{version}\n"),
    )
    .unwrap();
    assert_eq!(
        super::super::read_bundle_fingerprint(directory.path()).unwrap(),
        version
    );
    for invalid in [
        "sha256-wrong".to_owned(),
        version.to_uppercase(),
        format!("{version}\nextra"),
    ] {
        std::fs::write(directory.path().join("product-version.txt"), invalid).unwrap();
        assert!(super::super::read_bundle_fingerprint(directory.path()).is_err());
    }
}

#[test]
fn stale_generated_version_cannot_admit_changed_candidates() {
    let manifest = json!({"schemaVersion":1,"id":"example.notes","application":{"entries":{"host":{"implementation":{"kind":"javascript","module":"entry.js","realm":"shared"},"configuration":{"type":"object"}}}}});
    let package = Package::new(BTreeMap::from([
        (
            "maka.extension.json".into(),
            serde_json::to_vec(&manifest).unwrap(),
        ),
        ("entry.js".into(), b"export function main() {}".to_vec()),
    ]))
    .unwrap();
    let packages = BTreeMap::from([("notes".to_owned(), package)]);
    let version = super::super::fingerprint(
        "author",
        packages
            .iter()
            .map(|(name, package)| (name.clone(), package.digest().to_owned())),
    );
    assert_eq!(
        super::super::verified_fingerprint("author", &packages, &version).unwrap(),
        version
    );
    assert!(super::super::verified_fingerprint("changed author", &packages, &version).is_err());
    let mut changed = packages.clone();
    let mut files = changed["notes"].files().clone();
    files
        .get_mut("entry.js")
        .unwrap()
        .extend_from_slice(b"\n// changed");
    changed.insert("notes".into(), Package::new(files).unwrap());
    assert!(super::super::verified_fingerprint("author", &changed, &version).is_err());
}

#[tokio::test]
async fn matching_reopen_skips_baseline_sources_and_changed_version_requires_them() {
    let directory = tempfile::tempdir().unwrap();
    let store = launch::open(directory.path().join("ui"), true, None)
        .await
        .unwrap();
    let deployment = Uuid::new_v4();
    store
        .begin_application_launch(Launch {
            deployment,
            configuration: json!({}),
            pending: Some(vec![1]),
        })
        .await
        .unwrap();
    let version = super::super::fingerprint("author", []);
    // A matching reopen needs neither typed baseline sources nor Host/package access.
    store
        .complete_application_product(deployment, json!({}), json!({"version": version}))
        .await
        .unwrap();
    let outcome = synchronize_version(&store, false, true, version.clone())
        .await
        .unwrap();
    assert!(!outcome.changed && !outcome.applied);
    assert!(
        synchronize_version(
            &store,
            false,
            true,
            super::super::fingerprint("changed", [])
        )
        .await
        .is_err()
    );
    assert!(
        synchronize_version(&store, true, false, version)
            .await
            .is_err()
    );
    store.drain().await;
}
