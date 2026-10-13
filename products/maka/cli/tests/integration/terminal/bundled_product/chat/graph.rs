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

fn command(tui: &mut Pty, name: &str) {
    tui.send(b"\x0b");
    tui.wait_for("Close commands");
    tui.send(b"\x1b[F");
    tui.send(&[0x7f; 40]);
    tui.send(name.as_bytes());
    click(tui, &format!("› {name}"));
}
fn value(input: &serde_json::Value) -> serde_json::Value {
    let message = input["messages"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|message| message["role"] == "tool")
        .unwrap();
    serde_json::from_str(message["content"].as_str().unwrap()).unwrap()
}

#[test]
#[ignore = "real default WorkHub graph delegation, multiple coordinator conversations and epoch recovery"]
fn default_product_graph_delegates_and_keeps_separate_conversation_epochs() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let (requests, received) = mpsc::channel();
    let server = model_server(listener, requests, 8, 0);
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let ui = directory.path().join("ui");
    let namespaces = RootNamespaces::for_current_account().unwrap();
    drop(RootOwner::create(&ui, &namespaces).unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(seed_connection(
        &ui.join("host"),
        &namespaces,
        &base,
        json!({"fixture-model":{"codeMode":true,"applyPatch":false}}),
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
    command(&mut tui, "WorkHub");
    click(&mut tui, "Set up WorkHub");
    click(&mut tui, "› Workspace");
    click(&mut tui, "Create here");
    tui.wait_for("Message…");
    click(&mut tui, "Graph / Swarm");
    click(&mut tui, "Start Swarm");
    tui.wait_for("swarm #1");
    click(&mut tui, "Close");
    click(&mut tui, "Message…");
    paste(
        &mut tui,
        "Delegate and verify the original graph result 中😀",
    );
    tui.send(b"\r");
    let (sent, mut socket) = next_request(&mut tui, &received);
    assert!(sent.to_string().contains("Agent Swarm supervisor"));
    assert!(sent["tools"].to_string().contains("agent_list"));
    code_mode::tool_response(&mut socket, "agent_list", json!({"operation":"workers"}));
    let (sent, mut socket) = next_request(&mut tui, &received);
    let catalog = value(&sent);
    assert_eq!(catalog["workers"][0]["name"], "worker");
    let model = catalog["supervisorModel"]["model"].clone();
    assert_eq!(model["modelId"], "fixture-model");
    code_mode::tool_response(
        &mut socket,
        "agent_list",
        json!({"operation":"directories","worker":"worker","location":{"kind":"roots"}}),
    );
    let (sent, mut socket) = next_request(&mut tui, &received);
    let directories = value(&sent);
    let workspace = directories["roots"][0]["selection"].clone();
    assert!(workspace.is_object(), "{directories}");
    code_mode::tool_response(
        &mut socket,
        "update_agent_graph",
        json!({"operation":"add_work","work":[{"target":{"kind":"worker","worker":"worker","title":"Original graph child","workspace":workspace,"tools":[],"model":model},"instruction":"Produce the original delegated graph result."}]}),
    );
    let mut workers = 0;
    let mut read_result = false;
    let mut selected = false;
    loop {
        let (sent, mut socket) = next_request(&mut tui, &received);
        if !sent.to_string().contains("Agent Swarm supervisor") {
            workers += 1;
            assert_eq!(workers, 1);
            assert!(
                sent.to_string()
                    .contains("Produce the original delegated graph result.")
            );
            delta(&mut socket, "Original delegated result 中😀");
            complete(&mut socket);
        } else if selected {
            assert!(
                value(&sent)["update"]["finish"]["resultIds"].is_array(),
                "{sent}"
            );
            delta(&mut socket, "Graph workflow complete.");
            complete(&mut socket);
            break;
        } else if read_result {
            let graph = value(&sent);
            assert_eq!(graph["work"][0]["status"], "completed", "{graph}");
            let result = graph["work"][0]["resultId"].clone();
            code_mode::tool_response(
                &mut socket,
                "update_agent_graph",
                json!({"operation":"finish","resultIds":[result],"reason":"Verified original worker result"}),
            );
            selected = true;
        } else if sent["messages"]
            .to_string()
            .contains("has new durable worker observations")
        {
            code_mode::tool_response(
                &mut socket,
                "view_agent_graph",
                json!({"operation":"status"}),
            );
            read_result = true;
        } else {
            assert_eq!(value(&sent)["revision"], 1, "{sent}");
            code_mode::tool_response(&mut socket, "yield_agent_graph", json!({}));
        }
    }
    assert_eq!(workers, 1);
    tui.wait_for("Graph workflow complete.");
    click(&mut tui, "Graph / Swarm");
    click(&mut tui, "Read result");
    tui.wait_for("Original delegated result 中😀");
    evidence(&tui, "graph-original-result");
    click(&mut tui, "Epoch history");
    click(&mut tui, "View epoch swarm #1");
    tui.wait_for("Historical epoch · read only");
    assert!(
        !tui.screen
            .snapshot()
            .unwrap()
            .screen
            .contains("Restart Swarm")
    );
    evidence(&tui, "graph-pinned-history");
    click(&mut tui, "Close");
    command(&mut tui, "New conversation");
    tui.wait_for("Ready to start");
    click(&mut tui, "Graph / Swarm");
    tui.wait_for("Mode: Ordinary conversation");
    click(&mut tui, "Start Graph");
    tui.wait_for("graph #1");
    click(&mut tui, "Stop epoch");
    tui.wait_for("Stopped");
    click(&mut tui, "Restart Swarm");
    tui.wait_for("swarm #2");
    evidence(&tui, "graph-second-conversation");
    assert!(received.try_recv().is_err());
    tui.send(b"\x11");
    tui.finish();
    let mut tui = Pty::spawn(&["application", "launch", "--root", ui.to_str().unwrap()]);
    tui.wait_until_with_timeout(|s| s.contains("Settings"), Duration::from_secs(60));
    command(&mut tui, "WorkHub");
    click(&mut tui, "Graph / Swarm");
    tui.wait_for("swarm #1");
    tui.wait_for("Selected final result");
    click(&mut tui, "Read result");
    tui.wait_for("Original delegated result 中😀");
    evidence(&tui, "graph-restarted-original");
    assert!(received.try_recv().is_err());
    tui.send(b"\x11");
    tui.finish();
    server.join().unwrap();
}
