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

//! Read-only facts from actual captured Host registrations.
use super::{Endpoints, wire};
use crate::Result;
use maka_plugin_core::{
    composition::InstanceId,
    contract::{Contract, Interface, Transport},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tokio::io::{AsyncRead, AsyncWrite, BufReader};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Description {
    pub provider: InstanceId,
    pub export: String,
    pub contract: Contract,
    /// None is a checked declaration without a current executable registration.
    pub registration_epoch: Option<Uuid>,
}
/// This catalogue is received over the embedding's authenticated bootstrap.
/// It grants no admission and never constructs a Host Binding from metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    endpoints: BTreeMap<String, Description>,
}
#[derive(Clone, Debug)]
pub struct Selection {
    pub(super) endpoint: String,
    pub(super) description: Description,
}
impl Selection {
    pub fn description(&self) -> &Description {
        &self.description
    }
}
impl Catalogue {
    pub(crate) fn captured(endpoints: &Endpoints) -> Self {
        Self {
            endpoints: endpoints.descriptions(),
        }
    }
    pub(crate) fn epochs(&self) -> BTreeMap<String, Option<Uuid>> {
        self.endpoints
            .iter()
            .map(|(name, description)| (name.clone(), description.registration_epoch))
            .collect()
    }
    pub(crate) fn check_names<'a>(&self, names: impl Iterator<Item = &'a String>) -> Result<()> {
        if !self.endpoints.keys().eq(names) {
            return Err("Host endpoint catalogue differs from configured selections".into());
        }
        Ok(())
    }
    pub fn describe(&self, endpoint: &str) -> Result<&Description> {
        self.endpoints
            .get(endpoint)
            .ok_or("Host endpoint not found".into())
    }
    pub fn select(&self, endpoint: &str) -> Result<Selection> {
        let description = self.describe(endpoint)?.clone();
        if description.registration_epoch.is_none() {
            return Err("Host endpoint is unavailable".into());
        }
        Ok(Selection {
            endpoint: endpoint.into(),
            description,
        })
    }
    /// Preserve buffered bytes from the exact same physical bootstrap stream.
    /// Receiving metadata does not authorize calls or retarget an old selection.
    pub async fn read<R: AsyncRead + Unpin>(reader: R) -> Result<(Self, BufReader<R>)> {
        let mut reader = wire::Reader::new(reader);
        let catalogue: Self = reader.read().await?;
        catalogue.validate()?;
        Ok((catalogue, reader.inner))
    }
    pub(super) fn validate(&self) -> Result<()> {
        if self.endpoints.is_empty() || self.endpoints.len() > 128 {
            return Err("invalid Host endpoint catalogue".into());
        }
        for (name, description) in &self.endpoints {
            if name.is_empty()
                || name.len() > 256
                || description
                    .registration_epoch
                    .is_some_and(|epoch| epoch.is_nil())
                || description.provider.0.is_empty()
                || description.provider.0.len() > 64
                || description.provider.0.iter().any(Uuid::is_nil)
                || description.export.is_empty()
                || description.export.len() > 256
                || description.contract.transport != Transport::Remote
                || !matches!(description.contract.interface, Interface::Service { .. })
            {
                return Err("invalid Host endpoint description".into());
            }
            description.contract.validate().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    /// Only the trusted embedding publishes the actual captured catalogue.
    pub async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<()> {
        wire::write(writer, self).await
    }
}
