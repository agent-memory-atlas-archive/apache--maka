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

use crate::{connections, contract::*};
use futures_util::future::BoxFuture;
use maka_config::{
    ConfigurationStore,
    oauth::enrollment::{LoginCompletion, LoginPreparation, LoginRejection},
};
use maka_plugins::{
    model::{Connect, Error, Socket, Transport},
    provider::{self, authentication::Authenticate},
};
use maka_runtime::{
    oauth::{Failure, LoginStart, Phase, Target},
    provider::AuthenticationInput,
};
use maka_runtime_host::application::Context;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

/// API-key authentication only validates and wraps local input. Its Context does
/// not acquire network authority merely to satisfy the provider interface.
struct NoNetwork;
impl Transport for NoNetwork {
    fn identity(&self) -> u64 {
        0
    }
    fn request(
        &self,
        _: maka_plugins::http::Request,
    ) -> BoxFuture<'_, Result<maka_plugins::http::Response, Error>> {
        Box::pin(async {
            Err(Error::Adapter(
                "settings_authentication_network_unavailable".into(),
            ))
        })
    }
    fn connect(&self, _: Connect) -> BoxFuture<'_, Result<Arc<dyn Socket>, Error>> {
        Box::pin(async {
            Err(Error::Adapter(
                "settings_authentication_network_unavailable".into(),
            ))
        })
    }
}
fn rejected(reason: CredentialRejection) -> Result<Value, String> {
    connections::encode(CredentialSaveResult::Rejected { reason })
}
pub(crate) async fn save(
    store: &Arc<ConfigurationStore>,
    input: Value,
    context: Context,
) -> Result<Value, String> {
    let operation = context.operation().ok_or("settings_mutation_required")?;
    let Ok(request) = serde_json::from_value::<SaveCredential>(input) else {
        return rejected(CredentialRejection::InvalidInput);
    };
    let Ok(definition) = connections::definition(&request.expected.provider) else {
        return rejected(CredentialRejection::ProviderUnavailable);
    };
    if !definition
        .descriptor()
        .authentication
        .iter()
        .any(|method| method.id == "api-key" && !method.interactive)
    {
        return rejected(CredentialRejection::AuthenticationUnavailable);
    }
    let Some(resource) = context.attachments().get("secret") else {
        return rejected(CredentialRejection::InvalidSecret);
    };
    let bytes = resource.bytes();
    if bytes == 0 || bytes > 65536 {
        return rejected(CredentialRejection::InvalidSecret);
    }
    let Ok(secret) = resource.read(0..bytes) else {
        return rejected(CredentialRejection::SecretUnavailable);
    };
    let Ok(secret) = String::from_utf8(secret) else {
        return rejected(CredentialRejection::InvalidSecret);
    };
    if secret.len() != bytes || secret.chars().any(char::is_control) {
        return rejected(CredentialRejection::InvalidSecret);
    }
    let attempt_id = format!(
        "settings-{:x}",
        Sha256::digest(serde_json::to_vec(operation).map_err(|_| "settings_operation_invalid")?)
    );
    let input = LoginStart {
        attempt_id,
        target: Target::Existing {
            expected: request.expected.clone(),
            configuration: request.expected.configuration.clone(),
        },
        expected_credential: request.expected_credential,
        authentication: AuthenticationInput {
            method: "api-key".into(),
            input: json!({"apiKey":secret}),
        },
    };
    if input.validate().is_err() {
        return rejected(CredentialRejection::InvalidInput);
    }
    let authentication = input.authentication.clone();
    let prepared = store
        .prepare_oauth_login(input)
        .await
        .map_err(|_| "settings_effect_unknown")?;
    let ticket = match prepared {
        LoginPreparation::Ready(ticket) => ticket,
        LoginPreparation::Finished(receipt) if receipt.phase == Phase::Authenticated => {
            return connections::encode(CredentialSaveResult::Saved {
                connection: receipt.connection,
            });
        }
        LoginPreparation::Finished(_) | LoginPreparation::OutcomeUnknown(_) => {
            return Err("settings_effect_unknown".into());
        }
        LoginPreparation::Rejected(reason) => {
            return rejected(match reason {
                LoginRejection::AttemptConflict => CredentialRejection::AttemptConflict,
                LoginRejection::ConnectionNotFound | LoginRejection::ConnectionChanged => {
                    CredentialRejection::ConnectionChanged
                }
                LoginRejection::CredentialChanged => CredentialRejection::CredentialChanged,
                LoginRejection::CatalogFull | LoginRejection::AttemptsFull => {
                    CredentialRejection::Capacity
                }
                LoginRejection::SlugTaken => CredentialRejection::SlugTaken,
            });
        }
    };
    if !ticket
        .claim()
        .await
        .map_err(|_| "settings_effect_unknown")?
    {
        return Err("settings_effect_unknown".into());
    }
    let row = ticket.connection();
    let credential = definition
        .authenticate(
            Authenticate {
                connection: provider::Connection {
                    id: row.connection_id.clone(),
                    revision: row.revision,
                    configuration: row.configuration.clone(),
                },
                method: authentication.method,
                input: authentication.input,
            },
            provider::Context {
                transport: Arc::new(NoNetwork),
                cancellation: context.scope().cancellation.clone(),
                interaction: None,
            },
        )
        .await;
    let credential = match credential {
        Ok(credential) => credential,
        Err(_) => {
            ticket
                .finish_failure(Phase::Failed {
                    failure: Failure::AuthorizationFailed,
                })
                .await
                .map_err(|_| "settings_effect_unknown")?;
            return rejected(CredentialRejection::Authentication);
        }
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "settings_clock_unavailable")?
        .as_secs();
    match ticket
        .complete(credential, now)
        .await
        .map_err(|_| "settings_effect_unknown")?
    {
        LoginCompletion::Committed(receipt) => connections::encode(CredentialSaveResult::Saved {
            connection: receipt.connection,
        }),
        LoginCompletion::ConnectionChanged => rejected(CredentialRejection::ConnectionChanged),
        LoginCompletion::CredentialChanged => rejected(CredentialRejection::CredentialChanged),
        LoginCompletion::AttemptConflict => rejected(CredentialRejection::AttemptConflict),
        LoginCompletion::SlugTaken => rejected(CredentialRejection::SlugTaken),
    }
}
