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

use crate::contract::{Directories, Directory, DirectoryPage, DirectorySelection, Workspace};
use maka_fs_tools::{
    directory,
    workspace::{self, directory::PublishedDirectory},
};
use maka_runtime_host::server::DirectoryRootSpec;
use std::{path::Path, sync::Arc};

pub(crate) const PAGE_BYTES: usize = 48 * 1024;
#[derive(Clone)]
pub(crate) struct Root {
    name: String,
    directory: Arc<PublishedDirectory>,
}

pub(crate) fn open(specs: Vec<DirectoryRootSpec>, managed: &Path) -> Result<Vec<Root>, String> {
    let roots: Vec<_> = DirectoryRootSpec::normalize(specs)
        .map_err(|_| "agent_workspace_configuration_invalid")?
        .into_iter()
        .map(|spec| {
            if spec.path.starts_with(managed) {
                return Err("agent_workspace_configuration_invalid".into());
            }
            Ok(Root {
                name: spec.label,
                directory: Arc::new(
                    PublishedDirectory::open(&spec.path)
                        .map_err(|_| "agent_workspace_configuration_invalid")?,
                ),
            })
        })
        .collect::<Result<_, String>>()?;
    if roots.is_empty() || !fits(&root_page(&roots)?) {
        return Err("agent_workspace_configuration_invalid".into());
    }
    Ok(roots)
}
fn root<'a>(roots: &'a [Root], id: &str) -> Result<&'a Root, String> {
    roots
        .iter()
        .enumerate()
        .find(|(index, _)| id == format!("root-{}", index + 1))
        .map(|(_, root)| root)
        .ok_or("agent_directory_unavailable".into())
}
fn choice(
    root: &Root,
    root_id: String,
    segments: Vec<String>,
    name: String,
) -> Result<Directory, String> {
    if segments.len() > 64
        || segments
            .iter()
            .any(|s| s.len() > 255 || s.chars().any(char::is_control))
    {
        return Err("agent_directory_invalid".into());
    }
    let selected = root
        .directory
        .resolve(&segments)
        .map_err(|_| "agent_directory_unavailable")?;
    let (path, identity) =
        directory::capture(&selected).map_err(|_| "agent_directory_unavailable")?;
    root.directory
        .validate(&path)
        .map_err(|_| "agent_directory_unavailable")?;
    let path = host_path(&path)?;
    Ok(Directory {
        name,
        path,
        selection: DirectorySelection {
            root_id,
            segments,
            directory_identity: identity,
        },
    })
}
fn host_path(path: &Path) -> Result<String, String> {
    let path = workspace::project::host_path(path).map_err(|_| "agent_directory_invalid")?;
    if path.len() > 4096 {
        return Err("agent_directory_invalid".into());
    }
    Ok(path.into())
}
fn root_page(roots: &[Root]) -> Result<DirectoryPage, String> {
    Ok(DirectoryPage::Roots {
        roots: roots
            .iter()
            .enumerate()
            .map(|(index, root)| {
                choice(
                    root,
                    format!("root-{}", index + 1),
                    vec![],
                    root.name.clone(),
                )
            })
            .collect::<Result<_, _>>()?,
    })
}
pub(crate) fn fits<T: serde::Serialize>(value: &T) -> bool {
    serde_json::to_value(value)
        .is_ok_and(|value| maka_js_runtime::plugin::check_json_delivery(&value, PAGE_BYTES).is_ok())
}
pub(crate) async fn query(roots: Vec<Root>, input: Directories) -> Result<DirectoryPage, String> {
    query_excluding(roots, input, None).await
}
pub(crate) async fn query_excluding(
    roots: Vec<Root>,
    input: Directories,
    excluded: Option<std::path::PathBuf>,
) -> Result<DirectoryPage, String> {
    tokio::task::spawn_blocking(move || {
        let Directories::Children { selection, after } = input else {
            return root_page(&roots);
        };
        let root = root(&roots, &selection.root_id)?;
        let name = selection
            .segments
            .last()
            .cloned()
            .unwrap_or_else(|| root.name.clone());
        let selected = choice(
            root,
            selection.root_id.clone(),
            selection.segments.clone(),
            name,
        )?;
        if selected.selection.directory_identity != selection.directory_identity
            || excluded
                .as_ref()
                .is_some_and(|path| Path::new(&selected.path).starts_with(path))
        {
            return Err("agent_directory_unavailable".into());
        }
        if after.as_ref().is_some_and(|name| {
            name.is_empty()
                || name.len() > 255
                || name.contains(['/', '\\', '\0'])
                || name.chars().any(char::is_control)
        }) {
            return Err("agent_directory_invalid".into());
        }
        let mut names: Vec<_> = root
            .directory
            .directory_names(&selection.segments, 4096)
            .map_err(|_| "agent_directory_unavailable")?
            .into_iter()
            .filter(|name| {
                !name.chars().any(char::is_control)
                    && after
                        .as_ref()
                        .is_none_or(|cursor| name.encode_utf16().cmp(cursor.encode_utf16()).is_gt())
            })
            .collect();
        if let Some(excluded) = &excluded {
            names.retain(|name| {
                let mut segments = selection.segments.clone();
                segments.push(name.clone());
                root.directory
                    .resolve(&segments)
                    .is_ok_and(|path| !path.starts_with(excluded))
            });
        }
        let mut children = Vec::new();
        for name in names.iter().take(32) {
            let mut segments = selection.segments.clone();
            segments.push(name.clone());
            children.push(choice(
                root,
                selection.root_id.clone(),
                segments,
                name.clone(),
            )?);
        }
        let mut page = DirectoryPage::Children {
            directory: selected,
            after: (names.len() > children.len())
                .then(|| children.last().expect("nonempty page").name.clone()),
            children,
        };
        while !fits(&page) {
            let DirectoryPage::Children {
                children, after, ..
            } = &mut page
            else {
                unreachable!()
            };
            if children.len() <= 1 {
                return Err("agent_directory_unavailable".into());
            }
            children.pop();
            *after = children.last().map(|child| child.name.clone());
        }
        let path = root
            .directory
            .resolve(&selection.segments)
            .map_err(|_| "agent_directory_unavailable")?;
        directory::open(&path, &selection.directory_identity)
            .map_err(|_| "agent_directory_unavailable")?;
        Ok(page)
    })
    .await
    .map_err(|_| "agent_directory_unavailable")?
}

/// None is a confirmed pre-effect refusal. Marker initialization errors remain Unknown.
pub(crate) async fn initialize(
    roots: &[Root],
    selection: DirectorySelection,
    managed: &Path,
) -> Result<Option<Workspace>, String> {
    let Ok(root) = root(roots, &selection.root_id) else {
        return Ok(None);
    };
    if selection.segments.len() > 64
        || selection
            .segments
            .iter()
            .any(|s| s.len() > 255 || s.chars().any(char::is_control))
    {
        return Ok(None);
    }
    let published = root.directory.clone();
    let captured = published.clone();
    let managed = managed.to_owned();
    let expected = selection.directory_identity.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        let path = captured.resolve(&selection.segments).ok()?;
        if path.starts_with(&managed) {
            return None;
        }
        host_path(&path).ok()?;
        let directory = directory::open(&path, &expected).ok()?;
        captured.validate(&path).ok()?;
        match workspace::read_directory_identity(path.clone(), directory.try_clone().ok()?) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return None,
        }
        Some((path, directory))
    })
    .await
    .map_err(|_| "agent_workspace_unavailable")?;
    let Some((path, directory)) = prepared else {
        return Ok(None);
    };
    let identity = workspace::ensure_directory_identity(path.clone(), directory)
        .await
        .map_err(|_| "agent_workspace_initialization_failed")?;
    published
        .validate(&path)
        .map_err(|_| "agent_workspace_initialization_failed")?;
    directory::open(&path, &selection.directory_identity)
        .map_err(|_| "agent_workspace_initialization_failed")?;
    Ok(Some(Workspace {
        project_id: None,
        path: host_path(&path)?,
        directory_identity: selection.directory_identity,
        identity,
    }))
}
pub(crate) async fn validate(
    roots: &[Root],
    selected: &Workspace,
    managed: &Path,
) -> Result<(), String> {
    let roots = roots.to_vec();
    let selected = selected.clone();
    let managed = managed.to_owned();
    tokio::task::spawn_blocking(move || {
        let (path, observed) = directory::capture(Path::new(&selected.path))
            .map_err(|_| "agent_workspace_unavailable")?;
        if path.starts_with(&managed)
            || host_path(&path)? != selected.path
            || observed != selected.directory_identity
        {
            return Err("agent_workspace_unavailable".into());
        }
        let root = roots
            .iter()
            .find(|root| root.directory.validate(&path).is_ok())
            .ok_or("agent_workspace_unavailable")?;
        let directory = directory::open(&path, &selected.directory_identity)
            .map_err(|_| "agent_workspace_unavailable")?;
        if workspace::read_directory_identity(path.clone(), directory)
            .map_err(|_| "agent_workspace_unavailable")?
            != selected.identity
        {
            return Err("agent_workspace_unavailable".into());
        }
        root.directory
            .validate(&path)
            .map_err(|_| "agent_workspace_unavailable".into())
    })
    .await
    .map_err(|_| "agent_workspace_unavailable")?
}

/// Re-observe a published directory without creating a workspace marker.
pub(crate) async fn selected(
    roots: Vec<Root>,
    selection: DirectorySelection,
    managed: std::path::PathBuf,
) -> Result<Option<Directory>, String> {
    resolve_selection(roots, selection, managed, true).await
}
pub(crate) async fn observed(
    roots: Vec<Root>,
    selection: DirectorySelection,
    excluded: std::path::PathBuf,
) -> Result<Option<Directory>, String> {
    resolve_selection(roots, selection, excluded, false).await
}
async fn resolve_selection(
    roots: Vec<Root>,
    selection: DirectorySelection,
    managed: std::path::PathBuf,
    workspace_marker: bool,
) -> Result<Option<Directory>, String> {
    tokio::task::spawn_blocking(move || {
        let Ok(root) = root(&roots, &selection.root_id) else {
            return None;
        };
        let selected = choice(
            root,
            selection.root_id.clone(),
            selection.segments.clone(),
            selection
                .segments
                .last()
                .cloned()
                .unwrap_or_else(|| root.name.clone()),
        )
        .ok()?;
        if selected.selection.directory_identity != selection.directory_identity
            || Path::new(&selected.path).starts_with(&managed)
        {
            return None;
        }
        let path = std::path::PathBuf::from(&selected.path);
        let directory = directory::open(&path, &selection.directory_identity).ok()?;
        if !workspace_marker {
            return Some(selected);
        }
        match workspace::read_directory_identity(path, directory) {
            Ok(_) => Some(selected),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(selected),
            Err(_) => None,
        }
    })
    .await
    .map_err(|_| "agent_directory_unavailable".into())
}

pub(crate) async fn selection_at(
    roots: Vec<Root>,
    path: String,
    managed: std::path::PathBuf,
) -> Result<Option<DirectorySelection>, String> {
    tokio::task::spawn_blocking(move || {
        let path = Path::new(&path);
        if path.starts_with(&managed) {
            return None;
        }
        for (index, root) in roots.iter().enumerate() {
            let Ok(relative) = path.strip_prefix(root.directory.path()) else {
                continue;
            };
            let segments = relative
                .components()
                .map(|part| part.as_os_str().to_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?;
            let Ok(selected) = choice(
                root,
                format!("root-{}", index + 1),
                segments,
                root.name.clone(),
            ) else {
                continue;
            };
            if Path::new(&selected.path) == path {
                return Some(selected.selection);
            }
        }
        None
    })
    .await
    .map_err(|_| "agent_directory_unavailable".into())
}
