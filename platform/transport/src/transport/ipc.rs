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

//! Fixed physical channels for an authenticated independent Host session.
use super::native;
use crate::Result;
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

/// Physical channels for one independently owned, already authenticated Host
/// session. This snapshot never retains the anchor or retargets a restarted Host.
pub struct Connector {
    opener: maka_host_runtime::application::host::CallChannelOpener,
}
impl Connector {
    pub fn new(opener: maka_host_runtime::application::host::CallChannelOpener) -> Self {
        Self { opener }
    }
}
impl native::Connector for Connector {
    fn connect(&self, stop: CancellationToken) -> BoxFuture<'static, Result<native::Stream>> {
        let opener = self.opener.clone();
        Box::pin(async move {
            // No business Open or resources have been sent. Cancellation drops
            // only inert IPC; the independent Host retains its own channel tasks.
            let stream = tokio::select! {
                biased;
                () = stop.cancelled() => return Err("public opening cancelled".into()),
                stream = opener.open() => stream?,
            };
            Ok(Box::new(stream) as native::Stream)
        })
    }
}
