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
use base64::{Engine as _, engine::general_purpose::STANDARD};
use maka_runtime::{
    artifact::content_digest,
    event::{EventWrite, Fact, InvocationInput, InvocationOutcome, RuntimeEvent},
    model::{ModelEvent, ModelPart, ModelStep, ModelToolCall, ModelUsage},
    tool_call::ToolCallIdentity,
    tool_output::ToolSuccess,
};

#[tokio::test]
async fn canonical_images_use_original_agent_grants_and_bounded_base64() {
    let fixture = Fixture::new("http://127.0.0.1:9/v1", 1, None).await;
    let session = fixture.create("Images").await;
    let invocation = Invocation {
        session_id: session.clone(),
        turn_id: "image-turn".into(),
        run_id: "image-run".into(),
        invocation_id: "image-invocation".into(),
    };
    let write = |fact| EventWrite::plain(RuntimeEvent::new(invocation.clone(), fact)).unwrap();
    fixture
        .store
        .append(&write(Fact::InvocationOpened {
            configuration: None,
            input: InvocationInput::Message {
                content: "Read image".into(),
                source_messages: Vec::new(),
                request_fingerprint: None,
            },
        }))
        .await
        .unwrap();
    let call = ModelToolCall {
        id: "image".into(),
        name: "Read".into(),
        input: json!({}),
        provider_options: None,
        provider_executed: false,
    };
    fixture
        .store
        .append(&write(Fact::ModelRequested {
            item_acceptance: false,
            purpose: maka_runtime::context::ModelPurpose::Main,
            context: None,
            checkpoint_event_id: None,
            step_id: "image-step".into(),
            model_id: "fixture-model".into(),
            source_scope: maka_runtime::event::LogScope::Root,
            source_high_water: 0,
            source_digest: "fixture".into(),
            effective_source_digest: None,
            input_digest: "fixture".into(),
            route_identity: "fixture".into(),
        }))
        .await
        .unwrap();
    fixture
        .store
        .append(&write(Fact::ModelObserved {
            step_id: "image-step".into(),
            event: ModelEvent::ToolCall(call.clone()),
        }))
        .await
        .unwrap();
    fixture
        .store
        .append(&write(Fact::ModelCompleted {
            step_id: "image-step".into(),
            output: ModelStep {
                parts: vec![ModelPart::ToolCall { call }],
                finish_reason: maka_runtime::model::ModelFinishReason::ToolCalls,
                usage: ModelUsage::default(),
                provider_options: None,
                response_id: None,
                model: None,
                timestamp: None,
            },
        }))
        .await
        .unwrap();
    let old = fixture
        .store
        .append(&write(Fact::ToolDispatched {
            title: None,
            operation_id: "image-step:image".into(),
            call: ToolCallIdentity::provider("image-step".into(), "image".into()),
            name: "Read".into(),
            input: json!({}),
        }))
        .await
        .unwrap();
    let mut bytes = vec![
        71, 73, 70, 56, 57, 97, 1, 0, 1, 0, 128, 0, 0, 0, 0, 0, 255, 255, 255, 33, 249, 4, 1, 0, 0,
        0, 0, 44, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 1, 68, 0,
    ];
    bytes.extend([0x21, 0xfe]);
    for _ in 0..160 {
        bytes.push(255);
        bytes.extend([b'x'; 255]);
    }
    bytes.extend([0, 0x3b]);
    let (result, _) = EventWrite::tool_success(
        "native-image-result".into(),
        std::time::SystemTime::now(),
        invocation.clone(),
        "image-step:image".into(),
        ToolSuccess::image(bytes.clone(), "image/gif".into()).unwrap(),
    )
    .unwrap();
    fixture.store.append(&result).await.unwrap();
    fixture
        .store
        .append(&write(Fact::InvocationEnded {
            outcome: InvocationOutcome::Completed,
        }))
        .await
        .unwrap();
    let fence = *fixture.store.subscribe_commits().borrow();
    let mut position = json!({"kind":"start"});
    let sequence = loop {
        let page = fixture
            .query(
                "history",
                json!({"sessionId":session,"through":fence,"position":position}),
            )
            .await
            .unwrap();
        if page["kind"] == "preparing" {
            continue;
        }
        if let Some(row) = page["chunks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["role"] == "tool_result")
        {
            break row["sequence"].as_u64().unwrap();
        }
        assert!(
            !page["after"].is_null(),
            "image row must be discoverable by ordinary bounded history"
        );
        position = json!({"kind":"after","cursor":page["after"]});
    };
    let request = json!({"sessionId":session,"through":fence,"sequence":sequence,"offset":0});
    let mut assembled = Vec::new();
    loop {
        let mut input = request.clone();
        input["offset"] = json!(assembled.len());
        let page = fixture.query("toolImage", input).await.unwrap();
        assert_eq!(page["kind"], "ready");
        assert_eq!(page["through"], fence);
        assert_eq!(page["sequence"], sequence);
        assert_eq!(page["offset"], assembled.len());
        assert_eq!(page["totalBytes"], bytes.len());
        assert_eq!(page["mimeType"], "image/gif");
        assert_eq!(page["digest"], content_digest(&bytes));
        assert!(serde_json::to_vec(&page).unwrap().len() <= 64 * 1024);
        let data = page["data"].as_str().unwrap();
        assert!(data.len() <= 43692);
        let chunk = STANDARD.decode(data).unwrap();
        assert!(chunk.len() <= 32 * 1024);
        if assembled.is_empty() {
            assert_eq!(
                chunk.len(),
                32 * 1024,
                "whole-envelope budget must permit useful remote chunks"
            );
        }
        if chunk.is_empty() {
            break;
        }
        assembled.extend(chunk);
    }
    assert_eq!(assembled, bytes);
    for (key, value) in [
        ("sessionId", json!("foreign_session")),
        ("through", json!(fence + 1)),
        ("through", json!(old)),
        ("offset", json!(bytes.len() + 1)),
        ("artifactId", json!("guessed")),
    ] {
        let mut input = request.clone();
        input[key] = value;
        assert!(fixture.query("toolImage", input).await.is_err());
    }
    assert!(
        fixture
            .app
            .client()
            .start(
                fixture.binding(2).await,
                "toolImage".into(),
                request.clone(),
                scope(&fixture.issuer, false),
                Attachments::new().into()
            )
            .await
            .is_err()
    );
    let cancelled = scope(&fixture.issuer, true);
    cancelled.cancellation.cancel();
    if let Ok(read) = fixture
        .app
        .client()
        .start(
            fixture.binding(2).await,
            "toolImage".into(),
            request,
            cancelled,
            Attachments::new().into(),
        )
        .await
    {
        assert!(read.result().await.is_err());
    }
    fixture.app.shutdown().await.unwrap();
    fixture.config.shutdown().await.unwrap();
}
