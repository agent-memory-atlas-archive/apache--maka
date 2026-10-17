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

//! Independent public Agent consumer; no ACP, TUI or GPUI adapter participates.
use super::support::{self, Fixture};
use maka_agent_api::contract::{DeliveryFact, DeliveryPage, HistoryPage};
use maka_agent_ports::session::history::Role;
use maka_host_runtime::application::{
    host::{Client, LocalCall},
    transport::wire::Reply,
};
use serde_json::{Value, json};
use std::time::Duration;

async fn watch(client: &Client, session: &str, after: u64) -> LocalCall {
    let mut call = client
        .call(
            "agent",
            "watch".into(),
            json!({"sessionId":session,"after":after}),
            Default::default(),
        )
        .await
        .unwrap();
    call.start().await.unwrap();
    assert!(matches!(call.receive().await.unwrap(), Reply::Subscribed));
    call
}
async fn page(call: &mut LocalCall) -> DeliveryPage {
    let Reply::Item { value } = call.pull().await.unwrap() else {
        panic!("Agent watch ended before the original invocation settled");
    };
    serde_json::from_value(value).unwrap()
}
async fn history_page(
    client: &Client,
    session: &str,
    mut through: Option<u64>,
    position: Value,
) -> HistoryPage {
    loop {
        let page: HistoryPage = serde_json::from_value(
            support::query(
                client,
                "agent",
                "history",
                json!({
                    "sessionId":session,"through":through,"position":position,
                }),
            )
            .await,
        )
        .unwrap();
        let fence = match &page {
            HistoryPage::Preparing { through } | HistoryPage::Ready { through, .. } => *through,
        };
        if let Some(through) = through {
            assert_eq!(fence, through);
        }
        through = Some(fence);
        if matches!(page, HistoryPage::Ready { .. }) {
            return page;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_public_agent_client_reconnects_history_watch_and_pending_interaction() {
    tokio::time::timeout(Duration::from_secs(40), async {
        let mut fixture = Fixture::open().await;
        let session = fixture.session.clone();
        let prompt = format!("Agent reconnect fixture: {}", "中文🙂".repeat(7000));
        let models = support::query(&fixture.native, "settings", "models", json!({"kind":"start"})).await;
        let model = &models["choices"][0]["selection"];
        assert_eq!(model["modelId"], "fixture-model");
        let accepted = support::mutate(&fixture.native, "agent", "send", json!({
            "sessionId":session,"message":prompt,"model":model,"thinkingLevel":"off",
        })).await;
        assert_eq!(accepted["kind"], "started");
        let invocation = accepted["invocationId"].as_str().unwrap().to_owned();
        let message = accepted["messageId"].as_str().unwrap().to_owned();
        let first = fixture.model_requests.recv().await.unwrap();
        assert_eq!(first.body["model"], "fixture-model");
        assert!(first.body["tools"].as_array().unwrap().iter().any(|tool| tool["function"]["name"] == "AskUserQuestion"));
        let mut original_watch = watch(&fixture.native, &session, 0).await;
        first.reply.send(json!({"index":0,"delta":{"role":"assistant","tool_calls":[{
            "index":0,"id":"public-question","type":"function","function":{"name":"AskUserQuestion",
                "arguments":json!({"questions":[{"question":"Choose the answer","options":[{"label":"Alpha"},{"label":"Beta"}]}]}).to_string()}
        }]},"finish_reason":"tool_calls"})).unwrap();
        let mut pending = None;
        let mut opened = 0;
        let after = loop {
            match page(&mut original_watch).await {
                DeliveryPage::Interactions { pending: current } if !current.entries.is_empty() => pending = Some(current),
                DeliveryPage::Events { next_after, events, .. } => {
                    opened += events.iter().filter(|event| event.invocation_id == invocation && matches!(&event.fact, DeliveryFact::Opened)).count();
                    if pending.is_some() { break next_after; }
                }
                _ => {}
            }
        };
        assert_eq!(opened, 1);
        let pending = pending.unwrap();
        assert_eq!(pending.entries.len(), 1);
        let request = &pending.entries[0];
        let tool = request.tool_use_id.as_ref().unwrap();
        let original = support::query(&fixture.native, "agent", "interaction", json!({"sessionId":session,"requestId":request.request_id})).await;
        let HistoryPage::Ready { through, chunks: mut pinned, after: mut continuation, .. } = history_page(&fixture.native, &session, None, json!({"kind":"start"})).await else { unreachable!() };
        assert!(continuation.is_some(), "the original message must require real history pagination");

        original_watch.close().await.unwrap();
        assert!(original_watch.cleanup_confirmed());
        fixture.native.close().await.unwrap();
        fixture.client.close().await.unwrap();
        let status = Client::status(&fixture.endpoint, Some(fixture.epoch)).await.unwrap();
        assert_eq!(status.sessions, 0);
        assert!(!status.closing && !status.closed);
        assert!(fixture.model_requests.try_recv().is_err(), "disconnect must not resend or answer the pending question");

        let mut client = Client::connect(&fixture.endpoint, Some(fixture.epoch)).await.unwrap();
        let resumed = support::query(&client, "agent", "pendingInteractions", json!({"sessionId":session})).await;
        assert_eq!(resumed, serde_json::to_value(&pending).unwrap());
        let restored = support::query(&client, "agent", "interaction", json!({"sessionId":session,"requestId":request.request_id})).await;
        assert_eq!(restored, original);
        let mut updates = watch(&client, &session, after).await;
        loop {
            if let DeliveryPage::Interactions { pending: restored } = page(&mut updates).await {
                assert!(restored == pending, "watch must restore the same pending interaction snapshot");
                break;
            }
        }
        let answered = support::mutate(&client, "agent", "respond", json!({
            "sessionId":session,"requestId":request.request_id,"answer":{"kind":"question","answers":["Beta"]},
        })).await;
        assert_eq!(answered["kind"], "recorded");
        assert_eq!(answered["record"]["requestId"], request.request_id);
        let next = fixture.model_requests.recv().await.unwrap();
        let result = next.body["messages"].as_array().unwrap().iter().find(|entry| entry["role"] == "tool").unwrap();
        assert!(result["content"].as_str().unwrap().contains("Beta"));
        let stopped = support::mutate(&client, "agent", "stop", json!({"sessionId":session,"invocationId":invocation})).await;
        assert_eq!(stopped, json!({"requested":true}));
        loop {
            if let DeliveryPage::Events { events, .. } = page(&mut updates).await {
                assert!(events.iter().all(|event| event.sequence > after));
                opened += events.iter().filter(|event| event.invocation_id == invocation && matches!(&event.fact, DeliveryFact::Opened)).count();
                if events.iter().any(|event| event.invocation_id == invocation && matches!(&event.fact, DeliveryFact::Ended { .. })) { break; }
            }
        }
        assert_eq!(opened, 1, "reconnect observes the original invocation without another send");
        loop {
            let observed = support::query(&client, "agent", "observe", json!({"sessionId":session})).await;
            if observed["run"].is_null() { break; }
            tokio::task::yield_now().await;
        }
        let execution = support::query(&client, "agent", "execution", json!({"sessionId":session,"invocationId":invocation})).await;
        assert_eq!(execution["execution"]["state"], "cancelled");
        let _ = next.reply.send(json!({"index":0,"delta":{},"finish_reason":"stop"}));

        // Continue the pre-disconnect snapshot after new facts have committed.
        while let Some(cursor) = continuation {
            let HistoryPage::Ready { through: fence, chunks, after, .. } = history_page(&client, &session, Some(through), json!({"kind":"after","cursor":cursor})).await else { unreachable!() };
            assert_eq!(fence, through);
            pinned.extend(chunks);
            continuation = after;
        }
        let user: Vec<_> = pinned.iter().filter(|chunk| chunk.role == Role::User).collect();
        assert!(user.iter().all(|chunk| chunk.message_id == message));
        assert_eq!(user.iter().map(|chunk| chunk.text.as_str()).collect::<String>(), prompt);
        assert!(!pinned.iter().any(|chunk| chunk.role == Role::ToolResult), "the fixed fence predates the original question's answer");
        let final_history = support::canonical(&client, &session).await;
        let tool_result = final_history.iter().find(|chunk| chunk.role == Role::ToolResult && chunk.tool.as_ref().is_some_and(|entry| &entry.tool_use_id == tool)).unwrap();
        assert!(tool_result.text.contains("Beta"));
        updates.close().await.unwrap();
        assert!(updates.cleanup_confirmed());
        client.quit().await.unwrap();
        fixture.server.await.unwrap().unwrap();
        assert!(fixture.model_requests.try_recv().is_err(), "only the original question and its continuation reached the model");
    }).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_public_agent_client_reads_git_changes_after_reconnect() {
    tokio::time::timeout(Duration::from_secs(40), async {
        let mut fixture = Fixture::open().await;
        let session = &fixture.session;
        assert_eq!(
            support::query(&fixture.native, "agent", "gitStatus", json!({"sessionId":session})).await,
            Value::Null,
        );
        // Git only authors fixture data; queries below cross the public Host socket.
        for args in [
            vec!["init", "--quiet", "--initial-branch=main"],
            vec!["config", "core.autocrlf", "false"],
        ] {
            let output = std::process::Command::new("git")
                .arg("-C").arg(&fixture.workspace).args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", fixture.workspace.join("absent-global-config"))
                .output().unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        }
        std::fs::write(fixture.workspace.join(".git/info/exclude"), "*\n!tracked.txt\n").unwrap();
        std::fs::write(fixture.workspace.join("tracked.txt"), "staged\n").unwrap();
        let output = std::process::Command::new("git")
            .arg("-C").arg(&fixture.workspace).args(["add", "--", "tracked.txt"])
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", fixture.workspace.join("absent-global-config"))
            .output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let before = std::fs::read(fixture.workspace.join(".git/index")).unwrap();
        let status = support::query(&fixture.native, "agent", "gitStatus", json!({"sessionId":session})).await;
        assert_eq!(status, json!({
            "head":null,
            "entries":[{"area":"staged","path":{"bytesBase64":"dHJhY2tlZC50eHQ="},"kind":"added"}],
            "truncated":false,
        }));

        fixture.native.close().await.unwrap();
        fixture.client.close().await.unwrap();
        std::fs::write(fixture.workspace.join("tracked.txt"), "edited while detached\n").unwrap();
        let mut client = Client::connect(&fixture.endpoint, Some(fixture.epoch)).await.unwrap();
        let diff = support::query(&client, "agent", "gitDiff", json!({
            "sessionId":session,"area":"worktree","path":{"bytesBase64":"dHJhY2tlZC50eHQ="},
        })).await;
        assert_eq!(diff["kind"], "text");
        let changed: Vec<_> = diff["patch"].as_str().unwrap().lines().filter(|line| {
            (line.starts_with('+') && !line.starts_with("+++"))
                || (line.starts_with('-') && !line.starts_with("---"))
        }).collect();
        assert_eq!(changed, ["-staged", "+edited while detached"]);
        assert_eq!(std::fs::read(fixture.workspace.join(".git/index")).unwrap(), before);
        assert!(fixture.model_requests.try_recv().is_err(), "Git reads do not start a model invocation");
        client.quit().await.unwrap();
        fixture.server.await.unwrap().unwrap();
    }).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_public_agent_client_lists_and_watches_files_after_reconnect() {
    async fn item(call: &mut LocalCall) -> Value {
        let reply = tokio::time::timeout(Duration::from_secs(10), call.pull())
            .await
            .expect("file watch must settle its pending pull")
            .unwrap();
        let Reply::Item { value } = reply else {
            panic!("file watch ended before its expected invalidation: {reply:?}");
        };
        value
    }
    async fn subscribe(client: &Client, session: &str) -> (LocalCall, Value) {
        let mut call = client
            .call(
                "agent",
                "watchFiles".into(),
                json!({
                    "sessionId":session,"path":{"bytesBase64":"d2F0Y2hlZA=="},
                }),
                Default::default(),
            )
            .await
            .unwrap();
        call.start().await.unwrap();
        assert!(matches!(call.receive().await.unwrap(), Reply::Subscribed));
        assert!(!call.holds_caller_intent());
        let ready = item(&mut call).await;
        assert_eq!(ready["kind"], "ready");
        (call, ready["directoryIdentity"].clone())
    }
    async fn listing(client: &Client, session: &str, identity: &Value, expected: Value) {
        let page = support::query(
            client,
            "agent",
            "listFiles",
            json!({
                "sessionId":session,"path":{"bytesBase64":"d2F0Y2hlZA=="},"after":null,
            }),
        )
        .await;
        assert_eq!(&page["directoryIdentity"], identity);
        assert_eq!(page["entries"], expected);
        assert_eq!(page["next"], Value::Null);
    }
    async fn invalidated(call: &mut LocalCall) {
        assert_eq!(item(call).await, json!({"kind":"changed"}));
    }
    async fn intent_count(database: &std::path::Path) -> i64 {
        use sqlx::Connection;
        let mut connection = sqlx::SqliteConnection::connect_with(
            &sqlx::sqlite::SqliteConnectOptions::new()
                .filename(database)
                .read_only(true),
        )
        .await
        .unwrap();
        let count = sqlx::query_scalar("SELECT COUNT(*) FROM application_intents")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        connection.close().await.unwrap();
        count
    }
    tokio::time::timeout(Duration::from_secs(60), async {
        let mut fixture = Fixture::open().await;
        let session = fixture.session.clone();
        let directory = fixture.workspace.join("watched");
        std::fs::create_dir(&directory).unwrap();
        let before = intent_count(&fixture.database).await;
        // Registration and its actual Ready precede every initial directory read.
        let (mut changes, identity) = subscribe(&fixture.native, &session).await;
        listing(&fixture.native, &session, &identity, json!([])).await;
        std::fs::write(directory.join("alpha.txt"), "created\n").unwrap();
        invalidated(&mut changes).await;
        let alpha = json!([{"path":{"bytesBase64":"d2F0Y2hlZC9hbHBoYS50eHQ="},"kind":"file"}]);
        listing(&fixture.native, &session, &identity, alpha.clone()).await;
        std::fs::write(directory.join("alpha.txt"), "edited\n").unwrap();
        invalidated(&mut changes).await;
        listing(&fixture.native, &session, &identity, alpha).await;
        std::fs::rename(directory.join("alpha.txt"), directory.join("beta.txt")).unwrap();
        invalidated(&mut changes).await;
        listing(
            &fixture.native,
            &session,
            &identity,
            json!([
                {"path":{"bytesBase64":"d2F0Y2hlZC9iZXRhLnR4dA=="},"kind":"file"},
            ]),
        )
        .await;
        std::fs::remove_file(directory.join("beta.txt")).unwrap();
        invalidated(&mut changes).await;
        listing(&fixture.native, &session, &identity, json!([])).await;
        changes.close().await.unwrap();
        assert!(changes.cleanup_confirmed());

        // A terminal invalidation is also the handler's final delivered value;
        // it must not be overwritten by an invented Ready on completion.
        let (mut removed, _) = subscribe(&fixture.native, &session).await;
        std::fs::remove_dir(&directory).unwrap();
        let terminal = loop {
            let value = item(&mut removed).await;
            if value["kind"] == "changed" {
                continue;
            }
            assert_eq!(
                value,
                json!({"kind":"unavailable","reason":"directory_changed"})
            );
            break value;
        };
        let finished = tokio::time::timeout(Duration::from_secs(10), removed.pull())
            .await
            .expect("terminal file watch must confirm cleanup")
            .unwrap();
        let Reply::Finished { result, cleanup } = finished else {
            panic!("terminal file watch did not settle: {finished:?}");
        };
        assert_eq!(result.unwrap(), terminal);
        cleanup.unwrap();
        assert!(removed.cleanup_confirmed());
        fixture.native.close().await.unwrap();
        fixture.client.close().await.unwrap();
        let status = Client::status(&fixture.endpoint, Some(fixture.epoch))
            .await
            .unwrap();
        assert_eq!(status.sessions, 0);
        assert!(!status.closing && !status.closed);
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(
            directory.join("reconnected.txt"),
            "written while disconnected\n",
        )
        .unwrap();
        let mut client = Client::connect(&fixture.endpoint, Some(fixture.epoch))
            .await
            .unwrap();
        let (mut fresh, identity) = subscribe(&client, &session).await;
        listing(
            &client,
            &session,
            &identity,
            json!([
                {"path":{"bytesBase64":"d2F0Y2hlZC9yZWNvbm5lY3RlZC50eHQ="},"kind":"file"},
            ]),
        )
        .await;
        fresh.close().await.unwrap();
        assert!(fresh.cleanup_confirmed());
        assert_eq!(
            intent_count(&fixture.database).await,
            before,
            "file queries and subscriptions must not create mutation intents"
        );
        assert!(
            fixture.model_requests.try_recv().is_err(),
            "file browsing must not start a model request"
        );
        client.quit().await.unwrap();
        fixture.server.await.unwrap().unwrap();
    })
    .await
    .unwrap();
}
