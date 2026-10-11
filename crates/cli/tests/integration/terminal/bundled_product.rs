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
            |screen| screen.contains("Plugins"),
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
    tui.wait_until_with_timeout(|screen| screen.contains("Plugins"), Duration::from_secs(30));
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
#[ignore = "starts the standard product and original Host without an installation file"]
fn standard_product_launch_uses_system_language_and_reopens_without_reauthoring() {
    fn click_text(tui: &mut Pty, label: &str) {
        use unicode_width::UnicodeWidthStr;
        tui.wait_for(label);
        let screen = tui.screen.snapshot().unwrap().screen;
        let (row, column) = screen
            .lines()
            .enumerate()
            .find_map(|(row, line)| {
                line.find(label).and_then(|at| {
                    line[at + label.len()..]
                        .starts_with(' ')
                        .then(|| (row + 1, line[..at].width() + 1))
                })
            })
            .unwrap();
        tui.send(format!("\x1b[<0;{column};{row}M\x1b[<0;{column};{row}m").as_bytes());
    }
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let root = directory.path().join("ui");
    let home = directory.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let mut declined = Pty::spawn_with(
        &["application", "launch", "--root", root.to_str().unwrap()],
        false,
        |command| {
            command
                .current_dir(&workspace)
                .env("HOME", &home)
                .env("LC_ALL", "zh_CN.UTF-8");
        },
    );
    declined.wait_for_output("继续 [y/N]", Duration::from_secs(10));
    declined.send(b"n\n");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        declined.read();
        if let Some(status) = declined.child.try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        assert!(Instant::now() < deadline);
    }
    assert!(!root.join("host").exists());
    for first in [true, false] {
        let mut args = vec!["application", "launch", "--root", root.to_str().unwrap()];
        if first {
            args.extend(["--workspace", workspace.to_str().unwrap()]);
        }
        let mut tui = Pty::spawn_with(&args, false, |command| {
            command
                .current_dir(&workspace)
                .env("HOME", &home)
                .env("PATH", directory.path())
                .env("LC_ALL", "zh_CN.UTF-8")
                .env_remove("NODE_PATH");
        });
        if first {
            tui.wait_for_output("继续 [y/N]", Duration::from_secs(10));
            tui.send(b"y\n");
        }
        tui.wait_until_with_timeout(|screen| screen.contains("插件"), Duration::from_secs(60));
        tui.send(b"\x0b");
        tui.wait_for("搜索命令或对话");
        tui.send("外观".as_bytes());
        tui.wait_for("› 外观");
        tui.send(b"\r");
        tui.wait_for("保存并应用");
        if first {
            tui.send(b"\r\x1b[B\x1b[B\x1b[B\x1b[B\r");
            click_text(&mut tui, "保存到文件");
            tui.wait_for("主题文件已保存");
            let file = root.join("appearance/theme.json");
            let mut theme: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
            theme["name"] = json!("External acceptance");
            std::fs::write(&file, serde_json::to_vec(&theme).unwrap()).unwrap();
            let size = libc::winsize {
                ws_row: 18,
                ws_col: 50,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            tui.frames.resize();
            tui.screen
                .resize(TerminalSize::new(50, 18).unwrap())
                .unwrap();
            assert_eq!(
                unsafe {
                    libc::ioctl(
                        tui.master.as_ref().unwrap().as_raw_fd(),
                        libc::TIOCSWINSZ,
                        &size,
                    )
                },
                0
            );
            click_text(&mut tui, "保存到文件");
            tui.wait_for("文件已被外部修改");
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&std::fs::read(&file).unwrap())
                    .unwrap()["name"],
                "External acceptance"
            );
            click_text(&mut tui, "重新读取");
            tui.wait_for("文件已载入预览");
            click_text(&mut tui, "保存并应用");
        } else {
            tui.wait_for("External acceptance");
            tui.send(b"\x1b");
        }
        tui.wait_until(|screen| !screen.contains("保存并应用"));
        tui.send(b"\x11");
        tui.finish();
        assert!(!tui.screen.snapshot().unwrap().alternate_screen);
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let namespaces = RootNamespaces::for_current_account().unwrap();
            let owner = std::sync::Arc::new(RootOwner::open(&root, &namespaces).unwrap());
            let store = maka_event_log::EventLog::for_root(owner).await.unwrap();
            let launch = store.application_launch().await.unwrap().unwrap();
            assert!(launch.pending.is_none());
            let source = store
                .load_application(launch.deployment)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(source.source.reference.revision, 1);
            assert_eq!(
                source.source.input.instances["workspace"]
                    .configuration
                    .values["locale"],
                "auto"
            );
            store.close().await.unwrap();
            drop(RootOwner::open(&root.join("host"), &namespaces).unwrap());
        });
    }
}
