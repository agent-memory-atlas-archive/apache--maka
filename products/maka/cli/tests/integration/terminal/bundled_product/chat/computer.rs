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

#[test]
#[ignore = "real default product, Computer approval, original receipt and process restart"]
fn default_product_computer_approval_and_restart_keep_grant_but_reset_vm() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let (requests, received) = mpsc::channel();
    let server = std::thread::spawn(move || {
        for _ in 0..7 {
            let (mut socket, _) = listener.accept().unwrap();
            let value = request(&mut socket);
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
            socket.flush().unwrap();
            if requests.send((value, socket)).is_err() {
                break;
            }
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let ui = directory.path().join("ui");
    let host = ui.join("host");
    let namespaces = RootNamespaces::for_current_account().unwrap();
    drop(RootOwner::create(&ui, &namespaces).unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(seed_connection(&host, &namespaces, &base, json!({})));
    let mut tui = Pty::spawn(&[
        "application",
        "launch",
        "--root",
        ui.to_str().unwrap(),
        "--workspace",
        workspace.to_str().unwrap(),
        "--trust-workspace",
        "--locale",
        "en",
    ]);
    tui.wait_until_with_timeout(|s| s.contains("Settings"), Duration::from_secs(60));
    tui.send(b"\x0b");
    tui.wait_for("Commands");
    tui.send(b"New conversation");
    tui.wait_for("› New conversation");
    tui.send(b"\r");
    tui.wait_for("Message…");
    click(&mut tui, "Message…");
    paste(&mut tui, "Use Computer Use in this conversation.");
    tui.wait_for("Use Computer Use in this conversation.");
    tui.send(b"\r");
    let (_, mut socket) = next_request(&mut tui, &received);
    code_mode::tool_response(&mut socket, "tool_search", json!({"query":"cua_repl"}));
    let (discovered, mut socket) = next_request(&mut tui, &received);
    assert!(
        discovered["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "cua_repl")
    );
    code_mode::tool_response(
        &mut socket,
        "cua_repl",
        json!({"code":"globalThis.denied = true;"}),
    );
    tui.wait_for("Review request");
    click(&mut tui, "Review request");
    tui.wait_for("Computer Use access");
    tui.wait_for("Deny access");
    evidence(&tui, "computer-deny");
    click(&mut tui, "Confirm choice");
    let (denied, mut socket) = next_request(&mut tui, &received);
    assert!(denied.to_string().contains("not granted"));
    click(&mut tui, "Close");
    tui.wait_until(|s| {
        !s.contains("Confirm choice") && !s.contains("Review request") && s.contains("Message…")
    });
    code_mode::tool_response(
        &mut socket,
        "cua_repl",
        json!({"code":"if (globalThis.denied) throw Error('denied cell ran'); globalThis.persisted = 42; await cua.listBrowsers(); nodeRepl.write('computer-original-effect');"}),
    );
    tui.wait_for("Review request");
    click(&mut tui, "Review request");
    tui.wait_for("Computer Use access");
    click(&mut tui, "Allow for this conversation");
    click(&mut tui, "Confirm choice");
    let (allowed, mut socket) = next_request(&mut tui, &received);
    assert!(
        allowed.to_string().contains("computer-original-effect"),
        "{allowed}"
    );
    click(&mut tui, "Close");
    tui.wait_until(|s| {
        !s.contains("Confirm choice") && !s.contains("Review request") && s.contains("Message…")
    });
    delta(&mut socket, "Computer access is ready.");
    complete(&mut socket);
    tui.wait_for("Computer access is ready.");
    tui.send(b"\x11");
    tui.finish();
    // Reopen the saved default composition and original Session in a new Host.
    let mut tui = Pty::spawn(&["application", "launch", "--root", ui.to_str().unwrap()]);
    tui.wait_until_with_timeout(
        |s| s.contains("Computer access is ready."),
        Duration::from_secs(60),
    );
    // History and the saved draft restore independently. Respect the visible
    // draft recovery state before editing the original conversation.
    tui.wait_until(|s| s.contains("Message…") && !s.contains("Restoring draft…"));
    click(&mut tui, "Message…");
    paste(&mut tui, "Check the Computer REPL after restart.");
    tui.wait_for("Check the Computer REPL after restart.");
    tui.send(b"\r");
    // Discovery, like the Rust baseline, is process-local. Re-discover against
    // the new Host before using its fresh REPL; the Session grant is durable.
    let (_, mut socket) = next_request(&mut tui, &received);
    code_mode::tool_response(&mut socket, "tool_search", json!({"query":"cua_repl"}));
    let (restarted, mut socket) = next_request(&mut tui, &received);
    let repl = restarted["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["function"]["name"] == "cua_repl")
        .expect("Computer tool discovered in the new Host");
    assert!(
        repl["function"]["description"]
            .as_str()
            .unwrap()
            .contains("fresh")
    );
    code_mode::tool_response(
        &mut socket,
        "cua_repl",
        json!({"code":"nodeRepl.write('after-restart-' + typeof globalThis.persisted);"}),
    );
    let (fresh, mut socket) = next_request(&mut tui, &received);
    assert!(fresh.to_string().contains("after-restart-undefined"));
    // The persisted Session grant permits a new VM; no second approval is needed.
    delta(&mut socket, "Restart verified.");
    complete(&mut socket);
    tui.wait_for("Restart verified.");
    evidence(&tui, "computer-restarted");
    tui.send(b"\x11");
    tui.finish();
    drop(received);
    server.join().unwrap();
    runtime.block_on(async {
        let owner = Arc::new(RootOwner::open(&host, &namespaces).unwrap());
        let store = maka_event_log::EventLog::for_root(owner).await.unwrap();
        let prefix = store.prefix(256, 4 * 1024 * 1024).await.unwrap();
        use maka_agent_facts::{
            event::{Fact, ToolOutcome},
            tool_output::ToolOutput,
        };
        let settled = prefix
            .events
            .iter()
            .filter(|event| {
                matches!(
                    &event.event.fact,
                    Fact::ToolSettled {
                        outcome: ToolOutcome::Succeeded { .. },
                        ..
                    }
                )
            })
            .collect::<Vec<_>>();
        let mut original = 0;
        for event in settled {
            let output = store
                .resolve_tool_result(&event.event.invocation.session_id, &event.event.id)
                .await
                .unwrap();
            if matches!(output, ToolOutput::Operation(_)) {
                original += 1;
            }
        }
        assert!(original >= 3, "original Computer receipts were retained");
        store.close().await.unwrap();
    });
}
