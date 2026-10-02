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
use maka_js_runtime::plugin::Module;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Actual {
    contract: Contract,
    service: Vec<String>,
}

pub(super) async fn activate(
    module: Module,
    entry: maka_plugin_core::package::EntryDeclaration,
    configuration: Value,
) -> Result<BTreeMap<String, Published<Service>>> {
    let actual: BTreeMap<String, Actual> = serde_json::from_value(
        module
            .call(vec!["activate".into()], vec![configuration])
            .await
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    entry
        .check_exports(
            &actual
                .iter()
                .map(|(n, a)| (n.clone(), a.contract.clone()))
                .collect(),
        )
        .map_err(|e| e.to_string())?;
    let mut batch = BTreeMap::new();
    for (name, actual) in actual {
        let Interface::Service { methods } = &actual.contract.interface else {
            return Err("Host export must declare a service".into());
        };
        if actual.service.is_empty() {
            return Err("service namespace is missing".into());
        }
        let callbacks = module
            .capture(actual.service, methods.keys().cloned().collect())
            .await
            .map_err(|e| e.to_string())?;
        batch.insert(
            name,
            Published {
                value: Service::new(
                    actual.contract.clone(),
                    ImplementationValue::JavaScript(callbacks),
                )?,
                contract: actual.contract,
            },
        );
    }
    Ok(batch)
}

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
enum Operation {
    Call {
        import: String,
        provider: usize,
        method: String,
        input: Value,
        attachments: BTreeMap<String, String>,
    },
    Metadata {
        slot: String,
    },
    Read {
        slot: String,
        start: usize,
        end: usize,
    },
}

impl maka_js_runtime::plugin::RequestBridge for Context {
    fn invoke(
        &self,
        input: Value,
    ) -> BoxFuture<'static, std::result::Result<Value, maka_js_runtime::plugin::Error>> {
        let operation = serde_json::from_value::<Operation>(input).map_err(|e| e.to_string());
        let result = operation.and_then(|operation| match operation {
            Operation::Call {
                import,
                provider,
                method,
                input,
                attachments,
            } => {
                let attachments = attachments
                    .into_iter()
                    .map(|(target, source)| {
                        self.attachments
                            .get(&source)
                            .cloned()
                            .map(|resource| (target, resource))
                            .ok_or_else(|| "unknown request resource slot".to_string())
                    })
                    .collect::<Result<Attachments>>()?;
                self.call(&import, provider, method, input, attachments)
            }
            operation => {
                let result = self.read(operation);
                Ok(Box::pin(async move { result }) as BoxFuture<'static, Result<Value>>)
            }
        });
        Box::pin(async move {
            match result {
                Ok(observer) => observer.await,
                Err(error) => Err(error),
            }
            .map_err(maka_js_runtime::plugin::Error)
        })
    }
}
impl Context {
    fn read(&self, operation: Operation) -> Result<Value> {
        if self.scope.cancellation.is_cancelled() {
            return Err("Host request was revoked".into());
        }
        let slot = match &operation {
            Operation::Metadata { slot } | Operation::Read { slot, .. } => slot,
            Operation::Call { .. } => unreachable!(),
        };
        let resource = self
            .attachments
            .get(slot)
            .ok_or("unknown request resource slot")?;
        let mut ticket = self.scope.resources.reserve().map_err(|e| e.to_string())?;
        ticket.start();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match operation {
            Operation::Metadata { .. } => {
                let value = serde_json::json!({"type":resource.resource_type(),"bytes":resource.bytes(),"metadata":resource.metadata()});
                check_json(&value)?;
                Ok(value)
            }
            Operation::Read { start, end, .. } => {
                if end < start || end - start > 64 * 1024 || end > resource.bytes() {
                    return Err("invalid request resource byte range".into());
                }
                let bytes = resource.read(start..end)?;
                if bytes.len() != end - start {
                    return Err("request resource returned a different byte count".into());
                }
                Ok(serde_json::json!(bytes))
            }
            Operation::Call { .. } => unreachable!(),
        }));
        match result {
            Ok(result) => {
                ticket.complete(Ok(()));
                result
            }
            Err(_) => {
                let error = "native request resource panicked; cleanup unconfirmed".to_string();
                ticket.complete(Err(error.clone()));
                Err(error)
            }
        }
    }
}
