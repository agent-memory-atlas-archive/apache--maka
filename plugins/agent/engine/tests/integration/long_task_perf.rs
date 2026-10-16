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

//! Ignored engine-only benchmark; no Host admission or agent.send transport.
//! Run a release test binary with --ignored --exact long_task_perf::engine_long_history --nocapture.
//! MAKA_ENGINE_PERF_TURNS=250,1000 MAKA_ENGINE_PERF_REPETITIONS=5
//! MAKA_ENGINE_PERF_CHECKPOINT=1 adds a real compact Run before each timed Run.
//! MAKA_ENGINE_PERF_TOOL_EVERY=20 MAKA_ENGINE_PERF_TOOLS_PER_TURN=1 models sparse tools.
//! Set those to 1 and 8 for dense stress; MAKA_ENGINE_PERF_RESULT_BYTES=128 controls payloads.
use crate::support::{context as fixture, image_projection::identity};
use maka_agent_facts::{
    context::ModelPurpose,
    event::{EventWrite, Fact, InvocationOutcome, LogScope, RuntimeEvent},
    input::InvocationInput,
    model::{ModelEvent, ModelFinishReason, ModelUsage, TextKind, assembly::StepBuilder},
    tool_call::ToolCallIdentity,
    tool_output::ToolOutput,
};
use maka_event_log::EventLog;
use serde_json::json;
use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

fn setting(name: &str, default: usize) -> usize {
    std::env::var(name).map_or(default, |value| {
        value.parse().expect("positive integer setting")
    })
}

// Tool turns follow stale_pruning; ordinary turns persist validated streamed text.
// Batch each closed turn, keeping fixture setup out of the measured interval.
async fn seed(
    log: &EventLog,
    turns: usize,
    result_bytes: usize,
    tool_every: usize,
    tools_per_turn: usize,
) -> (usize, usize) {
    let mut event_count = 0;
    let mut tool_results = 0;
    for turn in 0..turns {
        let invocation = identity(&format!("history-{turn}"));
        let step = format!("step-{turn}");
        let tool_count = if turn % tool_every == 0 {
            tools_per_turn
        } else {
            0
        };
        let calls: Vec<_> = (0..tool_count)
            .map(|call| {
                json!({
                    "kind":"tool_call", "call":{
                        "id":format!("call-{turn}-{call}"), "name":"Read",
                        "input":{"path":format!("file-{turn}-{call}")}, "provider_executed":false
                    }
                })
            })
            .collect();
        let mut writes = Vec::new();
        for fact in [
            Fact::InvocationOpened {
                configuration: None,
                input: InvocationInput::Message {
                    source_messages: Vec::new(),
                    content: format!("question-history-{turn}").into(),
                    request_fingerprint: None,
                },
            },
            Fact::ModelRequested {
                item_acceptance: tool_count == 0,
                purpose: ModelPurpose::Main,
                context: None,
                checkpoint_event_id: None,
                effective_source_digest: None,
                step_id: step.clone(),
                model_id: "test".into(),
                source_scope: LogScope::Session {
                    id: "session".into(),
                },
                source_high_water: (event_count + 1) as u64,
                source_digest: "legacy".into(),
                input_digest: "legacy".into(),
                route_identity: "legacy".into(),
            },
        ] {
            writes.push(EventWrite::plain(RuntimeEvent::new(invocation.clone(), fact)).unwrap());
        }
        let output = if tool_count > 0 {
            serde_json::from_value(
                json!({"parts": calls, "finish_reason":"tool-calls", "usage":{}}),
            )
            .unwrap()
        } else {
            let mut observations = vec![ModelEvent::PartStarted {
                id: "text".into(),
                text_kind: TextKind::Text,
                provider_options: None,
            }];
            for _ in 0..13 {
                observations.push(ModelEvent::PartDelta {
                    id: "text".into(),
                    text: "durable conversation text. ".into(),
                    provider_options: None,
                });
            }
            observations.push(ModelEvent::PartFinished {
                id: "text".into(),
                provider_options: None,
            });
            observations.push(ModelEvent::Finished {
                reason: ModelFinishReason::Stop,
                usage: ModelUsage::default(),
                provider_options: None,
            });
            let mut builder = StepBuilder::for_step(&step).unwrap();
            for event in observations {
                builder.push(event.clone()).unwrap();
                writes.push(
                    EventWrite::plain(RuntimeEvent::new(
                        invocation.clone(),
                        Fact::ModelObserved {
                            step_id: step.clone(),
                            event,
                        },
                    ))
                    .unwrap(),
                );
            }
            builder.finish().unwrap()
        };
        writes.push(
            EventWrite::plain(RuntimeEvent::new(
                invocation.clone(),
                Fact::ModelCompleted {
                    step_id: step.clone(),
                    output,
                },
            ))
            .unwrap(),
        );
        for call in 0..tool_count {
            let call_id = format!("call-{turn}-{call}");
            let operation = format!("{step}:{call_id}");
            writes.push(
                EventWrite::plain(RuntimeEvent::new(
                    invocation.clone(),
                    Fact::ToolDispatched {
                        presentation: None,
                        title: None,
                        operation_id: operation.clone(),
                        call: ToolCallIdentity::provider(step.clone(), call_id),
                        name: "Read".into(),
                        input: json!({"path":format!("file-{turn}-{call}")}),
                    },
                ))
                .unwrap(),
            );
            writes.push(
                EventWrite::tool_success(
                    format!("result-{turn}-{call}"),
                    SystemTime::now(),
                    invocation.clone(),
                    operation,
                    ToolOutput::Json(json!({"content":"x".repeat(result_bytes)})).into(),
                )
                .unwrap()
                .0,
            );
        }
        writes.push(
            EventWrite::plain(RuntimeEvent::new(
                invocation,
                Fact::InvocationEnded {
                    outcome: InvocationOutcome::Completed,
                },
            ))
            .unwrap(),
        );
        assert_eq!(
            writes.len(),
            if tool_count == 0 {
                20
            } else {
                4 + 2 * tool_count
            }
        );
        event_count += writes.len();
        tool_results += tool_count;
        log.append_batch(&writes).await.unwrap();
    }
    (event_count, tool_results)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "release performance harness: creates disposable long histories"]
async fn engine_long_history() {
    assert!(
        !cfg!(debug_assertions),
        "run this performance harness with --release"
    );
    let turns: Vec<usize> = std::env::var("MAKA_ENGINE_PERF_TURNS")
        .unwrap_or_else(|_| "250,1000".into())
        .split(',')
        .map(|value| value.trim().parse().expect("comma-separated turn counts"))
        .collect();
    let repetitions = setting("MAKA_ENGINE_PERF_REPETITIONS", 5);
    let result_bytes = setting("MAKA_ENGINE_PERF_RESULT_BYTES", 128);
    let tool_every = setting("MAKA_ENGINE_PERF_TOOL_EVERY", 20);
    let tools_per_turn = setting("MAKA_ENGINE_PERF_TOOLS_PER_TURN", 1);
    assert!(tool_every > 0 && tools_per_turn > 0);
    let checkpoint = setting("MAKA_ENGINE_PERF_CHECKPOINT", 0) == 1;
    assert!(repetitions > 0 && turns.iter().all(|turns| *turns > 0));
    for turns in turns {
        for repetition in 0..repetitions {
            // Fresh store/engine per sample; seeding and optional compaction are untimed.
            let directory = tempfile::tempdir().unwrap();
            let log = Arc::new(
                EventLog::open(&directory.path().join("events.sqlite"))
                    .await
                    .unwrap(),
            );
            let (history_events, history_tool_results) =
                seed(&log, turns, result_bytes, tool_every, tools_per_turn).await;
            let worker = fixture::engine(log.clone());
            if checkpoint {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let base = format!("http://{}/v1", listener.local_addr().unwrap());
                let server = tokio::spawn(async move {
                    let (mut socket, _) = listener.accept().await.unwrap();
                    let request = fixture::read_request(&mut socket).await;
                    assert!(request["messages"].as_array().unwrap().len() > 1);
                    fixture::respond(&mut socket, fixture::SUMMARY, "stop").await;
                });
                tokio::time::timeout(
                    Duration::from_secs(300),
                    worker.run(
                        fixture::input(&base, "compact", true),
                        CancellationToken::new(),
                    ),
                )
                .await
                .expect("compact timed out")
                .expect("compact failed");
                server.await.unwrap();
            }
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}/v1", listener.local_addr().unwrap());
            let input = fixture::input(&base, "measured", false);
            let started = Instant::now();
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let request = fixture::read_request(&mut socket).await;
                let receipt_ms = started.elapsed().as_secs_f64() * 1000.0;
                let messages = request["messages"].to_string();
                assert!(messages.contains("question-measured"));
                if checkpoint {
                    assert!(messages.contains("summary-marker"));
                } else {
                    assert!(messages.contains("question-history-0"));
                    assert!(messages.contains(&format!("question-history-{}", turns - 1)));
                }
                let message_count = request["messages"].as_array().unwrap().len();
                let tool_message_count = request["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|message| message["role"] == "tool")
                    .count();
                let bytes = serde_json::to_vec(&request).unwrap().len();
                fixture::respond(&mut socket, "benchmark-done", "stop").await;
                (
                    receipt_ms,
                    bytes,
                    messages.len(),
                    message_count,
                    tool_message_count,
                )
            });
            let invocation = tokio::time::timeout(
                Duration::from_secs(300),
                worker.run(input, CancellationToken::new()),
            )
            .await
            .expect("engine Run timed out")
            .expect("engine Run failed");
            let completion_ms = started.elapsed().as_secs_f64() * 1000.0;
            let (receipt_ms, request_bytes, message_bytes, message_count, tool_message_count) =
                server.await.unwrap();
            worker.drain().await;
            // Verification deliberately outside measured interval. run() success alone is insufficient.
            let prefix = log
                .prefix(history_events * 2 + 100, 256 * 1024 * 1024)
                .await
                .unwrap();
            let measured: Vec<_> = prefix
                .events
                .iter()
                .filter(|event| event.event.invocation.invocation_id == invocation.invocation_id)
                .collect();
            assert!(measured.iter().any(|event| matches!(
                event.event.fact,
                Fact::InvocationEnded {
                    outcome: InvocationOutcome::Completed
                }
            )));
            assert!(measured.iter().any(|event| {
                match &event.event.fact {
                    Fact::ModelCompleted { output, .. } => serde_json::to_string(output)
                        .unwrap()
                        .contains("benchmark-done"),
                    _ => false,
                }
            }));
            if checkpoint {
                assert!(prefix.events.iter().any(|event| matches!(
                    event.event.fact,
                    Fact::ContextCheckpointRecorded { .. }
                )));
            }
            println!(
                "{}",
                json!({
                    "benchmark":"engine_long_history", "checkpoint":checkpoint,
                    "history_turns":turns, "history_events":history_events,
                    "history_tool_results":history_tool_results, "tool_every":tool_every, "tools_per_turn":tools_per_turn, "tool_result_content_bytes":result_bytes,
                    "repetition":repetition, "request_receipt_ms":receipt_ms,
                    "engine_completion_ms":completion_ms, "request_bytes":request_bytes,
                    "request_messages":message_count, "request_tool_messages":tool_message_count,
                    // A wire-size proxy, not model tokenizer output or fake server's fixed usage.
                    "message_bytes_div_3_proxy":message_bytes.div_ceil(3),
                    "outcome":"completed", "errors":0,
                })
            );
        }
    }
}
