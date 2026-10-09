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
use maka_runtime_host::application::{Caller, Subscription};

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
) -> maka_runtime_host::application::Mutation {
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
async fn terminal_real_pty_lease_input_replay_resize_release_and_model_exclusivity() {
    tokio::time::timeout(Duration::from_secs(90), async {
        let (listener, f, session) = processes::fixture().await;
        let owner = caller();
        let foreign = caller();
        let target = start(&f, &owner, &session).await;
        let (stream, controller) = acquire(&f, &owner, &target).await;
        let mut competitor = subscribe(&f, &foreign, "acquire", target.clone()).await;
        assert!(competitor.recv().await.is_none());
        let competitor = competitor.completion().await;
        assert_eq!(competitor.result.unwrap()["kind"], "failed");
        competitor.cleanup.unwrap();
        let input = command(&controller, 1, "printf x >> terminal.txt");
        let denied = mutation(&f, &foreign, "control", input.clone(), Uuid::new_v4()).await;
        assert_eq!(denied["kind"], "failed");
        assert!(!f.workspace.join("terminal.txt").exists());
        let written = mutation(&f, &owner, "control", input.clone(), Uuid::new_v4()).await;
        assert_eq!(written["kind"], "completed", "{written}");
        wait_file(&f, "terminal.txt", "x").await;
        assert_eq!(
            mutation(&f, &owner, "control", input, Uuid::new_v4()).await,
            written
        );
        let conflict = mutation(
            &f,
            &owner,
            "control",
            command(&controller, 1, "printf wrong >> terminal.txt"),
            Uuid::new_v4(),
        )
        .await;
        assert_eq!(conflict["kind"], "failed");
        let blocked = processes::roundtrip(
            &listener,
            &f,
            &session,
            "WriteStdin",
            json!({"ref":target["ref"],"input":"printf bad >> terminal.txt\n"}),
        )
        .await;
        assert!(blocked.to_string().contains("controlled"), "{blocked}");
        let resized = mutation(
            &f,
            &owner,
            "control",
            json!({"controller":controller,"sequence":2,"size":{"cols":100,"rows":30}}),
            Uuid::new_v4(),
        )
        .await;
        assert_eq!(resized["kind"], "completed", "{resized}");
        assert_eq!(resized["value"]["resized"], true);
        let read = query(&f, "read", target.clone()).await;
        assert_eq!(read["value"]["result"]["output"]["cols"], 100);
        assert_eq!(read["value"]["result"]["output"]["rows"], 30);
        let released = mutation(&f, &owner, "release", controller.clone(), Uuid::new_v4()).await;
        assert_eq!(released["value"]["released"], true, "{released}");
        stream.completion().await.cleanup.unwrap();
        assert_eq!(
            query(&f, "read", target.clone()).await["value"]["result"]["status"],
            "running"
        );
        let (mut next, replacement) = acquire(&f, &owner, &target).await;
        assert_ne!(controller["controllerId"], replacement["controllerId"]);
        let stale = mutation(
            &f,
            &owner,
            "control",
            command(&controller, 3, "printf stale >> terminal.txt"),
            Uuid::new_v4(),
        )
        .await;
        assert_eq!(stale["kind"], "failed");
        next.cancel();
        next.completion().await.cleanup.unwrap();
        let (last, _) = acquire(&f, &foreign, &target).await;
        drop(last); // Observer EOF releases demand without terminating the PTY.
        let (reopened, _) = acquire(&f, &owner, &target).await;
        assert_eq!(
            query(&f, "read", target.clone()).await["value"]["result"]["status"],
            "running"
        );
        drop(reopened);
        let stopped = mutation(&f, &owner, "stop", target, Uuid::new_v4()).await;
        assert_eq!(stopped["kind"], "completed", "{stopped}");
        assert_eq!(
            std::fs::read_to_string(f.workspace.join("terminal.txt")).unwrap(),
            "x"
        );
        processes::close(&f).await;
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
        let target = start(&f, &owner, &session).await;
        let record = f.store.read_shell_run(&session, maka_runtime_host::shell::request::parse_ref(target["ref"].as_str().unwrap()).unwrap()).await.unwrap().unwrap();
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
        foreign_record.state = maka_runtime::shell_run::ShellState::Starting;
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
        f.store.patch_shell_run(&session,"other-provider-resource",maka_runtime::shell_run::ShellPatch {
            state:Some(maka_runtime::shell_run::ShellState::Terminal {completed_at:crate::owner::now(),outcome:maka_runtime::shell_run::ShellOutcome::Failed {message:"fixture ended".into()},observed_at:None}),
            ..Default::default()
        }).await.unwrap();
        processes::close(&f).await;
    }).await.unwrap();
}

#[tokio::test]
async fn terminal_release_joins_accepted_backpressured_input_without_killing_shell() {
    tokio::time::timeout(Duration::from_secs(90), async {
        let (_, f, session) = processes::fixture().await;
        let owner = caller();
        let target = start(&f, &owner, &session).await;
        let (lease, controller) = acquire(&f, &owner, &target).await;
        let arm = "stty -echo -icanon min 1 time 0; printf armed > controller-armed; while [ ! -f controller-release ]; do sleep 0.02; done; dd bs=1 count=32000 of=accepted.bin 2>/dev/null";
        let sent = mutation(&f,&owner,"control",command(&controller,1,arm),Uuid::new_v4()).await;
        assert_eq!(sent["kind"],"completed","{sent}");
        wait_file(&f,"controller-armed","armed").await;
        let input=json!({"controller":controller,"sequence":2,"actions":[{"type":"text","text":"X".repeat(32000)}]});
        let accepted=begin(&f,&owner,"control",input,Uuid::new_v4()).await;
        let completion=accepted.completion();
        tokio::pin!(completion);
        assert!(tokio::time::timeout(Duration::from_secs(1), &mut completion).await.is_err(),"native input must be backpressured before release");
        let released=begin(&f,&owner,"release",controller,Uuid::new_v4()).await;
        let release_completion=released.completion();
        tokio::pin!(release_completion);
        assert!(tokio::time::timeout(Duration::from_millis(150), &mut release_completion).await.is_err(),"release must join accepted native input");
        // Let the original shell drain its PTY. Ending the UI lease must neither
        // cancel this accepted prefix nor stop the background shell.
        std::fs::write(f.workspace.join("controller-release"), "read").unwrap();
        let settled=completion.await;
        settled.cleanup.unwrap();
        let value=outcome(settled.reply);
        assert_eq!(value["kind"],"completed","{value}");
        assert_eq!(value["value"]["acceptedBytes"],32000);
        let released=release_completion.await;
        released.cleanup.unwrap();
        assert_eq!(outcome(released.reply)["value"]["released"],true);
        lease.completion().await.cleanup.unwrap();
        wait_file(&f,"accepted.bin",&"X".repeat(32000)).await;
        assert_eq!(query(&f,"read",target.clone()).await["value"]["result"]["status"],"running");
        assert_eq!(mutation(&f,&owner,"stop",target,Uuid::new_v4()).await["kind"],"completed");
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
