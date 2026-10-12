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
use maka_host_runtime::application::Mutation;
use maka_plugin_core::operation::OperationCleanup;
use tokio::io::{AsyncBufReadExt, BufReader};

async fn begin(f: &Fixture, action: Value) -> (Scope, Mutation) {
    let scope = scope(&f.issuer, true);
    let client = f.app.client();
    let prepared = client
        .prepare_mutation(
            f.binding(2).await,
            "executorSetup".into(),
            json!({"executor":"external","action":action}),
            scope.clone(),
            Uuid::new_v4(),
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
            Value::Null,
        )
        .await
        .unwrap();
    (scope, client.mutate(prepared, intent).await.unwrap())
}
async fn process(control: &TcpListener, protocol: u8) -> (BufReader<TcpStream>, String) {
    let count = if protocol == 1 { 2 } else { 1 };
    for index in 0..count {
        let (socket, _) = control.accept().await.unwrap();
        let mut socket = BufReader::new(socket);
        let mut pid = String::new();
        socket.read_line(&mut pid).await.unwrap();
        assert!(pid.trim().parse::<u32>().is_ok());
        if index + 1 == count {
            return (socket, pid);
        }
    }
    unreachable!()
}
fn reaped(pid: &str) {
    assert!(
        !std::process::Command::new("/bin/kill")
            .args(["-0", pid.trim()])
            .output()
            .unwrap()
            .status
            .success(),
        "setup must reap its process before returning confirmed cleanup"
    );
}
async fn result(f: &Fixture, authority: Scope, mut mutation: Mutation) -> Value {
    assert!(mutation.recv().await.is_none(), "unexpected setup progress");
    let completed = mutation.completion().await;
    completed.cleanup.unwrap();
    let reference = completed.reply.reference.clone();
    let Receipt::Recorded {
        outcome: OperationOutcome::Returned { value },
        cleanup: OperationCleanup::Confirmed,
    } = completed.reply.receipt
    else {
        panic!("setup must return its business outcome and confirmed cleanup")
    };
    authority.finish().await.unwrap();
    let recovered = f
        .app
        .client()
        .recover(
            f.app.client().activation(&id(2)).unwrap(),
            reference,
            scope(&f.issuer, true),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        recovered.outcome,
        OperationOutcome::Returned {
            value: value.clone()
        }
    );
    assert_eq!(recovered.cleanup, OperationCleanup::Confirmed);
    value
}
#[tokio::test]
async fn application_setup_v1_and_v2_checks_authenticates_cancels_and_recovers() {
    tokio::time::timeout(Duration::from_secs(90), async {
        for protocol in [1, 2] {
            let (mut f, control, trace, configuration) = setup(protocol).await;
            activate_backend(&mut f, true, None, true, Some(configuration), &[]).await;
            eprintln!("setup v{protocol}: activated");
            let listed = f.query("executors", json!({})).await.unwrap();
            assert_eq!(listed["setup"], json!(["external"]));
            let (scope, mutation) = begin(&f, json!({"kind":"check"})).await;
            let (_socket, pid) = process(&control, protocol).await;
            eprintln!("setup v{protocol}: check process {pid:?} started");
            let checked = result(&f, scope, mutation).await;
            assert_eq!(checked["kind"], "checked");
            assert_eq!(checked["methods"][0]["id"], "login");
            assert_eq!(checked["methods"][0]["supported"], true);
            reaped(&pid);
            eprintln!("setup v{protocol}: check returned");
            for cancel in [false, true] {
                let (scope, mut mutation) = begin(&f, json!({"kind":"authenticate","methodId":"login"})).await;
                let (mut socket, pid) = process(&control, protocol).await;
                assert_eq!(mutation.recv().await.unwrap(), json!({"kind":"authorization_url","url":"https://login.example.invalid/authorize?state=fixture"}));
                eprintln!("setup v{protocol}: auth URL delivered");
                if cancel { scope.cancellation.cancel(); }
                else { socket.get_mut().write_all(b"continue\n").await.unwrap(); }
                eprintln!("setup v{protocol}: awaiting final outcome");
                let value = result(&f, scope, mutation).await;
                assert_eq!(value["kind"], if cancel { "unresolved" } else { "authenticated" });
                reaped(&pid);
            }
            let calls: Vec<Value> = std::fs::read_to_string(trace).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect();
            assert_eq!(calls.iter().filter(|v| v["method"] == "spawn").count(), if protocol == 1 { 6 } else { 3 });
            assert_eq!(calls.iter().filter(|v| v["method"] == "authenticate" || v["method"] == "auth/login").count(), 2);
            assert!(!calls.iter().any(|v| v["method"].as_str().is_some_and(|m| m.starts_with("session/"))), "setup must not create an Agent conversation");
            f.app.shutdown().await.unwrap();
            f.config.shutdown().await.unwrap();
        }
    }).await.unwrap();
}
#[tokio::test]
async fn application_setup_requires_both_setup_and_process_launch_grants() {
    tokio::time::timeout(Duration::from_secs(60), async {
        for denied in ["setup", "launch", "directory"] {
            let (mut f, _control, trace, configuration) = setup(2).await;
            activate_backend(&mut f, true, None, true, Some(configuration), &[denied]).await;
            let (scope, mut mutation) = begin(&f, json!({"kind":"check"})).await;
            assert!(mutation.recv().await.is_none());
            let completed = mutation.completion().await;
            completed.cleanup.unwrap();
            assert!(!matches!(completed.reply.receipt, Receipt::Recorded { outcome: OperationOutcome::Returned { ref value }, .. } if value["kind"] == "checked"));
            scope.finish().await.unwrap();
            assert!(!trace.exists(), "denied setup must not spawn a process");
            f.app.shutdown().await.unwrap();
            f.config.shutdown().await.unwrap();
        }
    }).await.unwrap();
}

async fn protocol(f: &Fixture, authority: Scope, method: &str, input: Value) -> Mutation {
    let client = f.app.client();
    let binding = client
        .bind(client.activation(&id(21)).unwrap(), "processes".into())
        .await
        .unwrap()
        .remove(0);
    let prepared = client
        .prepare_mutation(
            binding,
            method.into(),
            input,
            authority,
            Uuid::new_v4(),
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
            Value::Null,
        )
        .await
        .unwrap();
    client.mutate(prepared, intent).await.unwrap()
}
#[tokio::test]
async fn application_process_input_is_request_bound_and_private_root_remains_denied() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let (mut f, _control, _trace, configuration) = setup(2).await;
        activate_backend(&mut f, true, None, true, Some(configuration), &[]).await;
        let authority = scope(&f.issuer, true);
        let mut process = protocol(
            &f,
            authority.clone(),
            "launch",
            json!({"executable":"/bin/cat"}),
        )
        .await;
        let id = process.recv().await.unwrap()["id"].clone();
        let foreign = scope(&f.issuer, true);
        let denied = protocol(&f, foreign.clone(), "input", json!({"id":id,"bytes":[88]}))
            .await
            .completion()
            .await;
        denied.cleanup.unwrap();
        assert!(matches!(
            denied.reply.receipt,
            Receipt::Recorded {
                outcome: OperationOutcome::Unknown { .. },
                cleanup: OperationCleanup::Confirmed
            }
        ));
        foreign.finish().await.unwrap();
        for input in [
            json!({"id":id,"bytes":[65,0,255]}),
            json!({"id":id,"bytes":null}),
        ] {
            let completed = protocol(&f, authority.clone(), "input", input)
                .await
                .completion()
                .await;
            completed.cleanup.unwrap();
            assert!(matches!(
                completed.reply.receipt,
                Receipt::Recorded {
                    outcome: OperationOutcome::Returned { .. },
                    cleanup: OperationCleanup::Confirmed
                }
            ));
        }
        let mut output = Vec::new();
        while let Some(chunk) = process.recv().await {
            output.extend(serde_json::from_value::<Vec<u8>>(chunk["bytes"].clone()).unwrap());
        }
        let completed = process.completion().await;
        completed.cleanup.unwrap();
        assert_eq!(
            output,
            [65, 0, 255],
            "foreign request must not write to the original pipe"
        );
        authority.finish().await.unwrap();
        let secret = f.root.join("setup-private-fixture.txt");
        std::fs::write(&secret, "protected-host-data").unwrap();
        let authority = scope(&f.issuer, true);
        let mut process = protocol(
            &f,
            authority.clone(),
            "launch",
            json!({"executable":"/bin/cat","args":[secret]}),
        )
        .await;
        let mut output = Vec::new();
        while let Some(chunk) = process.recv().await {
            if chunk["stream"] == "stdout" {
                output.extend(serde_json::from_value::<Vec<u8>>(chunk["bytes"].clone()).unwrap());
            }
        }
        let completed = process.completion().await;
        completed.cleanup.unwrap();
        assert!(
            output.is_empty(),
            "setup installation ceiling must not expose the Host private Root"
        );
        let Receipt::Recorded {
            outcome: OperationOutcome::Returned { value },
            cleanup: OperationCleanup::Confirmed,
        } = completed.reply.receipt
        else {
            panic!("process receipt missing")
        };
        assert_eq!(value["success"], false);
        authority.finish().await.unwrap();
        f.app.shutdown().await.unwrap();
        f.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}
