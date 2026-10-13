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
use maka_agent_api::session::SessionConfiguration;
use maka_event_log::{
    EventLog,
    usage::{Origin, Outcome, Query},
};

#[test]
#[ignore = "real default product title generation, manual rename, accounting and process restart"]
fn default_product_title_updates_once_and_preserves_manual_name_after_restart() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let (requests, received) = mpsc::channel();
    let server = model_server(listener, requests, 2, 1);
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
    paste(&mut tui, "Explain how the workspace is organized.");
    tui.wait_for("Explain how the workspace is organized.");
    tui.send(b"\r");
    let (sent, mut socket) = next_request(&mut tui, &received);
    assert!(!is_title(&sent));
    delta(&mut socket, "The workspace is ready.");
    complete(&mut socket);
    tui.wait_for("The workspace is ready.");
    tui.wait_for("Workspace conversation");
    evidence(&tui, "automatic-title");
    tui.send(b"\x11");
    tui.finish();
    let mut restored = Pty::spawn(&["application", "launch", "--root", ui.to_str().unwrap()]);
    restored.wait_until_with_timeout(
        |s| s.contains("Workspace conversation"),
        Duration::from_secs(60),
    );
    restored.send(b"\x0b");
    restored.wait_for("Commands");
    restored.send(b"Conversation details");
    restored.wait_for("› Conversation details");
    restored.send(b"\r");
    restored.wait_for("Save details");
    // The details editor focuses its title after loading the committed metadata.
    restored.send(b"\x1b[F");
    restored.send(&[0x7f; 22]);
    paste(&mut restored, "My deliberate title");
    restored.wait_for("My deliberate title");
    click(&mut restored, "Save details");
    restored.wait_for("Conversation details saved.");
    click(&mut restored, "Close");
    restored.wait_for("Message…");
    click(&mut restored, "Message…");
    paste(&mut restored, "Continue this conversation.");
    restored.wait_for("Continue this conversation.");
    restored.send(b"\r");
    let (sent, mut socket) = next_request(&mut restored, &received);
    assert!(!is_title(&sent));
    delta(&mut socket, "Continuing under the chosen title.");
    complete(&mut socket);
    restored.wait_for("Continuing under the chosen title.");
    restored.wait_for("My deliberate title");
    evidence(&restored, "manual-title");
    restored.send(b"\x11");
    restored.finish();
    server.join().unwrap();
    runtime.block_on(async {
        let owner = Arc::new(RootOwner::open(&host, &namespaces).unwrap());
        let store = EventLog::for_root(owner).await.unwrap();
        let sessions = store
            .list_sessions::<SessionConfiguration>(None, None, 10)
            .await
            .unwrap();
        assert_eq!(sessions.sessions.len(), 1);
        let session = &sessions.sessions[0];
        assert_eq!(session.configuration.title, "My deliberate title");
        assert!(session.configuration.title_is_manual);
        let attempts = store
            .model_attempts(
                Query {
                    from: 0.0,
                    to: 253402300799999.0,
                    session_id: Some(session.id.clone()),
                    through: None,
                },
                0,
                100,
            )
            .await
            .unwrap()
            .attempts;
        assert_eq!(attempts.len(), 3);
        let auxiliary: Vec<_> = attempts
            .iter()
            .filter(|attempt| matches!(attempt.origin, Origin::Auxiliary { .. }))
            .collect();
        assert_eq!(auxiliary.len(), 1);
        assert_eq!(auxiliary[0].outcome, Outcome::Success);
        assert_eq!(auxiliary[0].usage.input_tokens, Some(1));
        assert_eq!(auxiliary[0].usage.output_tokens, Some(2));
        assert!(auxiliary[0].quote.is_some());
        store.close().await.unwrap();
    });
}
