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

//! Caller-side physical custody. Errors never imply remote cleanup.
use super::wire::{self, Control, Open, Reader, Reply, Source};
use crate::Result;
use maka_event_log::applications::intents::RecordedIntent;
use maka_plugin_core::operation::RecoveryRef;
use maka_plugins::call::{Attachments, ResourceEncoding};
use maka_runtime_host::application::{RESOURCE_CHUNK_LIMIT, ResourceDescriptor};
use serde_json::Value;
use std::collections::BTreeMap;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

/// Keep this original owner after an I/O error. It retains the actual caller
/// SQL token and local immutable inputs until this invocation's cleanup ack.
/// A recovered business receipt does not close this request or replay it.
pub struct Call<R, W> {
    channel: Option<Channel<R, W>>,
    open: Option<Open>,
    source: Source,
    resources: Attachments,
    reference: Option<RecoveryRef>,
    recorded: Option<RecordedIntent>,
    phase: Phase,
}
struct Channel<R, W> {
    reader: Reader<R>,
    writer: W,
}
#[derive(PartialEq, Eq)]
enum Phase {
    New,
    Open,
    Streaming,
    Prepared,
    Dispatched,
    Closed,
    Lost,
}
impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Call<R, W> {
    /// Constructs inert custody. The connector, not Source, authenticates the
    /// actual physical endpoint; NativeService adapters derive Source from the
    /// checked ServiceRequest rather than from guest-provided JSON.
    pub fn new(
        reader: R,
        writer: W,
        endpoint: String,
        source: Source,
        method: String,
        input: Value,
        resources: Attachments,
    ) -> Self {
        let open = Open {
            endpoint,
            source: source.clone(),
            method,
            input,
        };
        Self {
            channel: Some(Channel {
                reader: Reader::new(reader),
                writer,
            }),
            open: Some(open),
            source,
            resources,
            reference: None,
            recorded: None,
            phase: Phase::New,
        }
    }
    pub fn reference(&self) -> Option<&RecoveryRef> {
        self.reference.as_ref()
    }
    pub fn cleanup_confirmed(&self) -> bool {
        self.phase == Phase::Closed
    }
    pub fn holds_caller_intent(&self) -> bool {
        self.recorded.is_some()
    }
    /// Disconnects only the physical channel, retaining this invocation's
    /// unconfirmed token and immutable inputs. Remote EOF requests cancellation;
    /// it is not an acknowledgement of remote execution cleanup.
    pub fn disconnect(&mut self) {
        if self.phase != Phase::Closed {
            self.phase = Phase::Lost;
        }
        if let Some(Channel { reader, writer }) = self.channel.take() {
            // Drop both halves even if an embedding's destructor panics. Keep
            // original custody outside these unwind boundaries.
            let input = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(reader)));
            let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(writer)));
            if let Err(panic) = input.and(output) {
                std::panic::resume_unwind(panic);
            }
        }
    }
    /// Transfer descriptor observations, then chunks only after original Host
    /// reservation. No whole-payload clone or socket-backed ReadResource exists.
    pub async fn start(&mut self) -> Result<()> {
        if self.phase != Phase::New {
            return Err("remote call is not new".into());
        }
        self.phase = Phase::Lost;
        let open = self.open.take().expect("new request");
        let mut descriptors = BTreeMap::new();
        for (slot, resource) in &self.resources {
            descriptors.insert(
                slot.clone(),
                ResourceDescriptor {
                    encoding: resource.encoding(),
                    resource: resource.resource_type(),
                    length: resource.bytes() as u64,
                    metadata: resource.metadata(),
                },
            );
        }
        let channel = self.channel.as_mut().expect("new channel");
        wire::write(&mut channel.writer, &open).await?;
        wire::write(&mut channel.writer, &descriptors).await?;
        let reply = channel.reader.read::<Reply>().await?;
        if !matches!(reply, Reply::Ready) {
            self.accept_reply(&reply)?;
            return match reply {
                Reply::Aborted { reason, .. } => Err(reason),
                _ => Err("remote call did not reserve resources".into()),
            };
        }
        let channel = self.channel.as_mut().expect("open channel");
        for (slot, descriptor) in descriptors {
            let resource = &self.resources[&slot];
            let mut offset = 0usize;
            let length =
                usize::try_from(descriptor.length).map_err(|_| "invalid remote resource length")?;
            while offset < length {
                let mut end = length.min(offset + RESOURCE_CHUNK_LIMIT);
                let mut value = resource.read(offset..end);
                // Native UTF-8 resources reject interior byte boundaries. A
                // transport chunk may end at the preceding scalar boundary.
                if descriptor.encoding == ResourceEncoding::Utf8 {
                    for _ in 0..3 {
                        if value.is_ok() {
                            break;
                        }
                        end -= 1;
                        if end <= offset {
                            break;
                        }
                        value = resource.read(offset..end);
                    }
                }
                let value = value?;
                if value.len() != end - offset || value.is_empty() {
                    return Err("resource returned a different byte range".into());
                }
                channel
                    .writer
                    .write_u32(value.len() as u32)
                    .await
                    .map_err(|e| e.to_string())?;
                channel
                    .writer
                    .write_all(&value)
                    .await
                    .map_err(|e| e.to_string())?;
                offset = end;
            }
        }
        channel.writer.flush().await.map_err(|e| e.to_string())?;
        self.phase = Phase::Open;
        Ok(())
    }
    /// Query completion or original mutation preparation. This is an observer;
    /// dropping its future does not establish cleanup or permit request replay.
    pub async fn receive(&mut self) -> Result<Reply> {
        if self.phase == Phase::New || self.phase == Phase::Closed || self.phase == Phase::Lost {
            return Err("remote call cannot receive in this phase".into());
        }
        let channel = self.channel.as_mut().expect("active channel");
        let reply = match channel.reader.read::<Reply>().await {
            Ok(reply) => reply,
            Err(e) => {
                self.phase = Phase::Lost;
                return Err(e);
            }
        };
        if let Err(e) = self.accept_reply(&reply) {
            self.phase = Phase::Lost;
            return Err(e);
        }
        Ok(reply)
    }
    /// One demand successor; no background socket reader prefetches items.
    pub async fn pull(&mut self) -> Result<Reply> {
        if self.phase != Phase::Streaming {
            return Err("remote call cannot pull in this phase".into());
        }
        self.phase = Phase::Lost;
        let channel = self.channel.as_mut().expect("streaming channel");
        if let Err(e) = wire::write(&mut channel.writer, &Control::Pull).await {
            self.phase = Phase::Lost;
            return Err(e);
        }
        self.phase = Phase::Streaming;
        self.receive().await
    }
    pub async fn commit(&mut self, recorded: RecordedIntent) -> Result<Reply> {
        if self.phase != Phase::Prepared || self.recorded.is_some() {
            return Err("remote mutation is not awaiting caller confirmation".into());
        }
        // Retain even a mismatching confirmed token through preparation abort.
        // The caller must cancel/drain this same original owner after rejection.
        self.recorded = Some(recorded);
        let intent = self.recorded.as_ref().unwrap().intent();
        if intent.caller != self.source.deployment
            || intent.consumer != self.source.consumer
            || self.reference.as_ref() != Some(&intent.reference)
        {
            return Err("caller token differs from the original remote preparation".into());
        }
        let control = Control::Commit {
            intent: Box::new(intent.clone()),
        };
        // A cancelled partial write poisons this channel. No later frame may
        // append to it, while the actual token remains in this owner.
        self.phase = Phase::Lost;
        let channel = self.channel.as_mut().expect("prepared channel");
        wire::write(&mut channel.writer, &control).await?;
        self.phase = Phase::Dispatched;
        self.receive().await
    }
    /// Cancel asks the same owner to settle. EOF/error retains unconfirmed custody.
    pub async fn cancel(&mut self) -> Result<Reply> {
        if matches!(self.phase, Phase::New | Phase::Closed | Phase::Lost) {
            return Err("remote call cannot cancel in this phase".into());
        }
        let phase = std::mem::replace(&mut self.phase, Phase::Lost);
        let channel = self.channel.as_mut().expect("active channel");
        if let Err(e) = wire::write(&mut channel.writer, &Control::Cancel).await {
            self.phase = Phase::Lost;
            return Err(e);
        }
        self.phase = phase;
        self.receive().await
    }
    fn accept_reply(&mut self, reply: &Reply) -> Result<()> {
        match reply {
            Reply::Subscribed if self.phase == Phase::Open => self.phase = Phase::Streaming,
            Reply::Subscribed => return Err("unexpected subscription acceptance".into()),
            Reply::Item { .. } if self.phase != Phase::Streaming => {
                return Err("unexpected subscription item".into());
            }
            Reply::Finished { .. } if !matches!(self.phase, Phase::Open | Phase::Streaming) => {
                return Err("unexpected query completion".into());
            }
            Reply::Mutation { .. } if self.phase != Phase::Dispatched => {
                return Err("unexpected mutation completion".into());
            }
            Reply::Prepared { reference } => {
                reference.validate().map_err(|e| e.to_string())?;
                if self.phase != Phase::Open || reference.operation != self.source.request {
                    self.phase = Phase::Lost;
                    return Err("remote preparation changed original operation".into());
                }
                self.reference = Some(*reference.clone());
                self.phase = Phase::Prepared;
            }
            Reply::Mutation { reply, .. } if self.reference.as_ref() != Some(&reply.reference) => {
                self.phase = Phase::Lost;
                return Err("remote completion changed original reference".into());
            }
            Reply::Ready => {
                self.phase = Phase::Lost;
                return Err("unexpected resource reservation reply".into());
            }
            _ => {}
        }
        if let Some(cleanup) = reply.cleanup() {
            if cleanup.is_ok() {
                self.recorded.take();
                self.resources.clear();
                self.phase = Phase::Closed;
            } else {
                self.phase = Phase::Lost;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context, Poll},
    };
    struct Partial {
        written: Arc<Mutex<Vec<u8>>>,
        released: Arc<std::sync::atomic::AtomicBool>,
    }
    impl AsyncWrite for Partial {
        fn poll_write(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            let mut written = self.written.lock().unwrap();
            if !written.is_empty() {
                if !self.released.load(std::sync::atomic::Ordering::SeqCst) {
                    return Poll::Pending;
                }
                written.extend_from_slice(bytes);
                return Poll::Ready(Ok(bytes.len()));
            }
            written.extend_from_slice(&bytes[..2]);
            Poll::Ready(Ok(2))
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
    fn source() -> Source {
        Source {
            deployment: maka_plugin_core::deployment::DeploymentRef {
                state_domain: "root".into(),
                deployment: uuid::Uuid::new_v4(),
                revision: 1,
            },
            consumer: maka_plugin_core::composition::InstanceId(vec![uuid::Uuid::new_v4()]),
            request: uuid::Uuid::new_v4(),
        }
    }
    #[tokio::test]
    async fn dropping_a_partial_control_write_cannot_append_a_second_frame() {
        for streaming in [true, false] {
            let written = Arc::new(Mutex::new(Vec::new()));
            let released = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let mut call = Call::new(
                tokio::io::empty(),
                Partial {
                    written: written.clone(),
                    released: released.clone(),
                },
                "endpoint".into(),
                source(),
                "method".into(),
                Value::Null,
                Attachments::new(),
            );
            call.phase = if streaming {
                Phase::Streaming
            } else {
                Phase::Prepared
            };
            if streaming {
                let mut future = Box::pin(call.pull());
                assert!(futures_util::poll!(&mut future).is_pending());
                drop(future);
            } else {
                let mut future = Box::pin(call.cancel());
                assert!(futures_util::poll!(&mut future).is_pending());
                drop(future);
            }
            assert_eq!(written.lock().unwrap().len(), 2);
            released.store(true, std::sync::atomic::Ordering::SeqCst);
            assert!(call.cancel().await.is_err());
            assert!(call.receive().await.is_err());
            assert_eq!(written.lock().unwrap().len(), 2);
            assert!(!call.cleanup_confirmed());
        }
    }
}
