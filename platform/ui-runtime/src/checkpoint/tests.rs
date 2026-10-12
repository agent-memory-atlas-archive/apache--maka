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
use crate::editor::{EditorTextRef, TextRange};
use maka_event_log::root::{ROOT_DATABASE, RootNamespaces, RootOwner};
use maka_packages::MANIFEST_FILE;
use maka_packages::Package;
use maka_plugin_core::deployment::DeploymentRef;
use maka_plugin_core::{
    composition::{
        Configuration, Environment, ExecutionLocation, Input, Node, PackageUse, PresentationTarget,
    },
    package::{Declaration, EntryDeclaration, EntryTarget, Implementation},
};
use serde_json::json;
use std::{collections::BTreeMap, time::Duration};
use uuid::Uuid;

async fn store() -> (tempfile::TempDir, Arc<EventLog>) {
    let dir = tempfile::tempdir().unwrap();
    let root =
        Arc::new(RootOwner::create(&dir.path().join("root"), &namespaces(dir.path())).unwrap());
    let store = Arc::new(EventLog::for_root(root).await.unwrap());
    (dir, store)
}
fn namespaces(dir: &std::path::Path) -> RootNamespaces {
    RootNamespaces {
        ownership: dir.join("owners"),
        control: dir.join("control"),
    }
}
fn policy() -> EditorPersistence {
    EditorPersistence {
        key: "conversation".into(),
        format: "test@1".into(),
        data_source: json!({"host":"original"}),
        metadata: json!({"attachments":["artifact"]}),
        restore: None,
        retry: 0,
    }
}
async fn scope(store: &Arc<EventLog>, source: &DeploymentRef) -> Scope {
    Scope {
        store: Arc::downgrade(store),
        source: store
            .load_application_at(source)
            .await
            .unwrap()
            .unwrap()
            .snapshot(),
        instance: instance(),
        scheduler: tokio::runtime::Handle::current(),
    }
}
fn snapshot(owner: &str, revision: u64, text: &str) -> EditorSnapshot {
    EditorSnapshot {
        text: EditorTextRef {
            owner: owner.into(),
            revision,
            bytes: text.len(),
        },
        selection: TextRange { start: 1, end: 6 },
        reversed: true,
        marked: None,
    }
}
async fn until(
    editor: &mut EditorCheckpoint,
    policy: &EditorPersistence,
    check: impl Fn(&EditorCheckpoint) -> bool,
) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            editor.poll(policy);
            if check(editor) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap();
}
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn final_coalesced_snapshot_flushes_before_cleanup_and_restores_after_root_reopen() {
    let (dir, store) = store().await;
    let source = source(&store, Uuid::new_v4(), 0).await;
    let policy = policy();
    let resources = Arc::new(Resources::default());
    let budget = NativeBudget::default();
    let recipe_scope = scope(&store, &source).await;
    let mut editor = EditorCheckpoint::new(
        Some(recipe_scope.clone()),
        &policy,
        &resources,
        budget.clone(),
    );
    assert!(!editor.accepts_input());
    until(&mut editor, &policy, EditorCheckpoint::writable).await;
    assert!(
        !editor.accepts_input(),
        "new lineage must be committed before input"
    );
    let text = "甲👩‍💻z";
    editor.save(&snapshot("old-owner", 1, text), text, &policy.metadata);
    until(&mut editor, &policy, EditorCheckpoint::accepts_input).await;
    let blocker = rusqlite::Connection::open(dir.path().join("root").join(ROOT_DATABASE)).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    for revision in 2..=100 {
        let body = format!("{text}{revision}");
        editor.save(
            &snapshot("old-owner", revision, &body),
            &body,
            &policy.metadata,
        );
    }
    editor.close();
    assert!(
        !resources.seal().unwrap(),
        "close must retain the actual pending writer"
    );
    drop(editor);
    let retained = Arc::downgrade(&store);
    drop(store);
    assert!(retained.upgrade().is_some(), "worker retains original Root");
    assert!(RootOwner::open(&dir.path().join("root"), &namespaces(dir.path())).is_err());
    blocker.execute_batch("COMMIT").unwrap();
    drop(blocker);
    resources.finish().await.unwrap();
    assert_eq!(budget.usage(), (0, 0, 0));
    tokio::time::timeout(Duration::from_secs(5), async {
        while retained.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let root =
        Arc::new(RootOwner::open(&dir.path().join("root"), &namespaces(dir.path())).unwrap());
    let store = Arc::new(EventLog::for_root(root).await.unwrap());
    assert!(
        recipe_scope.open(&store).is_err(),
        "old evidence cannot cross a Root lease reopen"
    );
    let resources = Arc::new(Resources::default());
    let mut editor = EditorCheckpoint::new(
        Some(scope(&store, &source).await),
        &policy,
        &resources,
        budget.clone(),
    );
    until(&mut editor, &policy, |e| {
        matches!(e.phase, Phase::Restore(_))
    })
    .await;
    let Phase::Restore(loaded) = &editor.phase else {
        unreachable!()
    };
    assert_eq!(loaded.text, format!("{text}100"));
    assert_eq!(loaded.saved.metadata, policy.metadata);
    assert_eq!(loaded.saved.snapshot.text.revision, 100);
    assert_eq!(
        selection_bytes(&loaded.text, Some(&loaded.saved.snapshot)),
        Some((14, 3))
    );
    let mut policy = policy;
    policy.restore = Some(EditorRestoration {
        owner: "new-owner".into(),
        version: loaded.version,
        clear_text: false,
    });
    assert!(editor.restoration("other-owner", &policy).is_none());
    assert_eq!(
        editor.restoration("new-owner", &policy).unwrap().text(),
        format!("{text}100")
    );
    editor.restored();
    assert!(!editor.accepts_input());
    let body = format!("{text}100");
    editor.save(&snapshot("new-owner", 1, &body), &body, &policy.metadata);
    until(&mut editor, &policy, EditorCheckpoint::accepts_input).await;
    editor.close();
    drop(editor);
    resources.finish().await.unwrap();
    let saved = store
        .ui_checkpoints(&source, &instance())
        .await
        .unwrap()
        .read(&policy.key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.metadata["snapshot"]["text"]["owner"], "new-owner");
    store.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_older_ack_cannot_hide_a_newer_capture_failure_and_retry_accounts_for_it() {
    let (_dir, store) = store().await;
    let source = source(&store, Uuid::new_v4(), 0).await;
    let mut policy = policy();
    let resources = Arc::new(Resources::default());
    let budget = NativeBudget::default();
    let mut editor = EditorCheckpoint::new(
        Some(scope(&store, &source).await),
        &policy,
        &resources,
        budget.clone(),
    );
    until(&mut editor, &policy, EditorCheckpoint::writable).await;
    let body = "甲👩‍💻z";
    editor.save(&snapshot("owner", 1, body), body, &policy.metadata);
    let mut invalid = snapshot("owner", 2, body);
    invalid.text.bytes = MAX_CONTENT + 1;
    editor.save(&invalid, body, &policy.metadata);
    until(&mut editor, &policy, |e| e.saved_sequence == 1).await;
    assert!(!editor.accepts_input());
    assert!(matches!(editor.phase, Phase::Failed { writing: true, .. }));
    policy.retry = 1;
    editor.poll(&policy);
    editor.save(&snapshot("owner", 2, body), body, &policy.metadata);
    until(&mut editor, &policy, |e| e.saved_sequence == 2).await;
    editor.close();
    drop(editor);
    resources.finish().await.unwrap();
    assert_eq!(budget.usage(), (0, 0, 0));
}

#[tokio::test]
async fn source_mismatch_and_conflicting_writers_cannot_overwrite_saved_content() {
    let (_dir, store) = store().await;
    let source = source(&store, Uuid::new_v4(), 0).await;
    let policy = policy();
    let a = Arc::new(Resources::default());
    let b = Arc::new(Resources::default());
    let mut first = EditorCheckpoint::new(
        Some(scope(&store, &source).await),
        &policy,
        &a,
        NativeBudget::default(),
    );
    let mut second = EditorCheckpoint::new(
        Some(scope(&store, &source).await),
        &policy,
        &b,
        NativeBudget::default(),
    );
    until(&mut first, &policy, EditorCheckpoint::writable).await;
    until(&mut second, &policy, EditorCheckpoint::writable).await;
    let body = "甲👩‍💻z";
    first.save(&snapshot("first", 1, body), body, &policy.metadata);
    until(&mut first, &policy, EditorCheckpoint::accepts_input).await;
    second.save(&snapshot("second", 1, body), body, &policy.metadata);
    until(&mut second, &policy, |e| {
        matches!(e.phase, Phase::Failed { .. })
    })
    .await;
    second.close();
    drop(second);
    assert!(b.finish().await.is_err());
    first.close();
    drop(first);
    a.finish().await.unwrap();
    let mut foreign = policy.clone();
    foreign.data_source = json!({"host":"replacement"});
    let c = Arc::new(Resources::default());
    let mut replacement = EditorCheckpoint::new(
        Some(scope(&store, &source).await),
        &foreign,
        &c,
        NativeBudget::default(),
    );
    until(&mut replacement, &foreign, |e| {
        matches!(e.phase, Phase::Failed { .. })
    })
    .await;
    assert!(!replacement.accepts_input());
    replacement.close();
    drop(replacement);
    c.finish().await.unwrap();
    assert_eq!(
        store
            .ui_checkpoints(&source, &instance())
            .await
            .unwrap()
            .read(&policy.key)
            .await
            .unwrap()
            .unwrap()
            .metadata["snapshot"]["text"]["owner"],
        "first"
    );
}

#[test]
fn restored_selections_require_exact_graphemes_and_preserve_direction() {
    let body = "甲👩‍💻z";
    let mut saved = snapshot("owner", 1, body);
    assert_eq!(selection_bytes(body, Some(&saved)), Some((14, 3)));
    saved.selection.start = 2;
    assert_eq!(selection_bytes(body, Some(&saved)), None);
    assert_eq!(selection_bytes("", None), Some((0, 0)));
}
