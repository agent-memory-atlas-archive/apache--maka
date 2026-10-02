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

//! Durable operation observations, never execution or recovery grants.
use crate::{
    composition::InstanceId,
    contract::{Contract, Interface, MethodMode},
    deployment::DeploymentRef,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryProtocol {
    JournalV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveryRef {
    pub source: DeploymentRef,
    pub namespace: InstanceId,
    pub export: String,
    pub method: String,
    pub operation: Uuid,
    pub fingerprint: String,
    pub contract: Contract,
    pub recovery: RecoveryProtocol,
}
impl RecoveryRef {
    pub fn validate(&self) -> Result<(), crate::Error> {
        self.source.validate().map_err(crate::Error::Composition)?;
        if self.namespace.0.is_empty()
            || self.namespace.0.len() > 64
            || self.namespace.0.iter().any(Uuid::is_nil)
            || self.operation.is_nil()
        {
            return Err(crate::Error::Composition(
                "invalid operation namespace or identity".into(),
            ));
        }
        crate::name(&self.export)?;
        crate::name(&self.method)?;
        if !crate::content_hash(&self.fingerprint) {
            return Err(crate::Error::Composition(
                "invalid operation fingerprint".into(),
            ));
        }
        self.contract.validate()?;
        let Interface::Service { methods } = &self.contract.interface else {
            return Err(crate::Error::Contract(
                "operation requires a service contract".into(),
            ));
        };
        if methods.get(&self.method).is_none_or(|method| {
            method.mode != MethodMode::Mutation || !method.resources.is_empty()
        }) {
            return Err(crate::Error::Contract(
                "operation requires a declared resource-free mutation method".into(),
            ));
        }
        Ok(())
    }
}

/// Unresolved means no durable result is known, including after process loss.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationOutcome {
    Unresolved,
    Returned { value: Value },
    Unknown { diagnostic: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationCleanup {
    Unconfirmed,
    Confirmed,
    Failed { diagnostic: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationObservation {
    pub reference: RecoveryRef,
    pub outcome: OperationOutcome,
    pub cleanup: OperationCleanup,
}
