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
use maka_event_log::root::{RootNamespaces, RootOwner};
use serde_json::json;

#[test]
#[ignore = "starts the bundled product and its real Host in a native PTY"]
fn bundled_product_launch_initializes_and_reopens_original_roots_without_node() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let host = directory.path().join("host");
    let ui = directory.path().join("ui");
    let configuration = directory.path().join("installation.json");
    std::fs::write(
        &configuration,
        serde_json::to_vec(&json!({
            "presentation": "tui", "locale": "en", "scheme": "dark",
            "hostRoot": host,
            "agent": {
                "workspaceRoots": [{"label": "Workspace", "path": workspace}],
                "maxActiveRuns": 1, "maxSteps": 4, "tools": [],
                "permissions": {"access": "read_only", "requestPermissions": true}
            },
            "skills": {"bundled": true, "sources": []}
        }))
        .unwrap(),
    )
    .unwrap();
    let namespaces = RootNamespaces::for_current_account().unwrap();
    for first in [true, false] {
        let mut args = vec!["application", "launch", "--root", ui.to_str().unwrap()];
        if first {
            args.extend(["--configuration", configuration.to_str().unwrap()]);
        }
        let started = Instant::now();
        let mut tui = Pty::spawn_with(&args, false, |command| {
            command
                .current_dir(directory.path())
                .env("PATH", directory.path())
                .env_remove("NODE_PATH");
        });
        // First launch now includes authoring and three durable source commits
        // before the same bounded UI startup previously exercised on its own.
        tui.wait_until_with_timeout(
            |screen| screen.contains("Settings"),
            Duration::from_secs(if first { 60 } else { 30 }),
        );
        eprintln!(
            "bundled launch first={first}: first frame in {:?}",
            started.elapsed()
        );
        tui.send(b"\x0b");
        tui.wait_for("Commands");
        tui.send(b"Skills");
        tui.wait_for("› Skills");
        tui.send(b"\r");
        tui.wait_until(|screen| {
            screen.contains("maka-plugin-authoring") && !screen.contains("Loading skills")
        });
        tui.send(b"\x1b");
        tui.wait_until(|screen| !screen.contains("╭Skills"));
        tui.send(b"\x11");
        tui.finish();
        assert!(!tui.screen.snapshot().unwrap().alternate_screen);
        if first {
            // No authoring input remains for reopening; the record selects the
            // committed deployment, whose user-updated head must remain authoritative.
            std::fs::remove_file(&configuration).unwrap();
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let owner = std::sync::Arc::new(RootOwner::open(&ui, &namespaces).unwrap());
                let store = maka_event_log::EventLog::for_root(owner).await.unwrap();
                let launch = store.application_launch().await.unwrap().unwrap();
                assert!(launch.pending.is_none());
                let mut loaded = store
                    .load_application(launch.deployment)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(loaded.source.reference.revision, 1);
                loaded
                    .source
                    .input
                    .instances
                    .get_mut("appearance")
                    .unwrap()
                    .configuration
                    .values
                    .insert("scheme".into(), json!("light"));
                store
                    .commit_application(
                        launch.deployment,
                        1,
                        loaded.source.input,
                        loaded.source.remote_exports,
                        loaded.packages,
                    )
                    .await
                    .unwrap();
                store.close().await.unwrap();
            });
        } else {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let owner = std::sync::Arc::new(RootOwner::open(&ui, &namespaces).unwrap());
                let store = maka_event_log::EventLog::for_root(owner).await.unwrap();
                let launch = store.application_launch().await.unwrap().unwrap();
                assert_eq!(
                    store
                        .read_application_metadata(launch.deployment)
                        .await
                        .unwrap()
                        .unwrap()
                        .source
                        .reference
                        .revision,
                    2
                );
                store.close().await.unwrap();
            });
        }
        for root in [&host, &ui] {
            drop(RootOwner::open(root, &namespaces).unwrap());
        }
    }
}

#[test]
#[ignore = "faults a real bootstrap after its Host commit, then resumes in a native PTY"]
fn bundled_product_launch_recovers_partial_commits_without_reauthoring() {
    use maka_event_log::EventLog;
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    use std::sync::Arc;
    let directory = tempfile::tempdir().unwrap();
    let host = directory.path().join("host");
    let ui = directory.path().join("ui");
    let configuration = directory.path().join("installation.json");
    std::fs::write(
        &configuration,
        serde_json::to_vec(&json!({
            "locale":"en", "scheme":"dark", "hostRoot":host,
            "agent":{"workspaceRoots":[{"label":"Workspace","path":directory.path()}],
                "maxActiveRuns":1,"maxSteps":4,"tools":[],
                "permissions":{"access":"read_only","requestPermissions":true}}
        }))
        .unwrap(),
    )
    .unwrap();
    let namespaces = RootNamespaces::for_current_account().unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let owner = Arc::new(RootOwner::create(&ui, &namespaces).unwrap());
        let store = EventLog::for_root(owner.clone()).await.unwrap();
        store.close().await.unwrap();
        let mut sql = SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(ui.join(maka_event_log::root::ROOT_DATABASE))).await.unwrap();
        // A deferred FK fails the actual management COMMIT after the independent
        // Host COMMIT succeeded. No production fault switch is involved.
        sqlx::raw_sql("CREATE TABLE launch_abort (deployment TEXT, revision INTEGER, FOREIGN KEY(deployment,revision) REFERENCES application_sources(deployment,revision) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER reject_launch_source AFTER INSERT ON application_sources BEGIN INSERT INTO launch_abort VALUES ('missing',1); END;")
            .execute(&mut sql).await.unwrap();
        sql.close().await.unwrap();
        drop(owner);
    });
    let failed = Command::new(env!("CARGO_BIN_EXE_maka"))
        .args([
            "application",
            "launch",
            "--root",
            ui.to_str().unwrap(),
            "--configuration",
            configuration.to_str().unwrap(),
        ])
        .current_dir(directory.path())
        .env("PATH", directory.path())
        .env_remove("NODE_PATH")
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(
        String::from_utf8_lossy(&failed.stderr).contains("commit"),
        "{}",
        String::from_utf8_lossy(&failed.stderr)
    );
    let (ui_id, host_id) = runtime.block_on(async {
        let owner = Arc::new(RootOwner::open(&ui, &namespaces).unwrap());
        let store = EventLog::for_root(owner.clone()).await.unwrap();
        let record = store.application_launch().await.unwrap().unwrap();
        assert!(record.pending.is_some());
        assert!(
            store
                .read_application_metadata(record.deployment)
                .await
                .unwrap()
                .is_none()
        );
        let host_id = record.configuration["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|host| host["location"]["kind"] == "process")
            .unwrap()["deployment"]
            .as_str()
            .unwrap()
            .parse::<uuid::Uuid>()
            .unwrap();
        store.close().await.unwrap();
        let mut sql = SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(ui.join(maka_event_log::root::ROOT_DATABASE)),
        )
        .await
        .unwrap();
        sqlx::raw_sql("DROP TRIGGER reject_launch_source; DROP TABLE launch_abort;")
            .execute(&mut sql)
            .await
            .unwrap();
        sql.close().await.unwrap();
        drop(owner);
        let owner = Arc::new(RootOwner::open(&host, &namespaces).unwrap());
        let store = EventLog::for_root(owner).await.unwrap();
        let mut loaded = store.load_application(host_id).await.unwrap().unwrap();
        assert_eq!(loaded.source.reference.revision, 1);
        loaded
            .source
            .input
            .instances
            .get_mut("agent")
            .unwrap()
            .configuration
            .values
            .insert("maxSteps".into(), json!(5));
        store
            .commit_application(
                host_id,
                1,
                loaded.source.input,
                loaded.source.remote_exports,
                loaded.packages,
            )
            .await
            .unwrap();
        store.close().await.unwrap();
        (record.deployment, host_id)
    });
    std::fs::remove_file(configuration).unwrap();
    let mut tui = Pty::spawn_with(
        &["application", "launch", "--root", ui.to_str().unwrap()],
        false,
        |command| {
            command
                .current_dir(directory.path())
                .env("PATH", directory.path())
                .env_remove("NODE_PATH");
        },
    );
    // Recovery first commits the two missing deployments, just as installation
    // does. Bound that preparation separately from the unchanged UI deadline.
    let preparation = Instant::now();
    tui.wait_for_output("Opening product…", Duration::from_secs(30));
    eprintln!("partial launch preparation: {:?}", preparation.elapsed());
    tui.wait_until_with_timeout(
        |screen| screen.contains("Settings"),
        Duration::from_secs(30),
    );
    tui.send(b"\x11");
    tui.finish();
    runtime.block_on(async {
        let owner = Arc::new(RootOwner::open(&ui, &namespaces).unwrap());
        let store = EventLog::for_root(owner).await.unwrap();
        let record = store.application_launch().await.unwrap().unwrap();
        assert_eq!(record.deployment, ui_id);
        assert!(record.pending.is_none());
        assert_eq!(
            store
                .read_application_metadata(ui_id)
                .await
                .unwrap()
                .unwrap()
                .source
                .reference
                .revision,
            1
        );
        store.close().await.unwrap();
        let owner = Arc::new(RootOwner::open(&host, &namespaces).unwrap());
        let store = EventLog::for_root(owner).await.unwrap();
        let loaded = store
            .read_application_metadata(host_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.source.reference.revision, 2);
        assert_eq!(
            loaded.source.input.instances["agent"].configuration.values["maxSteps"],
            json!(5)
        );
        store.close().await.unwrap();
    });
}

#[test]
#[ignore = "real foreground request processing and source reload in a native PTY"]
fn bundled_ui_authoring_installs_and_reloads_original_source_without_replaying() {
    use maka_capabilities::storage::{Data, Mutation, Namespace};
    use maka_event_log::EventLog;
    use maka_packages::Package;
    use maka_plugin_core::{composition::InstanceId, scope::Scope};
    use std::{collections::BTreeMap, sync::Arc};
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let host = directory.path().join("host");
    let ui = directory.path().join("ui");
    let configuration = directory.path().join("configuration.json");
    std::fs::write(&configuration,serde_json::to_vec(&json!({
        "presentation":"tui","locale":"en","scheme":"dark","hostRoot":host,
        "agent":{"workspaceRoots":[{"label":"Workspace","path":workspace}],"maxActiveRuns":1,"maxSteps":4,"tools":[],"permissions":{"access":"read_only","requestPermissions":true}}
    })).unwrap()).unwrap();
    let mut first = Pty::spawn(&[
        "application",
        "launch",
        "--root",
        ui.to_str().unwrap(),
        "--configuration",
        configuration.to_str().unwrap(),
    ]);
    first.wait_until_with_timeout(
        |screen| screen.contains("Settings"),
        Duration::from_secs(60),
    );
    first.send(b"\x11");
    first.finish();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let namespaces = RootNamespaces::for_current_account().unwrap();
    let namespace = Namespace::new("maka.application.authoring", Scope::Profile).unwrap();
    let ids = ["a".repeat(64), "b".repeat(64)];
    let (deployment,prefix)=runtime.block_on(async {
        let store=EventLog::for_root(Arc::new(RootOwner::open(&ui,&namespaces).unwrap())).await.unwrap();
        let launch=store.application_launch().await.unwrap().unwrap();
        let loaded=store.load_application(launch.deployment).await.unwrap().unwrap();
        let host_id=launch.configuration["hosts"].as_array().unwrap().iter().find(|v|v["name"]=="local").unwrap()["deployment"].as_str().unwrap().parse().unwrap();
        let footer=loaded.packages.iter().find(|p|p.manifest().id=="maka.agent.composer-footer").unwrap();
        let mut files=footer.files().clone();
        files.get_mut("tui.js").unwrap().extend_from_slice(b"\n// runtime authored footer\n");
        let authored=Package::new(files.clone()).unwrap().application_ref();
        let files:BTreeMap<_,_>=files.into_iter().map(|(path,bytes)|(path,String::from_utf8(bytes).unwrap())).collect();
        let mut input=serde_json::to_value(&loaded.source.input).unwrap();
        input["instances"]["composer-footer"]["package"]=json!({"source":"pinned","package":authored});
        input["instances"]["agent-screen"]["configuration"]["values"]["locale"]=json!("zh-CN");
        let mut packages:Vec<_>=loaded.packages.iter().map(Package::application_ref).collect();packages.push(authored);
        let commands=[
            json!({"method":"installSource","input":{"expectedRevision":1,"files":files}}),
            json!({"method":"replaceSource","input":{"expectedRevision":2,"document":{"input":input,"remoteExports":loaded.source.remote_exports,"packages":packages}}})
        ];
        let deployment=launch.deployment;
        store.shutdown().await.unwrap();drop(store);
        let store=EventLog::for_root(Arc::new(RootOwner::open(&host,&namespaces).unwrap())).await.unwrap();
        let source=store.load_application(host_id).await.unwrap().unwrap().source;
        let instance=InstanceId(vec![source.input.instances["ui-authoring"].key]);
        let prefix=maka_host_runtime::storage::instance_prefix(&source.reference.state_domain,host_id,&instance);
        // Seed accepted intents while the fixture Host is closed. The foreground,
        // management commits, journals and hot reload below are the shipped path.
        let mut mutations=vec![Mutation{key:format!("{prefix}queue"),expected_revision:None,data:Data::Present(json!(ids))}];
        for (id,command) in ids.iter().zip(commands) {mutations.push(Mutation{key:format!("{prefix}request/{id}"),expected_revision:None,data:Data::Present(json!({"session":"fixture","request":{"id":id,"command":command},"state":{"status":"queued"}}))});}
        store.plugin_data_batch(&namespace,mutations).await.unwrap();
        store.shutdown().await.unwrap();
        (deployment,prefix)
    });
    let mut resumed = Pty::spawn(&["application", "launch", "--root", ui.to_str().unwrap()]);
    resumed.wait_until_with_timeout(|screen| screen.contains("设置"), Duration::from_secs(45));
    // Let the reload's recovery callback publish its original receipt before exit.
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        resumed.read();
        std::thread::sleep(Duration::from_millis(10));
    }
    resumed.send(b"\x11");
    resumed.finish();
    runtime.block_on(async {
        let store = EventLog::for_root(Arc::new(RootOwner::open(&host, &namespaces).unwrap()))
            .await
            .unwrap();
        for id in &ids {
            let record = store
                .plugin_data(&namespace, &format!("{prefix}request/{id}"))
                .await
                .unwrap()
                .unwrap();
            let state = &record.data.value().unwrap()["state"];
            assert_eq!(state["status"], "completed", "{state}");
            assert_eq!(
                state["outcome"]["reply"]["receipt"]["outcome"]["value"]["status"], "committed",
                "{state}"
            );
        }
        store.shutdown().await.unwrap();
        let store = EventLog::for_root(Arc::new(RootOwner::open(&ui, &namespaces).unwrap()))
            .await
            .unwrap();
        assert_eq!(
            store
                .read_application_metadata(deployment)
                .await
                .unwrap()
                .unwrap()
                .source
                .reference
                .revision,
            3,
            "exactly one commit per requested edit"
        );
        store.shutdown().await.unwrap();
    });
    let restored = Command::new(env!("CARGO_BIN_EXE_maka"))
        .args([
            "application",
            "restore-defaults",
            "--root",
            ui.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&restored.stdout).unwrap();
    assert_eq!(result["revision"], 4);
    assert_eq!(result["deployment"], deployment.to_string());
    let mut restored_ui = Pty::spawn(&["application", "launch", "--root", ui.to_str().unwrap()]);
    restored_ui.wait_until_with_timeout(
        |screen| screen.contains("Settings"),
        Duration::from_secs(45),
    );
    restored_ui.send(b"\x11");
    restored_ui.finish();
    runtime.block_on(async {
        let store = EventLog::for_root(Arc::new(RootOwner::open(&host, &namespaces).unwrap()))
            .await
            .unwrap();
        // Restoration changes desired composition; accepted request history survives.
        for id in &ids {
            let record = store
                .plugin_data(&namespace, &format!("{prefix}request/{id}"))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(record.data.value().unwrap()["state"]["status"], "completed");
        }
        store.shutdown().await.unwrap();
    });
}
