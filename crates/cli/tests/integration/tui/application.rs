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

use super::{AsRawFd, Command, Pty};
use local_fixture::counter as fixture;
use maka_plugin_core::composition::PresentationTarget;
fn packages(directory: &std::path::Path, poisoned: bool) -> Vec<String> {
    let (mut input, mut packages) =
        fixture::dual_fixture("return frame({count:state.count+1,revision:state.revision+1});");
    input.environment.presentation = Some(PresentationTarget::Tui);
    if poisoned {
        use maka_plugins::package::{MANIFEST_FILE, Package};
        use std::collections::BTreeMap;
        let original = packages.pop().unwrap();
        let source = std::str::from_utf8(original.file("terminal.mjs").unwrap())
            .unwrap()
            .replace(
                "return {App:{contract,component:['app']}};",
                r"throw new Error('\u001b]52;c;hostile\u0007');",
            );
        let package = Package::new(BTreeMap::from([
            (
                MANIFEST_FILE.into(),
                original.file(MANIFEST_FILE).unwrap().to_vec(),
            ),
            (
                "index.mjs".into(),
                original.file("index.mjs").unwrap().to_vec(),
            ),
            ("terminal.mjs".into(), source.into_bytes()),
        ]))
        .unwrap();
        input.instances.get_mut("app").unwrap().package = package.application_ref().into();
        packages.push(package);
    }

    let source = directory.join("input.json");
    std::fs::write(&source, serde_json::to_vec(&input).unwrap()).unwrap();
    let root = directory.join("root");
    let ns = maka_event_log::root::RootNamespaces::for_current_account().unwrap();
    drop(maka_event_log::root::RootOwner::create(&root, &ns).unwrap());
    let deployment = uuid::Uuid::new_v4().to_string();
    let mut commit = Command::new(env!("CARGO_BIN_EXE_maka"));
    commit
        .args(["application", "commit", "--root"])
        .arg(&root)
        .args([
            "--deployment",
            &deployment,
            "--expected-revision",
            "0",
            "--input",
        ])
        .arg(&source);
    for package in packages {
        let path = directory.join(format!("{}.maka-plugin", package.manifest().id));
        package.export_to(&path).unwrap();
        commit.arg("--package").arg(&path);
    }
    let result = commit.output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    vec![
        "application".into(),
        "run".into(),
        "--root".into(),
        root.to_str().unwrap().into(),
        "--deployment".into(),
        deployment,
    ]
}
#[test]
fn plugin_application_uses_actual_foreground_transactions_and_restores_the_real_terminal() {
    let directory = tempfile::tempdir().unwrap();
    let args = packages(directory.path(), false);
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    let mut tui = Pty::spawn(&args);
    tui.wait_for("Increment");
    tui.send(b"\r");
    tui.wait_until(|screen| screen.lines().any(|line| line.trim() == "1"));
    tui.send(b"\x03");
    tui.finish();
    let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
    assert_eq!(
        unsafe { libc::tcgetattr(tui.master.as_ref().unwrap().as_raw_fd(), &mut termios) },
        0
    );
    assert_ne!(termios.c_lflag & libc::ICANON, 0);
    assert_ne!(termios.c_lflag & libc::ECHO, 0);
    let snapshot = tui.screen.snapshot().unwrap();
    assert!(!snapshot.alternate_screen);
    assert!(snapshot.cursor.visible);
}
#[test]
fn plugin_application_setup_rejection_is_clean_and_diagnostics_cannot_emit_escape_sequences() {
    let directory = tempfile::tempdir().unwrap();
    let args = packages(directory.path(), false);
    let output = Command::new(env!("CARGO_BIN_EXE_maka"))
        .args(&args)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("interactive terminal is required"),
        "{:?}",
        output
    );
    assert!(!output.stderr.contains(&0x1b));
    let mut malicious = args.clone();
    malicious[3] = "missing-\x1b]52;c;hostile\x07".into();
    let output = Command::new(env!("CARGO_BIN_EXE_maka"))
        .args(&malicious)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!output.stderr.contains(&0x1b));
    assert!(!output.stderr.contains(&7));
    let failing = tempfile::tempdir().unwrap();
    let poisoned = packages(failing.path(), true);
    let output = Command::new(env!("CARGO_BIN_EXE_maka"))
        .args(&poisoned)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("hostile"));
    assert!(!output.stderr.contains(&0x1b));
    assert!(!output.stderr.contains(&7));

    let mut input: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("input.json")).unwrap())
            .unwrap();
    // Application target validation fails before any terminal mutation.
    input["environment"]["presentation"] = serde_json::json!("desktop");
    let path = directory.path().join("input.json");
    std::fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
    let mut update = Command::new(env!("CARGO_BIN_EXE_maka"));
    update
        .args([
            "application",
            "commit",
            "--root",
            &args[3],
            "--deployment",
            &args[5],
            "--expected-revision",
            "1",
            "--input",
        ])
        .arg(&path);
    for entry in std::fs::read_dir(directory.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|s| s == "maka-plugin") {
            update.arg("--package").arg(path);
        }
    }
    let committed = update.output().unwrap();
    assert!(
        committed.status.success(),
        "{}",
        String::from_utf8_lossy(&committed.stderr)
    );
    let output = Command::new(env!("CARGO_BIN_EXE_maka"))
        .args(&args)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!output.stderr.contains(&0x1b));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("stored presentation target mismatch")
    );
}

#[allow(dead_code)]
#[path = "../../../../application-bridge/tests/support/mod.rs"]
mod local_fixture;
fn local_arguments(directory: &std::path::Path) -> Vec<String> {
    let timer = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_time()
        .build()
        .unwrap();
    let ns = maka_event_log::root::RootNamespaces::for_current_account().unwrap();
    let host = uuid::Uuid::new_v4();
    let caller = uuid::Uuid::new_v4();
    timer.block_on(async {
        for (name, id, target) in [
            ("host", host, None),
            ("caller", caller, Some(PresentationTarget::Tui)),
        ] {
            let owner = std::sync::Arc::new(
                maka_event_log::root::RootOwner::create(&directory.join(name), &ns).unwrap(),
            );
            let store = maka_event_log::EventLog::for_root(owner).await.unwrap();
            let (input, packages, observations) = if let Some(target) = target {
                local_fixture::ui(target)
            } else {
                let (input, packages) = local_fixture::host();
                (input, packages, vec![])
            };
            store
                .commit_application(id, 0, input, observations, packages)
                .await
                .unwrap();
            store.close().await.unwrap();
        }
    });
    let path = directory.join("connections.json");
    std::fs::write(&path,serde_json::to_vec(&serde_json::json!({"hosts":[{
        "name":"local","root":directory.join("host"),"deployment":host,"consumer":local_fixture::identity(13),
        "imports":{"business":{"methods":["read","write"],"recovery":false},"receipts":{"methods":["read"],"recovery":true}}
    }]})).unwrap()).unwrap();
    vec![
        "application".into(),
        "run".into(),
        "--root".into(),
        directory.join("caller").to_str().unwrap().into(),
        "--deployment".into(),
        caller.to_string(),
        "--connections".into(),
        path.to_str().unwrap().into(),
    ]
}
#[test]
fn production_terminal_connections_execute_host_mutation_and_read_original_receipt_before_exit() {
    let directory = tempfile::tempdir().unwrap();
    let args = local_arguments(directory.path());
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    let mut tui = Pty::spawn(&args);
    tui.wait_for("Read Host");
    tui.send(b"\r");
    tui.wait_for("\"effects\":0");
    tui.send(b"\t\r");
    tui.wait_for("\"effects\":1");
    tui.send(b"\t\r");
    tui.wait_for("Original receipt recorded");
    tui.send(b"\x03");
    tui.finish();
    let ns = maka_event_log::root::RootNamespaces::for_current_account().unwrap();
    drop(maka_event_log::root::RootOwner::open(&directory.path().join("caller"), &ns).unwrap());
    drop(maka_event_log::root::RootOwner::open(&directory.path().join("host"), &ns).unwrap());
    let snapshot = tui.screen.snapshot().unwrap();
    assert!(!snapshot.alternate_screen);
    assert!(snapshot.cursor.visible);
}
