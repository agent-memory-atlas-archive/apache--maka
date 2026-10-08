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
use maka_runtime::interaction::{InteractionOutcome, InteractionRecord};

pub(super) async fn question_response(socket: &mut TcpStream) {
    let input = json!({"questions":[
        {"question":"Choose\nformat", "options":[{"label":"Brief"},{"label":"Detailed"}]},
        {"question":"Any detail?", "options":[{"label":"Yes"},{"label":"No"}]},
        {"question":"Optional?", "options":[{"label":"Yes"},{"label":"No"}]}
    ]});
    let delta = json!({"id":"question-model","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"question-call","type":"function","function":{"name":"AskUserQuestion","arguments":input.to_string()}}]},"finish_reason":null}]});
    let body = format!(
        "data: {delta}\n\ndata: {{\"id\":\"question-model\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"tool_calls\"}}]}}\n\ndata: [DONE]\n\n"
    );
    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
}
async fn pending(fixture: &Fixture, session: &str) -> InteractionRecord {
    let mut commits = fixture.store.subscribe_commits();
    loop {
        if let Some(record) = fixture
            .store
            .pending_interactions(session)
            .await
            .unwrap()
            .into_iter()
            .next()
        {
            return record;
        }
        commits.changed().await.unwrap();
    }
}
#[tokio::test]
async fn native_question_roundtrip_is_bound_owned_and_first_wins() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fixture = Fixture::with_services(&format!("http://{}/v1", listener.local_addr().unwrap()), 1, None, None, Some(service_tools::Setup::questions())).await;
        let session = fixture.create("Question roundtrip").await;
        fixture.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        let first = request(&mut socket).await;
        assert_eq!(first["tools"][0]["function"]["name"], "AskUserQuestion");
        question_response(&mut socket).await;
        let record = pending(&fixture, &session).await;
        let original = fixture.query("interaction", json!({"sessionId":session,"requestId":record.request_id})).await.unwrap();
        assert_eq!(original["request"]["questions"][0]["question"], "Choose\\u{A}format");
        let before = fixture.query("observe", json!({"sessionId":session})).await.unwrap();
        assert_eq!(before["run"]["state"], "waiting_for_user");
        let through = before["through"].as_u64().unwrap();
        let mut watch = fixture.app.client().subscribe(fixture.binding(2).await, "watch".into(), json!({"sessionId":session,"after":through}), scope(&fixture.issuer, true), Attachments::new().into()).await.unwrap();
        loop {
            let item = watch.recv().await.unwrap();
            if item["kind"] == "interactions" { assert_eq!(item["pending"]["entries"][0]["requestId"], record.request_id); break; }
        }
        // A valid Session in this owner still cannot address another Session's request.
        let other = fixture.create("Other question session").await;
        assert!(fixture.query("interaction", json!({"sessionId":other,"requestId":record.request_id})).await.is_err());
        assert!(fixture.query("pendingInteractions", json!({"sessionId":"foreign"})).await.is_err());
        let wrong = fixture.mutation(2, "respond", json!({"sessionId":other,"requestId":record.request_id,"answer":{"kind":"question","answers":["Brief","text",null]}}), Uuid::new_v4()).await.unwrap();
        assert_eq!(wrong, json!({"kind":"rejected","reason":"unavailable"}));
        let invalid = fixture.mutation(2, "respond", json!({"sessionId":session,"requestId":record.request_id,"answer":{"kind":"question","answers":["Brief"]}}), Uuid::new_v4()).await.unwrap();
        assert_eq!(invalid, json!({"kind":"rejected","reason":"invalid_answer"}));
        let operation = Uuid::new_v4();
        let input = json!({"sessionId":session,"requestId":record.request_id,"answer":{"kind":"question","answers":["Brief","my text",null]}});
        let answered = fixture.mutation(2, "respond", input.clone(), operation).await.unwrap();
        assert_eq!(answered["kind"], "recorded");
        assert_eq!(fixture.mutation(2, "respond", input, operation).await.unwrap(), answered);
        let conflict = fixture.mutation(2, "respond", json!({"sessionId":session,"requestId":record.request_id,"answer":{"kind":"question","answers":["Detailed","changed",null]}}), Uuid::new_v4()).await.unwrap();
        assert_eq!(conflict, answered);
        loop {
            let item = watch.recv().await.unwrap();
            if item["kind"] == "interactions" && item["pending"]["entries"] == json!([]) { break; }
        }
        drop(watch);
        let (mut next, _) = listener.accept().await.unwrap();
        let resumed = request(&mut next).await;
        let message = resumed["messages"].as_array().unwrap().iter().find(|message| message["role"] == "tool").unwrap();
        assert_eq!(serde_json::from_str::<Value>(message["content"].as_str().unwrap()).unwrap(), json!({"kind":"answered","answers":[
            {"question":"Choose\nformat","answer":"Brief"},{"question":"Any detail?","answer":"my text"},{"question":"Optional?","answer":null}
        ]}));
        assert!(!message.to_string().contains("journal_v1"));
        response(&mut next).await;
        fixture.drained().await;
        assert!(matches!(fixture.store.interaction(&record.request_id).await.unwrap().unwrap().outcome, Some(InteractionOutcome::QuestionAnswer { .. })));
        fixture.app.shutdown().await.unwrap(); fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}
#[tokio::test]
async fn stopping_question_closes_original_request_and_releases_run_capacity() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fixture = Fixture::with_services(
            &format!("http://{}/v1", listener.local_addr().unwrap()),
            1,
            None,
            None,
            Some(service_tools::Setup::questions()),
        )
        .await;
        let session = fixture.create("Stop question").await;
        let started = fixture.send(&session).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        question_response(&mut socket).await;
        let record = pending(&fixture, &session).await;
        fixture
            .mutation(
                2,
                "stop",
                json!({"sessionId":session,"invocationId":started["invocationId"]}),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        fixture.drained().await;
        assert!(matches!(
            fixture
                .store
                .interaction(&record.request_id)
                .await
                .unwrap()
                .unwrap()
                .outcome,
            Some(InteractionOutcome::Closure { .. })
        ));
        assert_eq!(fixture.owner.permits.available_permits(), 1);
        assert!(fixture.owner.runs.lock().await.controls.is_empty());
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    })
    .await
    .unwrap();
}
