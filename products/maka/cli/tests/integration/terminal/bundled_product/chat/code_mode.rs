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
use maka_agent_facts::{
    event::{Fact, InvocationOutcome, LogScope, ToolOutcome},
    execution::{EditingTools, ToolMode},
    tool_output::ToolOutput,
};
use maka_event_log::{EventLog, message_admissions::PendingMessageAdmission};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};

pub(super) fn tool_response(socket: &mut TcpStream, name: &str, input: serde_json::Value) {
    packet(
        socket,
        json!({"id":"code-model","choices":[{"index":0,"delta":{"tool_calls":[{
        "index":0,"id":"code-call","type":"function","function":{"name":name,"arguments":input.to_string()}
    }]},"finish_reason":null}]}),
    );
    packet(
        socket,
        json!({"id":"code-model","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}),
    );
    socket
        .write_all(b"e\r\ndata: [DONE]\n\n\r\n0\r\n\r\n")
        .unwrap();
    socket.flush().unwrap();
}

#[test]
#[ignore = "real default product, original queued admission, process restart and native file effect"]
fn default_product_restores_accepted_code_mode_and_original_native_effect() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let (requests, received) = mpsc::channel();
    let server = std::thread::spawn(move || {
        for _ in 0..2 {
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
    // The standard Host is nested inside an already owned UI Root.
    drop(RootOwner::create(&ui, &namespaces).unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(seed_connection(
        &host,
        &namespaces,
        &base,
        json!({"fixture-model":{"codeMode":true,"applyPatch":false}}),
    ));
    // Use the standard installer, including every default native tool provider.
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
    let mut sql = runtime
        .block_on(SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(host.join(maka_event_log::root::ROOT_DATABASE)),
        ))
        .unwrap();
    runtime.block_on(sqlx::raw_sql("CREATE TRIGGER hold_opening BEFORE INSERT ON event_log WHEN NEW.kind='invocation_opened' BEGIN SELECT RAISE(ABORT, 'fixture opening failure'); END;").execute(&mut sql)).unwrap();
    click(&mut tui, "Message…");
    paste(
        &mut tui,
        "Create restored-code-mode.txt using the accepted editing policy.",
    );
    tui.send(b"\r");
    let mut pending = None;
    tui.wait_until_with_timeout(
        |_| {
            pending = runtime
                .block_on(
                    sqlx::query_scalar::<_, String>(
                        "SELECT record_json FROM message_admissions LIMIT 1",
                    )
                    .fetch_optional(&mut sql),
                )
                .unwrap();
            pending.is_some()
        },
        Duration::from_secs(30),
    );
    let pending: PendingMessageAdmission = serde_json::from_str(&pending.unwrap()).unwrap();
    let opened: i64 = runtime
        .block_on(
            sqlx::query_scalar("SELECT COUNT(*) FROM event_log WHERE kind='invocation_opened'")
                .fetch_one(&mut sql),
        )
        .unwrap();
    assert_eq!(opened, 0);
    assert!(received.try_recv().is_err());
    tui.send(b"\x11");
    tui.finish();

    // Change the next-message policy only after the original product has closed.
    runtime.block_on(async {
        let owner = Arc::new(RootOwner::open(&host, &namespaces).unwrap());
        let config = maka_config::ConfigurationStore::for_root(owner)
            .await
            .unwrap();
        let row = config.catalog().await.unwrap().connections.remove(0);
        assert!(
            matches!(config.update_connection(serde_json::from_value(json!({
            "expected":{"connectionId":row.connection_id,"revision":row.revision},
            "changes":{"modelOverrides":{"fixture-model":{"codeMode":false,"applyPatch":true}}}
        })).unwrap()).await.unwrap(), CatalogMutationResult::Committed { .. })
        );
        config.shutdown().await.unwrap();
        sqlx::raw_sql("DROP TRIGGER hold_opening")
            .execute(&mut sql)
            .await
            .unwrap();
    });
    let mut restored = Pty::spawn(&["application", "launch", "--root", ui.to_str().unwrap()]);
    let (prompt, mut socket) = next_request(&mut restored, &received);
    assert_eq!(prompt["model"], "fixture-model");
    assert!(
        prompt["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "exec"),
        "original Code Mode was not restored"
    );
    tool_response(
        &mut socket,
        "exec",
        json!({"code":
            "if (!ALL_TOOLS.some(t => t.name === 'Write') || ALL_TOOLS.some(t => t.name === 'ApplyPatch')) throw Error('accepted editing policy changed'); const result = await tools.Write({path:'restored-code-mode.txt',content:'original default product effect'}); if (result.kind !== 'completed' || result.value.kind !== 'file_write' || result.receipt !== undefined) throw Error('native result does not match its declared schema'); text(result);"
        }),
    );
    let (next, mut socket) = next_request(&mut restored, &received);
    let result = next["messages"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|message| message["role"] == "tool")
        .unwrap();
    assert!(
        result["content"].as_str().unwrap().contains("file_write"),
        "{result}"
    );
    assert_eq!(
        std::fs::read_to_string(workspace.join("restored-code-mode.txt")).unwrap(),
        "original default product effect"
    );
    delta(&mut socket, "Restored Code Mode completed.");
    complete(&mut socket);
    restored.wait_until_with_timeout(
        |_| {
            runtime
                .block_on(
                    sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM event_log WHERE kind='invocation_ended'",
                    )
                    .fetch_one(&mut sql),
                )
                .unwrap()
                == 1
        },
        Duration::from_secs(30),
    );
    restored.send(b"\x11");
    restored.finish();
    server.join().unwrap();
    runtime.block_on(async {
        sql.close().await.unwrap();
        let owner = Arc::new(RootOwner::open(&host, &namespaces).unwrap());
        let store = EventLog::for_root(owner).await.unwrap();
        let session = &pending.invocation.session_id;
        assert!(store.pending_messages(session).await.unwrap().is_empty());
        let events = store.scoped_prefix(LogScope::Session { id: session.clone() }, 256, 4 * 1024 * 1024).await.unwrap().events;
        let openings: Vec<_> = events.iter().filter(|event| matches!(event.event.fact, Fact::InvocationOpened { .. })).collect();
        assert_eq!(openings.len(), 1);
        assert_eq!(openings[0].event.invocation, pending.invocation);
        assert!(matches!(&openings[0].event.fact, Fact::InvocationOpened { configuration: Some(configuration), .. }
            if configuration.tool_mode == ToolMode::CodeMode && configuration.editing_tools == Some(EditingTools::Structured)));
        assert!(events.iter().any(|event| matches!(event.event.fact, Fact::InvocationEnded { outcome: InvocationOutcome::Completed })));
        let writes: Vec<_> = events.iter().filter_map(|event| match &event.event.fact {
            Fact::ToolDispatched { name, operation_id, .. } if name == "Write" => Some(operation_id),
            _ => None,
        }).collect();
        assert_eq!(writes.len(), 1, "recovery must execute the native write once");
        let settlements: Vec<_> = events.iter().filter(|event| matches!(&event.event.fact,
            Fact::ToolSettled { operation_id, outcome: ToolOutcome::Succeeded { .. } } if operation_id == writes[0]
        )).collect();
        assert_eq!(settlements.len(), 1);
        assert!(matches!(store.resolve_tool_result(session, &settlements[0].event.id).await.unwrap(), ToolOutput::Operation(_)), "business projection must preserve the original native receipt");
        store.close().await.unwrap();
    });
}
