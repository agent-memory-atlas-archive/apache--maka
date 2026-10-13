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
use serde_json::json;

#[tokio::test]
async fn selection_is_created_once_and_finishing_only_discards_preparation() {
    let (_dir, store) = crate::applications::tests::store().await;
    let id = Uuid::new_v4();
    assert!(store.application_launch().await.unwrap().is_none());
    let record = || Launch {
        deployment: id,
        configuration: json!({"trusted":"original"}),
        pending: Some(vec![1, 2, 3]),
    };
    store.begin_application_launch(record()).await.unwrap();
    assert!(store.begin_application_launch(record()).await.is_err());
    assert!(
        store
            .complete_application_launch(Uuid::new_v4())
            .await
            .is_err()
    );
    let original = store.application_launch().await.unwrap().unwrap();
    assert_eq!(original.pending, Some(vec![1, 2, 3]));
    store.complete_application_launch(id).await.unwrap();
    store.complete_application_launch(id).await.unwrap();
    let completed = store.application_launch().await.unwrap().unwrap();
    assert_eq!(completed.deployment, id);
    assert_eq!(completed.configuration, original.configuration);
    assert!(completed.pending.is_none());
    store.close().await.unwrap();
}

#[tokio::test]
async fn launcher_requires_root_and_bounds_stored_configuration_before_reading() {
    let directory = tempfile::tempdir().unwrap();
    let unowned = EventLog::open(&directory.path().join("unowned.sqlite"))
        .await
        .unwrap();
    assert!(unowned.application_launch().await.is_err());
    assert!(
        unowned
            .begin_application_launch(Launch {
                deployment: Uuid::new_v4(),
                configuration: json!({}),
                pending: Some(vec![1])
            })
            .await
            .is_err()
    );
    unowned.close().await.unwrap();
    let (_dir, store) = crate::applications::tests::store().await;
    for (id, pending) in [
        (Uuid::nil(), Some(vec![1])),
        (Uuid::new_v4(), None),
        (Uuid::new_v4(), Some(vec![])),
    ] {
        assert!(
            store
                .begin_application_launch(Launch {
                    deployment: id,
                    configuration: json!({}),
                    pending
                })
                .await
                .is_err()
        );
    }
    assert!(
        store
            .begin_application_launch(Launch {
                deployment: Uuid::new_v4(),
                configuration: json!({"large":"x".repeat(MAX_CONFIGURATION)}),
                pending: Some(vec![1])
            })
            .await
            .is_err()
    );
    store.connection.run(|connection| Box::pin(async move {
        sqlx::query("INSERT INTO application_launch (singleton,deployment,configuration,pending) VALUES (1,?,json_object('large',printf('%.*c',1048576,'x')),NULL)")
            .bind(Uuid::new_v4().to_string()).execute(connection).await?;
        Ok(())
    })).await.unwrap();
    assert!(store.application_launch().await.is_err());
    store.close().await.unwrap();
}

#[tokio::test]
async fn unknown_commit_is_not_reported_as_success_or_replaced() {
    let (_dir, store) = crate::applications::tests::store().await;
    store.connection.run(|connection| Box::pin(async move {
        sqlx::raw_sql("CREATE TABLE launcher_probe (id TEXT REFERENCES application_deployments(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER reject_launch AFTER INSERT ON application_launch BEGIN INSERT INTO launcher_probe VALUES ('missing'); END;").execute(connection).await?;
        Ok(())
    })).await.unwrap();
    let result = store
        .begin_application_launch(Launch {
            deployment: Uuid::new_v4(),
            configuration: json!({}),
            pending: Some(vec![1]),
        })
        .await;
    assert!(matches!(result, Err(StoreError::CommitUnknown(_))));
    assert!(store.application_launch().await.unwrap().is_none());
    store.close().await.unwrap();
}

#[tokio::test]
async fn product_baseline_and_connections_publish_atomically_with_completion() {
    let (_dir, store) = crate::applications::tests::store().await;
    let id = Uuid::new_v4();
    store
        .begin_application_launch(Launch {
            deployment: id,
            configuration: json!({"old":true}),
            pending: Some(vec![1]),
        })
        .await
        .unwrap();
    assert!(
        store
            .complete_application_product(Uuid::new_v4(), json!({}), json!({}))
            .await
            .is_err()
    );
    assert!(store.application_product().await.unwrap().is_none());
    store.connection.run(|connection| Box::pin(async move {
        sqlx::raw_sql("CREATE TABLE product_probe (id TEXT REFERENCES application_deployments(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER reject_product AFTER UPDATE ON application_launch BEGIN INSERT INTO product_probe VALUES ('missing'); END;").execute(connection).await?;
        Ok(())
    })).await.unwrap();
    assert!(matches!(
        store
            .complete_application_product(id, json!({"new":true}), json!({"baseline":2}))
            .await,
        Err(StoreError::CommitUnknown(_))
    ));
    let original = store.application_launch().await.unwrap().unwrap();
    assert_eq!(original.configuration, json!({"old":true}));
    assert_eq!(original.pending, Some(vec![1]));
    assert!(store.application_product().await.unwrap().is_none());
    store
        .connection
        .run(|connection| {
            Box::pin(async move {
                sqlx::query("DROP TRIGGER reject_product")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    store
        .complete_application_product(id, json!({"new":true}), json!({"baseline":2}))
        .await
        .unwrap();
    assert_eq!(
        store.application_product().await.unwrap(),
        Some(json!({"baseline":2}))
    );
    let completed = store.application_launch().await.unwrap().unwrap();
    assert_eq!(completed.configuration, json!({"new":true}));
    assert!(completed.pending.is_none());
    assert!(
        store
            .complete_application_product(id, json!({}), json!({}))
            .await
            .is_err()
    );
    store.close().await.unwrap();
}
