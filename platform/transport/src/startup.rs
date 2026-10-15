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

//! A finite trusted CLI helper holds startup residency until peer admission.
use crate::{HostEndpoint, Result};
use maka_host_runtime::application::transport::{NativeGrant, wire};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::{Child, ChildStdin, Command};
use uuid::Uuid;

pub const REQUEST_LIMIT: usize = 512 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub root: PathBuf,
    pub state_domain: Option<String>,
    pub deployment: Uuid,
    pub endpoints: BTreeMap<String, HostEndpoint>,
    pub native: Vec<NativeGrant>,
}
impl Request {
    pub fn validate(&self) -> Result<()> {
        if !self.root.is_absolute()
            || self.deployment.is_nil()
            || self
                .state_domain
                .as_ref()
                .is_some_and(|domain| !crate::connections::valid_domain(domain))
            || self.endpoints.is_empty()
            || self.endpoints.len() > 128
            || self.native.len() > 128
        {
            return Err("invalid local Host startup selection".into());
        }
        for (name, endpoint) in &self.endpoints {
            if name.is_empty() || name.len() > 256 {
                return Err("invalid Host endpoint name".into());
            }
            endpoint.validate()?;
        }
        for grant in &self.native {
            grant.validate()?;
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ready {
    pub root_id: String,
    pub epoch: Uuid,
}
/// Owns only a finite helper's startup pipe, never a Host process or UI calls.
/// Dropping it releases startup residency after the real peer is connected.
pub(crate) struct Anchor {
    _child: Child,
    _input: ChildStdin,
}
pub(crate) async fn launch(executable: &Path, request: Request) -> Result<(Anchor, Ready)> {
    request.validate()?;
    wire::encode_bounded(&request, REQUEST_LIMIT)?;
    let mut child = Command::new(executable)
        .args(["host", "ensure"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("local Host startup helper: {error}"))?;
    let mut input = child.stdin.take().expect("piped startup input");
    let output = child.stdout.take().expect("piped startup output");
    wire::write_bounded(&mut input, &request, REQUEST_LIMIT).await?;
    let ready: Ready = wire::Reader::new(output).read_bounded(4096).await?;
    if !crate::connections::valid_domain(&ready.root_id)
        || ready.epoch.is_nil()
        || request
            .state_domain
            .as_ref()
            .is_some_and(|expected| *expected != ready.root_id)
    {
        return Err("local Host startup returned a foreign identity".into());
    }
    Ok((
        Anchor {
            _child: child,
            _input: input,
        },
        ready,
    ))
}
