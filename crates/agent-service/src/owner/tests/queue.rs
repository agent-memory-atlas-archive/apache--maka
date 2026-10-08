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

#[tokio::test]
async fn queued_followup_uses_original_worker_capacity_and_a_new_invocation() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let fixture = Fixture::new(&base, 1, None).await;
        let session = fixture.create("Queued followup").await;
        let first = fixture.send(&session).await.unwrap();
        assert_eq!(first["kind"], "started");
        let (mut first_socket, _) = listener.accept().await.unwrap();
        request(&mut first_socket).await;
        let next = fixture.mutation(2, "send", json!({"sessionId":session,"message":"original followup","model":fixture.selection().await}), Uuid::new_v4()).await.unwrap();
        assert_eq!(next["kind"], "queued");
        assert_eq!(fixture.owner.permits.available_permits(), 0);
        assert_eq!(fixture.owner.runs.lock().await.controls.len(), 1);
        let pending = fixture.store.pending_messages(&session).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].invocation.invocation_id, first["invocationId"]);
        response(&mut first_socket).await;
        let (mut second_socket, _) = listener.accept().await.unwrap();
        let second = request(&mut second_socket).await;
        assert_eq!(second["model"], "fixture-model");
        assert!(second["messages"].as_array().unwrap().iter().any(|message| message["content"] == "original followup"));
        let current = fixture.query("observe", json!({"sessionId":session})).await.unwrap();
        assert_ne!(current["run"]["invocationId"], first["invocationId"]);
        assert!(fixture.store.pending_messages(&session).await.unwrap().is_empty());
        // Put a real send at the gate before the completed successor worker.
        // Its Running handle drops while we hold the gate, cancelling that
        // handle's token; this is not a user stop or a SQL interrupt fence.
        let selection = fixture.selection().await;
        let client = fixture.app.client();
        let prepared = client.prepare_mutation(fixture.binding(2).await, "send".into(), json!({"sessionId":session,"message":"tail followup","model":selection}), scope(&fixture.issuer, true), Uuid::new_v4(), Attachments::new().into()).await.unwrap();
        let reference = prepared.reference().clone();
        drop(prepared);
        let gate = fixture.owner.runs.lock().await;
        let completed = gate.controls[&session].cancellation.clone();
        let mut tail = Box::pin(fixture.owner.send(session.clone(), "tail followup".into(), selection, &reference));
        assert!(futures_util::poll!(tail.as_mut()).is_pending());
        response(&mut second_socket).await;
        completed.cancelled().await;
        drop(gate);
        assert!(matches!(tail.await.unwrap(), SendResult::Queued { .. }));
        let (mut third_socket, _) = listener.accept().await.unwrap();
        let third = request(&mut third_socket).await;
        assert!(third["messages"].as_array().unwrap().iter().any(|message| message["content"] == "tail followup"));
        response(&mut third_socket).await;
        fixture.drained().await;
        assert!(fixture.owner.runs.lock().await.controls.is_empty());
        assert_eq!(fixture.owner.permits.available_permits(), 1);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn stopping_original_run_retracts_queue_and_fences_tail_admission() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let fixture = Fixture::new(&base, 1, None).await;
        let session = fixture.create("Stop queue").await;
        let first = fixture.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        let queued = fixture.send(&session).await.unwrap();
        assert_eq!(queued["kind"], "queued");
        let selection = fixture.selection().await;
        let client = fixture.app.client();
        let stop = client
            .prepare_mutation(
                fixture.binding(2).await,
                "stop".into(),
                json!({"sessionId":session,"invocationId":first["invocationId"]}),
                scope(&fixture.issuer, true),
                Uuid::new_v4(),
                Attachments::new().into(),
            )
            .await
            .unwrap();
        let tail = client
            .prepare_mutation(
                fixture.binding(2).await,
                "send".into(),
                json!({"sessionId":session,"message":"after stop","model":selection}),
                scope(&fixture.issuer, true),
                Uuid::new_v4(),
                Attachments::new().into(),
            )
            .await
            .unwrap();
        let stop_reference = stop.reference().clone();
        let tail_reference = tail.reference().clone();
        drop((stop, tail));
        assert!(
            fixture
                .owner
                .stop_run(
                    &session,
                    first["invocationId"].as_str().unwrap(),
                    &stop_reference
                )
                .await
                .unwrap()
        );
        // This current-thread executor has not polled the cancelled worker
        // since stop returned. Retain its gate and queue the tail send before
        // yielding, so only the durable interrupt fence can reject admission.
        let gate = fixture.owner.runs.lock().await;
        assert_eq!(
            gate.controls[&session].invocation.invocation_id,
            first["invocationId"]
        );
        let mut tail = Box::pin(fixture.owner.send(
            session.clone(),
            "after stop".into(),
            selection,
            &tail_reference,
        ));
        assert!(futures_util::poll!(tail.as_mut()).is_pending());
        drop(gate);
        assert_eq!(tail.await.err().unwrap(), "agent_admission_failed");
        fixture.drained().await;
        assert!(
            fixture
                .store
                .pending_messages(&session)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            fixture
                .store
                .message_cancelled(&session, queued["messageId"].as_str().unwrap())
                .await
                .unwrap()
        );
        assert_eq!(fixture.owner.permits.available_permits(), 1);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn activation_recovers_more_pending_sessions_than_capacity_without_new_sends() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let fixture = Fixture::new(&base, 1, None).await;
        let deployment = fixture.app.client().snapshot().effective.unwrap().deployment;
        let mut sessions = Vec::new();
        let raw = rusqlite::Connection::open(fixture.root.join(maka_event_log::root::ROOT_DATABASE)).unwrap();
        raw.execute_batch("CREATE TRIGGER hold_opening BEFORE INSERT ON event_log WHEN NEW.kind='invocation_opened' BEGIN SELECT RAISE(ABORT, 'fixture opening failure'); END;").unwrap();
        for index in 0..3 {
            let session = fixture.create(&format!("Recovery {index}")).await;
            assert!(fixture.send(&session).await.is_err());
            assert_eq!(fixture.store.pending_messages(&session).await.unwrap().len(), 1);
            sessions.push(session);
        }
        fixture.app.shutdown().await.unwrap();
        raw.execute_batch("DROP TRIGGER hold_opening").unwrap();
        drop(raw);
        let mut native = NativeRegistry::default();
        crate::provide(&mut native).unwrap();
        maka_settings_service::provide(&mut native).unwrap();
        let app = Application::start_from_store(fixture.store.clone(), deployment, native, fixture.issuer.clone(), Arc::new(Grant(fixture.issuer.clone(), None)), maka_js_runtime::plugin::Limits::default()).await.unwrap();
        app.ready().await.unwrap();
        for _ in 0..3 {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert_eq!(request(&mut socket).await["model"], "fixture-model");
            response(&mut socket).await;
        }
        for session in sessions {
            while !fixture.store.pending_messages(&session).await.unwrap().is_empty() { tokio::task::yield_now().await; }
        }
        app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}
