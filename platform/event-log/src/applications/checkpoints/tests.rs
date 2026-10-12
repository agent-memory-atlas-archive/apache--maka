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
use crate::{
    applications::tests::store,
    root::{RootNamespaces, RootOwner},
};
use maka_packages::MANIFEST_FILE;
use maka_packages::Package;
use maka_plugin_core::{
    composition::{Configuration, Environment, Input, Node, PackageUse, PresentationTarget},
    package::{Declaration, EntryDeclaration, EntryTarget, Implementation},
};
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;

async fn source(store: &EventLog, deployment: Uuid, expected: u64) -> DeploymentRef {
    let package = Package::new(BTreeMap::from([(MANIFEST_FILE.into(),serde_json::to_vec(&json!({"schemaVersion":2,"id":"ui.checkpoint","application":Declaration {entries:BTreeMap::from([(EntryTarget::Tui,EntryDeclaration {scene:None,implementation:Implementation::Native {implementation:"test.checkpoint".into()},configuration:json!({"type":"object"}),imports:BTreeMap::new(),exports:BTreeMap::new()})]),composition:None}})).unwrap())])).unwrap();
    store
        .commit_application(
            deployment,
            expected,
            Input {
                environment: Environment {
                    location: ExecutionLocation::Ui,
                    presentation: Some(PresentationTarget::Tui),
                },
                instances: BTreeMap::from([(
                    "editor".into(),
                    Node {
                        key: Uuid::from_u128(42),
                        package: package.application_ref().into(),
                        use_package: PackageUse::Entry,
                        configuration: Configuration::default(),
                        parameters: Configuration::default(),
                        bindings: BTreeMap::new(),
                        disabled: false,
                        overrides: vec![],
                    },
                )]),
                recovery: None,
                launch: None,
            },
            vec![],
            vec![package],
        )
        .await
        .unwrap()
}
fn instance() -> InstanceId {
    InstanceId(vec![Uuid::from_u128(42)])
}

#[tokio::test]
async fn original_ui_namespace_survives_source_changes_and_root_reopen_without_cross_owner_writes()
{
    let (dir, store) = store().await;
    let store = Arc::new(store);
    let deployment = Uuid::new_v4();
    let original = source(&store, deployment, 0).await;
    let checkpoints = store.ui_checkpoints(&original, &instance()).await.unwrap();
    let data = json!({"domain":"host-a", "namespace":"agent-a"});
    let metadata =
        json!({"owner":"native-a", "revision":9, "selection":[1,3], "attachments":["artifact-a"]});
    let revision = checkpoints
        .write(
            "conversation",
            None,
            "editor@1",
            &data,
            &metadata,
            "甲👩‍💻z".as_bytes().to_vec(),
        )
        .await
        .unwrap();
    assert_eq!(revision, 1);
    let saved = checkpoints.read("conversation").await.unwrap().unwrap();
    let updated = source(&store, deployment, 1).await;
    let replacement = store.ui_checkpoints(&updated, &instance()).await.unwrap();
    assert_eq!(
        replacement.read("conversation").await.unwrap(),
        Some(saved.clone())
    );
    let other = source(&store, Uuid::new_v4(), 0).await;
    assert!(
        store
            .ui_checkpoints(&other, &instance())
            .await
            .unwrap()
            .read("conversation")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .ui_checkpoints(&original, &InstanceId(vec![Uuid::new_v4()]))
            .await
            .is_err()
    );
    let mut foreign = original.clone();
    foreign.state_domain = "a".repeat(64);
    assert!(store.ui_checkpoints(&foreign, &instance()).await.is_err());
    for (format, data) in [
        ("editor@2", data.clone()),
        ("editor@1", json!({"domain":"host-b"})),
    ] {
        assert!(
            replacement
                .write("conversation", Some(1), format, &data, &metadata, vec![])
                .await
                .is_err()
        );
        assert_eq!(
            replacement.read("conversation").await.unwrap(),
            Some(saved.clone())
        );
    }
    drop(checkpoints);
    drop(replacement);
    store.shutdown().await.unwrap();
    drop(store);
    let namespaces = RootNamespaces {
        ownership: dir.path().join("owners"),
        control: dir.path().join("control"),
    };
    let root = Arc::new(RootOwner::open(&dir.path().join("root"), &namespaces).unwrap());
    let reopened = Arc::new(EventLog::for_root(root).await.unwrap());
    let checkpoints = reopened
        .ui_checkpoints(&updated, &instance())
        .await
        .unwrap();
    assert_eq!(checkpoints.read("conversation").await.unwrap(), Some(saved));
    drop(checkpoints);
    reopened.shutdown().await.unwrap();
}

#[tokio::test]
async fn concurrent_mounts_cannot_replace_a_saved_or_cleared_original_checkpoint() {
    let (_dir, store) = store().await;
    let store = Arc::new(store);
    let source = source(&store, Uuid::new_v4(), 0).await;
    let first = store.ui_checkpoints(&source, &instance()).await.unwrap();
    let second = store.ui_checkpoints(&source, &instance()).await.unwrap();
    let (a, b) = tokio::join!(
        first.write(
            "draft",
            None,
            "v1",
            &Value::Null,
            &Value::Null,
            b"one".to_vec()
        ),
        second.write(
            "draft",
            None,
            "v1",
            &Value::Null,
            &Value::Null,
            b"two".to_vec()
        )
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert!(matches!(
        a.as_ref().err().or(b.as_ref().err()).unwrap(),
        StoreError::RevisionConflict { .. }
    ));
    let current = first.read("draft").await.unwrap().unwrap();
    first
        .write(
            "draft",
            Some(current.revision),
            "v1",
            &Value::Null,
            &Value::Null,
            vec![],
        )
        .await
        .unwrap();
    assert!(
        second
            .write(
                "draft",
                None,
                "v1",
                &Value::Null,
                &Value::Null,
                b"resurrection".to_vec()
            )
            .await
            .is_err()
    );
    assert!(
        second
            .write(
                "draft",
                Some(current.revision),
                "v1",
                &Value::Null,
                &Value::Null,
                b"stale".to_vec()
            )
            .await
            .is_err()
    );
    assert!(
        first
            .read("draft")
            .await
            .unwrap()
            .unwrap()
            .content
            .is_empty()
    );
}

#[tokio::test]
async fn raw_editor_limit_metadata_quota_and_corrupt_records_are_bounded_before_reading() {
    let (_dir, store) = store().await;
    let store = Arc::new(store);
    let source = source(&store, Uuid::new_v4(), 0).await;
    let checkpoints = store.ui_checkpoints(&source, &instance()).await.unwrap();
    let body = vec![b'"'; MAX_CONTENT];
    checkpoints
        .write(
            "limit",
            None,
            "v1",
            &Value::Null,
            &Value::Null,
            body.clone(),
        )
        .await
        .unwrap();
    assert_eq!(
        checkpoints.read("limit").await.unwrap().unwrap().content,
        body
    );
    assert!(
        checkpoints
            .write(
                "large",
                None,
                "v1",
                &Value::Null,
                &Value::Null,
                vec![0; MAX_CONTENT + 1]
            )
            .await
            .is_err()
    );
    assert!(
        checkpoints
            .write(
                "metadata",
                None,
                "v1",
                &Value::Null,
                &json!("\n".repeat(MAX_METADATA / 2)),
                vec![]
            )
            .await
            .is_err()
    );
    for index in 1..39 {
        checkpoints
            .write(
                &format!("item-{index}"),
                None,
                "v1",
                &Value::Null,
                &Value::Null,
                body.clone(),
            )
            .await
            .unwrap();
    }
    assert!(
        checkpoints
            .write("overflow", None, "v1", &Value::Null, &Value::Null, body)
            .await
            .is_err()
    );
    checkpoints
        .write("limit", Some(1), "v1", &Value::Null, &Value::Null, vec![])
        .await
        .unwrap();
    checkpoints
        .write(
            "overflow",
            None,
            "v1",
            &Value::Null,
            &Value::Null,
            vec![0; MAX_CONTENT],
        )
        .await
        .unwrap();
    store.connection.run(|connection| Box::pin(async move {
        sqlx::query("UPDATE application_ui_checkpoints SET content=zeroblob(1048577) WHERE key='limit'").execute(&mut *connection).await?;
        Ok(())
    })).await.unwrap();
    assert!(checkpoints.read("limit").await.is_err());
}
