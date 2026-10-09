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

//! Test-only physical relay. It forwards original bytes until one complete
//! real mutation reply is consumed, then drops that invocation's channel.
use maka_application_bridge::transport::Reply;
use maka_plugin_core::operation::OperationReply;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

pub(super) struct Lost {
    pub reply: OperationReply,
    pub cleanup: Result<(), String>,
}

#[derive(Default)]
pub(super) struct Loss {
    armed: Mutex<Option<String>>,
    lost: Mutex<Option<Lost>>,
    tasks: tokio_util::task::TaskTracker,
}
impl Loss {
    pub(super) fn arm(&self, method: &str) {
        assert!(self.armed.lock().unwrap().replace(method.into()).is_none());
        assert!(self.lost.lock().unwrap().is_none());
    }
    pub(super) fn take(&self) -> Option<Lost> {
        self.lost.lock().unwrap().take()
    }
    pub(super) async fn drain(&self) {
        self.tasks.close();
        self.tasks.wait().await;
    }
    pub(super) fn channel(self: &Arc<Self>, remote: DuplexStream) -> DuplexStream {
        let (client, relay) = tokio::io::duplex(64 * 1024);
        let own = self.clone();
        self.tasks.spawn(async move {
            let (mut input, mut output) = tokio::io::split(relay);
            let (mut replies, mut requests) = tokio::io::split(remote);
            let forward = async {
                loop {
                    let length = replies.read_u32().await? as usize;
                    assert!((1..=2 * 1024 * 1024).contains(&length));
                    let mut frame = vec![0; length];
                    replies.read_exact(&mut frame).await?;
                    if let Ok(Reply::Mutation { reply, cleanup }) = serde_json::from_slice(&frame) {
                        let mut armed = own.armed.lock().unwrap();
                        if armed.as_deref() == Some(&reply.reference.method) {
                            armed.take();
                            *own.lost.lock().unwrap() = Some(Lost {
                                reply: *reply,
                                cleanup,
                            });
                            return Ok::<(), std::io::Error>(());
                        }
                    }
                    output.write_u32(length as u32).await?;
                    output.write_all(&frame).await?;
                    output.flush().await?;
                }
            };
            tokio::select! {
                _ = forward => {},
                _ = tokio::io::copy(&mut input, &mut requests) => {},
            }
        });
        client
    }
}
