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

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Error;

/// Public contract definitions are package data, not live registration records.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contract {
    pub name: String,
    pub version: u32,
    pub transport: Transport,
    pub interface: Interface,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Local,
    Remote,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Interface {
    Service {
        methods: BTreeMap<String, Method>,
    },
    Component {
        props: Value,
        events: BTreeMap<String, Value>,
    },
    Value {
        schema: Value,
    },
    Resource {
        resource: String,
        schema: Value,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Method {
    pub mode: MethodMode,
    pub input: Value,
    pub output: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MethodMode {
    Query,
    Mutation,
    Subscription,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Import {
    pub contract: Contract,
    pub cardinality: Cardinality,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Cardinality {
    One,
    Optional,
    Many { min: usize, max: usize },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Export {
    pub contract: Contract,
    pub availability: Availability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Required,
    Optional,
}

impl Contract {
    pub fn validate(&self) -> Result<(), Error> {
        crate::name(&self.name)?;
        if self.version == 0 {
            return Err(Error::Declaration(
                "contract version must be positive".into(),
            ));
        }
        match &self.interface {
            Interface::Service { methods } => {
                for (name, method) in methods {
                    crate::name(name)?;
                    crate::schema(&method.input)?;
                    crate::schema(&method.output)?;
                }
            }
            Interface::Component { props, events } => {
                crate::schema(props)?;
                for (name, schema) in events {
                    crate::name(name)?;
                    crate::schema(schema)?;
                }
            }
            Interface::Value { schema } => crate::schema(schema)?,
            Interface::Resource { resource, schema } => {
                crate::name(resource)?;
                crate::schema(schema)?;
            }
        }
        Ok(())
    }

    /// Conservative compatibility: nominal identity AND the full public shape.
    /// Version numbers alone cannot admit a changed declaration.
    pub fn accepts(&self, provider: &Self) -> bool {
        self == provider
    }
}

impl Cardinality {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if let Self::Many { min, max } = self
            && (min > max || *max > 4096)
        {
            return Err(Error::Declaration("invalid import cardinality".into()));
        }
        Ok(())
    }

    pub(crate) fn accepts(&self, total: usize, guaranteed: usize) -> bool {
        match *self {
            Self::One => total == 1 && guaranteed == 1,
            Self::Optional => total <= 1,
            Self::Many { min, max } => guaranteed >= min && total <= max,
        }
    }
}
