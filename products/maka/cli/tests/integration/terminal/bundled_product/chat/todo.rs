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
#[ignore = "real default product checklist model tools, paging and process restart"]
fn default_product_checklist_tracks_model_progress_and_survives_restart() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let (requests, received) = mpsc::channel();
    let server = std::thread::spawn(move || {
        for _ in 0..4 {
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
    runtime.block_on(seed_connection(
        &host,
        &namespaces,
        &base,
        json!({"fixture-model":{"codeMode":true}}),
    ));
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
    paste(&mut tui, "Keep a checklist for these steps.");
    tui.wait_for("Keep a checklist for these steps.");
    tui.send(b"\r");
    let (sent, mut socket) = next_request(&mut tui, &received);
    assert!(
        sent["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "exec")
    );
    let items: Vec<_> = (1..=25).map(|index| json!({"content":format!("Step {index:02} 中文🙂"),"status":if index == 1 { "completed" } else if index == 2 { "in_progress" } else { "pending" }})).collect();
    code_mode::tool_response(
        &mut socket,
        "exec",
        json!({"code":format!("await tools.todo_write({}); text(await tools.todo_read({{}}));",json!({"todos":items}))}),
    );
    let (result, mut socket) = next_request(&mut tui, &received);
    assert!(result.to_string().contains("Step 25 中文🙂"), "{result}");
    delta(&mut socket, "Checklist is ready.");
    complete(&mut socket);
    tui.wait_for("Checklist is ready.");
    tui.wait_for("Checklist 1/25");
    click(&mut tui, "Checklist 1/25");
    tui.wait_for("[x] Step 01 中文🙂");
    tui.wait_for("[>] Step 02 中文🙂");
    click(&mut tui, "Next");
    tui.wait_for("[ ] Step 25 中文🙂");
    evidence(&tui, "checklist-page-two");
    click(&mut tui, "Close");
    tui.wait_for("Message…");
    tui.send(b"\x11");
    tui.finish();
    let mut restored = Pty::spawn(&["application", "launch", "--root", ui.to_str().unwrap()]);
    restored.wait_until_with_timeout(|s| s.contains("Checklist 1/25"), Duration::from_secs(60));
    click(&mut restored, "Checklist 1/25");
    restored.wait_for("[>] Step 02 中文🙂");
    click(&mut restored, "Next");
    restored.wait_for("[ ] Step 25 中文🙂");
    evidence(&restored, "checklist-restarted");
    click(&mut restored, "Close");
    restored.send(b"\x0b");
    restored.wait_for("Commands");
    restored.send(b"New conversation");
    restored.wait_for("› New conversation");
    restored.send(b"\r");
    restored.wait_until(|s| {
        s.contains("Message…") && !s.contains("Checklist 1/25") && !s.contains("Restoring draft…")
    });
    click(&mut restored, "Message…");
    paste(&mut restored, "Read this conversation checklist.");
    restored.wait_for("Read this conversation checklist.");
    restored.send(b"\r");
    let (_, mut socket) = next_request(&mut restored, &received);
    code_mode::tool_response(
        &mut socket,
        "exec",
        json!({"code":"const list = await tools.todo_read({}); if (list.items.length !== 0) throw Error('another session leaked'); text('separate checklist is empty');"}),
    );
    let (result, mut socket) = next_request(&mut restored, &received);
    assert!(
        result.to_string().contains("separate checklist is empty"),
        "{result}"
    );
    delta(&mut socket, "This conversation has no checklist.");
    complete(&mut socket);
    restored.wait_for("This conversation has no checklist.");
    restored.send(b"\x11");
    restored.finish();
    server.join().unwrap();
}
