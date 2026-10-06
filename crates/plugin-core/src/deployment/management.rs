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

//! Same-Root deployment management contracts; metadata never grants access.
use crate::{
    composition::{Input, InstanceId},
    contract::{Contract, Interface, Method, MethodMode, ResourceType, Transport},
    deployment::{Deployment, DeploymentRef, RemoteObservation},
    package::PackageRef,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeploymentRead {}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageInstall {
    #[schemars(range(min = 1, max = 9_007_199_254_740_990_u64))]
    pub expected_revision: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageInstalled {
    pub source: DeploymentRef,
    pub package: PackageRef,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeploymentReplace {
    #[schemars(range(min = 1, max = 9_007_199_254_740_990_u64))]
    pub expected_revision: u64,
    pub input: Input,
    #[schemars(length(max = 128))]
    pub remote_exports: Vec<RemoteObservation>,
    #[schemars(length(max = 128))]
    pub packages: Vec<PackageRef>,
}

/// A fixed committed source for continuation; no runtime or permission claim.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeploymentInstances {
    Start {},
    Continue {
        source: DeploymentRef,
        after: Option<InstanceId>,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagedInstance {
    pub instance: InstanceId,
    pub path: Vec<String>,
    pub package: PackageRef,
    pub structural: bool,
    pub disabled: bool,
    pub parent_disabled: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManagedInstancesPage {
    pub source: DeploymentRef,
    #[schemars(length(max = 8))]
    pub instances: Vec<ManagedInstance>,
    pub next: Option<InstanceId>,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstanceSetEnabled {
    #[schemars(range(min = 1, max = 9_007_199_254_740_990_u64))]
    pub expected_revision: u64,
    pub instance: InstanceId,
    pub enabled: bool,
}

/// Committed desired source and available immutable packages, not runtime readiness.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagedDeployment {
    pub source: Deployment,
    pub packages: Vec<PackageRef>,
}

/// A deterministic refusal is a recorded business result. Uncertain SQL/worker
/// failures remain in the original operation journal's Unknown outcome.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "ManagementResultFor{T}")]
pub enum ManagementResult<T> {
    Committed {
        value: T,
    },
    Conflict {
        #[serde(rename = "currentRevision")]
        current_revision: u64,
    },
    Rejected {
        reason: ManagementRejection,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ManagementRejection {
    InvalidBundle,
    InvalidSource,
    PackageUnavailable,
    ExecutionEnvironment,
    InstanceUnavailable,
    ParentDisabled,
}

pub fn package_bundle_type() -> ResourceType {
    ResourceType {
        name: "maka.plugin.PackageBundle".into(),
        version: 1,
    }
}

pub fn deployment_manager_contract() -> Contract {
    fn method<I: JsonSchema, O: JsonSchema>(mode: MethodMode) -> Method {
        Method {
            mode,
            input: serde_json::to_value(schemars::schema_for!(I)).expect("management input schema"),
            output: serde_json::to_value(schemars::schema_for!(O))
                .expect("management output schema"),
            resources: BTreeMap::new(),
        }
    }
    let mut install =
        method::<PackageInstall, ManagementResult<PackageInstalled>>(MethodMode::Mutation);
    install
        .resources
        .insert("package".into(), package_bundle_type());
    Contract {
        name: "maka.application.DeploymentManager".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([
                (
                    "read".into(),
                    method::<DeploymentRead, ManagedDeployment>(MethodMode::Query),
                ),
                ("install".into(), install),
                (
                    "instances".into(),
                    method::<DeploymentInstances, ManagedInstancesPage>(MethodMode::Query),
                ),
                (
                    "setEnabled".into(),
                    method::<InstanceSetEnabled, ManagementResult<DeploymentRef>>(
                        MethodMode::Mutation,
                    ),
                ),
                (
                    "replace".into(),
                    method::<DeploymentReplace, ManagementResult<DeploymentRef>>(
                        MethodMode::Mutation,
                    ),
                ),
            ]),
        },
    }
}
