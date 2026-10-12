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

//! Native capability ports, scoped resource ownership, and immutable application packages.

pub mod authorization;
pub mod background;
pub mod call;
pub mod client_capability;
pub mod computer;
pub mod credentials;
pub mod execution;
pub mod executor;
pub mod filesystem;
pub mod http;
pub mod llm;
pub mod model;
pub mod package;
pub mod permissions;
pub mod preferences;
pub mod pricing;
pub mod process;
pub mod provider;
pub mod remote;
pub mod revision;
pub mod session;
pub mod storage;
pub mod terminal;
pub mod usage;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("capability scope is retired")]
    Retired,
    #[error("invalid capability or package data: {0}")]
    Invalid(String),
}

pub(crate) fn name(value: &str) -> Result<(), Error> {
    if value.is_empty()
        || value.len() > 256
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(Error::Invalid(format!("invalid identifier: {value:?}")));
    }
    Ok(())
}

/// Validate a stable package or entry identifier.
pub fn identifier(value: &str) -> Result<(), Error> {
    if value.len() > 128
        || !value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        || value.split(['.', '_', ':', '-']).any(|part| {
            part.is_empty()
                || !part
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
    {
        return Err(Error::Invalid(format!(
            "invalid plugin identifier: {value:?}"
        )));
    }
    Ok(())
}
