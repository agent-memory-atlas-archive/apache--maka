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

//! Physical pairing on private current-account IPC, not network authentication.
use super::{native, wire};
use crate::Result;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Hello {
    pub nonce: Uuid,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Accepted {
    pub nonce: Uuid,
}

/// Only the trusted launcher constructs this from its actual child bootstrap.
/// The address and nonce stay fixed; a lost Host is never retargeted by alias.
pub struct Connector {
    path: PathBuf,
    nonce: Uuid,
}
impl Connector {
    pub fn new(path: PathBuf, nonce: Uuid) -> Result<Self> {
        if nonce.is_nil() {
            return Err("private channel nonce must be nonnil".into());
        }
        Ok(Self { path, nonce })
    }
}
impl native::Connector for Connector {
    fn connect(&self, stop: CancellationToken) -> BoxFuture<'static, Result<native::Stream>> {
        let path = self.path.clone();
        let nonce = self.nonce;
        Box::pin(async move {
            // Before Open, no Application request or immutable resource can be
            // admitted. Dropped OS-opening/Hello custody remains within the
            // original embedding's tracked Server, which its child owner drains.
            let mut stream = tokio::select! {
                biased;
                () = stop.cancelled() => return Err("private opening cancelled".into()),
                result = maka_runtime_host::ipc::connect(&path) => result.map_err(|e| e.to_string())?,
            };
            let hello = Hello { nonce };
            tokio::select! {
                biased;
                () = stop.cancelled() => return Err("private opening cancelled".into()),
                result = wire::write(&mut stream, &hello) => result?,
            }
            let mut reader = wire::Reader::new(stream);
            let accepted: Accepted = tokio::select! {
                biased;
                () = stop.cancelled() => return Err("private opening cancelled".into()),
                result = reader.read_bounded(512) => result?,
            };
            if accepted.nonce != nonce {
                return Err("private channel belongs to a different Host incarnation".into());
            }
            Ok(Box::new(reader.inner) as native::Stream)
        })
    }
}
