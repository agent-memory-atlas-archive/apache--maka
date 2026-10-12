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

use super::super::Result;
use super::super::transport::Catalogue;
use maka_plugin_core::{composition::InstanceId, deployment::DeploymentRef};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const VERSION: u32 = 1;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Opening {
    pub version: u32,
    pub epoch: Option<Uuid>,
    pub request: Request,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Request {
    Connect,
    Call { session: Uuid },
    Status,
    Retire,
}
#[derive(Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(super) enum Response {
    Connected(Connected),
    Accepted,
    Status(Status),
    Retired(Result<bool>),
    Closed,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum SessionControl {
    Close,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Connected {
    pub version: u32,
    pub epoch: Uuid,
    pub session: Uuid,
    pub source: DeploymentRef,
    pub consumers: BTreeMap<String, InstanceId>,
    pub catalogue: Catalogue,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub epoch: Uuid,
    pub source: DeploymentRef,
    pub closing: bool,
    pub closed: bool,
    pub cleanup_error: Option<String>,
    pub sessions: usize,
    /// Observation only; idle retirement also fences native background producers.
    pub active_calls: usize,
}
