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

use super::{HostFixture, Peer, Value, json, remote, success};
use maka_runtime::{
    event::{EventWrite, Fact, Invocation, InvocationInput, InvocationOutcome, RuntimeEvent},
    input::{DeliveredMessage, MessageInput},
    message::{MessageDisposition, Placement, RootSourceMessage},
};
use std::path::Path;

pub(super) async fn create(peer: &mut Peer, workspace: &Path) -> Value {
    success(peer.rpc("session.create", json!({"sessionId":"h","workspace":{"kind":"host_path","path":workspace},"executorId":"example.background","sandboxMode":"danger-full-access"})).await);
    success(peer.rpc("artifact.ingest", json!({"kind":"begin","sessionId":"h","uploadId":"a","name":"a","mimeType":"application/octet-stream","totalBytes":1,"contentSha256":maka_runtime::artifact::content_digest(b"\0")})).await);
    success(
        peer.rpc(
            "artifact.ingest",
            json!({"kind":"chunk","sessionId":"h","uploadId":"a","offset":0,"chunkBase64":"AA=="}),
        )
        .await,
    );
    success(
        peer.rpc(
            "artifact.ingest",
            json!({"kind":"commit","sessionId":"h","uploadId":"a"}),
        )
        .await,
    )["attachment"]
        .clone()
}

// Public canonical facts use the actual Artifact admitted above. The fixture
// validates each consumed message; it does not simulate provider execution.
pub(super) async fn seed(fixture: &HostFixture, attachment: &Value) {
    let log = fixture.log().await;
    let invocation = Invocation {
        session_id: "h".into(),
        turn_id: "wire-turn".into(),
        run_id: "wire-run".into(),
        invocation_id: "wire-invocation".into(),
    };
    let opening = Fact::InvocationOpened {
        configuration: None,
        input: InvocationInput::Message {
            content: "seed".into(),
            source_messages: Vec::new(),
            request_fingerprint: None,
        },
    };
    log.append(&EventWrite::plain(RuntimeEvent::new(invocation.clone(), opening)).unwrap())
        .await
        .unwrap();
    for index in 0..8 {
        let content: MessageInput = serde_json::from_value(
            json!({"text":format!("input-{index}"),"attachments":vec![attachment.clone();800]}),
        )
        .unwrap();
        assert!(content.text_bytes() < 64 * 1024);
        assert!(serde_json::to_vec(&content).unwrap().len() < 1024 * 1024);
        let message = DeliveredMessage {
            message_id: format!("wire-message-{index}"),
            submitted_content_digest: content.content_digest().unwrap(),
            content: content.clone(),
        };
        let source = RootSourceMessage {
            message: message.clone(),
            unprepared_content: content,
            submitted_placement: Placement::CurrentTurn,
            disposition: MessageDisposition::Steering,
            submitted_intent: None,
        };
        source.validate().unwrap();
        log.append(
            &EventWrite::plain(RuntimeEvent::new(
                invocation.clone(),
                Fact::MessageSteered {
                    message: Box::new(message),
                    source: Some(Box::new(source)),
                },
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    }
    log.append(
        &EventWrite::plain(RuntimeEvent::new(
            invocation,
            Fact::InvocationEnded {
                outcome: InvocationOutcome::Completed,
            },
        ))
        .unwrap(),
    )
    .await
    .unwrap();
    log.close().await.unwrap();
}

pub(super) async fn verify(peer: &mut Peer, client: &Value, document: &Value) {
    let grant = success(peer.rpc("plugin.authorization", json!({"client":client,"scope":"profile","command":{"kind":"approve","request":{"operationId":uuid::Uuid::new_v4(),"title":"Read full history windows","target":{"kind":"profile"},"capabilities":["read_history"]}}})).await)["grant"].clone();
    let result = remote(
        peer,
        client,
        document,
        "history-window",
        json!({"grant":grant["id"],"operation":"wire"}),
    )
    .await;
    let expected: Vec<Value> = (0..8).map(|index| json!({"id":format!("wire-message-{index}"),"attachments":800,"text":format!("input-{index}\na") + &"\na".repeat(799)})).collect();
    assert_eq!(result["forward"], json!(expected));
    assert_eq!(result["reverse"], result["forward"]);
    assert!(result["forwardPages"].as_u64().unwrap() > 1);
    assert!(result["reversePages"].as_u64().unwrap() > 1);
}
