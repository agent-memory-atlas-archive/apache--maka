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

//! Public application caller: an anchored authenticated session, no legacy operations.
use super::{
    Status, VERSION,
    protocol::{Connected, Opening, Request, Response, SessionControl},
};
use crate::{
    application::{
        Result,
        transport::{Call, Catalogue, Source, wire},
    },
    ipc::{self, LocalStream},
};
use maka_plugins::call::Attachments;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::io::{BufReader, ReadHalf, WriteHalf};
use uuid::Uuid;

pub type LocalCall = Call<BufReader<ReadHalf<LocalStream>>, WriteHalf<LocalStream>>;
pub struct Client {
    address: PathBuf,
    connected: Connected,
    anchor: wire::Reader<LocalStream>,
}
impl Client {
    pub async fn connect(address: &Path, epoch: Option<Uuid>) -> Result<Self> {
        let mut anchor = ipc::connect(address)
            .await
            .map_err(|error| error.to_string())?;
        wire::write(
            &mut anchor,
            &Opening {
                version: VERSION,
                epoch,
                request: Request::Connect,
            },
        )
        .await?;
        let mut anchor = wire::Reader::new(anchor);
        let Response::Connected(connected) = anchor.read().await? else {
            return Err("Host did not establish an application session".into());
        };
        if connected.version != VERSION
            || connected.epoch.is_nil()
            || connected.session.is_nil()
            || epoch.is_some_and(|epoch| connected.epoch != epoch)
        {
            return Err("Host protocol or incarnation changed".into());
        }
        connected.source.validate()?;
        connected.catalogue.validate()?;
        connected
            .catalogue
            .check_names(connected.consumers.keys())?;
        Ok(Self {
            address: address.into(),
            connected,
            anchor,
        })
    }
    pub fn catalogue(&self) -> &Catalogue {
        &self.connected.catalogue
    }
    pub fn epoch(&self) -> Uuid {
        self.connected.epoch
    }
    /// Open inert custody for one exact captured endpoint. A caller retains this
    /// Call through errors; reconnection never retries an unconfirmed mutation.
    pub async fn call(
        &self,
        endpoint: &str,
        method: String,
        input: Value,
        resources: Attachments,
    ) -> Result<LocalCall> {
        let selection = self.connected.catalogue.select(endpoint)?;
        let consumer = self
            .connected
            .consumers
            .get(endpoint)
            .ok_or("Host endpoint missing")?
            .clone();
        let stream = ipc::connect(&self.address)
            .await
            .map_err(|error| error.to_string())?;
        let (input_stream, mut output) = tokio::io::split(stream);
        wire::write(
            &mut output,
            &Opening {
                version: VERSION,
                epoch: Some(self.connected.epoch),
                request: Request::Call {
                    session: self.connected.session,
                },
            },
        )
        .await?;
        let mut reader = wire::Reader::new(input_stream);
        if !matches!(reader.read::<Response>().await?, Response::Accepted) {
            return Err("Host did not accept application channel".into());
        }
        Ok(Call::new(
            reader.inner,
            output,
            selection,
            Source {
                deployment: self.connected.source.clone(),
                consumer,
                mount: self.connected.session,
                request: Uuid::new_v4(),
            },
            method,
            input,
            resources,
        ))
    }
    pub async fn close(&mut self) -> Result<()> {
        wire::write(&mut self.anchor.inner, &SessionControl::Close).await?;
        match self.anchor.read::<Response>().await? {
            Response::Closed => Ok(()),
            _ => Err("Host session cleanup remains unconfirmed".into()),
        }
    }
    pub async fn status(address: &Path, epoch: Option<Uuid>) -> Result<Status> {
        match control(address, epoch, Request::Status).await? {
            Response::Status(status) => Ok(status),
            _ => Err("Host status response mismatch".into()),
        }
    }
    pub async fn retire(address: &Path, epoch: Uuid) -> Result<bool> {
        match control(address, Some(epoch), Request::Retire).await? {
            Response::Retired(result) => result,
            _ => Err("Host retirement response mismatch".into()),
        }
    }
}
async fn control(address: &Path, epoch: Option<Uuid>, request: Request) -> Result<Response> {
    let mut stream = ipc::connect(address)
        .await
        .map_err(|error| error.to_string())?;
    wire::write(
        &mut stream,
        &Opening {
            version: VERSION,
            epoch,
            request,
        },
    )
    .await?;
    wire::Reader::new(stream).read().await
}
