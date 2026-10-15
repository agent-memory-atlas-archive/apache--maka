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

//! Skill-local text captured while discovery's read capabilities are alive.
use crate::{DiscoveryFailure, ScanError};
use maka_agent_facts::read::ReadInput;
use maka_agent_facts::read::ReadPage;
use maka_capabilities::filesystem::Reader;
use maka_capabilities::filesystem::Symlinks;
use maka_capabilities::filesystem::entries::Kind;
use maka_capabilities::filesystem::entries::ReadFile;
use std::{
    collections::BTreeMap,
    path::{Component, Path},
    sync::Arc,
};
use tokio_util::sync::CancellationToken;

const MAX_FILE: usize = 1024 * 1024;
const MAX_FILES: usize = 256;

pub(crate) struct Budget {
    bytes: usize,
    entries: usize,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            bytes: 16 * 1024 * 1024,
            entries: 16_384,
        }
    }
}

#[derive(Debug, Clone)]
enum Resource {
    Text { content: Arc<str>, hash: String },
    Unavailable(DiscoveryFailure),
}

#[derive(Debug, Clone, Default)]
pub struct Resources {
    files: BTreeMap<String, Resource>,
    incomplete: bool,
}

/// Build assets are immutable and carry no filesystem authority. Parse and hash
/// them once, then apply each discovery's remaining shared budget independently.
#[derive(Debug, Clone)]
pub(crate) struct Embedded {
    files: &'static [(&'static str, &'static [u8])],
    prepared: Resources,
}
impl Embedded {
    pub fn new(files: &'static [(&'static str, &'static [u8])]) -> Result<Self, ScanError> {
        Ok(Self {
            files,
            prepared: Resources::embedded(files, &mut Budget::default())?,
        })
    }
    pub fn capture(&self, budget: &mut Budget) -> Result<Resources, ScanError> {
        Resources::capture_embedded(self.files, budget, |path, _| {
            // A call never has more than the original default budget. Every
            // resource it can reach therefore exists in this prepared prefix.
            self.prepared
                .files
                .get(path)
                .cloned()
                .ok_or(ScanError::Unavailable)
        })
    }
}
impl Resources {
    pub(crate) fn embedded(
        files: &[(&str, &[u8])],
        budget: &mut Budget,
    ) -> Result<Self, ScanError> {
        Self::capture_embedded(files, budget, |_, bytes| {
            Ok(match std::str::from_utf8(bytes) {
                Ok(content) => Resource::Text {
                    content: content.into(),
                    hash: maka_agent_facts::artifact::content_digest(bytes),
                },
                Err(_) => Resource::Unavailable(DiscoveryFailure::ReadFailed),
            })
        })
    }
    fn capture_embedded(
        files: &[(&str, &[u8])],
        budget: &mut Budget,
        mut resource: impl FnMut(&str, &[u8]) -> Result<Resource, ScanError>,
    ) -> Result<Self, ScanError> {
        let mut result = Self::default();
        for (index, (path, bytes)) in files.iter().enumerate() {
            if index == MAX_FILES || budget.entries == 0 || budget.bytes == 0 {
                result.incomplete = true;
                break;
            }
            budget.entries -= 1;
            if !valid_path(path) || path.split('/').count() > 9 {
                return Err(ScanError::Unavailable);
            }
            let limit = MAX_FILE.min(budget.bytes);
            budget.bytes = budget.bytes.saturating_sub(bytes.len().min(limit));
            let resource = if bytes.len() > limit {
                Resource::Unavailable(DiscoveryFailure::SourceTooLarge)
            } else {
                resource(path, bytes)?
            };
            if result.files.insert((*path).into(), resource).is_some() {
                return Err(ScanError::Unavailable);
            }
        }
        Ok(result)
    }
    pub(crate) fn capture(
        reader: &Reader<'_>,
        directory: &Path,
        budget: &mut Budget,
        cancellation: &CancellationToken,
    ) -> Result<Self, ScanError> {
        let root = directory
            .components()
            .filter_map(|part| match part {
                Component::Normal(value) => Some(value.to_str()),
                Component::CurDir => None,
                _ => Some(None),
            })
            .collect::<Option<Vec<_>>>()
            .ok_or(ScanError::Unavailable)?
            .join("/");
        let mut result = Self::default();
        let mut remaining = MAX_FILES;
        result.walk(reader, &root, "", budget, &mut remaining, cancellation)?;
        Ok(result)
    }

    fn walk(
        &mut self,
        reader: &Reader<'_>,
        root: &str,
        relative: &str,
        budget: &mut Budget,
        remaining: &mut usize,
        cancellation: &CancellationToken,
    ) -> Result<(), ScanError> {
        super::discovery::check_cancelled(cancellation)?;
        if relative.split('/').count() > 8
            || *remaining == 0
            || budget.entries == 0
            || budget.bytes == 0
        {
            self.incomplete = true;
            return Ok(());
        }
        let path = if relative.is_empty() {
            root.into()
        } else {
            format!("{root}/{relative}")
        };
        let mut entries = Vec::new();
        let read = reader.visit_directory(&path, Symlinks::Reject, |entry| {
            if budget.entries == 0 || *remaining == 0 {
                return Err(maka_capabilities::filesystem::ReadError::ScanLimit {
                    max: MAX_FILES as u64,
                });
            }
            budget.entries -= 1;
            *remaining -= 1;
            if !entry.name.starts_with('.')
                && entry.name != "SKILL.md"
                && entry.name != "skill.lock.json"
            {
                entries.push(entry);
            }
            Ok(())
        });
        if matches!(read, Err(maka_capabilities::filesystem::ReadError::Retired)) {
            return Err(ScanError::Cancelled);
        }
        if read.is_err() {
            self.incomplete = true;
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        for entry in entries {
            super::discovery::check_cancelled(cancellation)?;
            let relative = if relative.is_empty() {
                entry.name
            } else {
                format!("{relative}/{}", entry.name)
            };
            if !valid_path(&relative) {
                continue;
            }
            match entry.kind {
                Kind::Directory => {
                    self.walk(reader, root, &relative, budget, remaining, cancellation)?
                }
                Kind::Other => {
                    self.files.insert(
                        relative,
                        Resource::Unavailable(DiscoveryFailure::BlockedPath),
                    );
                }
                Kind::File => {
                    let limit = MAX_FILE.min(budget.bytes);
                    let resource = if limit == 0 {
                        Resource::Unavailable(DiscoveryFailure::SourceTooLarge)
                    } else {
                        match reader.read(maka_capabilities::filesystem::ReadViewInput {
                            file: ReadFile {
                                path: format!("{root}/{relative}"),
                                offset: 0,
                                limit,
                            },
                            symlinks: Symlinks::Reject,
                        }) {
                            Ok(page) => {
                                budget.bytes = budget.bytes.saturating_sub(page.bytes.len());
                                if page.next_offset.is_some() {
                                    Resource::Unavailable(DiscoveryFailure::SourceTooLarge)
                                } else {
                                    match String::from_utf8(page.bytes) {
                                        Ok(content) => Resource::Text {
                                            hash: maka_agent_facts::artifact::content_digest(
                                                content.as_bytes(),
                                            ),
                                            content: content.into(),
                                        },
                                        Err(_) => {
                                            Resource::Unavailable(DiscoveryFailure::ReadFailed)
                                        }
                                    }
                                }
                            }
                            Err(maka_capabilities::filesystem::ReadError::Retired) => {
                                return Err(ScanError::Cancelled);
                            }
                            Err(_) => Resource::Unavailable(DiscoveryFailure::ReadFailed),
                        }
                    };
                    self.files.insert(relative, resource);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn fingerprint(&self) -> (bool, Vec<(&str, &str)>) {
        (
            self.incomplete,
            self.files
                .iter()
                .map(|(path, resource)| {
                    (
                        path.as_str(),
                        match resource {
                            Resource::Text { hash, .. } => hash.as_str(),
                            Resource::Unavailable(DiscoveryFailure::BlockedPath) => "blocked",
                            Resource::Unavailable(DiscoveryFailure::ReadFailed) => "unreadable",
                            Resource::Unavailable(DiscoveryFailure::SourceTooLarge) => "too_large",
                        },
                    )
                })
                .collect(),
        )
    }

    pub fn page(&self, reference: &str, input: &ReadInput) -> Result<(String, ReadPage), String> {
        self.page_with_budget(reference, input, maka_agent_facts::read::MAX_PAGE_CHARS)
    }

    /// Reserve space for the calling service's result envelope in the model page budget.
    pub fn page_with_budget(
        &self,
        reference: &str,
        input: &ReadInput,
        budget: usize,
    ) -> Result<(String, ReadPage), String> {
        let mut request = input.resolve().map_err(|e| e.to_string())?;
        let prefix = format!("{reference}/");
        let path = if request.is_continuation() {
            request
                .path()
                .strip_prefix(&prefix)
                .ok_or("Skill continuation belongs to a different skill")?
                .to_owned()
        } else {
            request.path().to_owned()
        };
        if !valid_path(&path) {
            return Err("Skill resource path must stay inside this skill directory".into());
        }
        let content = match self.files.get(&path) {
            Some(Resource::Text { content, .. }) => content,
            Some(Resource::Unavailable(DiscoveryFailure::SourceTooLarge)) => {
                return Err("Skill resource exceeds the bounded text capture limit".into());
            }
            Some(Resource::Unavailable(_)) => {
                return Err(
                    "Skill resource was not a readable regular UTF-8 file at request capture"
                        .into(),
                );
            }
            None if self.incomplete => {
                return Err("Skill resource unavailable: directory capture was incomplete".into());
            }
            None => return Err("Skill resource not found in this request's snapshot".into()),
        };
        if !request.is_continuation() {
            request = ReadInput {
                path: format!("{prefix}{path}"),
                offset: input.offset,
                limit: input.limit,
            }
            .resolve()
            .map_err(|e| e.to_string())?;
        }
        Ok((
            path,
            request
                .page_with_budget(content, budget)
                .map_err(|e| e.to_string())?,
        ))
    }
}

fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && !path.contains(['\\', ':'])
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_embedded_resources_keep_request_budgets_independent() {
        static FILES: &[(&str, &[u8])] = &[("one", b"abc"), ("two", b"de"), ("binary", &[255])];
        let prepared = Embedded::new(FILES).unwrap();
        for bytes in 0..=8 {
            for entries in 0..=4 {
                let mut direct_budget = Budget { bytes, entries };
                let direct = Resources::embedded(FILES, &mut direct_budget).unwrap();
                let mut cached_budget = Budget { bytes, entries };
                let cached = prepared.capture(&mut cached_budget).unwrap();
                assert_eq!(cached.fingerprint(), direct.fingerprint());
                assert_eq!(
                    (cached_budget.bytes, cached_budget.entries),
                    (direct_budget.bytes, direct_budget.entries)
                );
            }
        }
        let first = prepared.capture(&mut Budget::default()).unwrap();
        let second = prepared.capture(&mut Budget::default()).unwrap();
        let Resource::Text { content: a, .. } = &first.files["one"] else {
            panic!("text")
        };
        let Resource::Text { content: b, .. } = &second.files["one"] else {
            panic!("text")
        };
        assert!(
            Arc::ptr_eq(a, b),
            "immutable bytes are shared across requests"
        );
        assert_eq!(a.as_ref(), "abc");
        assert!(matches!(
            second.files["binary"],
            Resource::Unavailable(DiscoveryFailure::ReadFailed)
        ));
    }
    #[test]
    fn embedded_resources_enforce_paths_and_shared_capture_limits() {
        for path in [
            "../secret",
            "/secret",
            "a//b",
            "a/./b",
            "a\\b",
            "a:b",
            "a/.hidden",
            "a/b/c/d/e/f/g/h/i/j",
        ] {
            assert!(Resources::embedded(&[(path, b"text")], &mut Budget::default()).is_err());
        }
        assert!(
            Resources::embedded(&[("same", b"a"), ("same", b"b")], &mut Budget::default()).is_err()
        );
        let mut budget = Budget {
            bytes: 5,
            entries: 3,
        };
        let resources = Resources::embedded(
            &[
                ("binary", &[255]),
                ("large", b"oversized"),
                ("absent", b"x"),
            ],
            &mut budget,
        )
        .unwrap();
        assert_eq!(budget.bytes, 0);
        let read = |path: &str| {
            resources.page(
                "bundled:test",
                &ReadInput {
                    path: path.into(),
                    offset: None,
                    limit: None,
                },
            )
        };
        assert!(read("binary").unwrap_err().contains("UTF-8"));
        assert!(read("large").unwrap_err().contains("limit"));
        assert!(read("absent").unwrap_err().contains("incomplete"));
        let mut budget = Budget {
            bytes: 10,
            entries: 1,
        };
        let resources = Resources::embedded(&[("one", b"1"), ("two", b"2")], &mut budget).unwrap();
        assert_eq!(budget.entries, 0);
        assert!(resources.incomplete);
        assert_eq!(resources.files.len(), 1);
    }
}
