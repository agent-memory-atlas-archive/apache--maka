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

//! Owned image presentation. Source metadata correlates content, never grants reads.
use crate::appearance::Color;
use maka_plugin_core::contract::{Contract, Interface, Transport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageDescriptor {
    #[schemars(length(min = 1, max = 128))]
    pub key: String,
    #[schemars(regex(pattern = "^sha256:[a-f0-9]{64}$"))]
    pub digest: String,
    #[schemars(regex(pattern = "^image/(png|jpeg|gif|webp|avif)$"))]
    pub mime_type: String,
    #[schemars(range(min = 1, max = 5242880))]
    pub total_bytes: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImagePacket {
    #[schemars(length(min = 1, max = 128))]
    pub owner: String,
    #[schemars(length(min = 1, max = 128))]
    pub source: String,
    #[schemars(range(min = 1, max = 9007199254740991u64))]
    pub sequence: u64,
    #[schemars(range(max = 5242880))]
    pub offset: u64,
    #[schemars(
        length(min = 4, max = 43692),
        regex(pattern = "^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$")
    )]
    pub data: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImageFit {
    Contain,
    Cover,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageProps {
    pub source: Option<ImageDescriptor>,
    pub packet: Option<ImagePacket>,
    #[schemars(length(max = 4096))]
    pub alt: String,
    pub fit: ImageFit,
    #[schemars(range(min = 1, max = 8192))]
    pub height: f32,
    pub background: Color,
    pub border: Color,
    #[schemars(range(min = 0, max = 128))]
    pub radius: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImagePhase {
    Empty,
    Receiving,
    Decoding,
    Ready,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImageError {
    BudgetExhausted,
    LimitExceeded,
    SourceConflict,
    PacketRejected,
    DigestMismatch,
    UnsupportedFormat,
    InvalidImage,
    DecodeFailed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageReceipt {
    pub owner: String,
    pub source: String,
    pub sequence: u64,
    pub accepted: bool,
    pub error: Option<ImageError>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageObservation {
    pub owner: String,
    pub revision: u64,
    pub source: Option<String>,
    pub received: u64,
    pub phase: ImagePhase,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub error: Option<ImageError>,
    pub receipt: Option<ImageReceipt>,
}
pub fn observation_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(ImageObservation)).expect("image observation schema")
}
pub fn image_contract() -> Contract {
    Contract {
        name: "maka.ui.Image".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Component {
            props: serde_json::to_value(schemars::schema_for!(ImageProps)).expect("image props"),
            events: BTreeMap::from([("observed".into(), observation_schema())]),
            captures: BTreeMap::new(),
        },
    }
}

impl From<crate::budget::BudgetExhausted> for ImageError {
    fn from(_: crate::budget::BudgetExhausted) -> Self {
        Self::BudgetExhausted
    }
}
