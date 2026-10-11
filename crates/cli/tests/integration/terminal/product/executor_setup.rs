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
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

fn process(tui: &mut Pty, listener: &TcpListener) -> (TcpStream, String) {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match listener.accept() {
            Ok((socket, _)) => {
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(socket);
                let mut pid = String::new();
                reader.read_line(&mut pid).unwrap();
                assert!(pid.trim().parse::<u32>().is_ok());
                return (reader.into_inner(), pid);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                tui.read();
                assert!(
                    Instant::now() < deadline,
                    "setup did not start: {}",
                    tui.screen.snapshot().unwrap().screen
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("{error}"),
        }
    }
}
fn reaped(pid: &str) {
    assert!(
        !Command::new("/bin/kill")
            .args(["-0", pid.trim()])
            .output()
            .unwrap()
            .status
            .success()
    );
}
#[test]
#[ignore = "opt-in production ExecutionPicker, ACP authentication and foreground PTY"]
fn executor_setup_shows_live_login_url_cancels_and_reads_original_receipt() {
    let (dist, packages) = built_packages();
    let refs = packages
        .iter()
        .map(|(name, package)| (name.clone(), package.application_ref()))
        .collect();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("ui");
    let agent_root = directory.path().join("agent");
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let trace = workspace.join("auth-trace.jsonl");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let node = Command::new("node")
        .args(["-p", "process.execPath"])
        .output()
        .unwrap();
    assert!(node.status.success());
    let deployment = Uuid::new_v4();
    let agent_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let (graph, _, accent) = authored(
        &dist.join("authoring/agent-product/src/composition.mjs"),
        json!({
            "presentation":"tui","locale":"en","scheme":"light", "hostRoot":agent_root,
            "hostDeployment":agent_id,"managementDeployment":manager_id,
            "studioRoot":directory.path().join("studio"),"studioDeployment":Uuid::new_v4(),
            "workspaceRoots":[{"label":"Workspace","path":workspace}],
            "acpExecutors":[{"name":"fixture-login","configuration":{"agent":{
                "id":"fixture.login","displayName":"Fixture login","executable":String::from_utf8(node.stdout).unwrap().trim(),
                "args":[Path::new(env!("CARGO_MANIFEST_DIR")).join("../runtime-host/tests/support/acp-agent.mjs"),trace,listener.local_addr().unwrap().port().to_string(),"2"]
            }}}],
            "executorProcesses":{"filesystem":{"default":"write","rules":[],"denyGlobs":[]},"network":"allowed","helper":null,"setupDirectory":workspace}
        }),
        &refs,
    );
    let connections = directory.path().join("connections.json");
    std::fs::write(
        &connections,
        serde_json::to_vec(&graph["connections"]).unwrap(),
    )
    .unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let ns = RootNamespaces::for_current_account().unwrap();
    runtime.block_on(async {
        for (path, id, input) in [
            (&agent_root, agent_id, &graph["host"]),
            (&root, manager_id, &graph["managementHost"]),
        ] {
            let store = EventLog::for_root(Arc::new(RootOwner::create(path, &ns).unwrap()))
                .await
                .unwrap();
            store
                .commit_application(
                    id,
                    0,
                    serde_json::from_value(input.clone()).unwrap(),
                    vec![],
                    packages.values().cloned().collect(),
                )
                .await
                .unwrap();
            store.shutdown().await.unwrap();
        }
        let store = EventLog::for_root(Arc::new(RootOwner::open(&root, &ns).unwrap()))
            .await
            .unwrap();
        store
            .commit_application(
                deployment,
                0,
                serde_json::from_value(graph["ui"].clone()).unwrap(),
                serde_json::from_value(graph["remoteExports"].clone()).unwrap(),
                packages.values().cloned().collect(),
            )
            .await
            .unwrap();
        store.shutdown().await.unwrap();
    });
    let mut tui = Pty::spawn(&[
        "application",
        "run",
        "--root",
        root.to_str().unwrap(),
        "--deployment",
        &deployment.to_string(),
        "--connections",
        connections.to_str().unwrap(),
    ]);
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        tui.read();
        if tui.frames.ready()
            && tui
                .screen
                .snapshot()
                .unwrap()
                .screen
                .contains("Execution backend")
        {
            break;
        }
        assert!(
            tui.child.try_wait().unwrap().is_none() && Instant::now() < deadline,
            "setup UI startup: {}",
            String::from_utf8_lossy(&tui.output)
        );
    }
    click(&mut tui, "[ Execution backend ]", accent);
    click(&mut tui, "[ fixture-login ]", accent);
    click(&mut tui, "[ Check backend ]", accent);
    let (_, pid) = process(&mut tui, &listener);
    tui.wait_for("fixture-v2 1");
    reaped(&pid);
    let checked = tui.screen.snapshot().unwrap().screen;
    click(&mut tui, "[ Sign in: login ]", accent);
    let (mut login, pid) = process(&mut tui, &listener);
    tui.wait_for("Copy authorization URL");
    let live = tui.screen.snapshot().unwrap().screen;
    assert!(live.contains("login.example.invalid"));
    assert!(!live.contains("Authentication confirmed."));
    tui.send(b"\x1b");
    tui.wait_until(|screen| !screen.contains("Cancel setup"));
    click(&mut tui, "[ Execution backend ]", accent);
    tui.wait_for("Copy authorization URL");
    let reopened = tui.screen.snapshot().unwrap().screen;
    assert!(
        reopened.contains("Cancel setup"),
        "closing the dialog must retain the original live setup"
    );
    login.write_all(b"continue\n").unwrap();
    tui.wait_for("Authentication confirmed.");
    reaped(&pid);
    let authenticated = tui.screen.snapshot().unwrap().screen;
    click(&mut tui, "[ Check backend ]", accent);
    let (_, pid) = process(&mut tui, &listener);
    tui.wait_for("fixture-v2 1");
    reaped(&pid);
    click(&mut tui, "[ Sign in: cancel ]", accent);
    let (_, pid) = process(&mut tui, &listener);
    tui.wait_for("Copy authorization URL");
    click(&mut tui, "[ Cancel setup ]", accent);
    tui.wait_for("unconfirmed.");
    reaped(&pid);
    click(&mut tui, "[ Read original receipt ]", accent);
    tui.wait_for("unconfirmed.");
    let cancelled = tui.screen.snapshot().unwrap().screen;
    assert!(!cancelled.contains("Authentication confirmed."));
    assert!(
        cancelled.contains("Close"),
        "setup controls must remain visible after cancellation"
    );
    assert!(
        !cancelled.contains("Copy authorization URL"),
        "completed setup must not offer an expired login URL"
    );
    let requests: Vec<Value> = std::fs::read_to_string(&trace)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        requests.iter().filter(|v| v["method"] == "spawn").count(),
        4
    );
    assert_eq!(
        requests
            .iter()
            .filter(|v| v["method"] == "auth/login")
            .count(),
        2
    );
    assert!(!requests.iter().any(|v| {
        v["method"]
            .as_str()
            .is_some_and(|m| m.starts_with("session/"))
    }));
    if let Ok(path) = std::env::var("MAKA_EXECUTOR_SETUP_EVIDENCE") {
        std::fs::create_dir_all(&path).unwrap();
        for (name, value) in [
            ("checked.txt", checked),
            ("live.txt", live),
            ("reopened.txt", reopened),
            ("authenticated.txt", authenticated),
            ("cancelled.txt", cancelled),
        ] {
            std::fs::write(Path::new(&path).join(name), value).unwrap();
        }
        std::fs::write(Path::new(&path).join("terminal.ansi"), &tui.output).unwrap();
    }
    tui.send(b"\x03");
    tui.finish();
    drop(RootOwner::open(&agent_root, &ns).unwrap());
}
