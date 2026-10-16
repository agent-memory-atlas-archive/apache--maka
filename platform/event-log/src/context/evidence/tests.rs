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
use crate::EventLog;
use maka_agent_facts::event::{
    EventWrite, Fact, Invocation, InvocationInput, InvocationOutcome, RuntimeEvent,
};
use sqlx::Connection;

#[tokio::test]
async fn streamed_boundaries_match_complete_prefixes_and_reject_unselected_cuts() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("events.sqlite");
    let log = EventLog::open(&path).await.unwrap();
    let scope = Selection::session("session").scope;
    let mut expected = vec![
        log.scoped_prefix(scope.clone(), 100, 1024 * 1024)
            .await
            .unwrap(),
    ];
    let mut foreign = 0;
    let mut large_event = String::new();
    for (session, id, text) in [
        ("session", "large", "漢\0\r\n".repeat(20_000)),
        ("foreign", "excluded", "excluded".into()),
        ("session", "small", "last".into()),
    ] {
        let invocation = Invocation {
            session_id: session.into(),
            turn_id: id.into(),
            run_id: id.into(),
            invocation_id: id.into(),
        };
        let opening = RuntimeEvent::new(
            invocation.clone(),
            Fact::InvocationOpened {
                configuration: None,
                input: InvocationInput::Message {
                    source_messages: Vec::new(),
                    content: text.into(),
                    request_fingerprint: None,
                },
            },
        );
        if id == "large" {
            large_event = opening.id.clone();
        }
        let sequence = log
            .append(&EventWrite::plain(opening).unwrap())
            .await
            .unwrap();
        if session == "session" {
            expected.push(
                log.scoped_prefix(scope.clone(), 100, 1024 * 1024)
                    .await
                    .unwrap(),
            );
        } else {
            foreign = sequence;
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
    }
    expected.push(log.scoped_prefix(scope, 100, 1024 * 1024).await.unwrap());
    log.connection
        .run(move |connection| {
            Box::pin(async move {
                let mut tx = connection.begin().await?;
                let selection = Selection::session("session");
                let throughs: Vec<_> = expected.iter().map(|prefix| prefix.high_water).collect();
                let actual = digests(&mut tx, &selection, &throughs).await?;
                assert_eq!(
                    actual,
                    expected
                        .iter()
                        .map(|prefix| prefix.digest.clone())
                        .collect::<Vec<_>>()
                );
                let last = *throughs.last().unwrap();
                assert_eq!(
                    digests(&mut tx, &selection, &[last]).await?,
                    [actual.last().unwrap().clone()]
                );
                assert!(
                    digests(&mut tx, &selection, &[foreign, last])
                        .await
                        .is_err(),
                    "a valid last cut must not hide an unselected intermediate cut"
                );
                assert!(digests(&mut tx, &selection, &[last + 1]).await.is_err());
                sqlx::query("UPDATE event_log SET event_json=' ' || event_json WHERE event_id=?")
                    .bind(large_event)
                    .execute(&mut *tx)
                    .await?;
                let changed = digests(&mut tx, &selection, &throughs).await?;
                assert_eq!(
                    changed[0], actual[0],
                    "the empty prefix contains no changed bytes"
                );
                for (before, after) in actual[1..].iter().zip(&changed[1..]) {
                    assert_ne!(
                        before, after,
                        "every prefix containing the oversized row authenticates its raw bytes"
                    );
                }
                tx.rollback().await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    log.close().await.unwrap();
}

#[tokio::test]
async fn duplicate_copied_membership_at_a_cut_cannot_reuse_its_original_digest() {
    let directory = tempfile::tempdir().unwrap();
    let log = EventLog::open(&directory.path().join("events.sqlite"))
        .await
        .unwrap();
    log.create_session("source", "create", &serde_json::json!({}), 1)
        .await
        .unwrap();
    let revision = log
        .get_session::<serde_json::Value>("source")
        .await
        .unwrap()
        .unwrap()
        .revision;
    log.copy_session(
        crate::sessions::SessionCopy {
            source_session_id: "source".into(),
            target_session_id: "copy".into(),
            expected_source_revision: revision,
            purpose: maka_agent_facts::session::CopyPurpose::Branch {
                turn_id: None,
                side_conversation: false,
            },
        },
        &serde_json::json!({}),
        2,
    )
    .await
    .unwrap();
    let invocation = Invocation {
        session_id: "copy".into(),
        turn_id: "native".into(),
        run_id: "native".into(),
        invocation_id: "native".into(),
    };
    for fact in [
        Fact::InvocationOpened {
            configuration: None,
            input: InvocationInput::Message {
                source_messages: Vec::new(),
                content: "native copied-session message".into(),
                request_fingerprint: None,
            },
        },
        Fact::InvocationEnded {
            outcome: InvocationOutcome::Completed,
        },
    ] {
        log.append(&EventWrite::plain(RuntimeEvent::new(invocation.clone(), fact)).unwrap())
            .await
            .unwrap();
    }
    let prefix = log
        .scoped_prefix(Selection::session("copy").scope, 10, 8192)
        .await
        .unwrap();
    log.connection
        .run(move |connection| {
            Box::pin(async move {
                let mut tx = connection.begin().await?;
                let selection = Selection::session("copy");
                assert_eq!(
                    selected(&mut tx, &selection, prefix.high_water)
                        .await?
                        .digest,
                    prefix.digest
                );
                sqlx::query(
                    "INSERT INTO session_history_members(session_id,sequence) VALUES('copy',?)",
                )
                .bind(prefix.high_water as i64)
                .execute(&mut *tx)
                .await?;
                let error = selected(&mut tx, &selection, prefix.high_water)
                    .await
                    .unwrap_err();
                assert!(
                    error.to_string().contains("source selection repeats"),
                    "{error}"
                );
                tx.rollback().await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    log.close().await.unwrap();
}
