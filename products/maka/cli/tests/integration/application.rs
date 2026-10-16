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

use maka_event_log::root::{RootNamespaces, RootOwner};
use maka_plugin_core::composition::{Environment, ExecutionLocation, Input};
use serde_json::Value;
use std::{collections::BTreeMap, process::Command};

#[test]
fn durable_application_commands_commit_read_cas_and_never_claim_readiness() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let namespaces = RootNamespaces::for_current_account().unwrap();
    let owner = RootOwner::create(&root, &namespaces).unwrap();
    drop(owner);
    let input = Input {
        environment: Environment {
            location: ExecutionLocation::Host,
            presentation: None,
        },
        instances: BTreeMap::new(),
        recovery: None,
        launch: None,
    };
    let path = dir.path().join("input.json");
    std::fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let commit = || {
        Command::new(env!("CARGO_BIN_EXE_maka"))
            .args([
                "application",
                "commit",
                "--root",
                root.to_str().unwrap(),
                "--deployment",
                &id,
                "--expected-revision",
                "0",
                "--input",
                path.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };
    let output = commit();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let source: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(source["deployment"], id);
    assert_eq!(source["revision"], 1);
    assert!(source.get("ready").is_none());
    let read = Command::new(env!("CARGO_BIN_EXE_maka"))
        .args([
            "application",
            "read",
            "--root",
            root.to_str().unwrap(),
            "--deployment",
            &id,
        ])
        .output()
        .unwrap();
    assert!(
        read.status.success(),
        "{}",
        String::from_utf8_lossy(&read.stderr)
    );
    let loaded: Value = serde_json::from_slice(&read.stdout).unwrap();
    assert_eq!(loaded["reference"], source);
    assert_eq!(loaded["input"], serde_json::to_value(input).unwrap());
    assert!(!commit().status.success());
    assert!(RootOwner::open(&root, &namespaces).is_ok());
}

#[test]
fn source_rejection_diagnostics_cannot_execute_terminal_control_sequences() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.json");
    let root = dir.path().join("absent-root");
    std::fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({"\u{1b}]52;c;hostile\u{7}": 1})).unwrap(),
    )
    .unwrap();
    for command in ["check", "commit"] {
        let mut process = Command::new(env!("CARGO_BIN_EXE_maka"));
        process.args(["application", command, "--input"]).arg(&path);
        if command == "commit" {
            process.arg("--root").arg(&root).args([
                "--deployment",
                "00000000-0000-0000-0000-000000000001",
                "--expected-revision",
                "0",
            ]);
        }
        let output = process.output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("unknown field"));
        assert!(!output.stderr.contains(&0x1b));
        assert!(!output.stderr.contains(&0x07));
        assert!(!root.exists());
    }
}

#[test]
fn url_install_commits_validated_bytes_without_changing_the_composition() {
    use sha2::{Digest, Sha256};
    use std::{
        io::{Read, Write},
        sync::Arc,
    };
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root");
    let owner = RootOwner::create(&root, &RootNamespaces::for_current_account().unwrap()).unwrap();
    let deployment = uuid::Uuid::new_v4();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let input = Input {
        environment: Environment {
            location: ExecutionLocation::Host,
            presentation: None,
        },
        instances: BTreeMap::new(),
        recovery: None,
        launch: None,
    };
    runtime.block_on(async {
        let store = maka_event_log::EventLog::for_root(Arc::new(owner))
            .await
            .unwrap();
        store
            .commit_application(deployment, 0, input.clone(), vec![], vec![])
            .await
            .unwrap();
        store.shutdown().await.unwrap();
    });
    let package = maka_packages::Package::new(BTreeMap::from([
        ("maka.extension.json".into(), br#"{"schemaVersion":1,"id":"example.url-cli","application":{"entries":{"host":{"implementation":{"kind":"javascript","module":"host.mjs","realm":"shared"},"configuration":{"type":"object"},"imports":{},"exports":{}}}}}"#.to_vec()),
        ("host.mjs".into(), b"export function activate() { return {}; }".to_vec()),
    ])).unwrap();
    let bytes = package.to_bundle();
    let digest = hex::encode(Sha256::digest(&bytes));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!(
        "http://{}/plugin.maka-plugin",
        listener.local_addr().unwrap()
    );
    let worker = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                        .unwrap();
                    let mut request = [0; 8192];
                    let mut received = 0;
                    while !request[..received]
                        .windows(4)
                        .any(|bytes| bytes == b"\r\n\r\n")
                    {
                        assert!(
                            received < request.len(),
                            "package request headers exceed the fixture limit"
                        );
                        let read = stream.read(&mut request[received..]).unwrap();
                        assert!(read > 0, "package request ended before its headers");
                        received += read;
                    }
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    )
                    .unwrap();
                    stream.write_all(&bytes).unwrap();
                    return;
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(10))
                }
                Err(error) => panic!("package fixture: {error}"),
            }
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_maka"))
        .args([
            "application",
            "install-url",
            "--root",
            root.to_str().unwrap(),
            "--deployment",
            &deployment.to_string(),
            "--url",
            &url,
            "--sha256",
            &digest,
            "--expected-revision",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    worker.join().unwrap();
    let installed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(installed["source"]["revision"], 2);
    assert_eq!(
        installed["package"],
        serde_json::to_value(package.application_ref()).unwrap()
    );
    runtime.block_on(async {
        let owner =
            RootOwner::open(&root, &RootNamespaces::for_current_account().unwrap()).unwrap();
        let store = maka_event_log::EventLog::for_root(Arc::new(owner))
            .await
            .unwrap();
        let loaded = store.load_application(deployment).await.unwrap().unwrap();
        let (loaded_source, loaded_packages, _, _) = loaded.into_parts();
        assert_eq!(
            serde_json::to_value(loaded_source.input).unwrap(),
            serde_json::to_value(input).unwrap()
        );
        assert_eq!(
            loaded_packages[0].application_ref(),
            package.application_ref()
        );
        store.shutdown().await.unwrap();
    });
}
