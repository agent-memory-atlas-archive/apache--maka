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
