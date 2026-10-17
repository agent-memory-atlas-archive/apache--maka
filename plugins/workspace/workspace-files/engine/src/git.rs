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

//! Read-only Git comparisons within the Session's original workspace.
use crate::worktree::Binding;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use gix::bstr::ByteSlice;
use maka_agent_facts::execution::DirectoryIdentity;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

mod diff;
pub(crate) mod image;
mod repository;
mod scan;
#[cfg(test)]
mod tests;

const MAX_ENTRIES: usize = 100_000;
const MAX_METADATA: usize = 50 * 1024 * 1024;
const MAX_CONTROL: usize = 256 * 1024;
const MAX_DIFF_INPUT: usize = 1024 * 1024;
const MAX_OUTPUT: usize = 512 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitPath {
    #[schemars(length(max = 5464))]
    pub bytes_base64: String,
}
impl GitPath {
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let path = Self {
            bytes_base64: STANDARD.encode(bytes),
        };
        path.validate()?;
        Ok(path)
    }
    pub fn validate(&self) -> io::Result<()> {
        self.bytes().map(|_| ())
    }
    fn bytes(&self) -> io::Result<Vec<u8>> {
        if self.bytes_base64.len() > 5464 {
            return Err(failure("git_invalid_path"));
        }
        let bytes = STANDARD
            .decode(&self.bytes_base64)
            .map_err(|_| failure("git_invalid_path"))?;
        if bytes.is_empty()
            || bytes.len() > 4096
            || bytes.contains(&0)
            || bytes
                .split(|b| *b == b'/')
                .any(|p| p.is_empty() || p == b"." || p == b".." || p.eq_ignore_ascii_case(b".git"))
            || STANDARD.encode(&bytes) != self.bytes_base64
        {
            return Err(failure("git_invalid_path"));
        }
        #[cfg(windows)]
        if bytes.contains(&b'\\') {
            return Err(failure("git_invalid_path"));
        }
        let path =
            gix::path::try_from_bstr(bytes.as_bstr()).map_err(|_| failure("git_invalid_path"))?;
        if path.is_absolute()
            || !path
                .components()
                .all(|p| matches!(p, std::path::Component::Normal(_)))
        {
            return Err(failure("git_invalid_path"));
        }
        Ok(bytes)
    }
}
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GitArea {
    Staged,
    Worktree,
    Untracked,
}
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GitChangeKind {
    Added,
    Modified,
    Deleted,
    TypeChanged,
    Conflicted,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitStatusEntry {
    pub area: GitArea,
    pub path: GitPath,
    pub kind: GitChangeKind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitStatus {
    #[schemars(with = "maka_plugin_core::wire_schema::RequiredNullable<String>")]
    pub head: Option<String>,
    pub entries: Vec<GitStatusEntry>,
    pub truncated: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GitDiff {
    Text { patch: String },
    Binary,
    TooLarge,
    Unchanged,
    Unsupported,
}

/// Blocking work: the caller retains its resource ticket and joins on cancellation.
pub fn status(
    workspace: &Path,
    identity: &DirectoryIdentity,
    private_root: &Path,
    binding: Option<&Binding>,
    cancel: Arc<AtomicBool>,
) -> io::Result<Option<GitStatus>> {
    expose((|| {
        let Some(repo) =
            repository::Repository::open(workspace, identity, private_root, binding, cancel)?
        else {
            return Ok(None);
        };
        let result = repo.status()?;
        repo.finish()?;
        Ok(Some(result))
    })())
}
pub fn diff(
    workspace: &Path,
    identity: &DirectoryIdentity,
    private_root: &Path,
    binding: Option<&Binding>,
    area: GitArea,
    path: &GitPath,
    cancel: Arc<AtomicBool>,
) -> io::Result<Option<GitDiff>> {
    expose((|| {
        let bytes = path.bytes()?;
        let Some(repo) =
            repository::Repository::open(workspace, identity, private_root, binding, cancel)?
        else {
            return Ok(None);
        };
        let result = diff::read(&repo, area, &bytes)?;
        repo.finish()?;
        Ok(Some(result))
    })())
}

#[derive(Debug)]
struct Failure(&'static str);
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Failure {}
fn failure(code: &'static str) -> io::Error {
    io::Error::other(Failure(code))
}
fn expose<T>(result: io::Result<T>) -> io::Result<T> {
    result.map_err(|e| {
        if e.get_ref().is_some_and(|v| v.is::<Failure>()) {
            e
        } else {
            failure("git_unavailable")
        }
    })
}
fn interrupted(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(failure("git_cancelled"))
    } else {
        Ok(())
    }
}
