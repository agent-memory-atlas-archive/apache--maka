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
async fn observe_exposes_last_known_context_only_for_an_active_matching_backend() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fixture = Fixture::new(
            &format!("http://{}/v1", listener.local_addr().unwrap()),
            None,
        )
        .await;
        let session = fixture.create("Context metrics").await;
        fixture.send(&session).await.unwrap();
        let (mut first, _) = listener.accept().await.unwrap();
        request(&mut first).await;
        response(&mut first).await;
        fixture.drained().await;

        let uploaded = attachments::upload(
            &fixture,
            &session,
            "context.txt",
            b"new context whose token count is unknown",
            Uuid::new_v4(),
        )
        .await
        .unwrap();
        let sent = fixture
            .mutation(
                2,
                "send",
                json!({"sessionId":session,"message":"Read this","attachments":[uploaded["attachment"]["id"]],"model":fixture.selection().await}),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        let (mut second, _) = listener.accept().await.unwrap();
        request(&mut second).await;
        let active = fixture.query("observe", json!({"sessionId":session})).await.unwrap();
        assert_eq!(active["run"]["invocationId"], sent["invocationId"]);
        assert_eq!(active["metrics"]["contextTokens"], 3);
        assert_eq!(active["metrics"]["contextLastKnown"], true);

        let record = fixture.store.get_session::<SessionConfiguration>(&session).await.unwrap().unwrap();
        let original_backend = record.configuration.backend.clone();
        fixture.store.update_session_metadata::<SessionConfiguration, _>(&session, record.revision, |configuration| {
            configuration.backend = Some(contract::BackendPreference::Model {
                connection_id: "different-connection".into(),
                model_id: "different-model".into(),
                thinking_level: None,
            });
            Ok(())
        }).await.unwrap();
        let mismatch = fixture.query("observe", json!({"sessionId":session})).await.unwrap();
        assert!(mismatch["metrics"]["contextTokens"].is_null());
        assert_eq!(mismatch["metrics"]["contextLastKnown"], false);

        let record = fixture.store.get_session::<SessionConfiguration>(&session).await.unwrap().unwrap();
        fixture.store.update_session_metadata::<SessionConfiguration, _>(&session, record.revision, move |configuration| {
            configuration.backend = original_backend;
            Ok(())
        }).await.unwrap();
        fixture.mutation(2, "stop", json!({"sessionId":session,"invocationId":sent["invocationId"]}), Uuid::new_v4()).await.unwrap();
        fixture.drained().await;
        assert!(fixture.store.context_usage(&session).await.unwrap().1.unwrap().last_known);
        let idle = fixture.query("observe", json!({"sessionId":session})).await.unwrap();
        assert!(idle["run"].is_null());
        assert!(idle["metrics"]["contextTokens"].is_null());
        assert_eq!(idle["metrics"]["contextLastKnown"], false);
        fixture.app.shutdown().await.unwrap();
        fixture.config.shutdown().await.unwrap();
    }).await.unwrap();
}
