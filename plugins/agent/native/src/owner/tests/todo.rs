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
    event::{Fact, ToolOutcome},
    tool_call::{ToolCallIdentity, ToolRejection},
    tools::{ToolCallContext, ToolJournal, ToolPreparer},
};

async fn fixture(base: &str) -> Fixture {
    Fixture::configured(base, None, None, None, |settings| {
        settings.todo = true;
        settings.max_steps = Some(3);
        settings.permissions.access = WorkspaceAccess::ReadOnly;
    })
    .await
}
fn last_tool(sent: &Value) -> &str {
    let messages = sent["messages"].as_array().unwrap();
    let tool = messages
        .iter()
        .rev()
        .find(|message| message["role"] == "tool")
        .unwrap_or_else(|| {
            panic!(
                "Expected a tool result; received: {:?}",
                messages
                    .iter()
                    .map(|message| (
                        message["role"].clone(),
                        message["content"]
                            .to_string()
                            .chars()
                            .take(1000)
                            .collect::<String>()
                    ))
                    .collect::<Vec<_>>()
            )
        });
    tool["content"].as_str().unwrap()
}
async fn page(f: &Fixture, session: &str, offset: usize) -> Value {
    f.query("checklist", json!({"sessionId":session,"offset":offset}))
        .await
        .unwrap()
}
pub(super) async fn reopen(f: &mut Fixture) {
    let deployment = f.app.source().unwrap().deployment;
    f.app.shutdown().await.unwrap();
    let mut native = NativeRegistry::default();
    maka_settings_native::provide(&mut native).unwrap();
    provide_all(&mut native).unwrap();
    f.app = Application::start_from_store(
        f.store.clone(),
        deployment,
        native,
        f.issuer.clone(),
        Arc::new(Grant(f.issuer.clone(), None)),
        Some(maka_host_js::factory(Default::default())),
    )
    .await
    .unwrap();
    f.app.ready().await.unwrap();
}
#[tokio::test]
async fn todo_real_model_direct_and_code_mode_keep_full_list_isolation_and_restart() {
    tokio::time::timeout(Duration::from_secs(90), async {
        for code in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let mut f = fixture(&format!("http://{}/v1", listener.local_addr().unwrap())).await;
            selection::edit(&f, |changes| {
                changes.model_overrides = Patch::Set(BTreeMap::from([("fixture-model".into(), ModelOverride { code_mode: Some(code), context_window: Some(1_000_000), input_limit: Some(900_000), ..Default::default() })]));
            }).await;
            let session = f.create("Checklist source").await;
            let other = f.create("Separate checklist").await;
            let items: Vec<Value> = (0..240).map(|index| json!({"content":format!("{index:03} {}", "😀".repeat(256)),"status":if index == 0 { "completed" } else if index == 1 { "in_progress" } else { "pending" }})).collect();
            f.send(&session).await.unwrap();
            let (mut socket, _) = listener.accept().await.unwrap();
            let sent = request(&mut socket).await;
            let tool = if code { "exec" } else { "todo_write" };
            assert!(sent["tools"].as_array().unwrap().iter().any(|t| t["function"]["name"] == tool));
            let input = if code { json!({"code":"const todos = Array.from({length:240}, (_, index) => ({content:String(index).padStart(3, '0') + ' ' + '😀'.repeat(256), status:index === 0 ? 'completed' : index === 1 ? 'in_progress' : 'pending'})); await tools.todo_write({todos}); const read = await tools.todo_read({}); if (JSON.stringify(read.items) !== JSON.stringify(todos)) throw Error('read differs from complete write'); text(read.items.at(-1));"}) } else { json!({"todos":items}) };
            service_tools::named_tool_response(&mut socket, tool, input).await;
            let (mut socket, _) = listener.accept().await.unwrap();
            let written = request(&mut socket).await;
            if code {
                assert!(last_tool(&written).contains("239"), "Code Mode must read the complete list");
            } else {
                // Ordinary large tool results use the existing bounded archive projection.
                let archived: Value = serde_json::from_str(last_tool(&written)).unwrap();
                assert_eq!(archived["kind"], "maka.archived_tool_result");
                assert_eq!(archived["toolName"], "todo_write");
                assert!(archived["page"]["next"].is_object());
            }
            if !code {
                service_tools::named_tool_response(&mut socket,"todo_read",json!({})).await;
                let (mut next, _) = listener.accept().await.unwrap();
                let read = request(&mut next).await;
                let archived: Value = serde_json::from_str(last_tool(&read)).unwrap();
                assert_eq!(archived["kind"], "maka.archived_tool_result");
                assert_eq!(archived["toolName"], "todo_read");
                response(&mut next).await;
            } else { response(&mut socket).await; }
            f.drained().await;
            let first = page(&f,&session,0).await;
            assert_eq!(first["total"],240);
            assert_eq!(first["completed"],1);
            assert_eq!(first["current"],items[1]);
            assert_eq!(first["items"],json!(items[..20]));
            assert!(serde_json::to_vec(&first).unwrap().len() < 48 * 1024);
            let mut all = vec![];
            for offset in (0..240).step_by(20) {
                let current = page(&f,&session,offset).await;
                all.extend(current["items"].as_array().unwrap().iter().cloned());
                assert_eq!(current["revision"],first["revision"]);
            }
            assert_eq!(all,items);
            assert_eq!(page(&f,&other,0).await["total"],0);
            assert!(f.query("checklist",json!({"sessionId":"foreign-session","offset":0})).await.is_err());
            let facts = service_tools::facts(&f,&session).await;
            let dispatched: Vec<_> = facts.iter().filter_map(|fact| match fact { Fact::ToolDispatched { name, operation_id, .. } if name == "todo_write" || name == "todo_read" => Some(operation_id), _ => None }).collect();
            assert_eq!(dispatched.len(),2);
            let original = f.store.scoped_prefix(maka_agent_facts::event::LogScope::Session { id: session.clone() }, 256, 4 * 1024 * 1024).await.unwrap();
            for id in dispatched {
                let event = original.events.iter().find(|event| matches!(&event.event.fact, Fact::ToolSettled { operation_id, .. } if operation_id == id)).unwrap();
                let output = f.store.resolve_tool_result(&session, &event.event.id).await.unwrap();
                let maka_agent_facts::tool_output::ToolOutput::Json(document) = output else { panic!("original checklist JSON is retained") };
                assert_eq!(document["items"],json!(items), "every item survives in each original write/read receipt");
                assert_eq!(facts.iter().filter(|fact| matches!(fact, Fact::ToolSettled { operation_id, outcome: ToolOutcome::Succeeded { .. } } if operation_id == id)).count(),1);
            }
            reopen(&mut f).await;
            assert_eq!(page(&f,&session,0).await,first);
            assert_eq!(page(&f,&other,0).await["total"],0);
            f.app.shutdown().await.unwrap();
            f.config.shutdown().await.unwrap();
        }
    }).await.unwrap();
}

#[tokio::test]
async fn todo_prepared_replacements_conflict_and_cancelled_or_closed_invocations_cannot_write() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let f = fixture(&format!("http://{}/v1", listener.local_addr().unwrap())).await;
        let session = f.create("Concurrent checklist").await;
        f.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        let invocation = f
            .owner
            .runs
            .lock()
            .await
            .controls
            .get(&session)
            .unwrap()
            .invocation
            .clone();
        let journal = ToolJournal::new(f.store.clone(), invocation.clone());
        let context = |id: &str| ToolCallContext {
            invocation: invocation.clone(),
            operation_id: id.into(),
        };
        let input = |text: &str| json!({"todos":[{"content":text,"status":"in_progress"}]});
        let first = f
            .owner
            .todo
            .prepare(
                "todo_write".into(),
                input("First"),
                context("first"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        let second = f
            .owner
            .todo
            .prepare(
                "todo_write".into(),
                input("Second"),
                context("second"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        for (id, text, prepared, success) in [
            ("first", "First", first, true),
            ("second", "Second", second, false),
        ] {
            let outcome = journal
                .invoke_prepared_call(
                    id.into(),
                    ToolCallIdentity::standalone(id.into()),
                    "todo_write".into(),
                    input(text),
                    CancellationToken::new(),
                    prepared,
                )
                .await;
            assert_eq!(outcome.is_ok(), success, "{outcome:?}");
        }
        let before = page(&f, &session, 0).await;
        assert_eq!(before["items"][0]["content"], "First");
        let token = CancellationToken::new();
        token.cancel();
        assert!(matches!(
            f.owner
                .todo
                .prepare(
                    "todo_write".into(),
                    input("Cancelled"),
                    context("cancelled"),
                    token
                )
                .await,
            Err(ToolRejection::Cancelled)
        ));
        let pending = f
            .owner
            .todo
            .prepare(
                "todo_write".into(),
                input("Late"),
                context("late"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        response(&mut socket).await;
        f.drained().await;
        assert!(
            f.owner
                .todo
                .prepare(
                    "todo_write".into(),
                    input("Closed"),
                    context("closed"),
                    CancellationToken::new()
                )
                .await
                .is_err()
        );
        assert!(
            journal
                .invoke_prepared_call(
                    "late".into(),
                    ToolCallIdentity::standalone("late".into()),
                    "todo_write".into(),
                    input("Late"),
                    CancellationToken::new(),
                    pending
                )
                .await
                .is_err()
        );
        assert_eq!(page(&f, &session, 0).await, before);
        f.app.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}
