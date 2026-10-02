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
use crate::applications::tests::{input, store};
use maka_plugin_core::contract::{Interface, Method, MethodMode, Transport};
use std::collections::BTreeMap;

async fn request(store: &EventLog) -> OperationRequest {
    OperationRequest {
        source: store
            .commit_application(Uuid::new_v4(), 0, input(), vec![], vec![])
            .await
            .unwrap(),
        namespace: InstanceId(vec![Uuid::new_v4()]),
        consumer: InstanceId(vec![Uuid::new_v4()]),
        export: "service".into(),
        method: "change".into(),
        operation: Uuid::new_v4(),
        contract: Contract {
            name: "changes".into(),
            version: 1,
            transport: Transport::Local,
            interface: Interface::Service {
                methods: BTreeMap::from([(
                    "change".into(),
                    Method {
                        mode: MethodMode::Mutation,
                        input: serde_json::json!({}),
                        output: serde_json::json!({}),
                        resources: BTreeMap::new(),
                    },
                )]),
            },
        },
        input: serde_json::json!({"outer":{"z":1,"a":2},"text":"original"}),
        origin: Identity::Remote {
            request_id: Uuid::new_v4(),
        },
    }
}
#[tokio::test]
async fn concurrent_dispatch_has_one_claim_and_duplicate_preserves_original_cleanup() {
    let (_dir, store) = store().await;
    let request = request(&store).await;
    let reference = request.reference().unwrap();
    let (a, b) = tokio::join!(
        store.dispatch_operation(request.clone()),
        store.dispatch_operation(request.clone())
    );
    let claim = match (a.unwrap(), b.unwrap()) {
        (Dispatch::New(claim), Dispatch::Existing(observation))
        | (Dispatch::Existing(observation), Dispatch::New(claim)) => {
            assert_eq!(observation.outcome, OperationOutcome::Unresolved);
            claim
        }
        _ => panic!("exactly one dispatch may execute"),
    };
    let outcome = OperationOutcome::Returned {
        value: serde_json::json!({"changed":true}),
    };
    let cleanup = OperationCleanup::Failed {
        diagnostic: "actual resource remains open".into(),
    };
    let settled = store
        .finish_operation(&claim, outcome.clone(), cleanup.clone())
        .await
        .unwrap();
    assert_eq!(
        store
            .finish_operation(&claim, outcome, cleanup)
            .await
            .unwrap(),
        settled
    );
    assert!(matches!(
        store
            .finish_operation(
                &claim,
                OperationOutcome::Unknown {
                    diagnostic: "rewrite".into()
                },
                OperationCleanup::Confirmed
            )
            .await,
        Err(StoreError::EventConflict)
    ));
    let Dispatch::Existing(duplicate) = store.dispatch_operation(request).await.unwrap() else {
        panic!("no second claim")
    };
    assert_eq!(duplicate, settled);
    assert_eq!(
        store
            .operation_observation(&reference)
            .await
            .unwrap()
            .unwrap(),
        settled
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn fingerprint_ignores_object_order_and_new_scope_but_preserves_original_request() {
    let (_dir, store) = store().await;
    let original = request(&store).await;
    let reference = original.reference().unwrap();
    let mut reordered = original.clone();
    reordered.origin = Identity::Remote {
        request_id: Uuid::new_v4(),
    };
    reordered.input = serde_json::from_str(r#"{"text":"original","outer":{"a":2,"z":1}}"#).unwrap();
    assert_eq!(reordered.reference().unwrap(), reference);
    assert!(matches!(
        store.dispatch_operation(original.clone()).await.unwrap(),
        Dispatch::New(_)
    ));
    assert!(matches!(
        store.dispatch_operation(reordered).await.unwrap(),
        Dispatch::Existing(_)
    ));
    for change in 0..5 {
        let mut changed = original.clone();
        match change {
            0 => changed.input = serde_json::json!("changed"),
            1 => changed.consumer = InstanceId(vec![Uuid::new_v4()]),
            2 => changed.export = "other".into(),
            3 => changed.contract.version += 1,
            _ => {
                changed.source = store
                    .commit_application(original.source.deployment, 1, input(), vec![], vec![])
                    .await
                    .unwrap();
            }
        }
        assert!(matches!(
            store.dispatch_operation(changed).await,
            Err(StoreError::EventConflict)
        ));
    }
    store.close().await.unwrap();
}
#[tokio::test]
async fn fixed_original_receipt_survives_replacement_and_reopen_without_execution_claim() {
    let (dir, store) = store().await;
    let request = request(&store).await;
    let reference = request.reference().unwrap();
    let Dispatch::New(claim) = store.dispatch_operation(request).await.unwrap() else {
        panic!()
    };
    store
        .commit_application(reference.source.deployment, 1, input(), vec![], vec![])
        .await
        .unwrap();
    let settled = store
        .finish_operation(
            &claim,
            OperationOutcome::Unknown {
                diagnostic: "callback ended after possible effect".into(),
            },
            OperationCleanup::Confirmed,
        )
        .await
        .unwrap();
    drop(claim);
    store.close().await.unwrap();
    let namespaces = crate::root::RootNamespaces {
        ownership: dir.path().join("owners"),
        control: dir.path().join("control"),
    };
    let owner = Arc::new(RootOwner::open(&dir.path().join("root"), &namespaces).unwrap());
    let reopened = EventLog::for_root(owner).await.unwrap();
    assert_eq!(
        reopened
            .operation_observation(&reference)
            .await
            .unwrap()
            .unwrap(),
        settled
    );
    assert_eq!(
        reopened
            .load_application_at(&reference.source)
            .await
            .unwrap()
            .unwrap()
            .source
            .reference,
        reference.source
    );
    let mut missing = reference.clone();
    missing.operation = Uuid::new_v4();
    assert!(
        reopened
            .operation_observation(&missing)
            .await
            .unwrap()
            .is_none()
    );
    let (_other_dir, other) = crate::applications::tests::store().await;
    assert!(
        other
            .operation_observation(&reference)
            .await
            .unwrap_err()
            .to_string()
            .contains("foreign operation state domain")
    );
    other.close().await.unwrap();
    reopened.close().await.unwrap();
}
#[tokio::test]
async fn dispatch_requires_original_source_and_settlement_requires_original_claim() {
    let (_dir, store) = store().await;
    let original = request(&store).await;
    let mut missing = original.clone();
    missing.source.revision += 1;
    assert!(store.dispatch_operation(missing.clone()).await.is_err());
    assert!(
        store
            .operation_observation(&missing.reference().unwrap())
            .await
            .unwrap()
            .is_none()
    );
    let mut nil = original.clone();
    nil.operation = Uuid::nil();
    assert!(store.dispatch_operation(nil).await.is_err());
    let Dispatch::New(claim) = store.dispatch_operation(original).await.unwrap() else {
        panic!()
    };
    let (_other_dir, other) = crate::applications::tests::store().await;
    assert!(
        other
            .finish_operation(
                &claim,
                OperationOutcome::Returned { value: Value::Null },
                OperationCleanup::Confirmed
            )
            .await
            .is_err()
    );
    assert!(
        store
            .finish_operation(
                &claim,
                OperationOutcome::Unresolved,
                OperationCleanup::Confirmed
            )
            .await
            .is_err()
    );
    assert!(
        store
            .finish_operation(
                &claim,
                OperationOutcome::Unknown {
                    diagnostic: "x".repeat(MAX_DIAGNOSTIC + 1)
                },
                OperationCleanup::Confirmed
            )
            .await
            .is_err()
    );
    other.close().await.unwrap();
    store.close().await.unwrap();
}
#[tokio::test]
async fn stored_receipt_bounds_precede_materialization() {
    for column in ["request_json", "outcome_json", "cleanup_json"] {
        let (_dir, store) = store().await;
        let original = request(&store).await;
        let reference = original.reference().unwrap();
        store.dispatch_operation(original).await.unwrap();
        store.connection.run(move |conn| Box::pin(async move {
            let sql = match column {
                "request_json" => "UPDATE application_operations SET request_json=json_object('huge', printf('%.*c', 4194304, 'x'))",
                "outcome_json" => "UPDATE application_operations SET outcome_json=json_object('huge', printf('%.*c', 4194304, 'x'))",
                _ => "UPDATE application_operations SET cleanup_json=json_object('huge', printf('%.*c', 4194304, 'x'))",
            };
            sqlx::query(sql).execute(conn).await?; Ok(())
        })).await.unwrap();
        assert!(
            store
                .operation_observation(&reference)
                .await
                .unwrap_err()
                .to_string()
                .contains("exceeds limit")
        );
        store.close().await.unwrap();
    }
}
