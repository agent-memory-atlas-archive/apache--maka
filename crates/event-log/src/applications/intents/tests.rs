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
use crate::applications::tests::store;
use maka_plugin_core::{
    composition::{Configuration, Environment, ExecutionLocation, Input, Node, PackageUse},
    contract::{Contract, Interface, Method, MethodMode, Transport},
    operation::RecoveryProtocol,
    package::{Declaration, EntryDeclaration, EntryTarget, Implementation},
};
use maka_plugins::package::{MANIFEST_FILE, Package};
use serde_json::json;
use std::collections::BTreeMap;

fn reference() -> RecoveryRef {
    RecoveryRef {
        source: DeploymentRef {
            state_domain: "a".repeat(64),
            deployment: Uuid::from_u128(900),
            revision: 1,
        },
        namespace: InstanceId(vec![Uuid::from_u128(11)]),
        export: "Service".into(),
        method: "change".into(),
        operation: Uuid::new_v4(),
        fingerprint: format!("sha256-{}", "b".repeat(64)),
        contract: Contract {
            name: "test.Change".into(),
            version: 1,
            transport: Transport::Remote,
            interface: Interface::Service {
                methods: BTreeMap::from([(
                    "change".into(),
                    Method {
                        mode: MethodMode::Mutation,
                        input: json!({}),
                        output: json!({}),
                        resources: BTreeMap::new(),
                    },
                )]),
            },
        },
        recovery: RecoveryProtocol::JournalV1,
    }
}
async fn source(store: &EventLog, deployment: Uuid, expected: u64) -> DeploymentRef {
    let package = Package::new(BTreeMap::from([(MANIFEST_FILE.into(),serde_json::to_vec(&json!({"schemaVersion":2,"id":"caller","application":Declaration {entries:BTreeMap::from([(EntryTarget::Host,EntryDeclaration {scene:None,implementation:Implementation::Native {implementation:"test.caller".into()},configuration:json!({"type":"object"}),imports:BTreeMap::new(),exports:BTreeMap::new()})]),composition:None}})).unwrap())])).unwrap();
    store
        .commit_application(
            deployment,
            expected,
            Input {
                environment: Environment {
                    location: ExecutionLocation::Host,
                    presentation: None,
                },
                instances: BTreeMap::from([(
                    "caller".into(),
                    Node {
                        key: Uuid::from_u128(42),
                        package: package.application_ref().into(),
                        use_package: PackageUse::Entry,
                        configuration: Configuration::default(),
                        parameters: Configuration::default(),
                        bindings: BTreeMap::new(),
                        disabled: false,
                        overrides: vec![],
                    },
                )]),
                recovery: None,
                launch: None,
            },
            vec![],
            vec![package],
        )
        .await
        .unwrap()
}
fn consumer() -> InstanceId {
    InstanceId(vec![Uuid::from_u128(42)])
}
#[tokio::test]
async fn caller_history_keeps_first_source_and_original_owner_across_revisions_and_conflicts() {
    let (_dir, store) = store().await;
    let deployment = Uuid::new_v4();
    let first = source(&store, deployment, 0).await;
    let original = reference();
    let (a, b) = tokio::join!(
        store.record_application_intent(first.clone(), consumer(), original.clone()),
        store.record_application_intent(first.clone(), consumer(), original.clone())
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.intent(), b.intent());
    assert_eq!(a.intent().reference, original);
    assert!(
        store.operation_observation(&original).await.is_err(),
        "caller Root is not the original business journal"
    );
    let second = source(&store, deployment, 1).await;
    let again = store
        .record_application_intent(second, consumer(), original.clone())
        .await
        .unwrap();
    assert_eq!(again.intent().caller, first);
    let mut rebound = original.clone();
    rebound.source.state_domain = "c".repeat(64);
    assert!(matches!(
        store
            .record_application_intent(first.clone(), consumer(), rebound)
            .await,
        Err(StoreError::EventConflict)
    ));
    let other = reference();
    let other = store
        .record_application_intent(first.clone(), consumer(), other)
        .await
        .unwrap();
    let page = store
        .application_intents(deployment, consumer(), 0, 1)
        .await
        .unwrap();
    assert_eq!(page, [a.intent().clone()]);
    let next = store
        .application_intents(deployment, consumer(), page[0].cursor, 128)
        .await
        .unwrap();
    assert_eq!(next, [other.intent().clone()]);
    assert!(
        store
            .application_intents(deployment, consumer(), other.intent().cursor, 128)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .application_intents(deployment, consumer(), 0, 129)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn caller_requires_owned_exact_source_and_actual_executable_namespace() {
    let (_dir, store) = store().await;
    let first = source(&store, Uuid::new_v4(), 0).await;
    let mut foreign = first.clone();
    foreign.state_domain = "e".repeat(64);
    assert!(
        store
            .record_application_intent(foreign, consumer(), reference())
            .await
            .is_err()
    );
    let mut missing = first.clone();
    missing.revision = 2;
    assert!(
        store
            .record_application_intent(missing, consumer(), reference())
            .await
            .is_err()
    );
    assert!(
        store
            .record_application_intent(
                first.clone(),
                InstanceId(vec![Uuid::from_u128(43)]),
                reference()
            )
            .await
            .is_err()
    );
    assert!(
        store
            .record_application_intent(first, InstanceId(vec![]), reference())
            .await
            .is_err()
    );
    assert!(
        store
            .application_intents(Uuid::new_v4(), consumer(), 0, 128)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn corrupted_history_is_bounded_and_never_overwritten_as_a_fresh_intent() {
    let (_dir, store) = store().await;
    let deployment = Uuid::new_v4();
    let caller = source(&store, deployment, 0).await;
    let original = reference();
    let token = store
        .record_application_intent(caller.clone(), consumer(), original.clone())
        .await
        .unwrap();
    let cursor = token.intent().cursor;
    drop(token);
    let operation = original.operation.to_string();
    store
        .connection
        .run(move |connection| {
            Box::pin(async move {
                sqlx::query("UPDATE application_intents SET operation=? WHERE cursor=?")
                    .bind(format!("{operation}\0extra"))
                    .bind(cursor as i64)
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    assert!(
        store
            .application_intents(deployment, consumer(), 0, 128)
            .await
            .is_err()
    );
    let operation = original.operation.to_string();
    store
        .connection
        .run(move |connection| {
            Box::pin(async move {
                sqlx::query(
                    "UPDATE application_intents SET operation=?, reference_json=? WHERE cursor=?",
                )
                .bind(operation)
                .bind(serde_json::to_string(&"x".repeat(MAX_REFERENCE)).unwrap())
                .bind(cursor as i64)
                .execute(&mut *connection)
                .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    assert!(
        store
            .application_intents(deployment, consumer(), 0, 128)
            .await
            .is_err()
    );
    assert!(
        store
            .record_application_intent(caller, consumer(), original)
            .await
            .is_err()
    );
    let rows: i64 = store
        .connection
        .run(|connection| {
            Box::pin(async move {
                Ok(
                    sqlx::query_scalar("SELECT COUNT(*) FROM application_intents")
                        .fetch_one(connection)
                        .await?,
                )
            })
        })
        .await
        .unwrap();
    assert_eq!(rows, 1);
}

#[tokio::test]
async fn caller_commit_error_produces_no_dispatch_token() {
    let (_dir, store) = store().await;
    let deployment = Uuid::new_v4();
    let caller = source(&store, deployment, 0).await;
    store.connection.run(|connection| Box::pin(async move {
        sqlx::raw_sql("CREATE TABLE caller_commit_probe (deployment TEXT, revision INTEGER, FOREIGN KEY(deployment,revision) REFERENCES application_sources(deployment,revision) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER reject_caller_commit AFTER INSERT ON application_intents BEGIN INSERT INTO caller_commit_probe VALUES ('missing',1); END;")
            .execute(connection).await?;
        Ok(())
    })).await.unwrap();
    assert!(matches!(
        store
            .record_application_intent(caller, consumer(), reference())
            .await,
        Err(StoreError::CommitUnknown(_))
    ));
    assert!(
        store
            .application_intents(deployment, consumer(), 0, 128)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn history_pages_bound_the_complete_encoded_array_without_skipping_the_next_cursor() {
    let (_dir, store) = store().await;
    let deployment = Uuid::new_v4();
    let caller = source(&store, deployment, 0).await;
    let mut expected = Vec::new();
    for _ in 0..5 {
        let mut original = reference();
        let Interface::Service { methods } = &mut original.contract.interface else {
            unreachable!()
        };
        methods.get_mut("change").unwrap().input = json!({"description": "\"\\\n".repeat(70_000)});
        let token = store
            .record_application_intent(caller.clone(), consumer(), original)
            .await
            .unwrap();
        expected.push(token.intent().clone());
    }
    assert!(serde_json::to_vec(&expected[..3]).unwrap().len() > MAX_PAGE);
    let mut all = Vec::new();
    let mut after = 0;
    let mut lengths = Vec::new();
    loop {
        let page = store
            .application_intents(deployment, consumer(), after, 128)
            .await
            .unwrap();
        assert!(serde_json::to_vec(&page).unwrap().len() <= MAX_PAGE);
        if page.is_empty() {
            break;
        }
        assert!(page[0].cursor > after);
        after = page.last().unwrap().cursor;
        lengths.push(page.len());
        all.extend(page);
    }
    assert_eq!(lengths, [2, 2, 1]);
    assert_eq!(all, expected);
}
