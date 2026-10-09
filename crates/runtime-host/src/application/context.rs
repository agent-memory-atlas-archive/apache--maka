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

/// Actual request capabilities shared by native and JavaScript services.
/// Imports are captured from the consumer activation, never resolved by a
/// guest-supplied global name. Calls preserve the original private Scope evidence.
pub struct Context {
    pub(super) stream: Option<subscriptions::Emitter>,
    pub(super) scope: Scope,
    pub(super) consumer: Activation,
    pub(super) operation: Option<RecoveryRef>,
    pub(super) attachments: Attachments,
    pub(super) imports: Arc<BTreeMap<String, Vec<Binding>>>,
    pub(super) dispatch: Weak<Dispatch>,
}
impl Context {
    /// Publish one bounded, schema-checked subscription item. Backpressure waits
    /// for the reader; producer completion or cancellation rejects further emits.
    pub fn emit(&self, value: Value) -> BoxFuture<'static, Result<()>> {
        match &self.stream {
            Some(stream) => stream.emit(value),
            None => Box::pin(async { Err("request is not a subscription".into()) }),
        }
    }
    pub fn operation(&self) -> Option<&RecoveryRef> {
        self.operation.as_ref()
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn attachments(&self) -> &Attachments {
        &self.attachments
    }
    /// Enqueues work before returning its observer. Dropping that observer cannot
    /// acknowledge cleanup or cancel an accepted child. The parent waits for the
    /// actual executor's cleanup, independently of its business result.
    pub fn call(
        &self,
        import: &str,
        provider: usize,
        method: String,
        input: Value,
        attachments: Attachments,
    ) -> Result<BoxFuture<'static, Result<Value>>> {
        let (reply, result) = oneshot::channel();
        self.enqueue(
            import,
            provider,
            method,
            input,
            attachments,
            self.scope.clone(),
            CallKind::Query(reply),
        )?;
        Ok(Box::pin(async move {
            result
                .await
                .map_err(|_| "Host deployment is closed".to_string())??
                .result()
                .await
        }))
    }
    /// Captures and enqueues a child subscription before returning its observer.
    /// Closing demand cancels only this child; the parent Ticket waits for the
    /// actual executor's cleanup, including when the observer is dropped.
    pub fn subscribe(
        &self,
        import: &str,
        provider: usize,
        method: String,
        input: Value,
        attachments: Attachments,
    ) -> Result<BoxFuture<'static, Result<Subscription>>> {
        let scope = self.scope.child().map_err(|error| error.to_string())?;
        let guard = scope.cancellation.clone().drop_guard();
        let buffer = self
            .dispatch
            .upgrade()
            .ok_or("Host deployment is closed")?
            .stream_buffers
            .clone()
            .try_acquire_owned()
            .map_err(|_| "Host subscription buffer capacity exhausted".to_string())?;
        let (reply, result) = oneshot::channel();
        self.enqueue(
            import,
            provider,
            method,
            input,
            attachments,
            scope,
            CallKind::Subscription { reply, buffer },
        )?;
        Ok(Box::pin(async move {
            let subscription = result
                .await
                .map_err(|_| "Host deployment is closed".to_string())??;
            guard.disarm();
            Ok(subscription)
        }))
    }
    /// The observer's outer Err is admission failure. Once accepted, business
    /// uncertainty is always an OperationReply with its original RecoveryRef.
    /// A clean refusal additionally requires this request Scope's real finish.
    #[allow(clippy::too_many_arguments)]
    pub fn mutate(
        &self,
        import: &str,
        provider: usize,
        method: String,
        input: Value,
        operation: Uuid,
        attachments: Attachments,
    ) -> Result<BoxFuture<'static, Result<OperationReply>>> {
        let (reply, result) = oneshot::channel();
        self.enqueue(
            import,
            provider,
            method,
            input,
            attachments,
            self.scope.clone(),
            CallKind::Mutation {
                operation,
                confirmation: None,
                reply,
            },
        )?;
        Ok(Box::pin(async move {
            Ok(result
                .await
                .map_err(|_| "Host deployment is closed".to_string())??
                .result()
                .await)
        }))
    }
    /// Uses the original reference, never the current import/provider selection.
    pub fn recover(
        &self,
        reference: RecoveryRef,
    ) -> Result<BoxFuture<'static, Result<Option<OperationObservation>>>> {
        if self.scope.cancellation.is_cancelled() {
            return Err("Host request was revoked".into());
        }
        let dispatch = self.dispatch.upgrade().ok_or("Host deployment is closed")?;
        let ticket = self.scope.resources.reserve().map_err(|e| e.to_string())?;
        let result = dispatch.recover(
            self.consumer.clone(),
            reference,
            self.scope.clone(),
            Some(ticket),
        )?;
        Ok(Box::pin(async move {
            result
                .await
                .map_err(|_| "Host deployment is closed".to_string())?
        }))
    }
    #[allow(clippy::too_many_arguments)]
    fn enqueue(
        &self,
        import: &str,
        provider: usize,
        method: String,
        input: Value,
        attachments: Attachments,
        scope: Scope,
        kind: CallKind,
    ) -> Result<()> {
        if self.scope.cancellation.is_cancelled() {
            return Err("Host request was revoked".into());
        }
        let binding = self
            .imports
            .get(import)
            .and_then(|providers| providers.get(provider))
            .ok_or("unknown request import provider")?
            .clone();
        let dispatch = self.dispatch.upgrade().ok_or("Host deployment is closed")?;
        // Reserve before the first descriptor trait call. A native preparation
        // panic belongs to this already accepted parent's cleanup facts.
        let ticket = self.scope.resources.reserve().map_err(|e| e.to_string())?;
        dispatch.enqueue(
            binding,
            method,
            input,
            scope,
            attachments.into(),
            Some(ticket),
            kind,
        )?;
        Ok(())
    }
}
