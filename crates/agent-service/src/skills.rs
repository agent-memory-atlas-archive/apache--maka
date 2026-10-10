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

//! Ordinary skill library service, reusing the original bounded discovery/parser.
//! Files are data; neither references nor declared tools authorize execution.
pub mod contract;
mod management;
pub mod outcomes;
mod preferences;
use contract::*;
pub use contract::{configuration_schema, contract};
use futures_util::future::BoxFuture;
use maka_plugins::{call::Scope, filesystem::ReadRoot};
use maka_runtime_host::application::{
    Context, Handler, NativeActivation, NativeContext, NativeOwner, NativeRegistry,
};
use maka_sandbox::filesystem::{Access, Policy, Rule};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};

pub const IMPLEMENTATION: &str = "maka.skills.library";
pub const EXPORT: &str = "skills";
// Even maximally escaped control bytes leave room for metadata in the JSON budget.
const TEXT_BYTES: usize = 4096;

pub(crate) fn provide(registry: &mut NativeRegistry) -> Result<(), String> {
    registry.insert(
        IMPLEMENTATION.into(),
        Arc::new(|context, configuration| Box::pin(activate(context, configuration))),
    )?;
    outcomes::provide(registry)
}
async fn activate(context: NativeContext, configuration: Value) -> NativeActivation {
    let opened = async {
        let persistence = context.persistence().ok_or("skills_require_persistence")?;
        let root = persistence
            .store()
            .owned_root()
            .map_err(|_| "skills_root_unavailable")?;
        let managed = root.canonical_path().to_owned();
        let configuration: Configuration =
            serde_json::from_value(configuration).map_err(|_| "skills_configuration_invalid")?;
        let manage = configuration.manage;
        let namespace = crate::owner::namespace(
            &persistence.source().state_domain,
            persistence.source().deployment,
            context.activation().instance(),
        );
        let preferences = preferences::Store::new(persistence.store().clone(), namespace.clone());
        let mut library =
            tokio::task::spawn_blocking(move || Library::open(configuration, managed, preferences))
                .await
                .map_err(|_| "skills_open_failed")??;
        if manage {
            library.managed = Some(management::Managed::open(root, namespace).await?);
        }
        Ok::<_, String>(library)
    }
    .await;
    match opened {
        Ok(owner) => {
            let owner = Arc::new(owner);
            NativeActivation {
                exports: Ok(BTreeMap::from([(
                    EXPORT.into(),
                    maka_plugin_core::runtime::Published {
                        contract: contract(),
                        value: owner.clone() as Arc<dyn Handler>,
                    },
                )])),
                owner,
            }
        }
        Err(error) => NativeActivation {
            exports: Err(error),
            owner: Arc::new(crate::Uninitialized),
        },
    }
}
#[derive(Clone)]
struct Library {
    sources: Vec<(Source, ReadRoot)>,
    preferences: preferences::Store,
    preferences_writable: bool,
    managed: Option<Arc<management::Managed>>,
}
impl Library {
    fn open(
        configuration: Configuration,
        managed: PathBuf,
        preferences: preferences::Store,
    ) -> Result<Self, String> {
        if configuration.sources.len() > 8 {
            return Err("skills_configuration_invalid".into());
        }
        let mut ids = BTreeSet::new();
        let policy = Arc::new(
            Policy {
                default: Access::Read,
                rules: vec![Rule::subtree(managed, Access::Deny)],
                deny_globs: vec![],
            }
            .compile()
            .map_err(|_| "skills_configuration_invalid")?,
        );
        let mut sources = Vec::new();
        for source in configuration.sources {
            if source.id.is_empty()
                || source.id.len() > 64
                || !source
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-')
                || !source.id.as_bytes()[0].is_ascii_alphanumeric()
                || !ids.insert(source.id.clone())
                || source.label.trim().is_empty()
                || source.label.len() > 128
                || source.path.len() > 4096
                || !std::path::Path::new(&source.path).is_absolute()
            {
                return Err("skills_configuration_invalid".into());
            }
            let root = ReadRoot::capture(&source.path)
                .map_err(|_| "skills_source_unavailable")?
                .restrict(policy.clone())
                .map_err(|_| "skills_source_unavailable")?;
            sources.push((source, root));
        }
        Ok(Self {
            sources,
            preferences,
            preferences_writable: configuration.preferences,
            managed: None,
        })
    }
    async fn snapshot(&self, scope: &Scope) -> Result<Snapshot, String> {
        let mut sources: Vec<_> = self
            .sources
            .iter()
            .map(|(source, root)| {
                let access = root.bind_scope(scope);
                maka_skills::Source {
                    root: access.location(),
                    access,
                    directory: PathBuf::new(),
                    scope: maka_skills::SkillScope::Custom,
                    source: maka_skills::SkillSource::Custom,
                    reference_prefix: format!("source:{}", source.id),
                }
            })
            .collect();
        if let Some(managed) = &self.managed {
            let access = managed.read.bind_scope(scope);
            sources.insert(
                0,
                maka_skills::Source {
                    root: access.location(),
                    access,
                    directory: PathBuf::from("skills"),
                    scope: maka_skills::SkillScope::Workspace,
                    source: maka_skills::SkillSource::Maka,
                    reference_prefix: "installed".into(),
                },
            );
        }
        let discovery = maka_skills::scan(&sources, &scope.cancellation)
            .await
            .map_err(|_| "skills_discovery_unavailable")?;
        let preferences = self.preferences.read(scope).await?;
        let label = |reference: &str| {
            if reference.starts_with("installed:") {
                return "Installed";
            }
            self.sources
                .iter()
                .find(|(source, _)| reference.starts_with(&format!("source:{}:", source.id)))
                .map(|(source, _)| source.label.as_str())
                .unwrap_or("")
        };
        let mut entries = Vec::new();
        for skill in discovery.inventory {
            let entry = entry(
                &skill.location.reference,
                label(&skill.location.reference),
                &skill.document.manifest.name,
                &skill.document.manifest.description,
                true,
                skill.shadowed_by,
                &skill.document.issues,
            );
            entries.push((entry, skill.content_sha256, skill.document.body));
        }
        for skill in discovery.rejected {
            let entry = entry(
                &skill.location.reference,
                label(&skill.location.reference),
                skill
                    .document
                    .manifest
                    .name
                    .as_deref()
                    .unwrap_or(&skill.location.id),
                skill.document.manifest.description.as_deref().unwrap_or(""),
                false,
                None,
                &skill.document.issues,
            );
            entries.push((entry, skill.content_sha256, skill.document.body));
        }
        for (entry, _, _) in &mut entries {
            entry.preference = preferences
                .entries
                .get(&entry.reference)
                .copied()
                .unwrap_or_default();
        }
        entries.sort_by(|a, b| a.0.reference.cmp(&b.0.reference));
        let diagnostics: Vec<_> = discovery
            .diagnostics
            .iter()
            .map(|diagnostic| {
                let label = if self.managed.is_some() && diagnostic.precedence == 0 {
                    "Installed"
                } else {
                    &self.sources[diagnostic.precedence - usize::from(self.managed.is_some())]
                        .0
                        .label
                };
                format!("{label}: {:?}", diagnostic.reason)
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .take(32)
            .collect();
        let revision = maka_runtime::artifact::content_digest(
            &serde_json::to_vec(&(
                self.managed.is_some(),
                self.preferences_writable,
                preferences.revision,
                self.sources
                    .iter()
                    .map(|(source, _)| source)
                    .collect::<Vec<_>>(),
                entries
                    .iter()
                    .map(|(entry, digest, _)| (entry, digest))
                    .collect::<Vec<_>>(),
                &diagnostics,
            ))
            .map_err(|_| "skills_encoding_failed")?,
        );
        Ok(Snapshot {
            writable: self.managed.is_some(),
            preferences_writable: self.preferences_writable,
            preference_revision: preferences.revision,
            entries,
            diagnostics,
            revision,
        })
    }
}
struct Snapshot {
    writable: bool,
    preferences_writable: bool,
    preference_revision: u64,
    entries: Vec<(Entry, String, String)>,
    diagnostics: Vec<String>,
    revision: String,
}
fn bound(value: &str, limit: usize) -> String {
    let mut end = value.len().min(limit);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}
fn entry(
    reference: &str,
    source: &str,
    name: &str,
    description: &str,
    valid: bool,
    shadowed_by: Option<String>,
    issues: &[maka_skills::Issue],
) -> Entry {
    let short_name = bound(name, 256);
    let short_description = bound(description, 2048);
    Entry {
        installed: reference.starts_with("installed:"),
        preference: Default::default(),
        reference: reference.into(),
        source: source.into(),
        metadata_truncated: short_name != name
            || short_description != description
            || issues.len() > 4
            || issues.iter().take(4).any(|issue| issue.message.len() > 256),
        name: short_name,
        description: short_description,
        valid,
        shadowed_by,
        issues: issues
            .iter()
            .take(4)
            .map(|issue| bound(&issue.message, 256))
            .collect(),
    }
}
impl Snapshot {
    fn list(self, input: List) -> Result<Page, String> {
        if input.query.len() > 1024 || input.query.chars().count() > 256 {
            return Err("skills_query_invalid".into());
        }
        let query = input.query.to_lowercase();
        let filtered: Vec<_> = self
            .entries
            .into_iter()
            .filter(|(entry, _, _)| {
                query.split_whitespace().all(|term| {
                    format!(
                        "{} {} {} {}",
                        entry.reference, entry.source, entry.name, entry.description
                    )
                    .to_lowercase()
                    .contains(term)
                })
            })
            .map(|(entry, _, _)| entry)
            .collect();
        let offset = match input.cursor {
            Some(cursor) if cursor.revision != self.revision => return Ok(Page::Changed),
            Some(cursor) if cursor.query != input.query => {
                return Err("skills_cursor_invalid".into());
            }
            Some(cursor) => cursor.offset,
            None => 0,
        };
        if offset > filtered.len() {
            return Err("skills_cursor_invalid".into());
        }
        let mut entries = Vec::new();
        let after = |n| {
            (offset + n < filtered.len()).then(|| Cursor {
                revision: self.revision.clone(),
                query: input.query.clone(),
                offset: offset + n,
            })
        };
        for entry in filtered.iter().skip(offset).take(32) {
            entries.push(entry.clone());
            let page = Page::Page {
                writable: self.writable,
                preferences_writable: self.preferences_writable,
                preference_revision: self.preference_revision,
                revision: self.revision.clone(),
                entries: entries.clone(),
                after: after(entries.len()),
                diagnostics: self.diagnostics.clone(),
            };
            if !crate::workspace::fits(&page) {
                entries.pop();
                break;
            }
        }
        if offset < filtered.len() && entries.is_empty() {
            return Err("skills_page_too_large".into());
        }
        let after = after(entries.len());
        Ok(Page::Page {
            writable: self.writable,
            preferences_writable: self.preferences_writable,
            preference_revision: self.preference_revision,
            revision: self.revision,
            after,
            entries,
            diagnostics: self.diagnostics,
        })
    }
    fn read(self, input: Read) -> Result<Document, String> {
        if input.revision != self.revision {
            return Ok(Document::Changed);
        }
        let Some((entry, _, text)) = self
            .entries
            .into_iter()
            .find(|(entry, _, _)| entry.reference == input.reference)
        else {
            return Ok(Document::Unavailable);
        };
        let start = input.offset;
        if start > text.len() || !text.is_char_boundary(start) {
            return Err("skills_position_invalid".into());
        }
        let mut end = (start + TEXT_BYTES).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let mut before = start.saturating_sub(TEXT_BYTES);
        while !text.is_char_boundary(before) {
            before += 1;
        }
        Ok(Document::Page {
            writable: self.writable,
            preferences_writable: self.preferences_writable,
            preference_revision: self.preference_revision,
            entry: Box::new(entry),
            revision: self.revision,
            text: text[start..end].into(),
            offset: start,
            before: (start > 0).then_some(before),
            after: (end < text.len()).then_some(end),
            total_bytes: text.len(),
        })
    }
}
fn encode(value: impl Serialize) -> Result<Value, String> {
    let value = serde_json::to_value(value).map_err(|_| "skills_encoding_failed")?;
    maka_js_runtime::plugin::check_json_delivery(&value, crate::workspace::PAGE_BYTES)
        .map_err(|_| "skills_page_too_large")?;
    Ok(value)
}
impl Handler for Library {
    fn call(
        &self,
        method: String,
        input: Value,
        context: Context,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let library = self.clone();
        Box::pin(async move {
            match method.as_str() {
                "list" => {
                    let input =
                        serde_json::from_value(input).map_err(|_| "skills_query_invalid")?;
                    encode(library.snapshot(context.scope()).await?.list(input)?)
                }
                "read" => {
                    let input =
                        serde_json::from_value(input).map_err(|_| "skills_query_invalid")?;
                    encode(library.snapshot(context.scope()).await?.read(input)?)
                }
                "preview" => {
                    let input =
                        serde_json::from_value(input).map_err(|_| "skills_change_invalid")?;
                    encode(library.preview(input, context.scope()).await?)
                }
                "setPreference" => {
                    let input =
                        serde_json::from_value(input).map_err(|_| "skills_preference_invalid")?;
                    context.operation().ok_or("skills_mutation_required")?;
                    if !library.preferences_writable {
                        return encode(PreferenceResult::Rejected {
                            reason: "read_only_preferences".into(),
                        });
                    }
                    encode(library.preferences.set(input, context.scope()).await?)
                }
                "apply" => {
                    let input =
                        serde_json::from_value(input).map_err(|_| "skills_change_invalid")?;
                    let operation = context.operation().ok_or("skills_mutation_required")?;
                    encode(library.apply(input, operation, context.scope()).await?)
                }
                _ => Err("skills_method_unavailable".into()),
            }
        })
    }
}
impl NativeOwner for Library {
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        // All reads retain their original call's resource group. No independent
        // background scan, process, watcher or mutable catalog is owned here.
        Box::pin(async { Ok(()) })
    }
}
#[cfg(test)]
mod tests;
