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
use maka_fs_tools::workspace::MARKER_FILE;
use maka_runtime_host::server::DirectoryRootSpec;

async fn publish(fixture: &mut Fixture, roots: Vec<DirectoryRootSpec>) {
    let source = fixture.app.source().unwrap().clone();
    fixture.app.shutdown().await.unwrap();
    let mut configuration = fixture.owner.settings.clone();
    configuration.workspace_roots = roots;
    let (input, packages) = input(&configuration);
    fixture
        .store
        .commit_application(source.deployment, source.revision, input, vec![], packages)
        .await
        .unwrap();
    let mut native = NativeRegistry::default();
    maka_settings_service::provide(&mut native).unwrap();
    crate::provide(&mut native).unwrap();
    fixture.app = Application::start_from_store(
        fixture.store.clone(),
        source.deployment,
        native,
        fixture.issuer.clone(),
        Arc::new(Grant(fixture.issuer.clone(), None)),
        Default::default(),
    )
    .await
    .unwrap();
    fixture.app.ready().await.unwrap();
}

#[tokio::test]
async fn published_directories_are_readonly_bounded_and_create_only_observed_workspaces() {
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    for index in 0..36 {
        std::fs::create_dir(fixture.workspace.join(format!("dir-{index:02}"))).unwrap();
    }
    let roots = fixture
        .query("directories", json!({"kind":"roots"}))
        .await
        .unwrap();
    assert_eq!(roots["roots"].as_array().unwrap().len(), 2);
    assert!(!fixture.workspace.join(MARKER_FILE).exists());
    assert!(!fixture.root.join(MARKER_FILE).exists());
    let selection = roots["roots"][0]["selection"].clone();
    let page = fixture
        .query(
            "directories",
            json!({"kind":"children","selection":selection,"after":null}),
        )
        .await
        .unwrap();
    assert_eq!(page["children"].as_array().unwrap().len(), 32);
    assert_eq!(page["after"], "dir-31");
    let tail = fixture
        .query(
            "directories",
            json!({"kind":"children","selection":selection,"after":page["after"]}),
        )
        .await
        .unwrap();
    assert_eq!(tail["children"].as_array().unwrap().len(), 4);
    assert!(tail["after"].is_null());
    assert!(!fixture.workspace.join(MARKER_FILE).exists());
    for (field, value) in [
        ("rootId", json!("unknown")),
        ("segments", json!([".."])),
        (
            "directoryIdentity",
            json!(format!("sha256:{}", "0".repeat(64))),
        ),
    ] {
        let mut invalid = selection.clone();
        invalid[field] = value;
        let result = fixture.mutation(2, "create", json!({"title":"Rejected","labels":[],"workspace":{"kind":"directory","selection":invalid}}), Uuid::new_v4()).await.unwrap();
        assert_eq!(
            result,
            json!({"kind":"rejected","reason":"workspace_unavailable"})
        );
    }
    let child = &page["children"][3];
    let result = fixture.mutation(2, "create", json!({"title":"Selected child","labels":["research"],"workspace":{"kind":"directory","selection":child["selection"]}}), Uuid::new_v4()).await.unwrap();
    assert_eq!(result["kind"], "created");
    let session = &result["session"];
    assert_eq!(session["workspace"]["path"], child["path"]);
    assert_eq!(
        session["workspace"]["directoryIdentity"],
        child["selection"]["directoryIdentity"]
    );
    assert_eq!(session["labels"], json!(["research"]));
    assert!(fixture.workspace.join("dir-03").join(MARKER_FILE).exists());
    assert!(!fixture.workspace.join(MARKER_FILE).exists());
    let reused = fixture.mutation(2, "create", json!({"title":"Reused workspace","labels":[],"workspace":{"kind":"session","sessionId":session["id"]}}), Uuid::new_v4()).await.unwrap();
    assert_eq!(reused["session"]["workspace"], session["workspace"]);
    assert_ne!(reused["session"]["id"], session["id"]);
    for index in 0..4097 {
        std::fs::write(fixture.workspace.join(format!("file-{index}")), []).unwrap();
    }
    assert!(
        fixture
            .query(
                "directories",
                json!({"kind":"children","selection":selection,"after":null})
            )
            .await
            .is_err()
    );
    assert!(!fixture.workspace.join(MARKER_FILE).exists());
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}

#[tokio::test]
async fn malformed_existing_marker_is_a_recorded_refusal_without_repair() {
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    let input = fixture.create_input("Malformed workspace").await;
    let marker = fixture.workspace.join(MARKER_FILE);
    std::fs::write(&marker, b"{broken").unwrap();
    let operation = Uuid::new_v4();
    let refused = fixture
        .mutation(2, "create", input.clone(), operation)
        .await
        .unwrap();
    assert_eq!(
        refused,
        json!({"kind":"rejected","reason":"workspace_unavailable"})
    );
    assert_eq!(
        fixture
            .mutation(2, "create", input, operation)
            .await
            .unwrap(),
        refused
    );
    assert_eq!(std::fs::read(&marker).unwrap(), b"{broken");
    assert!(
        fixture
            .query("list", json!({"kind":"start"}))
            .await
            .unwrap()["sessions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!fixture.root.join(MARKER_FILE).exists());
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}

#[tokio::test]
async fn original_uppercase_marker_and_two_workspaces_reach_exact_execution_configuration() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                request(&mut socket).await;
                response(&mut socket).await;
            }
        });
        let fixture = Fixture::new(&base, 1, None).await;
        let uppercase = "A1234567-B89A-4CDE-8F01-23456789ABCD";
        let marker = fixture.workspace.join(MARKER_FILE);
        let bytes = serde_json::to_vec(&json!({"schemaVersion":1,"workspaceId":uppercase})).unwrap();
        std::fs::write(&marker, &bytes).unwrap();
        let first = fixture.create("Original uppercase").await;
        let roots = fixture.query("directories", json!({"kind":"roots"})).await.unwrap();
        let second = fixture.mutation(2, "create", json!({"title":"Other workspace","labels":[],"workspace":{"kind":"directory","selection":roots["roots"][1]["selection"]}}), Uuid::new_v4()).await.unwrap()["session"]["id"].as_str().unwrap().to_owned();
        for id in [&first, &second] {
            fixture.send(id).await.unwrap();
            fixture.drained().await;
        }
        server.await.unwrap();
        let observed = fixture.query("observe", json!({"sessionId":first})).await.unwrap();
        assert_eq!(observed["session"]["workspace"]["identity"], format!("workspace:v1:{uppercase}"));
        assert_eq!(std::fs::read(&marker).unwrap(), bytes);
        let listed = fixture.query("list", json!({"kind":"start"})).await.unwrap();
        assert_eq!(listed["sessions"].as_array().unwrap().len(), 2);
        let events = fixture.store.prefix(128, 512 * 1024).await.unwrap();
        let mut configurations = BTreeMap::new();
        for stored in events.events {
            if let maka_runtime::event::Fact::InvocationOpened { configuration: Some(configuration), .. } = stored.event.fact {
                configurations.insert(stored.event.invocation.session_id, configuration);
            }
        }
        assert_eq!(configurations.len(), 2);
        for id in [&first, &second] {
            let session = fixture.owner.session(id).await.unwrap();
            let configuration = &configurations[id];
            assert_eq!(configuration.cwd, session.configuration.workspace.path);
            assert_eq!(configuration.workspace_identity, Some(session.configuration.workspace.identity));
            assert_eq!(configuration.workspace_origin, WorkspaceOrigin::Selected);
            assert_ne!(std::path::Path::new(&configuration.cwd), fixture.root.canonicalize().unwrap());
        }
        assert_ne!(configurations[&first].cwd, configurations[&second].cwd);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn metadata_cas_replay_and_noop_preserve_active_run_workspace_and_activity() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let entered = Arc::new(Semaphore::new(0));
        let release = Arc::new(Semaphore::new(0));
        let (accepted, finish) = (entered.clone(), release.clone());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            accepted.add_permits(1);
            finish.acquire().await.unwrap().forget();
            response(&mut socket).await;
        });
        let fixture = Fixture::new(&base, 1, None).await;
        let id = fixture.create("Original title").await;
        let started = fixture.send(&id).await.unwrap();
        entered.acquire().await.unwrap().forget();
        let before = fixture.query("observe", json!({"sessionId":id})).await.unwrap();
        let original_catalog = fixture.query("list", json!({"kind":"start"})).await.unwrap();
        let input = json!({"sessionId":id,"expectedRevision":before["session"]["revision"],"title":"Research","labels":["orchid","rust"]});
        let operation = Uuid::new_v4();
        let updated = fixture.mutation(2, "metadata", input.clone(), operation).await.unwrap();
        assert_eq!(updated["kind"], "committed");
        assert_eq!(updated["session"]["title"], "Research");
        assert_eq!(updated["session"]["labels"], json!(["orchid","rust"]));
        assert_eq!(updated["session"]["workspace"], before["session"]["workspace"]);
        assert_eq!(updated["session"]["updatedAt"], before["session"]["updatedAt"]);
        assert_eq!(fixture.mutation(2, "metadata", input.clone(), operation).await.unwrap(), updated);
        let conflict = fixture.mutation(2, "metadata", input, Uuid::new_v4()).await.unwrap();
        assert_eq!(conflict, json!({"kind":"revision_conflict","expected":before["session"]["revision"],"actual":updated["session"]["revision"]}));
        let noop = fixture.mutation(2, "metadata", json!({"sessionId":id,"expectedRevision":updated["session"]["revision"],"title":"Research","labels":["orchid","rust"]}), Uuid::new_v4()).await.unwrap();
        assert_eq!(noop, updated);
        let invalid = fixture.mutation(2, "metadata", json!({"sessionId":id,"expectedRevision":updated["session"]["revision"],"title":" ","labels":[]}), Uuid::new_v4()).await.unwrap();
        assert_eq!(invalid, json!({"kind":"rejected","reason":"title_invalid"}));
        let foreign = fixture.mutation(2, "metadata", json!({"sessionId":"foreign","expectedRevision":1,"title":"Other","labels":[]}), Uuid::new_v4()).await.unwrap();
        assert_eq!(foreign, json!({"kind":"rejected","reason":"session_unavailable"}));
        let after = fixture.query("observe", json!({"sessionId":id})).await.unwrap();
        assert_eq!(after["through"], before["through"]);
        assert_eq!(after["run"]["invocationId"], started["invocationId"]);
        assert!(fixture.query("list", json!({"kind":"after","revision":original_catalog["revision"],"cursor":id})).await.is_err());
        release.add_permits(1);
        server.await.unwrap();
        fixture.drained().await;
        let after = fixture.query("observe", json!({"sessionId":id})).await.unwrap();
        assert!(after["run"].is_null());
        assert_eq!(after["session"]["title"], "Research");
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn full_session_projection_budget_preserves_forward_and_reverse_catalog_boundaries() {
    let fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    for index in 0..14 {
        let mut input = fixture.create_input(&format!("Session {index}")).await;
        input["labels"] = json!(vec!["\"".repeat(128); 32]);
        let result = fixture
            .mutation(2, "create", input, Uuid::new_v4())
            .await
            .unwrap();
        assert_eq!(result["kind"], "created");
    }
    let expected = fixture
        .store
        .scoped_sessions::<SessionConfiguration>(
            maka_event_log::sessions::CatalogScope::IdPrefix(fixture.owner.prefix.clone()),
            None,
            maka_event_log::sessions::SessionPosition::Start,
            false,
        )
        .await
        .unwrap()
        .sessions
        .into_iter()
        .map(|session| session.id)
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 14);
    let mut page = fixture
        .query("list", json!({"kind":"start"}))
        .await
        .unwrap();
    assert!(page["sessions"].as_array().unwrap().len() < 14);
    let revision = page["revision"].clone();
    let mut forward = Vec::new();
    loop {
        assert!(serde_json::to_vec(&page).unwrap().len() <= workspace::PAGE_BYTES);
        let sessions = page["sessions"].as_array().unwrap();
        forward.extend(
            sessions
                .iter()
                .map(|s| s["id"].as_str().unwrap().to_owned()),
        );
        if page["after"].is_null() {
            break;
        }
        assert_eq!(page["after"], sessions.last().unwrap()["id"]);
        page = fixture
            .query(
                "list",
                json!({"kind":"after","revision":revision,"cursor":page["after"]}),
            )
            .await
            .unwrap();
    }
    assert_eq!(forward, expected);
    let mut reverse = page["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    while !page["before"].is_null() {
        assert_eq!(page["before"], page["sessions"][0]["id"]);
        page = fixture
            .query(
                "list",
                json!({"kind":"before","revision":revision,"cursor":page["before"]}),
            )
            .await
            .unwrap();
        assert!(serde_json::to_vec(&page).unwrap().len() <= workspace::PAGE_BYTES);
        let mut rows = page["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        rows.extend(reverse);
        reverse = rows;
    }
    assert_eq!(reverse, expected);
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn original_directory_moved_into_managed_worktrees_cannot_be_reborrowed_by_symlink() {
    let mut fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    let root = fixture._dir.path().canonicalize().unwrap();
    publish(
        &mut fixture,
        vec![DirectoryRootSpec {
            label: "Parent".into(),
            path: root,
        }],
    )
    .await;
    let roots = fixture
        .query("directories", json!({"kind":"roots"}))
        .await
        .unwrap();
    let children = fixture
        .query(
            "directories",
            json!({"kind":"children","selection":roots["roots"][0]["selection"],"after":null}),
        )
        .await
        .unwrap();
    let selected = children["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "workspace")
        .unwrap();
    let created = fixture.mutation(2, "create", json!({"title":"Original object","labels":[],"workspace":{"kind":"directory","selection":selected["selection"]}}), Uuid::new_v4()).await.unwrap();
    let id = created["session"]["id"].as_str().unwrap();
    let destination = fixture.root.join("subagent-worktrees/borrowed");
    std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
    std::fs::rename(&fixture.workspace, &destination).unwrap();
    std::os::unix::fs::symlink(&destination, &fixture.workspace).unwrap();
    assert!(
        fixture
            .send(id)
            .await
            .unwrap_err()
            .contains("agent_workspace_unavailable")
    );
    assert!(
        fixture
            .store
            .session_projection::<SessionConfiguration>(id)
            .await
            .unwrap()
            .unwrap()
            .root_turn
            .is_none()
    );
    let reused = fixture.mutation(2, "create", json!({"title":"Forbidden reuse","labels":[],"workspace":{"kind":"session","sessionId":id}}), Uuid::new_v4()).await.unwrap();
    assert_eq!(
        reused,
        json!({"kind":"rejected","reason":"workspace_unavailable"})
    );
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}

#[tokio::test]
async fn replaced_directory_marker_and_removed_publication_refuse_before_message_admission() {
    let mut fixture = Fixture::new("http://127.0.0.1:1/v1", 1, None).await;
    let input = fixture.create_input("Observed directory").await;
    let old = fixture.workspace.with_file_name("original-workspace");
    std::fs::rename(&fixture.workspace, &old).unwrap();
    std::fs::create_dir(&fixture.workspace).unwrap();
    let stale = fixture
        .mutation(2, "create", input, Uuid::new_v4())
        .await
        .unwrap();
    assert_eq!(
        stale,
        json!({"kind":"rejected","reason":"workspace_unavailable"})
    );
    assert!(!old.join(MARKER_FILE).exists());
    assert!(!fixture.workspace.join(MARKER_FILE).exists());
    std::fs::remove_dir(&fixture.workspace).unwrap();
    std::fs::rename(&old, &fixture.workspace).unwrap();
    let id = fixture.create("Original marker").await;
    let marker = fixture.workspace.join(MARKER_FILE);
    let original = std::fs::read(&marker).unwrap();
    std::fs::write(
        &marker,
        serde_json::to_vec(&json!({"schemaVersion":1,"workspaceId":Uuid::new_v4().to_string()}))
            .unwrap(),
    )
    .unwrap();
    assert!(
        fixture
            .send(&id)
            .await
            .unwrap_err()
            .contains("agent_workspace_unavailable")
    );
    assert!(
        fixture
            .store
            .session_projection::<SessionConfiguration>(&id)
            .await
            .unwrap()
            .unwrap()
            .root_turn
            .is_none()
    );
    std::fs::write(&marker, &original).unwrap();
    let other = fixture.owner.settings.workspace_roots[1].clone();
    publish(&mut fixture, vec![other]).await;
    assert!(
        fixture
            .send(&id)
            .await
            .unwrap_err()
            .contains("agent_workspace_unavailable")
    );
    assert!(
        fixture
            .store
            .session_projection::<SessionConfiguration>(&id)
            .await
            .unwrap()
            .unwrap()
            .root_turn
            .is_none()
    );
    assert_eq!(std::fs::read(&marker).unwrap(), original);
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}
