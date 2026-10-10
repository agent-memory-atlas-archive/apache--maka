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
use maka_event_log::root::RootNamespaces;

fn scope() -> Scope {
    maka_plugins::call::Issuer::default()
        .issue(
            maka_plugins::call::Identity::Remote {
                request_id: uuid::Uuid::new_v4(),
            },
            CancellationToken::new(),
        )
        .unwrap()
}
fn document(body: &str) -> Vec<u8> {
    format!("---\nname: review\ndescription: Review a change\n---\n{body}").into_bytes()
}
fn operation(review: &Review) -> Operation {
    Operation {
        id: uuid::Uuid::new_v4(),
        fingerprint: maka_runtime::artifact::content_digest(&serde_json::to_vec(review).unwrap()),
        binding: maka_runtime::artifact::content_digest(b"original native binding"),
    }
}
async fn review(library: &Library, kind: ChangeKind, reference: &str, scope: &Scope) -> Review {
    let change = Change {
        workspace: library.workspace.clone(),
        kind,
        reference: reference.into(),
        revision: library.snapshot(scope).await.unwrap().revision,
    };
    match library.preview(change, scope).await.unwrap() {
        Preview::Ready { review, .. } => review,
        _ => panic!("expected bounded change preview"),
    }
}
async fn library(root: Arc<RootOwner>, source: &std::path::Path, namespace: &str) -> Library {
    let mut library = Library::open(
        Configuration {
            home: None,
            project_roots: vec![],
            bundled: false,
            manage: true,
            preferences: false,
            sources: vec![Source {
                id: "local".into(),
                label: "Local".into(),
                path: source.to_str().unwrap().into(),
            }],
        },
        root.canonical_path().to_owned(),
        super::super::tests::preferences(source.parent().unwrap()).await,
    )
    .unwrap();
    library.managed = Some(Managed::open(root, namespace.into()).await.unwrap());
    library
}
#[tokio::test]
async fn managed_changes_preserve_resources_refuse_stale_reviews_and_replay_original_receipts() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("sources");
    std::fs::create_dir_all(source.join("review/scripts")).unwrap();
    std::fs::write(source.join("review/SKILL.md"), document("original")).unwrap();
    std::fs::write(source.join("review/scripts/check"), "source resource").unwrap();
    let root = Arc::new(
        RootOwner::create(
            &dir.path().join("host"),
            &RootNamespaces {
                ownership: dir.path().join("owners"),
                control: dir.path().join("control"),
            },
        )
        .unwrap(),
    );
    let target = root
        .canonical_path()
        .join("skill-libraries/one/skills/review");
    let call = scope();
    let mut first = library(root.clone(), &source, "one").await;
    let install = review(&first, ChangeKind::Install, "source:local:review", &call).await;
    std::fs::write(source.join("review/scripts/check"), "changed after preview").unwrap();
    assert!(matches!(
        first
            .apply_operation(install.clone(), &operation(&install), &call)
            .await
            .unwrap(),
        ChangeResult::Changed
    ));
    assert!(!target.exists());
    std::fs::write(source.join("review/scripts/check"), "source resource").unwrap();
    let install = review(&first, ChangeKind::Install, "source:local:review", &call).await;
    let install_operation = maka_plugin_core::operation::RecoveryRef {
        source: maka_plugin_core::deployment::DeploymentRef {
            state_domain: "a".repeat(64),
            deployment: uuid::Uuid::new_v4(),
            revision: 1,
        },
        namespace: maka_plugin_core::composition::InstanceId(vec![uuid::Uuid::new_v4()]),
        export: "skills".into(),
        method: "apply".into(),
        operation: uuid::Uuid::new_v4(),
        fingerprint: operation(&install)
            .fingerprint
            .replacen("sha256:", "sha256-", 1),
        contract: contract(),
        recovery: maka_plugin_core::operation::RecoveryProtocol::JournalV1,
    };
    install_operation.validate().unwrap();
    assert!(matches!(
        first
            .apply(install.clone(), &install_operation, &call)
            .await
            .unwrap(),
        ChangeResult::Applied { .. }
    ));
    assert_eq!(
        std::fs::read(target.join("scripts/check")).unwrap(),
        b"source resource"
    );
    let snapshot = first.snapshot(&call).await.unwrap();
    assert!(
        snapshot
            .entries
            .iter()
            .any(|(entry, _, body)| entry.installed && body == "original")
    );
    assert!(snapshot.entries.iter().any(|(entry, _, _)| !entry.installed
        && entry.shadowed_by.as_deref() == Some("installed:review")));
    let other = library(root.clone(), &source, "two").await;
    assert!(
        !other
            .snapshot(&call)
            .await
            .unwrap()
            .entries
            .iter()
            .any(|(entry, _, _)| entry.installed)
    );

    // Updating instructions retains local resources. Every preview binds the
    // complete reviewed target and source trees, not only SKILL.md.
    std::fs::write(target.join("scripts/check"), "local resource").unwrap();
    std::fs::write(target.join("SKILL.md"), document("local instructions")).unwrap();
    std::fs::write(source.join("review/SKILL.md"), document("updated")).unwrap();
    let update = review(&first, ChangeKind::Update, "source:local:review", &call).await;
    std::fs::write(target.join("scripts/check"), "edited after preview").unwrap();
    assert!(matches!(
        first
            .apply_operation(update.clone(), &operation(&update), &call)
            .await
            .unwrap(),
        ChangeResult::Changed
    ));
    assert_eq!(
        std::fs::read(target.join("SKILL.md")).unwrap(),
        document("local instructions")
    );
    let update = review(&first, ChangeKind::Update, "source:local:review", &call).await;
    assert!(matches!(
        first
            .apply_operation(update.clone(), &operation(&update), &call)
            .await
            .unwrap(),
        ChangeResult::Applied { .. }
    ));
    assert_eq!(
        std::fs::read(target.join("scripts/check")).unwrap(),
        b"edited after preview"
    );
    assert_eq!(
        std::fs::read(target.join("SKILL.md")).unwrap(),
        document("updated")
    );
    assert_eq!(
        std::fs::read(target.join(".maka/baseline/SKILL.md")).unwrap(),
        document("updated")
    );

    let delete = review(&first, ChangeKind::Delete, "installed:review", &call).await;
    let deletion = operation(&delete);
    assert!(matches!(
        first
            .apply_operation(delete.clone(), &deletion, &call)
            .await
            .unwrap(),
        ChangeResult::Applied { .. }
    ));
    assert!(!target.exists());
    assert_eq!(
        std::fs::read(source.join("review/scripts/check")).unwrap(),
        b"source resource"
    );
    drop(first);
    first = library(root.clone(), &source, "one").await;
    assert!(matches!(
        first
            .apply(install, &install_operation, &call)
            .await
            .unwrap(),
        ChangeResult::Applied { .. }
    ));
    assert!(
        !target.exists(),
        "the original install receipt must not reinstall a deleted result"
    );
    assert!(matches!(
        first
            .apply_operation(delete.clone(), &deletion, &call)
            .await
            .unwrap(),
        ChangeResult::Applied { .. }
    ));
    let mismatch = Operation {
        fingerprint: maka_runtime::artifact::content_digest(b"different input"),
        ..deletion
    };
    assert!(
        first
            .apply_operation(delete, &mismatch, &call)
            .await
            .is_err()
    );
    call.finish().await.unwrap();
}

#[tokio::test]
async fn management_rejects_external_deletion_disabled_access_and_links_into_private_records() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("sources");
    std::fs::create_dir_all(source.join("review")).unwrap();
    std::fs::write(source.join("review/SKILL.md"), document("original")).unwrap();
    let root = Arc::new(
        RootOwner::create(
            &dir.path().join("host"),
            &RootNamespaces {
                ownership: dir.path().join("owners"),
                control: dir.path().join("control"),
            },
        )
        .unwrap(),
    );
    let call = scope();
    let mut library = library(root.clone(), &source, "one").await;
    let change = Change {
        workspace: None,
        kind: ChangeKind::Delete,
        reference: "source:local:review".into(),
        revision: library.snapshot(&call).await.unwrap().revision,
    };
    assert!(
        matches!(library.preview(change, &call).await.unwrap(), Preview::Rejected { reason } if reason == "read_only_source")
    );
    let install = review(&library, ChangeKind::Install, "source:local:review", &call).await;
    let managed = library.managed.take().unwrap();
    assert!(
        matches!(library.apply_operation(install.clone(), &operation(&install), &call).await.unwrap(), ChangeResult::Rejected { reason } if reason == "management_disabled")
    );
    assert!(
        !root
            .canonical_path()
            .join("skill-libraries/one/skills/review")
            .exists()
    );
    #[cfg(unix)]
    {
        let data = root.canonical_path().join("skill-libraries/one");
        std::fs::write(data.join("receipts/private.json"), "private receipt").unwrap();
        std::os::unix::fs::symlink("../receipts", data.join("skills/leak")).unwrap();
        let read = managed.read.bind_scope(&call);
        assert!(
            read.read(maka_plugins::filesystem::entries::ReadFile {
                path: "skills/leak/private.json".into(),
                offset: 0,
                limit: 1024
            })
            .await
            .is_err()
        );
        assert!(
            read.read(maka_plugins::filesystem::entries::ReadFile {
                path: "receipts/private.json".into(),
                offset: 0,
                limit: 1024
            })
            .await
            .is_err()
        );
    }
    call.finish().await.unwrap();
}

#[test]
fn deletion_review_keeps_exact_paths_and_marks_bounded_path_excerpts() {
    let mut tree = Tree::empty();
    for index in 0..3 {
        tree.insert(&format!("{index}-{}", "\u{1b}".repeat(4090)), vec![])
            .unwrap();
    }
    let plan = Plan {
        review: Review {
            change: Change {
                workspace: None,
                kind: ChangeKind::Delete,
                reference: "installed:review".into(),
                revision: "sha256:".into(),
            },
            digest: maka_runtime::artifact::content_digest(b"review"),
        },
        id: "review".into(),
        expected: Some(tree),
        next: None,
        locally_modified: false,
    };
    let preview = plan.preview();
    let Preview::Ready {
        entries,
        paths,
        paths_truncated,
        ..
    } = &preview
    else {
        panic!()
    };
    assert_eq!(*entries, 3);
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0], format!("0-{}", "\u{1b}".repeat(4090)));
    assert!(*paths_truncated);
    assert!(encode(preview).is_ok());
}

mod recovery;

#[tokio::test]
async fn bundled_files_are_read_only_and_installed_overrides_can_be_removed() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("sources");
    super::super::tests::write(&source, "maka-plugin-authoring", "custom instructions");
    let root = Arc::new(
        RootOwner::create(
            &dir.path().join("host"),
            &RootNamespaces {
                ownership: dir.path().join("owners"),
                control: dir.path().join("control"),
            },
        )
        .unwrap(),
    );
    let mut library = library(root, &source, "bundles").await;
    library.bundled = true;
    library.preferences_writable = true;
    let call = scope();
    let snapshot = library.snapshot(&call).await.unwrap();
    let entry = snapshot
        .entries
        .iter()
        .find(|(e, _, _)| e.bundled && e.name == "maka-plugin-authoring")
        .unwrap();
    assert_eq!(
        entry.0.shadowed_by.as_deref(),
        Some("source:local:maka-plugin-authoring")
    );
    let revision = snapshot.revision.clone();
    let super::super::Document::Page {
        writable,
        preferences_writable,
        ..
    } = snapshot
        .read(super::super::Read {
            workspace: None,
            reference: "bundled:maka-plugin-authoring".into(),
            revision: revision.clone(),
            offset: 0,
        })
        .unwrap()
    else {
        panic!()
    };
    assert!(!writable);
    assert!(preferences_writable);
    for kind in [ChangeKind::Install, ChangeKind::Update, ChangeKind::Delete] {
        assert!(matches!(
            library
                .preview(
                    Change {
                        workspace: None,
                        kind,
                        reference: "bundled:maka-plugin-authoring".into(),
                        revision: revision.clone()
                    },
                    &call
                )
                .await
                .unwrap(),
            Preview::Rejected { .. }
        ));
    }
    let install = review(
        &library,
        ChangeKind::Install,
        "source:local:maka-plugin-authoring",
        &call,
    )
    .await;
    assert!(matches!(
        library
            .apply_operation(install.clone(), &operation(&install), &call)
            .await
            .unwrap(),
        ChangeResult::Applied { .. }
    ));
    let snapshot = library.snapshot(&call).await.unwrap();
    assert!(snapshot.entries.iter().any(|(e, _, _)| e.bundled
        && e.shadowed_by.as_deref() == Some("installed:maka-plugin-authoring")));
    std::fs::remove_dir_all(source.join("maka-plugin-authoring")).unwrap();
    let delete = review(
        &library,
        ChangeKind::Delete,
        "installed:maka-plugin-authoring",
        &call,
    )
    .await;
    assert!(matches!(
        library
            .apply_operation(delete.clone(), &operation(&delete), &call)
            .await
            .unwrap(),
        ChangeResult::Applied { .. }
    ));
    let snapshot = library.snapshot(&call).await.unwrap();
    assert_eq!(snapshot.entries.len(), 2);
    assert!(
        snapshot
            .entries
            .iter()
            .all(|(e, _, _)| e.bundled && !e.installed && e.shadowed_by.is_none())
    );
    call.finish().await.unwrap();
}

#[tokio::test]
async fn project_installation_keeps_its_reviewed_workspace_and_relative_resource_tree() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("sources");
    std::fs::create_dir_all(&source).unwrap();
    let project = dir.path().join("project");
    super::super::tests::write(
        &project.join(".agents/skills"),
        "review",
        "project instructions",
    );
    std::fs::create_dir_all(project.join(".agents/skills/review/references")).unwrap();
    std::fs::write(
        project.join(".agents/skills/review/references/guide"),
        "project resource",
    )
    .unwrap();
    let (project, directory_identity) = maka_fs_tools::directory::capture(&project).unwrap();
    let identity = maka_fs_tools::workspace::ensure_directory_identity(
        project.clone(),
        maka_fs_tools::directory::open(&project, &directory_identity).unwrap(),
    )
    .await
    .unwrap();
    let workspace = crate::contract::Workspace {
        path: project.to_str().unwrap().into(),
        directory_identity,
        identity,
    };
    let root = Arc::new(
        RootOwner::create(
            &dir.path().join("host"),
            &RootNamespaces {
                ownership: dir.path().join("owners"),
                control: dir.path().join("control"),
            },
        )
        .unwrap(),
    );
    let mut library = library(root.clone(), &source, "project").await;
    library.projects = vec![Arc::new(
        maka_fs_tools::workspace::directory::PublishedDirectory::open(&project).unwrap(),
    )];
    let call = scope();
    let scoped = library
        .for_workspace(Some(workspace.clone()), &call)
        .await
        .unwrap();
    let reference = scoped.snapshot(&call).await.unwrap().entries[0]
        .0
        .reference
        .clone();
    let install = review(&scoped, ChangeKind::Install, &reference, &call).await;
    assert_eq!(install.change.workspace, Some(workspace));
    let original = library
        .for_workspace(install.change.workspace.clone(), &call)
        .await
        .unwrap();
    assert!(matches!(
        original
            .apply_operation(install.clone(), &operation(&install), &call)
            .await
            .unwrap(),
        ChangeResult::Applied { .. }
    ));
    assert_eq!(
        std::fs::read_to_string(
            root.canonical_path()
                .join("skill-libraries/project/skills/review/references/guide")
        )
        .unwrap(),
        "project resource"
    );
    let snapshot = original.snapshot(&call).await.unwrap();
    assert!(
        snapshot
            .entries
            .iter()
            .any(|(e, _, _)| e.installed && e.shadowed_by.as_deref() == Some(reference.as_str()))
    );
    std::fs::remove_dir_all(project.join(".agents/skills/review")).unwrap();
    let snapshot = original.snapshot(&call).await.unwrap();
    assert!(
        snapshot
            .entries
            .iter()
            .all(|(e, _, _)| e.installed && e.shadowed_by.is_none())
    );
    call.finish().await.unwrap();
}
