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

use crate::Result;
use maka_event_log::applications::intents::RecordedIntent;
use maka_plugin_core::operation::{OperationReply, Receipt};
use maka_plugins::call::Scope;
use maka_runtime_host::application::{Binding, Client, PreparedMutation};
use maka_ui::actions::*;
use std::sync::{Arc, Mutex, OnceLock};

/// Trusted embedding admission. Called only on the asynchronous action path.
/// Must only construct an inert Scope; no native resource may escape an error.
/// Source observations do not authorize effects: Host Policy checks the evidence.
pub trait Scopes: Send + Sync + 'static {
    fn scope(&self, source: &ServiceSource) -> Result<Scope>;
}

pub struct LocalService {
    client: Client,
    binding: Binding,
    scopes: Arc<dyn Scopes>,
}
impl LocalService {
    pub fn new(client: Client, binding: Binding, scopes: Arc<dyn Scopes>) -> Self {
        Self {
            client,
            binding,
            scopes,
        }
    }
}
impl NativeService for LocalService {
    fn call(&self, request: ServiceRequest) -> ServiceFuture {
        let client = self.client.clone();
        let binding = self.binding.clone();
        let scopes = self.scopes.clone();
        Box::pin(async move {
            let scope = match scopes.scope(&request.source) {
                Ok(scope) => scope,
                Err(error) => {
                    return ServiceCompletion {
                        result: Err(error),
                        cleanup: Ok(()),
                    };
                }
            };
            let completion = match client
                .start(
                    binding,
                    request.method,
                    request.input,
                    scope.clone(),
                    request.resources,
                )
                .await
            {
                Ok(accepted) => {
                    let completion = accepted.completion().await;
                    ServiceCompletion {
                        result: completion.result,
                        cleanup: completion.cleanup,
                    }
                }
                Err(error) => ServiceCompletion {
                    result: Err(error),
                    cleanup: Ok(()),
                },
            };
            let own_cleanup = scope.finish().await.map_err(|e| e.to_string());
            ServiceCompletion {
                result: completion.result,
                cleanup: completion.cleanup.and(own_cleanup),
            }
        })
    }
    fn mutation(&self, request: ServiceRequest) -> Result<Arc<dyn NativeMutation>> {
        if !request.resources.is_empty() {
            return Err("mutation resource snapshots require their executor".into());
        }
        Ok(Arc::new(Session {
            client: self.client.clone(),
            binding: self.binding.clone(),
            scopes: self.scopes.clone(),
            request,
            scope: OnceLock::new(),
            phase: Mutex::new(Phase::New),
        }))
    }
}
enum Phase {
    New,
    Preparing,
    Ready(Box<PreparedMutation>),
    Closed,
}
struct Session {
    client: Client,
    binding: Binding,
    scopes: Arc<dyn Scopes>,
    request: ServiceRequest,
    scope: OnceLock<Scope>,
    phase: Mutex<Phase>,
}
impl NativeMutation for Session {
    fn prepare(self: Arc<Self>) -> ReferenceFuture {
        Box::pin(async move {
            let scope = {
                let mut phase = self.phase.lock().unwrap();
                if !matches!(*phase, Phase::New) {
                    return Err("mutation session is not new".into());
                }
                // Publication is atomic with respect to close. The trusted factory
                // is synchronous and inert; no resource can appear after close.
                let scope = self.scopes.scope(&self.request.source)?;
                self.scope
                    .set(scope.clone())
                    .map_err(|_| "mutation Scope already exists")?;
                *phase = Phase::Preparing;
                scope
            };
            let prepared = self
                .client
                .prepare_mutation(
                    self.binding.clone(),
                    self.request.method.clone(),
                    self.request.input.clone(),
                    scope,
                    self.request.source.request_id(),
                )
                .await?;
            let reference = prepared.reference().clone();
            let mut phase = self.phase.lock().unwrap();
            if !matches!(*phase, Phase::Preparing) {
                return Err("mutation session closed during preparation".into());
            }
            *phase = Phase::Ready(Box::new(prepared));
            Ok(reference)
        })
    }
    fn dispatch(self: Arc<Self>, recorded: RecordedIntent) -> MutationFuture {
        Box::pin(async move {
            let reference = recorded.intent().reference.clone();
            let prepared = match std::mem::replace(&mut *self.phase.lock().unwrap(), Phase::Closed)
            {
                Phase::Ready(prepared) => Some(prepared),
                _ => None,
            };
            let admission = (|| {
                let prepared = prepared.ok_or("mutation session is not prepared")?;
                if self.request.source.deployment() != Some(&recorded.intent().caller)
                    || self.request.source.consumer().instance() != &recorded.intent().consumer
                    || prepared.reference() != &reference
                {
                    return Err("caller confirmation differs from the original request".to_string());
                }
                Ok(prepared)
            })();
            let completion = match admission {
                Ok(prepared) => self.client.mutate(*prepared, recorded).await,
                Err(error) => Err(error),
            };
            let completion = match completion {
                Ok(accepted) => {
                    let completion = accepted.completion().await;
                    MutationCompletion {
                        reply: completion.reply,
                        cleanup: completion.cleanup,
                    }
                }
                Err(diagnostic) => MutationCompletion {
                    reply: OperationReply {
                        reference,
                        receipt: Receipt::Unavailable { diagnostic },
                    },
                    cleanup: Ok(()),
                },
            };
            let own_cleanup = self.finish_scope().await;
            MutationCompletion {
                reply: completion.reply,
                cleanup: completion.cleanup.and(own_cleanup),
            }
        })
    }
    fn close(self: Arc<Self>) -> CleanupFuture {
        Box::pin(async move {
            *self.phase.lock().unwrap() = Phase::Closed;
            self.finish_scope().await
        })
    }
}
impl Session {
    async fn finish_scope(&self) -> Result<()> {
        match self.scope.get() {
            Some(scope) => scope.finish().await.map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }
}
