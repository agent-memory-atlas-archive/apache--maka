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
use maka_agent_facts::configuration::*;
use maka_capabilities::http;
use maka_event_log::root::{RootNamespaces, RootOwner};
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

struct Body(Mutex<Option<Vec<u8>>>);
impl http::Body for Body {
    fn next(&self) -> BoxFuture<'_, Result<Option<Vec<u8>>, http::Error>> {
        Box::pin(async { Ok(self.0.lock().unwrap().take()) })
    }
    fn cancel(&self) {}
    fn close(&self) -> BoxFuture<'_, Result<(), http::Error>> {
        Box::pin(async { Ok(()) })
    }
}
#[derive(Default)]
struct Script {
    requests: Mutex<Vec<String>>,
    fail: bool,
}
impl Transport for Script {
    fn identity(&self) -> u64 {
        1
    }
    fn request(&self, request: http::Request) -> BoxFuture<'_, Result<http::Response, Error>> {
        Box::pin(async move {
            self.requests.lock().unwrap().push(request.url.clone());
            if self.fail {
                return Err(Error::Adapter("private transport diagnostic".into()));
            }
            let value = if request.url.ends_with("/usercode") {
                json!({"device_auth_id":"private-device-id", "user_code":"ABCD-EFGH", "interval":1})
            } else if request.url.ends_with("/deviceauth/token") {
                json!({"authorization_code":"private-code", "code_verifier":"private-verifier"})
            } else {
                assert_eq!(request.url, "https://auth.openai.com/oauth/token");
                json!({"access_token":"private-access", "refresh_token":"private-refresh", "expires_in":3600})
            };
            Ok(http::Response {
                head: http::Head {
                    status: 200,
                    url: request.url,
                    headers: vec![],
                },
                body: Arc::new(Body(Mutex::new(Some(serde_json::to_vec(&value).unwrap())))),
            })
        })
    }
    fn connect(&self, _: Connect) -> BoxFuture<'_, Result<Arc<dyn Socket>, Error>> {
        Box::pin(async { Err(Error::Adapter("unexpected websocket".into())) })
    }
}
struct Prompt {
    cancel: CancellationToken,
    cancel_on_prompt: bool,
    change: Option<(Arc<ConfigurationStore>, ConnectionVersionBasis)>,
    seen: Mutex<Vec<(String, Option<String>)>>,
}
impl provider::authentication::Interaction for Prompt {
    fn open_external(
        &self,
        url: String,
        code: Option<String>,
    ) -> BoxFuture<'_, Result<(), provider::Error>> {
        Box::pin(async move {
            self.seen.lock().unwrap().push((url, code));
            if self.cancel_on_prompt {
                self.cancel.cancel();
            }
            if let Some((store, expected)) = &self.change {
                let result = store.update_connection(serde_json::from_value(json!({"expected": expected, "changes":{"name":"Changed while signing in"}})).unwrap()).await.unwrap();
                assert!(matches!(result, CatalogMutationResult::Committed { .. }));
            }
            Ok(())
        })
    }
}
#[tokio::test]
async fn chatgpt_login_saves_privately_and_rejects_cancelled_or_changed_connections() {
    for scenario in ["success", "cancel", "changed", "network"] {
        let directory = tempfile::tempdir().unwrap();
        let namespaces = RootNamespaces {
            ownership: directory.path().join("owners"),
            control: directory.path().join("control"),
        };
        let root =
            Arc::new(RootOwner::create(&directory.path().join("root"), &namespaces).unwrap());
        let store = Arc::new(ConfigurationStore::for_root(root).await.unwrap());
        let identity: provider::Identity = serde_json::from_value(json!({"packageId":"maka.codex","entryId":"maka.codex","scope":"profile","name":"chatgpt"})).unwrap();
        let definition = connections::definition(&identity).unwrap();
        let revision = store.catalog().await.unwrap().revision;
        store.create_connection(serde_json::from_value(json!({"expectedCatalogRevision":revision,"connection":{"slug":"chatgpt","name":"ChatGPT","provider":identity,"configuration":definition.descriptor().configuration_defaults,"enabled":true,"enabledModelIds":[]}})).unwrap()).await.unwrap();
        let row = store.catalog().await.unwrap().connections.remove(0);
        let locator = CredentialLocator::Connection {
            connection_id: row.connection_id.clone(),
            kind: ConnectionCredentialKind::Provider,
        };
        let expected = ConnectionCredentialTarget {
            connection_id: row.connection_id.clone(),
            revision: row.revision,
            slug: row.slug.clone(),
            provider: row.provider.clone(),
            configuration: row.configuration.clone(),
        };
        let authentication = AuthenticationInput {
            method: "chatgpt".into(),
            input: json!({}),
        };
        let input = LoginStart {
            attempt_id: "test-login".into(),
            expected_credential: None,
            target: Target::Existing {
                expected,
                configuration: row.configuration,
            },
            authentication: authentication.clone(),
        };
        let LoginPreparation::Ready(ticket) = store.prepare_oauth_login(input).await.unwrap()
        else {
            panic!("ticket")
        };
        let cancellation = CancellationToken::new();
        let prompt = Arc::new(Prompt {
            cancel: cancellation.clone(),
            cancel_on_prompt: scenario == "cancel",
            change: (scenario == "changed").then(|| {
                (
                    store.clone(),
                    ConnectionVersionBasis {
                        connection_id: row.connection_id,
                        revision: row.revision,
                    },
                )
            }),
            seen: Mutex::new(vec![]),
        });
        let transport = Arc::new(Script {
            fail: scenario == "network",
            ..Default::default()
        });
        let result = authenticate(
            ticket,
            definition,
            authentication,
            provider::Context {
                transport: transport.clone(),
                cancellation,
                interaction: Some(prompt.clone()),
            },
            true,
        )
        .await
        .unwrap();
        let public = result.to_string();
        assert!(!public.contains("private-"));
        let secret = store.credential_secret(&locator, None).await.unwrap();
        if scenario == "success" {
            assert_eq!(result["kind"], "saved");
            assert!(secret.unwrap().contains("private-refresh"));
            assert_eq!(transport.requests.lock().unwrap().len(), 3);
        } else {
            assert!(secret.is_none());
            assert_eq!(
                result["reason"],
                match scenario {
                    "cancel" => "cancelled",
                    "changed" => "connection_changed",
                    _ => "transport",
                }
            );
        }
        if scenario != "network" {
            assert_eq!(
                *prompt.seen.lock().unwrap(),
                vec![(
                    "https://auth.openai.com/codex/device".into(),
                    Some("ABCD-EFGH".into())
                )]
            );
        }
        if scenario == "cancel" {
            assert_eq!(
                store
                    .oauth_login_receipt("test-login".into())
                    .await
                    .unwrap()
                    .unwrap()
                    .phase,
                Phase::Cancelled
            );
            assert_eq!(transport.requests.lock().unwrap().len(), 1);
        }
        store.shutdown().await.unwrap();
    }
}

#[test]
fn compiled_catalog_includes_chatgpt_and_still_bounds_provider_pages() {
    let mut offset = 0;
    let mut found = false;
    let mut count = 0;
    let mut bytes = 0;
    loop {
        let page = connections::providers(json!({"offset":offset})).unwrap();
        let rows = page["providers"].as_array().unwrap();
        assert!(rows.len() <= 8);
        count += rows.len();
        bytes += page.to_string().len();
        for row in rows {
            if row["identity"]["packageId"] == "maka.codex" {
                found = true;
                assert_eq!(row["descriptor"]["label"], "ChatGPT Subscription");
                assert_eq!(row["descriptor"]["authentication"][0]["interactive"], true);
            }
        }
        match page["next"].as_u64() {
            Some(next) => {
                assert!(next > offset);
                offset = next;
            }
            None => break,
        }
    }
    assert!(found);
    assert!(
        count < 1024 && bytes < 512 * 1024,
        "the searchable compiled catalog must fit in one UI frame"
    );
}
