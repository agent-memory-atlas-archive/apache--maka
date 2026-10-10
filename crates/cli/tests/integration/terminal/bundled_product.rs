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
use serde_json::{Value, json};
use uuid::Uuid;

#[test]
#[ignore = "starts the bundled product and its real Host in a native PTY"]
fn bundled_sources_install_and_reopen_with_original_roots_without_node() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let host = directory.path().join("host");
    let ui = directory.path().join("ui");
    let output = directory.path().join("authored product");
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
    let cli = |args: Vec<String>| {
        let result = Command::new(env!("CARGO_BIN_EXE_maka"))
            .args(args)
            .current_dir(directory.path())
            .env("PATH", directory.path())
            .env_remove("NODE_PATH")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    cli(vec![
        "application".into(),
        "compose".into(),
        "--configuration".into(),
        configuration.to_str().unwrap().into(),
        "--output".into(),
        output.to_str().unwrap().into(),
    ]);
    assert!(!host.exists());
    assert!(!ui.exists());
    let installation: Value =
        serde_json::from_slice(&std::fs::read(output.join("installation.json")).unwrap()).unwrap();
    let mut packages = std::fs::read_dir(output.join("packages"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    packages.sort();
    for root in [&host, &ui] {
        cli(vec![
            "host".into(),
            "init".into(),
            "--root".into(),
            root.to_str().unwrap().into(),
        ]);
    }
    let deployment = Uuid::new_v4().to_string();
    for (root, source, id) in [
        (
            &host,
            "host",
            installation["hostDeployment"].as_str().unwrap(),
        ),
        (
            &ui,
            "management-host",
            installation["managementDeployment"].as_str().unwrap(),
        ),
        (&ui, "ui", deployment.as_str()),
    ] {
        let mut args = vec![
            "application".into(),
            "commit".into(),
            "--root".into(),
            root.to_str().unwrap().into(),
            "--deployment".into(),
            id.into(),
            "--expected-revision".into(),
            "0".into(),
            "--input".into(),
            output
                .join(format!("{source}.json"))
                .to_str()
                .unwrap()
                .into(),
        ];
        if source == "ui" {
            args.extend([
                "--remote-exports".into(),
                output.join("remote-exports.json").to_str().unwrap().into(),
            ]);
        }
        for package in &packages {
            args.extend(["--package".into(), package.to_str().unwrap().into()]);
        }
        cli(args);
    }
    let namespaces = RootNamespaces::for_current_account().unwrap();
    let connections = output.join("connections.json");
    for _ in 0..2 {
        let mut tui = Pty::spawn_with(
            &[
                "application",
                "run",
                "--root",
                ui.to_str().unwrap(),
                "--deployment",
                &deployment,
                "--connections",
                connections.to_str().unwrap(),
            ],
            false,
            |command| {
                command
                    .current_dir(directory.path())
                    .env("PATH", directory.path())
                    .env_remove("NODE_PATH");
            },
        );
        tui.wait_until_with_timeout(|screen| screen.contains("Plugins"), Duration::from_secs(30));
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
        for root in [&host, &ui] {
            drop(RootOwner::open(root, &namespaces).unwrap());
        }
    }
}
