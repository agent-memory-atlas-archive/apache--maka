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

use super::{OperationError, invalid};
use cap_fs_ext::DirExt;
use cap_std::{ambient_authority, fs::Dir};
use maka_plugins::storage::Namespace;
use maka_runtime::execution::{WorkspaceIdentity, WorkspaceProjection, WorkspaceTarget};
use sha2::{Digest, Sha256};
use std::path::Path;

/// No plugin-provided path participates in allocation. This directory is apart
/// from plugin data: an agent cannot overwrite the plugin's ledger or secrets.
pub(super) async fn prepare(
    root: &Path,
    namespace: &Namespace,
) -> Result<(WorkspaceProjection, WorkspaceIdentity), OperationError> {
    let root = root.to_owned();
    let name = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(namespace.package(), namespace.scope())).map_err(invalid)?
        )
    );
    let (directory, path) = tokio::task::spawn_blocking(move || {
        let parent = Dir::open_ambient_dir(&root, ambient_authority()).map_err(invalid)?;
        let workspaces = child(&parent, "plugin-workspaces").map_err(invalid)?;
        let workspace = child(&workspaces, &name).map_err(invalid)?;
        let path = root.join("plugin-workspaces").join(name);
        Ok::<_, OperationError>((workspace, path))
    })
    .await
    .map_err(invalid)??;
    let identity = maka_fs_tools::workspace::ensure_directory_identity(path.clone(), directory)
        .await
        .map_err(invalid)?;
    let path = maka_fs_tools::workspace::project::host_path(&path)
        .map_err(invalid)?
        .to_owned();
    Ok((
        WorkspaceProjection {
            target: WorkspaceTarget::HostPath { path: path.clone() },
            host_cwd: path,
        },
        identity,
    ))
}
fn child(parent: &Dir, name: &str) -> std::io::Result<Dir> {
    match parent.create_dir(name) {
        Ok(()) => {
            #[cfg(unix)]
            parent.open(".")?.sync_all()?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    parent.open_dir_nofollow(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maka_plugins::composition::Scope;

    #[tokio::test]
    async fn allocated_plugin_workspace_does_not_modify_enclosing_git_repository() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let output = std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let exclude = root.join(".git/info/exclude");
        let original = std::fs::read(&exclude).unwrap();
        let namespace = Namespace::new("fixture.workspace", Scope::Profile).unwrap();
        let (workspace, identity) = prepare(root, &namespace).await.unwrap();
        assert!(
            Path::new(&workspace.host_cwd)
                .canonicalize()
                .unwrap()
                .starts_with(root.join("plugin-workspaces").canonicalize().unwrap())
        );
        assert_eq!(
            maka_fs_tools::workspace::read_identity(Path::new(&workspace.host_cwd)).unwrap(),
            identity
        );
        assert_eq!(std::fs::read(&exclude).unwrap(), original);
        assert!(!root.join(maka_fs_tools::workspace::MARKER_FILE).exists());
        let (same, same_identity) = prepare(root, &namespace).await.unwrap();
        assert_eq!(same, workspace);
        assert_eq!(same_identity, identity);
        assert_eq!(std::fs::read(&exclude).unwrap(), original);
    }
}
