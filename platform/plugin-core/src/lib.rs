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

//! Shared plugin declarations and composition planning. No business or platform policy.

pub mod composition;
pub mod contract;
pub mod deployment;
pub mod json;
pub mod operation;
pub mod package;
pub mod plan;
pub mod runtime;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid plugin declaration: {0}")]
    Declaration(String),
    #[error("invalid application composition: {0}")]
    Composition(String),
    #[error("capability contract mismatch: {0}")]
    Contract(String),
    #[error("plugin capability or activation is retired")]
    Retired,
    #[error("plugin capability is not ready: {0}")]
    NotReady(String),
    #[error("invalid plugin lifecycle transition: {0}")]
    Lifecycle(&'static str),
    #[error("plugin cleanup is pending")]
    CleanupPending,
    #[error("plugin cleanup is unconfirmed: {0:?}")]
    CleanupFailed(Vec<String>),
}

pub(crate) fn name(value: &str) -> Result<(), Error> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return Err(Error::Declaration(format!("invalid name: {value:?}")));
    }
    Ok(())
}

pub(crate) fn content_hash(value: &str) -> bool {
    value.strip_prefix("sha256-").is_some_and(|hash| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

pub(crate) fn schema(value: &serde_json::Value) -> Result<(), Error> {
    // Package imports repeat the same immutable public schemas. Cache only a
    // successful schema check, keyed by the complete bytes, never by contract
    // name/version. Configuration values are still validated separately.
    // JSON Schema is built without external file/network resolution features.
    #[derive(Default)]
    struct Checked {
        schemas: std::collections::HashSet<String>,
        bytes: usize,
    }
    static CHECKED: std::sync::OnceLock<std::sync::Mutex<Checked>> = std::sync::OnceLock::new();
    let checked = CHECKED.get_or_init(Default::default);
    let key = serde_json::to_string(value).expect("JSON schema value");
    if checked.lock().expect("schema cache").schemas.contains(&key) {
        return Ok(());
    }
    jsonschema::validator_for(value)
        .map_err(|error| Error::Declaration(format!("invalid schema: {error}")))?;
    const MAX_BYTES: usize = 4 * 1024 * 1024;
    if key.len() <= MAX_BYTES {
        let mut checked = checked.lock().expect("schema cache");
        if checked.bytes + key.len() > MAX_BYTES || checked.schemas.len() >= 1024 {
            checked.schemas.clear();
            checked.bytes = 0;
        }
        let size = key.len();
        if checked.schemas.insert(key) {
            checked.bytes += size;
        }
    }
    Ok(())
}

pub(crate) fn validate_value(
    schema: &serde_json::Value,
    value: &serde_json::Value,
) -> Result<(), Error> {
    jsonschema::validator_for(schema)
        .map_err(|error| Error::Declaration(format!("invalid schema: {error}")))?
        .validate(value)
        .map_err(|error| Error::Composition(format!("configuration: {error}")))
}

pub mod scope;
pub mod wire_schema;

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
        return Err(Error::Declaration(format!(
            "invalid plugin identifier: {value:?}"
        )));
    }
    Ok(())
}
