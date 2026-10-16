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
    usage::{AuxiliarySource, Origin, Outcome, Query},
};

fn open(tui: &mut Pty) {
    tui.send(b"\x0b");
    tui.wait_for("Close commands");
    // The palette retains its previous search while the editor is hidden.
    tui.send(b"\x1b[F");
    tui.send(&[0x7f; 16]);
    tui.send(b"Session recap");
    tui.wait_for("› Session recap");
    tui.send(b"\r");
}
#[test]
#[ignore = "real default product explicit recap, fee confirmation, draft retention and process restart"]
fn default_product_recap_reads_original_result_without_charging_on_refresh_or_restart() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let (requests, received) = mpsc::channel();
    let server = model_server(listener, requests, 2, 1);
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let ui = directory.path().join("ui");
    let host = ui.join("host");
    let host_guard = HostRootGuard::new(&host);
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
    tui.wait_for("Close commands");
    tui.send(b"New conversation");
    tui.wait_for("› New conversation");
    tui.send(b"\r");
    tui.wait_for("Message…");
    click(&mut tui, "Message…");
    paste(&mut tui, "Confirm the test result and remaining work.");
    tui.wait_for("Confirm the test result and remaining work.");
    tui.send(b"\r");
    let (_, mut socket) = next_request(&mut tui, &received);
    delta(&mut socket, "Tests passed; deployment is still pending.");
    complete(&mut socket);
    tui.wait_for("Tests passed; deployment is still pending.");
    tui.wait_for("Workspace conversation");
    click(&mut tui, "Message…");
    paste(&mut tui, "Keep this unfinished draft.");
    tui.wait_for("Keep this unfinished draft.");
    open(&mut tui);
    tui.wait_for("No saved recap yet.");
    click(&mut tui, "Generate a new recap");
    tui.wait_for("may incur");
    tui.wait_for("fixture-model");
    evidence(&tui, "recap-confirmation");
    assert!(
        received.try_recv().is_err(),
        "opening and fee review do not call the model"
    );
    click(&mut tui, "Generate recap");
    let (sent, mut socket) = next_request(&mut tui, &received);
    assert!(
        sent["messages"][0]["content"]
            .as_str()
            .unwrap()
            .starts_with("The user is returning to this session")
    );
    assert!(
        sent["messages"][1]["content"]
            .as_str()
            .unwrap()
            .contains("deployment is still pending")
    );
    assert!(sent["tools"].is_null() || sent["tools"] == json!([]));
    assert!(sent["max_tokens"] == 1024 || sent["max_completion_tokens"] == 1024);
    click(&mut tui, "Refresh saved result");
    tui.wait_for("without generating again");
    delta(&mut socket, "Tests passed. Deployment is next.");
    complete(&mut socket);
    tui.wait_for("Tests passed. Deployment is next.");
    click(&mut tui, "Refresh saved result");
    tui.wait_for("Tests passed. Deployment is next.");
    evidence(&tui, "recap-ready");
    click(&mut tui, "Close");
    tui.wait_for("Keep this unfinished draft.");
    tui.send(b"\x11");
    tui.finish();
    host_guard.stop();
    server.join().unwrap();
    let mut restored = Pty::spawn(&["application", "launch", "--root", ui.to_str().unwrap()]);
    restored.wait_until_with_timeout(
        |s| s.contains("Workspace conversation"),
        Duration::from_secs(60),
    );
    open(&mut restored);
    restored.wait_for("Tests passed. Deployment is next.");
    click(&mut restored, "Refresh saved result");
    restored.wait_for("Tests passed. Deployment is next.");
    evidence(&restored, "recap-restarted");
    restored.send(b"\x11");
    restored.finish();
    host_guard.stop();
    runtime.block_on(async {
        let store = EventLog::for_root(Arc::new(RootOwner::open(&host, &namespaces).unwrap()))
            .await
            .unwrap();
        let sessions = store
            .list_sessions::<SessionConfiguration>(None, None, 10)
            .await
            .unwrap();
        assert_eq!(sessions.sessions.len(), 1);
        let attempts = store
            .model_attempts(
                Query {
                    from: 0.0,
                    to: 253402300799999.0,
                    session_id: Some(sessions.sessions[0].id.clone()),
                    through: None,
                },
                0,
                100,
            )
            .await
            .unwrap()
            .attempts;
        assert_eq!(
            attempts.len(),
            3,
            "one main request, one title, one explicit recap"
        );
        let recap = attempts
            .iter()
            .filter(|a| {
                matches!(
                    a.origin,
                    Origin::Auxiliary {
                        source: AuxiliarySource::ApplicationOperation { .. }
                    }
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(recap.len(), 1);
        assert_eq!(recap[0].outcome, Outcome::Success);
        assert_eq!(recap[0].usage.input_tokens, Some(1));
        assert_eq!(recap[0].usage.output_tokens, Some(2));
        assert!(recap[0].quote.is_some());
        store.close().await.unwrap();
    });
}
