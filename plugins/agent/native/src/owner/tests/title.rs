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
use maka_event_log::usage::{AuxiliarySource, Origin, Outcome, Query};

async fn fixture(base: &str) -> Fixture {
    Fixture::configured(base, 1, None, None, None, |settings| {
        settings.automatic_titles = true
    })
    .await
}
async fn automatic(f: &Fixture) -> String {
    let mut input = f.create_input("Untitled").await;
    input["automaticTitle"] = json!(true);
    f.mutation(2, "create", input, Uuid::new_v4())
        .await
        .unwrap()["session"]["id"]
        .as_str()
        .unwrap()
        .into()
}
fn is_title(sent: &Value) -> bool {
    sent["messages"].as_array().unwrap().iter().any(|message| {
        message["role"] == "system"
            && message["content"]
                .as_str()
                .is_some_and(|text| text.starts_with("Generate a short session title"))
    })
}
async fn pair(listener: &TcpListener) -> ((Value, TcpStream), (Value, TcpStream)) {
    let (mut first, _) = listener.accept().await.unwrap();
    let a = request(&mut first).await;
    // Both physical requests are admitted while neither response has finished.
    let (mut second, _) = listener.accept().await.unwrap();
    let b = request(&mut second).await;
    assert_ne!(is_title(&a), is_title(&b));
    if is_title(&a) {
        ((b, second), (a, first))
    } else {
        ((a, first), (b, second))
    }
}
async fn attempts(f: &Fixture, session: &str) -> Vec<maka_agent_facts::accounting::ModelAttempt> {
    f.store
        .model_attempts(
            Query {
                from: 0.0,
                to: 253402300799999.0,
                session_id: Some(session.into()),
                through: None,
            },
            0,
            100,
        )
        .await
        .unwrap()
        .attempts
}
#[tokio::test]
async fn title_first_message_is_bounded_accounted_and_not_reissued_after_retry_or_restart() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut f = fixture(&format!("http://{}/v1", listener.local_addr().unwrap())).await;
        selection::thinking(&f, vec![ThinkingLevel::Off, ThinkingLevel::High], ThinkingLevel::High).await;
        let session = automatic(&f).await;
        let text = "中".repeat(5000);
        let input = json!({"sessionId":session,"message":text,"model":f.selection().await});
        let operation = Uuid::new_v4();
        let sent = f.mutation(2, "send", input.clone(), operation).await.unwrap();
        let ((main, mut main_socket), (title, mut title_socket)) = pair(&listener).await;
        assert_eq!(main["reasoning_effort"], "high");
        assert_eq!(title["reasoning_effort"], "none");
        assert_eq!(title["messages"][1]["content"], "中".repeat(4096));
        assert!(title["tools"].is_null() || title["tools"] == json!([]));
        assert!(title["max_tokens"] == 2048 || title["max_completion_tokens"] == 2048, "{title}");
        assert_eq!(f.mutation(2, "send", input, operation).await.unwrap(), sent);
        response(&mut main_socket).await;
        // An auxiliary title is owned by the Host, so main-turn completion does not cancel it.
        loop {
            let boundary = f.store.invocation_boundary(&session, sent["invocationId"].as_str().unwrap()).await.unwrap().unwrap().0;
            if matches!(boundary.state, maka_event_log::turns::InvocationState::Ended { .. }) { break; }
            tokio::task::yield_now().await;
        }
        response_with_text(&mut title_socket, "  cafe\u{301}\n\u{202e}任务😀  ").await;
        f.drained().await;
        let record = f.owner.session(&session).await.unwrap();
        assert_eq!(record.configuration.title, "café 任务😀");
        assert!(!record.configuration.title_is_manual);
        let original = attempts(&f, &session).await;
        assert_eq!(original.len(), 2);
        let auxiliary = original.iter().find(|attempt| matches!(attempt.origin, Origin::Auxiliary { .. })).unwrap();
        assert!(matches!(&auxiliary.origin, Origin::Auxiliary { source: AuxiliarySource::SessionTitle { invocation } } if invocation.invocation_id == sent["invocationId"].as_str().unwrap()));
        assert_eq!(auxiliary.outcome, Outcome::Success);
        assert_eq!(auxiliary.usage.input_tokens, Some(1));
        assert_eq!(auxiliary.usage.output_tokens, Some(2));
        assert!(auxiliary.quote.is_some());
        todo_tests::reopen(&mut f).await;
        assert_eq!(attempts(&f, &session).await.len(), 2);
        f.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        assert!(!is_title(&request(&mut socket).await));
        response(&mut socket).await;
        loop {
            let observation = f.query("observe", json!({"sessionId":session})).await.unwrap();
            if observation["run"].is_null() { break; }
            tokio::task::yield_now().await;
        }
        f.app.shutdown().await.unwrap();
        let current = attempts(&f, &session).await;
        assert_eq!(current.len(), 3);
        assert_eq!(current.iter().filter(|attempt| matches!(attempt.origin, Origin::Auxiliary { .. })).count(), 1);
        assert_eq!(f.store.get_session::<SessionConfiguration>(&session).await.unwrap().unwrap().configuration.title, "café 任务😀");
        f.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn title_manual_rename_even_to_placeholder_and_archive_win_in_flight() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let f = fixture(&format!("http://{}/v1", listener.local_addr().unwrap())).await;
        for archive in [false, true] {
            let session = automatic(&f).await;
            let sent = f.send(&session).await.unwrap();
            let ((_, mut main), (_, mut title)) = pair(&listener).await;
            response(&mut main).await;
            loop {
                let boundary = f.store.invocation_boundary(&session, sent["invocationId"].as_str().unwrap()).await.unwrap().unwrap().0;
                if matches!(boundary.state, maka_event_log::turns::InvocationState::Ended { .. }) { break; }
                tokio::task::yield_now().await;
            }
            if archive {
                f.store.set_session_archived::<SessionConfiguration>(&session, true, now()).await.unwrap();
            } else {
                let revision = f.owner.session(&session).await.unwrap().revision;
                let result = f.mutation(2, "metadata", json!({"sessionId":session,"expectedRevision":revision,"title":"Untitled","labels":[]}), Uuid::new_v4()).await.unwrap();
                assert_eq!(result["kind"], "committed", "{result}");
            }
            response_with_text(&mut title, "Must not replace the user's title").await;
            f.drained().await;
            let record = f.owner.session(&session).await.unwrap();
            assert_eq!(record.configuration.title, "Untitled");
            assert_eq!(record.archived, archive);
            assert_eq!(record.configuration.title_is_manual, !archive);
            assert_eq!(attempts(&f, &session).await.len(), 2);
        }
        f.app.shutdown().await.unwrap();
        f.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn title_skips_manual_and_managed_sessions_before_model_dispatch() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let f = fixture(&format!("http://{}/v1", listener.local_addr().unwrap())).await;
        let manual = f.create("My chosen title").await;
        let mut configuration = f.owner.session(&manual).await.unwrap().configuration;
        configuration.title_is_manual = false;
        let managed = format!("{}{}", f.owner.prefix, Uuid::new_v4().simple());
        f.store
            .create_plugin_session(
                &maka_event_log::sessions::PluginSession {
                    session_id: managed.clone(),
                    creator: maka_capabilities::storage::Namespace::new(
                        "title.fixture",
                        maka_plugin_core::scope::Scope::Profile,
                    )
                    .unwrap(),
                    fingerprint: "managed-title".into(),
                    managed: true,
                    authority_session_id: None,
                },
                &configuration,
                now(),
            )
            .await
            .unwrap();
        for session in [&manual, &managed] {
            f.send(session).await.unwrap();
            let (mut socket, _) = listener.accept().await.unwrap();
            assert!(!is_title(&request(&mut socket).await));
            response(&mut socket).await;
            f.drained().await;
            assert_eq!(attempts(&f, session).await.len(), 1);
            assert_eq!(
                f.owner.session(session).await.unwrap().configuration.title,
                "My chosen title"
            );
        }
        f.app.shutdown().await.unwrap();
        f.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn title_invalid_finish_keeps_observed_usage_and_never_retries() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let f = fixture(&format!("http://{}/v1", listener.local_addr().unwrap())).await;
        let session = automatic(&f).await;
        f.send(&session).await.unwrap();
        let ((_, mut main), (_, mut title)) = pair(&listener).await;
        response(&mut main).await;
        let end = json!({"id":"fixture","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":7,"completion_tokens":9,"total_tokens":16}});
        let body = format!("data: {end}\n\ndata: [DONE]\n\n");
        title.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        f.drained().await;
        let attempts = attempts(&f, &session).await;
        let auxiliary = attempts.iter().find(|attempt| matches!(attempt.origin, Origin::Auxiliary { .. })).unwrap();
        assert_eq!(auxiliary.outcome, Outcome::Error);
        assert_eq!(auxiliary.usage.input_tokens, Some(7));
        assert_eq!(auxiliary.usage.output_tokens, Some(9));
        assert_eq!(f.owner.session(&session).await.unwrap().configuration.title, "Untitled");
        f.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        assert!(!is_title(&request(&mut socket).await));
        response(&mut socket).await;
        f.drained().await;
        f.app.shutdown().await.unwrap();
        f.config.shutdown().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn title_host_shutdown_awaits_socket_cleanup_and_restart_does_not_replay() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut f = fixture(&format!("http://{}/v1", listener.local_addr().unwrap())).await;
        let session = automatic(&f).await;
        f.send(&session).await.unwrap();
        let ((_, mut main), (_, mut title)) = pair(&listener).await;
        response(&mut main).await;
        f.app.shutdown().await.unwrap();
        let mut buffer = [0; 1];
        assert_eq!(title.read(&mut buffer).await.unwrap(), 0);
        assert!(f.owner.auxiliary.resources.is_idle());
        let original = attempts(&f, &session).await;
        let auxiliary = original
            .iter()
            .find(|attempt| matches!(attempt.origin, Origin::Auxiliary { .. }))
            .unwrap();
        assert_eq!(auxiliary.outcome, Outcome::Aborted);
        todo_tests::reopen(&mut f).await;
        f.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        assert!(!is_title(&request(&mut socket).await));
        response(&mut socket).await;
        loop {
            if f.query("observe", json!({"sessionId":session}))
                .await
                .unwrap()["run"]
                .is_null()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        f.app.shutdown().await.unwrap();
        assert_eq!(
            attempts(&f, &session)
                .await
                .iter()
                .filter(|attempt| matches!(attempt.origin, Origin::Auxiliary { .. }))
                .count(),
            1
        );
        assert_eq!(
            f.store
                .get_session::<SessionConfiguration>(&session)
                .await
                .unwrap()
                .unwrap()
                .configuration
                .title,
            "Untitled"
        );
        f.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn title_settlement_fault_fences_owner_and_preserves_original_usage_for_recovery() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let f = fixture(&format!("http://{}/v1", listener.local_addr().unwrap())).await;
        let session = automatic(&f).await;
        f.send(&session).await.unwrap();
        let ((_, mut main), (_, mut title)) = pair(&listener).await;
        let raw = rusqlite::Connection::open(f.root.join(maka_event_log::root::ROOT_DATABASE)).unwrap();
        raw.execute_batch("CREATE TRIGGER hold_title_settlement BEFORE INSERT ON event_log WHEN NEW.kind='auxiliary_model_settled' BEGIN SELECT RAISE(ABORT, 'fixture title settlement failure'); END;").unwrap();
        response(&mut main).await;
        response_with_text(&mut title, "Uncommitted title").await;
        f.drained().await;
        assert!(f.owner.runs.lock().await.closing);
        assert!(f.owner.stop.is_cancelled());
        assert_eq!(f.send(&session).await.unwrap_err(), "agent_closed");
        assert_eq!(f.owner.session(&session).await.unwrap().configuration.title, "Untitled");
        let count: i64 = raw.query_row("SELECT COUNT(*) FROM event_log WHERE kind='auxiliary_model_settled'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 0);
        raw.execute_batch("DROP TRIGGER hold_title_settlement").unwrap();
        f.app.shutdown().await.unwrap();
        // The exclusive startup recovery owns this transition, never plugin activation.
        f.store.recover_auxiliary_models().await.unwrap();
        let original = attempts(&f, &session).await;
        let auxiliary = original.iter().find(|attempt| matches!(attempt.origin, Origin::Auxiliary { .. })).unwrap();
        assert_eq!(auxiliary.outcome, Outcome::Unknown);
        assert_eq!(auxiliary.usage.input_tokens, Some(1));
        assert_eq!(auxiliary.usage.output_tokens, Some(2));
        f.config.shutdown().await.unwrap();
    }).await.unwrap();
}
