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
#[ignore = "real default product, stopped PTY stdout and original model input"]
fn default_product_input_continues_while_the_original_stdout_owner_is_blocked() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let (requests, received) = mpsc::channel();
    let server = model_server(listener, requests, 2, 0);
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let host = directory.path().join("host");
    let ui = directory.path().join("ui");
    let configuration = directory.path().join("installation.json");
    let namespaces = RootNamespaces::for_current_account().unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(seed_connection(&host, &namespaces, &base, json!({})));
    std::fs::write(&configuration, serde_json::to_vec(&json!({
        "presentation":"tui", "locale":"en", "scheme":"dark", "hostRoot":host,
        "agent":{"workspaceRoots":[{"label":"Workspace","path":workspace}],"maxActiveRuns":1,"maxSteps":4,"tools":[],"permissions":{"access":"read_only","requestPermissions":true}}
    })).unwrap()).unwrap();
    let mut tui = Pty::spawn(&[
        "application",
        "launch",
        "--root",
        ui.to_str().unwrap(),
        "--configuration",
        configuration.to_str().unwrap(),
    ]);
    tui.wait_until_with_timeout(|s| s.contains("Settings"), Duration::from_secs(60));
    tui.send(b"\x0b");
    tui.wait_for("Close commands");
    tui.send(b"New conversation");
    tui.wait_for("› New conversation");
    tui.send(b"\r");
    tui.wait_until(|s| s.contains("Message…") && !s.contains("Restoring draft…"));
    click(&mut tui, "Message…");
    paste(&mut tui, "Prepare the original output baseline");
    tui.wait_for("Prepare the original output baseline");
    tui.send(b"\r");
    let (_, mut socket) = next_request(&mut tui, &received);
    delta(&mut socket, "The original model request completed.");
    complete(&mut socket);
    tui.wait_until(|s| {
        s.contains("Send")
            && !s.contains("Queue for next turn")
            && s.contains("The original model request completed.")
    });
    click(&mut tui, "Message…");
    let paused = tui.pause_output();
    // The large real resize produces a full ANSI frame. POLLOUT becoming false
    // and TIOCOUTQ remaining nonzero prove write-side congestion, independently
    // of the terminal emulator's display pause.
    tui.resize(240, 100);
    let queued = paused.wait_until_blocked();
    assert!(tui.child.try_wait().unwrap().is_none());
    paste(&mut tui, "Accepted while stdout is blocked 🙂");
    tui.send(b"\r");
    let (sent, mut socket) = received
        .recv_timeout(Duration::from_secs(30))
        .expect("original input must reach the model before TCOON");
    assert!(
        sent.to_string()
            .contains("Accepted while stdout is blocked 🙂")
    );
    assert!(!is_title(&sent));
    eprintln!(
        "original PTY stdout backpressure: queued={queued}; original model request received before TCOON"
    );
    delta(
        &mut socket,
        "Input progressed during real output congestion.",
    );
    complete(&mut socket);
    drop(paused);
    tui.wait_until(|s| {
        s.contains("Input progressed during real output congestion.")
            && s.contains("Send")
            && !s.contains("Queue for next turn")
    });
    evidence(&tui, "stdout-congestion-resumed");
    // Resolve coordinates from a completed resumed frame, then verify that the
    // same native editor is usable after output/geometry promotion.
    click(&mut tui, "Message…");
    paste(&mut tui, "Native input after original output completion");
    tui.wait_for("Native input after original output completion");
    tui.send(b"\x11");
    tui.finish();
    server.join().unwrap();
    assert!(!tui.screen.snapshot().unwrap().alternate_screen);
    drop(RootOwner::open(&host, &namespaces).unwrap());
}
