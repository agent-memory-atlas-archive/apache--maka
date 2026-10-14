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

// Actual native Seatbelt/PTY execution; other OS acceptance uses their runners.
#![cfg(target_os = "macos")]
use super::*;
use maka_host_runtime::application::{Caller, Subscription};

fn caller() -> Caller {
    Caller::new(
        Arc::new(()),
        maka_plugin_core::deployment::DeploymentRef {
            state_domain: "a".repeat(64),
            deployment: Uuid::new_v4(),
            revision: 1,
        },
        id(2),
        Uuid::new_v4(),
    )
}
fn caller_scope(f: &Fixture, caller: &Caller) -> Scope {
    f.issuer
        .issue_with(
            Identity::Remote {
                request_id: Uuid::new_v4(),
            },
            CallerGrant(caller.clone()),
            CancellationToken::new(),
        )
        .unwrap()
}
async fn binding(f: &Fixture) -> Binding {
    let client = f.app.client();
    client
        .bind(client.activation(&id(2)).unwrap(), "terminal".into())
        .await
        .unwrap()
        .remove(0)
}
async fn begin(
    f: &Fixture,
    caller: &Caller,
    method: &str,
    input: Value,
    operation: Uuid,
) -> maka_host_runtime::application::Mutation {
    let client = f.app.client();
    let prepared = client
        .prepare_mutation(
            binding(f).await,
            method.into(),
            input,
            caller_scope(f, caller),
            operation,
            Attachments::new().into(),
        )
        .await
        .unwrap();
    let intent = f
        .store
        .record_application_intent(
            prepared.reference().source.clone(),
            prepared.consumer().instance().clone(),
            prepared.reference().clone(),
            serde_json::Value::Null,
        )
        .await
        .unwrap();
    client.mutate(prepared, intent).await.unwrap()
}
async fn mutation(
    f: &Fixture,
    caller: &Caller,
    method: &str,
    input: Value,
    operation: Uuid,
) -> Value {
    let completion = begin(f, caller, method, input, operation)
        .await
        .completion()
        .await;
    completion.cleanup.unwrap();
    outcome(completion.reply)
}
fn outcome(reply: maka_plugin_core::operation::OperationReply) -> Value {
    match reply.receipt {
        Receipt::Recorded {
            outcome: OperationOutcome::Returned { value },
            ..
        } => value,
        other => panic!("Unexpected terminal receipt {other:?}"),
    }
}
async fn query(f: &Fixture, method: &str, input: Value) -> Value {
    f.app
        .client()
        .start(
            binding(f).await,
            method.into(),
            input,
            scope(&f.issuer, true),
            Attachments::new().into(),
        )
        .await
        .unwrap()
        .result()
        .await
        .unwrap()
}
async fn subscribe(f: &Fixture, caller: &Caller, method: &str, input: Value) -> Subscription {
    f.app
        .client()
        .subscribe(
            binding(f).await,
            method.into(),
            input,
            caller_scope(f, caller),
            Attachments::new().into(),
        )
        .await
        .unwrap()
}
async fn start(f: &Fixture, caller: &Caller, session: &str) -> Value {
    let result = mutation(
        f,
        caller,
        "start",
        json!({"sessionId":session, "size":{"cols":80,"rows":24}}),
        Uuid::new_v4(),
    )
    .await;
    assert_eq!(result["kind"], "completed", "{result}");
    assert_eq!(result["value"]["result"]["mode"], "pty");
    json!({"sessionId":session,"ref":result["value"]["result"]["ref"]})
}
async fn acquire(f: &Fixture, caller: &Caller, target: &Value) -> (Subscription, Value) {
    let mut stream = subscribe(f, caller, "acquire", target.clone()).await;
    let first = stream.recv().await.unwrap();
    assert_eq!(first["kind"], "completed", "{first}");
    assert_eq!(first["value"]["held"], true);
    assert_eq!(first["value"]["nextSequence"], 1);
    let controller = json!({"target":target,"controllerId":first["value"]["controllerId"]});
    (stream, controller)
}
fn command(controller: &Value, sequence: u64, source: &str) -> Value {
    json!({"controller":controller,"sequence":sequence,"actions":[{"type":"text","text":source},{"type":"key","key":"enter"}]})
}
async fn wait_file(f: &Fixture, name: &str, text: &str) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if std::fs::read_to_string(f.workspace.join(name)).is_ok_and(|s| s == text) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn terminal_original_receipts_scope_isolation_boundary_revoke_and_read_stop() {
    tokio::time::timeout(Duration::from_secs(90), async {
        let (_, f, session) = processes::fixture().await;
        let owner = caller();
        let operation = Uuid::new_v4();
        let input = json!({"sessionId":session,"command":"printf once >> once.txt"});
        let started = mutation(&f, &owner, "start", input.clone(), operation).await;
        assert_eq!(started["kind"], "completed", "{started}");
        let replayed = mutation(&f, &owner, "start", input, operation).await;
        assert_eq!(replayed, started);
        wait_file(&f, "once.txt", "once").await;
        files::change(&f, &session, "danger_full_access", false, true).await;
        let target = start(&f, &owner, &session).await;
        let record = f.store.read_shell_run(&session, maka_host_runtime::shell::request::parse_ref(target["ref"].as_str().unwrap()).unwrap()).await.unwrap().unwrap();
        assert!(record.source_run_id.is_none());
        assert_eq!(record.source_tool_call_id, record.source_turn_id);
        assert!(Uuid::parse_str(&record.source_turn_id).is_ok());
        let other = f.create("Other Session").await;
        let denied = query(&f, "read", json!({"sessionId":other,"ref":target["ref"]})).await;
        assert_eq!(denied["kind"], "failed");
        let denied = query(&f, "list", json!({"sessionId":"foreign-session"})).await;
        assert_eq!(denied["kind"], "failed");
        let mut no_caller = f.app.client().subscribe(binding(&f).await,"acquire".into(),target.clone(),scope(&f.issuer,true),Attachments::new().into()).await.unwrap();
        assert!(no_caller.recv().await.is_none());
        let no_caller = no_caller.completion().await;
        assert_eq!(no_caller.result.unwrap()["kind"], "failed");
        no_caller.cleanup.unwrap();
        let mut foreign_record = record.clone();
        foreign_record.id = "other-provider-resource".into();
        foreign_record.state = maka_agent_facts::shell_run::ShellState::Starting;
        foreign_record.revision = 1;
        f.store.create_shell_run(foreign_record).await.unwrap();
        let listed = query(&f, "list", json!({"sessionId":session})).await;
        assert_eq!(listed["value"]["total"], 2, "{listed}");
        assert!(listed["value"]["resources"].as_array().unwrap().iter().all(|r| r["result"]["output"].is_null()));
        let foreign = query(&f, "read", json!({"sessionId":session,"ref":"maka://runtime/background-tasks/other-provider-resource"})).await;
        assert_eq!(foreign["kind"], "failed");
        let (lease, controller) = acquire(&f, &owner, &target).await;
        files::change(&f, &session, "read_only", true, true).await;
        let denied = mutation(&f,&owner,"control",command(&controller,1,"printf revoked > revoked.txt"),Uuid::new_v4()).await;
        assert_eq!(denied["kind"], "failed");
        assert!(!f.workspace.join("revoked.txt").exists());
        assert_eq!(query(&f,"read",target.clone()).await["kind"],"completed");
        let stopped=mutation(&f,&owner,"stop",target,Uuid::new_v4()).await;
        assert_eq!(stopped["kind"],"completed","{stopped}");
        lease.completion().await.cleanup.unwrap();
        // The injected foreign durable fact is outside this provider's custody.
        f.store.patch_shell_run(&session,"other-provider-resource",maka_agent_facts::shell_run::ShellPatch {
            state:Some(maka_agent_facts::shell_run::ShellState::Terminal {completed_at:maka_agent_authority::now(),outcome:maka_agent_facts::shell_run::ShellOutcome::Failed {message:"fixture ended".into()},observed_at:None}),
            ..Default::default()
        }).await.unwrap();
        processes::close(&f).await;
    }).await.unwrap();
}

#[tokio::test]
async fn terminal_watch_uses_canonical_snapshots_and_user_base_sandbox() {
    tokio::time::timeout(Duration::from_secs(90), async {
        let (_, f, session) = processes::fixture().await;
        let owner = caller();
        let mut list=subscribe(&f,&owner,"watchList",json!({"sessionId":session})).await;
        assert_eq!(list.recv().await.unwrap()["value"]["total"],0);
        std::fs::write(f.root.join("terminal-private.txt"),"terminal-private-secret").unwrap();
        let started=mutation(&f,&owner,"start",json!({"sessionId":session,
            "command":"printf ready; while [ ! -f watch-release ]; do sleep 0.02; done; printf done; cat ../root/terminal-private.txt"}),Uuid::new_v4()).await;
        assert_eq!(started["kind"],"completed","{started}");
        let target=json!({"sessionId":session,"ref":started["value"]["result"]["ref"]});
        let listed=list.recv().await.unwrap();
        assert_eq!(listed["value"]["total"],1);
        assert!(listed["value"]["resources"][0]["result"]["output"].is_null());
        let mut watch=subscribe(&f,&owner,"watch",target.clone()).await;
        loop {
            let value=watch.recv().await.unwrap();
            assert_eq!(value["kind"],"completed","{value}");
            if value["value"]["result"]["output"]["stdout"].as_str().is_some_and(|s|s.contains("ready")) { break; }
        }
        std::fs::write(f.workspace.join("watch-release"),"finish").unwrap();
        loop {
            let value=watch.recv().await.unwrap();
            if value["value"]["result"]["status"]=="failed" {
                assert!(value["value"]["result"]["output"]["stdout"].as_str().unwrap().contains("readydone"));
                assert!(!value.to_string().contains("terminal-private-secret"));
                break;
            }
        }
        watch.cancel(); watch.completion().await.cleanup.unwrap();
        list.cancel(); list.completion().await.cleanup.unwrap();
        assert_eq!(query(&f,"read",target).await["kind"],"completed");
        processes::close(&f).await;
    }).await.unwrap();
}

#[tokio::test]
async fn workhub_terminal_selects_coordinator_namespace_and_rejects_original_agent_session() {
    tokio::time::timeout(Duration::from_secs(90), async {
        let (_, mut f, ordinary_session) = processes::fixture().await;
        workhub::activate(&mut f, true).await;
        let resolved = workhub::resolve(&f, workhub::selection(&f).await, Uuid::new_v4()).await;
        let coordinator = resolved["session"]["id"].as_str().unwrap();
        assert!(coordinator.starts_with("wh_"));
        let denied = query(&f, "list", json!({"sessionId": ordinary_session})).await;
        assert_eq!(denied["kind"], "failed", "{denied}");
        let listed = query(&f, "list", json!({"sessionId": coordinator})).await;
        assert_eq!(listed["kind"], "completed", "{listed}");
        let owner = caller();
        let started = mutation(&f, &owner, "start", json!({"sessionId": coordinator, "command": "printf coordinator > coordinator-terminal.txt"}), Uuid::new_v4()).await;
        assert_eq!(started["kind"], "completed", "{started}");
        wait_file(&f, "coordinator-terminal.txt", "coordinator").await;
        processes::close(&f).await;
    }).await.unwrap();
}
