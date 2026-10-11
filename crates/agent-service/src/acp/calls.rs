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
use maka_plugin_core::operation::{OperationCleanup, OperationOutcome, Receipt};
use maka_plugins::{
    call::{Attachments, Scope},
    execution::{OfferInteraction, Prompt},
    filesystem::{Operation, Output as FileOutput},
};
use maka_runtime_host::application::{NativeRequest, NativeServices};
use sha2::{Digest, Sha256};

pub(super) struct Calls {
    pub services: NativeServices,
    pub issuer: Issuer,
}
pub(super) fn digest(value: &impl Serialize) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("ACP identity"))
    )
}
pub(super) fn operation(value: &impl Serialize) -> Uuid {
    Uuid::from_bytes(
        Sha256::digest(serde_json::to_vec(value).expect("ACP identity"))[..16]
            .try_into()
            .expect("UUID digest"),
    )
}
impl Calls {
    pub fn origin<'a>(&self, scope: &'a Scope) -> Result<&'a Context, String> {
        let origin = self
            .issuer
            .evidence::<Arc<Context>>(scope)
            .ok_or("ACP call is not admitted")?;
        if scope.cancellation.is_cancelled()
            || origin.scope().cancellation.is_cancelled()
            || scope.identity != origin.scope().identity
        {
            return Err("ACP call is closed or changed".into());
        }
        Ok(origin.as_ref())
    }
    pub fn request(
        &self,
        scope: &Scope,
        identity: String,
        cancellation: CancellationToken,
    ) -> Result<NativeRequest, String> {
        let origin = self.origin(scope)?;
        let invocation = origin
            .scope()
            .identity
            .agent()
            .ok_or("ACP requires an invocation")?
            .clone();
        self.services.request(
            Identity::Agent {
                invocation,
                operation_id: Some(identity),
            },
            cancellation,
        )
    }
    pub async fn invoke(
        &self,
        scope: Scope,
        identity: String,
        cancellation: CancellationToken,
        target: (&str, &str),
        input: Value,
        mutation: bool,
    ) -> Result<Value, String> {
        let (import, method) = target;
        let request = self.request(&scope, identity.clone(), cancellation)?;
        let mut ticket = scope.resources.reserve().map_err(|e| e.to_string())?;
        ticket.start();
        let result = async {
            if mutation {
                let reply = request
                    .mutate(
                        import,
                        0,
                        method.into(),
                        input,
                        operation(&(scope.identity.agent(), &identity, import, method)),
                        Attachments::default(),
                    )?
                    .await?;
                returned(reply.receipt)
            } else {
                request
                    .call(import, 0, method.into(), input, Attachments::default())?
                    .await
            }
        }
        .await;
        let cleanup = request.finish().await;
        ticket.complete(cleanup.clone());
        cleanup?;
        result
    }
}
pub(super) fn returned(receipt: Receipt) -> Result<Value, String> {
    match receipt {
        Receipt::Recorded {
            outcome: OperationOutcome::Returned { value },
            cleanup: OperationCleanup::Confirmed,
        } => Ok(value),
        _ => Err("ACP service operation outcome or cleanup is unconfirmed".into()),
    }
}
impl maka_external_agent::ports::Callbacks for Calls {
    fn prompt_attachments(
        &self,
        scope: &Scope,
    ) -> Result<Vec<maka_external_agent::ports::PromptAttachment>, String> {
        let origin = self.origin(scope)?;
        if origin.attachments().len() > 8 {
            return Err("ACP attachment limit exceeded".into());
        }
        (0..origin.attachments().len())
            .map(|index| {
                let resource = origin
                    .attachments()
                    .get(&crate::executor::attachments::slot(index))
                    .ok_or("ACP attachment slots are invalid")?
                    .clone();
                let metadata: crate::executor::attachments::Metadata =
                    serde_json::from_value(resource.metadata())
                        .map_err(|_| "ACP attachment metadata is invalid")?;
                if resource.resource_type() != maka_plugin_core::contract::binary_resource_type()
                    || resource.encoding() != maka_plugins::call::ResourceEncoding::Bytes
                    || resource.bytes() as u64 != metadata.bytes
                {
                    return Err("ACP attachment descriptor is invalid".into());
                }
                Ok(maka_external_agent::ports::PromptAttachment {
                    name: metadata.name,
                    mime_type: metadata.mime_type,
                    resource,
                })
            })
            .collect()
    }

    fn file(
        &self,
        scope: Scope,
        identity: String,
        operation: Operation,
        cancellation: CancellationToken,
    ) -> BoxFuture<'_, Result<FileOutput, String>> {
        Box::pin(async move {
            let (method, input, mutation) = match operation {
                Operation::Read(input) => ("readText", serde_json::to_value(input), false),
                Operation::Write(input) => ("write", serde_json::to_value(input), true),
                _ => return Err("Unsupported ACP filesystem callback".into()),
            };
            let value = self
                .invoke(
                    scope,
                    format!("acp-file-{}", digest(&identity)),
                    cancellation,
                    ("files", method),
                    input.map_err(|e| e.to_string())?,
                    mutation,
                )
                .await?;
            match value.get("kind").and_then(Value::as_str) {
                Some("completed") => Ok(FileOutput::Value(
                    value
                        .get("value")
                        .cloned()
                        .ok_or("ACP file result missing")?,
                )),
                _ => Err(value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("ACP file request failed")
                    .into()),
            }
        })
    }
    fn form(
        &self,
        scope: Scope,
        offer: OfferInteraction,
        cancellation: CancellationToken,
    ) -> BoxFuture<'_, Result<Option<maka_runtime::capability::FormResult>, String>> {
        Box::pin(async move {
            let origin = self.origin(&scope)?;
            if origin.scope().identity.agent() != Some(&offer.invocation) {
                return Err("ACP form invocation changed".into());
            }
            let Prompt::Form { message, fields } = offer.prompt else {
                return Err("ACP expected a form".into());
            };
            let value = self
                .invoke(
                    scope,
                    offer.operation_id,
                    cancellation,
                    ("forms", "ask"),
                    serde_json::json!({"message":message,"fields":fields}),
                    true,
                )
                .await?;
            match value.get("kind").and_then(Value::as_str) {
                Some("answered") => serde_json::from_value(
                    value
                        .get("result")
                        .cloned()
                        .ok_or("ACP form answer missing")?,
                )
                .map(Some)
                .map_err(|e| e.to_string()),
                Some("closed") => Ok(None),
                _ => Err("ACP form request rejected".into()),
            }
        })
    }
}
