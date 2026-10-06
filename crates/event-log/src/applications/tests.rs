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
use crate::root::{RootNamespaces, RootOwner};
use maka_plugin_core::composition::{Environment, ExecutionLocation};
use std::sync::Arc;

pub(super) fn input() -> Input {
    Input {
        environment: Environment {
            location: ExecutionLocation::Host,
            presentation: None,
        },
        instances: BTreeMap::new(),
        launch: None,
    }
}
pub(super) async fn store() -> (tempfile::TempDir, EventLog) {
    let dir = tempfile::tempdir().unwrap();
    let namespaces = RootNamespaces {
        ownership: dir.path().join("owners"),
        control: dir.path().join("control"),
    };
    let owner = Arc::new(RootOwner::create(&dir.path().join("root"), &namespaces).unwrap());
    let store = EventLog::for_root(owner).await.unwrap();
    (dir, store)
}

#[tokio::test]
async fn exact_source_cas_and_domain() {
    let (_dir, store) = store().await;
    let id = Uuid::new_v4();
    assert!(store.load_application(id).await.unwrap().is_none());
    let first = store
        .commit_application(id, 0, input(), vec![], vec![])
        .await
        .unwrap();
    first.validate().unwrap();
    assert_eq!(
        first.state_domain,
        store.root_owner.as_ref().unwrap().root_id()
    );
    let loaded = store.load_application(id).await.unwrap().unwrap();
    assert_eq!(loaded.source.reference, first);
    assert_eq!(
        serde_json::to_value(loaded.source.input).unwrap(),
        serde_json::to_value(input()).unwrap()
    );
    assert!(matches!(
        store
            .commit_application(id, 0, input(), vec![], vec![])
            .await,
        Err(StoreError::RevisionConflict { .. })
    ));
    let second = store
        .commit_application(id, 1, input(), vec![], vec![])
        .await
        .unwrap();
    assert_eq!(second.revision, 2);
    assert_eq!(
        store
            .load_application(id)
            .await
            .unwrap()
            .unwrap()
            .source
            .reference,
        second
    );
    assert_eq!(
        store
            .load_application_at(&first)
            .await
            .unwrap()
            .unwrap()
            .source
            .reference,
        first
    );
    let mut absent = first.clone();
    absent.revision = 3;
    assert!(store.load_application_at(&absent).await.unwrap().is_none());
    let mut foreign = first.clone();
    foreign.state_domain = "0".repeat(64);
    assert!(store.load_application_at(&foreign).await.is_err());
    store.close().await.unwrap();
}

#[tokio::test]
async fn oversized_source_is_rejected_before_materialization() {
    let (_dir, store) = store().await;
    let id = Uuid::new_v4();
    store
        .commit_application(id, 0, input(), vec![], vec![])
        .await
        .unwrap();
    store.connection.run(move |conn| Box::pin(async move {
        sqlx::query("UPDATE application_sources SET input_json = json_object('huge', printf('%.*c', 1048576, 'x')) WHERE deployment = ?")
            .bind(id.to_string()).execute(conn).await?;
        Ok(())
    })).await.unwrap();
    let error = store.load_application(id).await.err().unwrap().to_string();
    assert!(error.contains("512 KiB"), "{error}");
    assert!(encode(&"x".repeat(MAX_JSON), MAX_JSON).is_err());
    store.close().await.unwrap();
}

#[tokio::test]
async fn requires_real_state_root_and_non_nil_identity() {
    let dir = tempfile::tempdir().unwrap();
    let unowned = EventLog::open(&dir.path().join("test.sqlite"))
        .await
        .unwrap();
    assert!(
        unowned
            .commit_application(Uuid::new_v4(), 0, input(), vec![], vec![])
            .await
            .is_err()
    );
    unowned.close().await.unwrap();
    let (_dir, owned) = store().await;
    assert!(
        owned
            .commit_application(Uuid::nil(), 0, input(), vec![], vec![])
            .await
            .is_err()
    );
    assert!(
        owned
            .commit_application(
                Uuid::new_v4(),
                9_007_199_254_740_991,
                input(),
                vec![],
                vec![]
            )
            .await
            .is_err()
    );
    owned.close().await.unwrap();
}

fn package(marker: &str) -> Package {
    use maka_plugin_core::package::{
        Declaration, EntryDeclaration, EntryTarget, Implementation, Realm,
    };
    let declaration = Declaration {
        entries: BTreeMap::from([(
            EntryTarget::Host,
            EntryDeclaration {
                scene: None,
                implementation: Implementation::JavaScript {
                    module: "host.mjs".into(),
                    realm: Realm::Shared,
                },
                configuration: serde_json::json!({"type":"object"}),
                imports: BTreeMap::new(),
                exports: BTreeMap::new(),
            },
        )]),
        composition: None,
    };
    Package::new(BTreeMap::from([
        (
            maka_plugins::package::MANIFEST_FILE.into(),
            serde_json::to_vec(
                &serde_json::json!({"schemaVersion":2,"id":"sample","application":declaration}),
            )
            .unwrap(),
        ),
        ("host.mjs".into(), marker.as_bytes().to_vec()),
    ]))
    .unwrap()
}
fn selected(package: &Package) -> Input {
    use maka_plugin_core::composition::{Configuration, Node, PackageUse};
    let mut input = input();
    input.instances.insert(
        "view".into(),
        Node {
            key: Uuid::from_u128(1),
            package: package.application_ref().into(),
            use_package: PackageUse::Entry,
            configuration: Configuration::default(),
            parameters: Configuration::default(),
            bindings: BTreeMap::new(),
            disabled: false,
            overrides: vec![],
        },
    );
    input
}

#[tokio::test]
async fn immutable_bytes_preserve_every_committed_source_revision() {
    let (_dir, store) = store().await;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let old = package("export default {}; // original");
    let new = package("export default {}; // replacement");
    let original = store
        .commit_application(a, 0, selected(&old), vec![], vec![old.clone()])
        .await
        .unwrap();
    store
        .commit_application(b, 0, selected(&old), vec![], vec![old.clone()])
        .await
        .unwrap();
    store
        .commit_application(a, 1, selected(&new), vec![], vec![new.clone()])
        .await
        .unwrap();
    assert_eq!(
        store.load_application(b).await.unwrap().unwrap().packages[0].files(),
        old.files()
    );
    assert_eq!(
        store.load_application(a).await.unwrap().unwrap().packages[0].files(),
        new.files()
    );
    store
        .commit_application(b, 1, input(), vec![], vec![])
        .await
        .unwrap();
    let count = store
        .connection
        .run(|conn| {
            Box::pin(async move {
                Ok(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM application_packages")
                        .fetch_one(conn)
                        .await?,
                )
            })
        })
        .await
        .unwrap();
    assert_eq!(count, 2);
    assert_eq!(
        store
            .load_application_at(&original)
            .await
            .unwrap()
            .unwrap()
            .packages[0]
            .files(),
        old.files()
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn corrupted_exact_bytes_cannot_be_loaded_or_silently_overwritten() {
    let (_dir, store) = store().await;
    let id = Uuid::new_v4();
    let package = package("export default {};");
    store
        .commit_application(id, 0, selected(&package), vec![], vec![package.clone()])
        .await
        .unwrap();
    store
        .connection
        .run(|conn| {
            Box::pin(async move {
                sqlx::query(
                    "UPDATE application_package_files SET payload = X'00' WHERE path = 'host.mjs'",
                )
                .execute(conn)
                .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    assert!(
        store
            .load_application(id)
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("digest mismatch")
    );
    assert!(
        store
            .commit_application(id, 1, selected(&package), vec![], vec![package])
            .await
            .is_err()
    );
    let revision = store
        .connection
        .run(move |conn| {
            Box::pin(async move {
                Ok(sqlx::query_scalar::<_, i64>(
                    "SELECT revision FROM application_deployments WHERE id = ?",
                )
                .bind(id.to_string())
                .fetch_one(conn)
                .await?)
            })
        })
        .await
        .unwrap();
    assert_eq!(revision, 1);
    store.close().await.unwrap();
}

#[tokio::test]
async fn stored_text_and_payload_limits_precede_package_materialization() {
    for corruption in [
        "UPDATE application_package_files SET path = printf('%.*c', 1048576, 'x') WHERE path='host.mjs'",
        "UPDATE application_package_files SET payload = zeroblob(8388609) WHERE path='host.mjs'",
        "UPDATE application_package_files SET payload = 'x' || char(0) || printf('%.*c', 8388609, 'z') WHERE path='host.mjs'",
    ] {
        let (_dir, store) = store().await;
        let id = Uuid::new_v4();
        let package = package("export default {};");
        store
            .commit_application(id, 0, selected(&package), vec![], vec![package])
            .await
            .unwrap();
        store
            .connection
            .run(move |conn| {
                Box::pin(async move {
                    sqlx::query(corruption).execute(conn).await?;
                    Ok(())
                })
            })
            .await
            .unwrap();
        let error = store.load_application(id).await.err().unwrap().to_string();
        if corruption.contains("char(0)") {
            assert!(error.contains("payload must be a BLOB"), "{error}");
        } else if corruption.contains("zeroblob") {
            assert!(error.contains("exceeds byte limits"), "{error}");
        } else {
            assert!(error.contains("stored package path"), "{error}");
        }
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn concurrent_cas_has_one_commit_and_reopen_keeps_original_domain() {
    let (dir, store) = store().await;
    let id = Uuid::new_v4();
    let (a, b) = tokio::join!(
        store.commit_application(id, 0, input(), vec![], vec![]),
        store.commit_application(id, 0, input(), vec![], vec![])
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let source = a.or(b).unwrap();
    store.close().await.unwrap();
    let namespaces = RootNamespaces {
        ownership: dir.path().join("owners"),
        control: dir.path().join("control"),
    };
    let owner = Arc::new(RootOwner::open(&dir.path().join("root"), &namespaces).unwrap());
    let store = EventLog::for_root(owner).await.unwrap();
    assert_eq!(
        store
            .load_application(id)
            .await
            .unwrap()
            .unwrap()
            .source
            .reference,
        source
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn duplicate_package_selection_and_missing_exact_revision_commit_nothing() {
    let (_dir, store) = store().await;
    let id = Uuid::new_v4();
    let package = package("export default {};");
    let error = store
        .commit_application(
            id,
            0,
            selected(&package),
            vec![],
            vec![package.clone(), package.clone()],
        )
        .await
        .err()
        .unwrap()
        .to_string();
    assert!(
        error.contains("duplicate application package revision"),
        "{error}"
    );
    assert!(store.load_application(id).await.unwrap().is_none());
    assert!(
        store
            .commit_application(id, 0, selected(&package), vec![], vec![])
            .await
            .is_err()
    );
    assert!(store.load_application(id).await.unwrap().is_none());
    store.close().await.unwrap();
}

#[tokio::test]
async fn package_materialization_checks_remaining_deployment_budget() {
    let (_dir, store) = store().await;
    let package = package("export default {};");
    let reference = package.application_ref();
    store
        .commit_application(Uuid::new_v4(), 0, selected(&package), vec![], vec![package])
        .await
        .unwrap();
    let result = store
        .connection
        .run(move |conn| {
            Box::pin(async move { read_package(conn, &reference, 1).await.map(|_| ()) })
        })
        .await;
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains("exceeds byte limits")
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn metadata_snapshot_never_materializes_package_payloads() {
    let (_dir, store) = store().await;
    let package = package(&"x".repeat(8 * 1024 * 1024));
    let reference = package.application_ref();
    let deployment = Uuid::new_v4();
    let source = store
        .commit_application(deployment, 0, selected(&package), vec![], vec![package])
        .await
        .unwrap();
    let metadata = store
        .read_application_metadata(deployment)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(metadata.source.reference, source);
    assert_eq!(metadata.packages, vec![reference]);
    let mut empty = metadata.source.input.clone();
    empty.instances.clear();
    let next = store
        .commit_application(deployment, 1, empty, vec![], vec![])
        .await
        .unwrap();
    assert!(
        store
            .read_application_metadata_at(&next)
            .await
            .unwrap()
            .unwrap()
            .packages
            .is_empty()
    );
    let original = serde_json::to_vec(&metadata).unwrap();
    assert!(original.len() < 4096);
    // A payload that cannot be loaded still must not enter metadata reads. This
    // tests the physical DAO boundary, not a profiler estimate or source string.
    store.connection.run(|connection| Box::pin(async move {
        sqlx::query("UPDATE application_package_files SET payload = zeroblob(8388609) WHERE path='host.mjs'").execute(connection).await?;
        Ok(())
    })).await.unwrap();
    assert_eq!(
        serde_json::to_vec(
            &store
                .read_application_metadata_at(&source)
                .await
                .unwrap()
                .unwrap()
        )
        .unwrap(),
        original
    );
    assert!(store.load_application_at(&source).await.is_err());
    assert_eq!(
        store
            .load_application(deployment)
            .await
            .unwrap()
            .unwrap()
            .source
            .reference,
        next
    );
    assert!(store.read_application_metadata(Uuid::nil()).await.is_err());
    assert!(
        store
            .read_application_metadata(Uuid::new_v4())
            .await
            .unwrap()
            .is_none()
    );
    store.close().await.unwrap();
}
