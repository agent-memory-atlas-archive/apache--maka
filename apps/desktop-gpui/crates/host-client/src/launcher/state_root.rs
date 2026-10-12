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

//! Creating a State Root, as a launcher must before it spawns a Host.
//!
//! A Host candidate never creates a State Root. It opens `--root` with
//! `resolveExistingStorageRoot` and `--expected-root-id`
//! (`startInteractiveRuntimeHostCandidate` in
//! `packages/runtime-host/src/server/candidate.ts`), which fails on a
//! directory without a marker. The TS launcher prepares the root first:
//! `connectOrSpawnRuntimeHostWithDependencies` in
//! `packages/runtime-host/src/client/connect-or-spawn.ts` calls
//! `resolveStorageRoot({ path, kind: 'interactive' })` in
//! `packages/storage/src/root-authority.ts`, which
//!
//! - creates the directory with mode 0700 (`ensureRootDirectory`);
//! - resolves it to its canonical path (`realpath`), which is what the
//!   candidate receives as `--root`;
//! - creates `.maka-storage-root.json` when it is missing (`ensureRootMarker`):
//!   `{"schemaVersion":1,"kind":"interactive","rootId":"<32 random bytes as
//!   hex>","rootIdentity":{"dev":"<st_dev>","ino":"<st_ino>"}}` and a newline.
//!   `publishMarkerFile` (`packages/storage/src/marker-file.ts`) writes it to
//!   `<marker>.<pid>.<uuid>.tmp` with mode 0600 and fsyncs it, hard-links it
//!   to the marker name so that a marker another process published first is
//!   kept, fsyncs the directory, and removes the temporary file;
//! - rejects a marker whose `rootIdentity` names a different directory
//!   (`root_identity_collision`), as a copied or restored State Root has.
//!
//! [`prepare_state_root`] does the same. It never rewrites an existing marker.
//!
//! # Repairing a marker that names another directory
//!
//! A copied or restored State Root has such a marker, but so does the
//! original after macOS mounts its volume again under a new device number.
//! Maka Desktop then asks the person (`resolveDesktopStorageRoot` in
//! `apps/desktop/src/main/storage-root-startup.ts`): it takes a repair
//! candidate (`prepareStorageRootIdentityRepair` in `root-authority.ts`) and,
//! once they confirm, replaces the marker's `rootIdentity`
//! (`repairStorageRootIdentity`, `replaceRootMarkerIdentity`).
//! [`StateRootError::IdentityMismatch`] carries the same candidate, a
//! [`StateRootRepair`]: the canonical path, the marker's bytes as read and
//! the directory's identity, all from the moment the mismatch was found.
//! [`repair_state_root`] applies it:
//!
//! - It takes Maka's marker lock (`withExclusiveRootMarker`): the artifact
//!   writer bootstrap lock of the directory's identity,
//!   `<control namespace>/artifact-writer-bootstrap/<sha256 of "dev:ino">.lock`
//!   (`prepareArtifactWriterBootstrapLockPathForIdentity`), in directories it
//!   makes private (mode 0700) as `ensurePrivateDirectory` does, opened with
//!   `O_NOFOLLOW` and mode 0600 and checked to be one regular file before and
//!   after locking, as `withArtifactWriterBootstrapLock` does. The lock itself
//!   is `flock`, which is what `fs-native-extensions` takes on macOS. On Linux
//!   Maka takes an open file description lock instead (`F_OFD_SETLKW`), which
//!   a `flock` does not exclude, so there only the checks below stand between
//!   this repair and a Maka process writing the marker.
//! - Under the lock it checks that the directory still has the identity the
//!   candidate saw and that the marker is byte for byte the one it read, and
//!   checks both again just before the new marker takes its place.
//! - It writes the marker with only `rootIdentity` replaced, in the form
//!   [`prepare_state_root`] writes, to a temporary file (mode 0600, synced),
//!   renames that over the marker and syncs the directory
//!   (`publishMarkerFile` with `publication: 'replace'`).
//! - Afterwards it checks the directory's identity again and that the marker
//!   on disk names the same `rootId` and this directory.
//!
//! It never creates a marker and never changes `rootId`. Two cases differ
//! from Desktop, both without writing: a marker that already names this
//! directory with the candidate's `rootId` (repaired meanwhile by another
//! window or by Maka) counts as repaired rather than as changed, and a marker
//! with fields beyond the four this client writes is refused instead of
//! rewritten with them kept.

use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::DiscoveryError;
#[cfg(unix)]
use crate::discovery::{STORAGE_ROOT_MARKER_FILE, read_marker};

/// A State Root that exists and carries a valid marker for its directory.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PreparedRoot {
    /// The resolved directory, as a Host candidate receives it in `--root`.
    pub canonical_path: PathBuf,
    /// The marker's `rootId`: 64 lowercase hex characters.
    pub root_id: String,
    /// Whether this call wrote the marker.
    pub created: bool,
}

/// A State Root whose marker records another directory identity than its
/// directory has, as [`prepare_state_root`] read both when it found the
/// mismatch: the candidate `prepareStorageRootIdentityRepair` takes. Only
/// [`repair_state_root`] acts on it, once a person has said that this is the
/// original State Root on this computer and not a copy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(unix), allow(dead_code))]
pub struct StateRootRepair {
    canonical_path: PathBuf,
    root_id: String,
    /// The marker's bytes as read.
    marker: Vec<u8>,
    /// The directory's identity then.
    identity: DirectoryIdentity,
}

impl StateRootRepair {
    /// The State Root's resolved directory.
    pub fn path(&self) -> &Path {
        &self.canonical_path
    }

    /// The marker's `rootId`, which a repair keeps.
    pub fn root_id(&self) -> &str {
        &self.root_id
    }
}

/// Why a State Root could not be prepared or repaired.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StateRootError {
    /// The path exists and is not a directory.
    #[error("{} is not a directory", .0.display())]
    NotADirectory(PathBuf),
    /// The marker was written for another directory
    /// (`root_identity_collision`): the State Root's volume was mounted again
    /// under a new device number, or it was copied or restored from
    /// elsewhere. Only a person can tell which; [`repair_state_root`] adopts
    /// it once they say it is the original.
    #[error(
        "the State Root marker in {} records a different directory: the disk may have been \
         mounted again, or the State Root copied or restored from elsewhere",
        .0.path().display()
    )]
    IdentityMismatch(Box<StateRootRepair>),
    /// The directory at the path is no longer the one that was checked
    /// (`root_identity_changed`).
    #[error("the State Root at {} changed while it was being checked", .0.display())]
    IdentityChanged(PathBuf),
    /// The marker is not the one the repair was offered for; nothing was
    /// written (`root_identity_changed`).
    #[error("the State Root marker in {} changed after the repair was offered", .0.display())]
    MarkerChanged(PathBuf),
    /// The marker holds fields this client does not write, which a repair
    /// would drop; nothing was written.
    #[error("the State Root marker in {} has fields this client cannot keep", .0.display())]
    MarkerNotRepairable(PathBuf),
    /// The marker exists but is invalid.
    #[error(transparent)]
    Marker(DiscoveryError),
    /// Creating or reading the directory or the marker failed.
    #[error("failed to prepare the State Root at {}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// Taking the marker lock, or reading or writing the marker during a
    /// repair failed; the marker is either the old one or the repaired one.
    #[error("repairing the State Root failed at {}", path.display())]
    Repair {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// Creating State Roots is not implemented on this platform yet.
    #[error("creating a State Root is not supported on this platform yet")]
    Unsupported,
}

/// Creates the State Root at `path` if needed and returns its canonical path
/// and `rootId`, the way `resolveStorageRoot` does (see the module docs).
///
/// File access goes through `async-fs`, so the future is safe to await from
/// any executor.
#[cfg(unix)]
pub async fn prepare_state_root(path: &Path) -> Result<PreparedRoot, StateRootError> {
    use async_fs::unix::DirBuilderExt as _;

    let io_error = |path: &Path| {
        let path = path.to_owned();
        move |source| StateRootError::Io { path, source }
    };
    let requested = std::path::absolute(path).map_err(io_error(path))?;
    let mut builder = async_fs::DirBuilder::new();
    builder.recursive(true).mode(0o700);
    if let Err(error) = builder.create(&requested).await {
        return Err(match async_fs::metadata(&requested).await {
            Ok(metadata) if !metadata.is_dir() => StateRootError::NotADirectory(requested),
            _ => io_error(&requested)(error),
        });
    }
    let canonical_path = async_fs::canonicalize(&requested).await.map_err(io_error(&requested))?;
    let identity = directory_identity(&canonical_path).await?;

    let marker_path = canonical_path.join(STORAGE_ROOT_MARKER_FILE);
    let created = match async_fs::symlink_metadata(&marker_path).await {
        Ok(_) => false,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            publish_marker(&canonical_path, &identity).await.map_err(io_error(&marker_path))?
        }
        Err(error) => return Err(io_error(&marker_path)(error)),
    };

    let marker = read_marker(&canonical_path).await.map_err(StateRootError::Marker)?;
    // The directory must still be the one the marker was checked against.
    if directory_identity(&canonical_path).await? != identity {
        return Err(StateRootError::IdentityChanged(canonical_path));
    }
    if marker.dev != identity.dev || marker.ino != identity.ino {
        return Err(StateRootError::IdentityMismatch(Box::new(StateRootRepair {
            canonical_path,
            root_id: marker.root_id,
            marker: marker.bytes,
            identity,
        })));
    }
    Ok(PreparedRoot { canonical_path, root_id: marker.root_id, created })
}

/// See the unix implementation.
#[cfg(not(unix))]
pub async fn prepare_state_root(_path: &Path) -> Result<PreparedRoot, StateRootError> {
    Err(StateRootError::Unsupported)
}

/// A directory's device and inode numbers, in the decimal form the marker
/// records (`rootStat.dev.toString()` of a bigint `stat`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(unix), allow(dead_code))]
struct DirectoryIdentity {
    dev: String,
    ino: String,
}

#[cfg(unix)]
async fn directory_identity(path: &Path) -> Result<DirectoryIdentity, StateRootError> {
    use std::os::unix::fs::MetadataExt as _;

    let metadata = async_fs::metadata(path)
        .await
        .map_err(|source| StateRootError::Io { path: path.to_owned(), source })?;
    if !metadata.is_dir() {
        return Err(StateRootError::NotADirectory(path.to_owned()));
    }
    Ok(DirectoryIdentity { dev: metadata.dev().to_string(), ino: metadata.ino().to_string() })
}

/// The marker's contents, byte for byte what `ensureRootMarker` writes.
#[cfg(unix)]
fn marker_contents(root_id: &str, identity: &DirectoryIdentity) -> String {
    let marker = serde_json::json!({
        "schemaVersion": 1,
        "kind": "interactive",
        "rootId": root_id,
        "rootIdentity": { "dev": identity.dev, "ino": identity.ino },
    });
    format!("{marker}\n")
}

/// 32 bytes from the OS random source as lowercase hex
/// (`randomBytes(32).toString('hex')`).
#[cfg(unix)]
fn new_root_id() -> io::Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(io::Error::other)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Publishes a new marker in `root` without replacing one that appears
/// meanwhile (`publishMarkerFile` with `publication: 'create'`). Returns
/// whether this call's marker won.
#[cfg(unix)]
async fn publish_marker(root: &Path, identity: &DirectoryIdentity) -> io::Result<bool> {
    use async_fs::unix::OpenOptionsExt as _;
    use futures_lite::AsyncWriteExt as _;

    let contents = marker_contents(&new_root_id()?, identity);
    let temporary = root.join(format!(
        "{STORAGE_ROOT_MARKER_FILE}.{}.{}.tmp",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let result = async {
        let mut file = async_fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .await?;
        file.write_all(contents.as_bytes()).await?;
        file.flush().await?;
        file.sync_all().await?;
        drop(file);
        match async_fs::hard_link(&temporary, root.join(STORAGE_ROOT_MARKER_FILE)).await {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
            Err(error) => return Err(error),
        }
        async_fs::File::open(root).await?.sync_all().await?;
        Ok(true)
    }
    .await;
    let _ = async_fs::remove_file(&temporary).await;
    result
}

/// The directory under the control namespace that holds Maka's artifact
/// writer bootstrap locks (`ARTIFACT_WRITER_BOOTSTRAP_DIRECTORY`).
#[cfg(unix)]
const BOOTSTRAP_LOCK_DIRECTORY: &str = "artifact-writer-bootstrap";

/// Applies `repair`, once a person has confirmed it, the way
/// `repairStorageRootIdentity` does (see the module docs), taking Maka's
/// marker lock under `control_namespace` (normally
/// [`crate::control_namespace`]). Returns the State Root as
/// [`prepare_state_root`] now finds it.
///
/// The work runs on `blocking`'s thread pool, so the future is safe to await
/// from any executor.
#[cfg(unix)]
pub async fn repair_state_root(
    repair: &StateRootRepair,
    control_namespace: &Path,
) -> Result<PreparedRoot, StateRootError> {
    let repair = repair.clone();
    let control_namespace = control_namespace.to_owned();
    // Waiting for the lock blocks in the kernel; the reads and writes under
    // it are a few small synchronous calls.
    blocking::unblock(move || repair_marker(&repair, &control_namespace)).await
}

/// See the unix implementation.
#[cfg(not(unix))]
pub async fn repair_state_root(
    _repair: &StateRootRepair,
    _control_namespace: &Path,
) -> Result<PreparedRoot, StateRootError> {
    Err(StateRootError::Unsupported)
}

#[cfg(unix)]
fn repair_marker(
    repair: &StateRootRepair,
    control_namespace: &Path,
) -> Result<PreparedRoot, StateRootError> {
    use crate::discovery::parse_marker;

    let root = repair.canonical_path.as_path();
    let marker_path = root.join(STORAGE_ROOT_MARKER_FILE);
    let prepared =
        |root_id: String| PreparedRoot { canonical_path: root.to_owned(), root_id, created: false };
    let names_this_directory = |root_id: &str, dev: &str, ino: &str| {
        root_id == repair.root_id && dev == repair.identity.dev && ino == repair.identity.ino
    };
    // `assertRootPathIdentity`, the marker read, `assertRootPathIdentity`.
    let read_current = || -> Result<Vec<u8>, StateRootError> {
        check_identity(root, &repair.identity)?;
        let bytes = read_marker_file(&marker_path)?;
        check_identity(root, &repair.identity)?;
        Ok(bytes)
    };

    // No lock file for an identity the directory no longer has.
    check_identity(root, &repair.identity)?;
    let _lock = lock_marker(control_namespace, &repair.identity)?;

    let current = read_current()?;
    if current != repair.marker {
        let record = parse_marker(&marker_path, current).map_err(StateRootError::Marker)?;
        if names_this_directory(&record.root_id, &record.dev, &record.ino) {
            return Ok(prepared(record.root_id));
        }
        return Err(StateRootError::MarkerChanged(root.to_owned()));
    }
    if !has_only_written_fields(&repair.marker) {
        return Err(StateRootError::MarkerNotRepairable(root.to_owned()));
    }
    let contents = marker_contents(&repair.root_id, &repair.identity);
    replace_marker(root, &contents, || {
        if read_current()? != repair.marker {
            return Err(StateRootError::MarkerChanged(root.to_owned()));
        }
        Ok(())
    })?;

    let adopted = parse_marker(&marker_path, read_current()?).map_err(StateRootError::Marker)?;
    if !names_this_directory(&adopted.root_id, &adopted.dev, &adopted.ino) {
        return Err(StateRootError::MarkerChanged(root.to_owned()));
    }
    Ok(prepared(adopted.root_id))
}

/// Whether `marker` holds exactly the fields [`marker_contents`] writes, so
/// that writing it in that form keeps every field but `rootIdentity`.
#[cfg(unix)]
fn has_only_written_fields(marker: &[u8]) -> bool {
    let Ok(serde_json::Value::Object(fields)) = serde_json::from_slice(marker) else {
        return false;
    };
    let identity_fields =
        fields.get("rootIdentity").and_then(serde_json::Value::as_object).map(|identity| {
            identity.len() == 2 && identity.contains_key("dev") && identity.contains_key("ino")
        });
    fields.len() == 4
        && ["schemaVersion", "kind", "rootId"].iter().all(|key| fields.contains_key(*key))
        && identity_fields == Some(true)
}

/// Fails unless the directory at `root` still has `identity`
/// (`assertRootPathIdentity`).
#[cfg(unix)]
fn check_identity(root: &Path, identity: &DirectoryIdentity) -> Result<(), StateRootError> {
    use std::os::unix::fs::MetadataExt as _;

    let same = match std::fs::metadata(root) {
        Ok(metadata) => {
            metadata.is_dir()
                && metadata.dev().to_string() == identity.dev
                && metadata.ino().to_string() == identity.ino
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(source) => return Err(StateRootError::Repair { path: root.to_owned(), source }),
    };
    if same { Ok(()) } else { Err(StateRootError::IdentityChanged(root.to_owned())) }
}

/// The marker's bytes, read without following a symbolic link, when it is
/// a regular file of at most the marker's size limit.
#[cfg(unix)]
fn read_marker_file(path: &Path) -> Result<Vec<u8>, StateRootError> {
    use std::io::Read as _;
    use std::os::unix::fs::OpenOptionsExt as _;

    use crate::discovery::{MAX_MARKER_BYTES, marker_not_a_file};

    let failed = |source| StateRootError::Repair { path: path.to_owned(), source };
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(failed)?;
    if !file.metadata().map_err(failed)?.is_file() {
        return Err(StateRootError::Marker(marker_not_a_file(path.to_owned())));
    }
    let mut bytes = Vec::new();
    file.take(MAX_MARKER_BYTES + 1).read_to_end(&mut bytes).map_err(failed)?;
    if bytes.len() as u64 > MAX_MARKER_BYTES {
        return Err(StateRootError::Marker(marker_not_a_file(path.to_owned())));
    }
    Ok(bytes)
}

/// Writes `contents` to a temporary file in `root` (mode 0600, synced),
/// runs `before_publish`, renames the file over the marker and syncs the
/// directory (`publishMarkerFile` with `publication: 'replace'`). The
/// temporary file is removed when anything fails.
#[cfg(unix)]
fn replace_marker(
    root: &Path,
    contents: &str,
    before_publish: impl FnOnce() -> Result<(), StateRootError>,
) -> Result<(), StateRootError> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;

    let failed = |path: &Path| {
        let path = path.to_owned();
        move |source| StateRootError::Repair { path, source }
    };
    let marker_path = root.join(STORAGE_ROOT_MARKER_FILE);
    let temporary = root.join(format!(
        "{STORAGE_ROOT_MARKER_FILE}.{}.{}.tmp",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(failed(&temporary))?;
    let published = file
        .write_all(contents.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(failed(&temporary))
        .and_then(|()| {
            drop(file);
            before_publish()
        })
        .and_then(|()| std::fs::rename(&temporary, &marker_path).map_err(failed(&marker_path)));
    if published.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    published?;
    std::fs::File::open(root).and_then(|directory| directory.sync_all()).map_err(failed(root))
}

/// Takes Maka's marker lock for the directory identity `identity`
/// (`withExclusiveRootMarker`; see the module docs). Closing the returned
/// file releases it.
#[cfg(unix)]
fn lock_marker(
    control_namespace: &Path,
    identity: &DirectoryIdentity,
) -> Result<std::fs::File, StateRootError> {
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};

    use sha2::{Digest as _, Sha256};

    let directory = control_namespace.join(BOOTSTRAP_LOCK_DIRECTORY);
    ensure_private_directory(control_namespace)?;
    ensure_private_directory(&directory)?;
    let digest = Sha256::digest(format!("{}:{}", identity.dev, identity.ino));
    let name: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let path = directory.join(format!("{name}.lock"));
    let failed = |source| StateRootError::Repair { path: path.clone(), source };
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)
        .map_err(failed)?;
    check_lock_file(&file, &path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600)).map_err(failed)?;
    file.lock().map_err(failed)?;
    check_lock_file(&file, &path)?;
    Ok(file)
}

/// Fails unless the open lock file and the path are one regular file
/// (`assertStableRegularFile`).
#[cfg(unix)]
fn check_lock_file(file: &std::fs::File, path: &Path) -> Result<(), StateRootError> {
    use std::os::unix::fs::MetadataExt as _;

    let failed = |source| StateRootError::Repair { path: path.to_owned(), source };
    let handle = file.metadata().map_err(failed)?;
    let at_path = std::fs::symlink_metadata(path).map_err(failed)?;
    if !handle.is_file()
        || !at_path.is_file()
        || handle.dev() != at_path.dev()
        || handle.ino() != at_path.ino()
    {
        return Err(failed(io::Error::other("the marker lock is not one stable regular file")));
    }
    Ok(())
}

/// Creates `path` (mode 0700) if needed and makes it private, as
/// `ensurePrivateDirectory` does. Maka refuses a directory another user
/// owns before it changes the mode; here changing the mode is what fails
/// for one.
#[cfg(unix)]
fn ensure_private_directory(path: &Path) -> Result<(), StateRootError> {
    use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _, PermissionsExt as _};

    let failed = |source| StateRootError::Repair { path: path.to_owned(), source };
    let not_private = || failed(io::Error::other("not a private directory"));
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(path).map_err(failed)?;
    if !std::fs::symlink_metadata(path).map_err(failed)?.is_dir() {
        return Err(not_private());
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(failed)?;
    let metadata = std::fs::symlink_metadata(path).map_err(failed)?;
    if !metadata.is_dir() || metadata.mode() & 0o077 != 0 {
        return Err(not_private());
    }
    Ok(())
}

#[cfg(all(test, unix))]
// Test setup and assertions touch the file system synchronously; no UI
// thread is involved.
#[allow(clippy::disallowed_methods)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    use futures_lite::future::block_on;
    use serde_json::Value;

    use super::*;
    use crate::{AttemptError, HostBlocker, LaunchError};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("host-client-state-root-{name}-{}", uuid::Uuid::new_v4().simple()));
        fs::create_dir_all(&dir).expect("scratch dir");
        fs::canonicalize(dir).expect("canonical scratch dir")
    }

    #[test]
    fn creates_the_directory_and_a_marker_like_resolve_storage_root() {
        let parent = scratch("create");
        let root = parent.join("nested/state-root");
        let prepared = block_on(prepare_state_root(&root)).expect("prepared");
        assert!(prepared.created);
        assert_eq!(prepared.canonical_path, root);
        assert!(host_protocol::is_root_id(&prepared.root_id));

        let directory = fs::metadata(&root).expect("root");
        assert_eq!(directory.permissions().mode() & 0o777, 0o700);
        let marker_path = root.join(STORAGE_ROOT_MARKER_FILE);
        let marker = fs::metadata(&marker_path).expect("marker");
        assert_eq!(marker.permissions().mode() & 0o777, 0o600);
        let text = fs::read_to_string(&marker_path).expect("marker text");
        assert!(text.ends_with("}\n"));
        let json: Value = serde_json::from_str(&text).expect("marker json");
        assert_eq!(
            json,
            serde_json::json!({
                "schemaVersion": 1,
                "kind": "interactive",
                "rootId": prepared.root_id,
                "rootIdentity": {
                    "dev": directory.dev().to_string(),
                    "ino": directory.ino().to_string()
                }
            })
        );
        // Key order matches `JSON.stringify` of the TS object literal.
        assert!(text.starts_with(r#"{"schemaVersion":1,"kind":"interactive","rootId":""#));
        // No temporary file is left behind.
        let entries: Vec<_> = fs::read_dir(&root).expect("entries").collect();
        assert_eq!(entries.len(), 1);
        fs::remove_dir_all(parent).ok();
    }

    #[test]
    fn an_existing_marker_is_kept() {
        let root = scratch("existing");
        let first = block_on(prepare_state_root(&root)).expect("first");
        let second = block_on(prepare_state_root(&root)).expect("second");
        assert!(!second.created);
        assert_eq!(first.root_id, second.root_id);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_marker_from_another_directory_is_rejected() {
        let parent = scratch("copied");
        let original = parent.join("original");
        block_on(prepare_state_root(&original)).expect("original");
        let copy = parent.join("copy");
        fs::create_dir(&copy).expect("copy dir");
        fs::copy(original.join(STORAGE_ROOT_MARKER_FILE), copy.join(STORAGE_ROOT_MARKER_FILE))
            .expect("copy marker");
        assert!(matches!(
            block_on(prepare_state_root(&copy)),
            Err(StateRootError::IdentityMismatch(_))
        ));
        fs::remove_dir_all(parent).ok();
    }

    #[test]
    fn a_file_is_not_a_state_root() {
        let parent = scratch("file");
        let file = parent.join("file");
        fs::write(&file, "x").expect("file");
        assert!(matches!(
            block_on(prepare_state_root(&file)),
            Err(StateRootError::NotADirectory(_))
        ));
        fs::remove_dir_all(parent).ok();
    }

    #[test]
    fn an_invalid_marker_is_reported() {
        let root = scratch("invalid");
        fs::write(root.join(STORAGE_ROOT_MARKER_FILE), "{}").expect("marker");
        assert!(matches!(block_on(prepare_state_root(&root)), Err(StateRootError::Marker(_))));
        fs::remove_dir_all(root).ok();
    }

    const ROOT_ID: &str = "67d440f2c07d4cf4e9f56a52aa2bf8e435c71602ddda6244987e201de0c4fb8d";

    /// A scratch directory with a State Root at `state-root` whose marker
    /// was written before its volume was mounted again: the directory's
    /// inode, but a device number one higher, as macOS hands out after a
    /// reboot. Also a control namespace for the marker lock, never the
    /// real one.
    struct Remounted {
        dir: PathBuf,
        root: PathBuf,
        namespace: PathBuf,
    }

    impl Remounted {
        fn new(name: &str) -> Self {
            let dir = scratch(name);
            let root = dir.join("state-root");
            fs::create_dir(&root).expect("root");
            let stale = Self::identity_of(&root, 1);
            fs::write(root.join(STORAGE_ROOT_MARKER_FILE), marker_contents(ROOT_ID, &stale))
                .expect("stale marker");
            Self { namespace: dir.join("runtime-hosts"), dir, root }
        }

        /// The identity of `path`, its device number moved by `dev_offset`.
        fn identity_of(path: &Path, dev_offset: u64) -> DirectoryIdentity {
            let metadata = fs::metadata(path).expect("metadata");
            DirectoryIdentity {
                dev: (metadata.dev() + dev_offset).to_string(),
                ino: metadata.ino().to_string(),
            }
        }

        fn marker_path(&self) -> PathBuf {
            self.root.join(STORAGE_ROOT_MARKER_FILE)
        }

        fn marker(&self) -> Vec<u8> {
            fs::read(self.marker_path()).expect("marker")
        }

        /// The repair [`prepare_state_root`] offers for the stale marker.
        fn offered(&self) -> StateRootRepair {
            match block_on(prepare_state_root(&self.root)) {
                Err(StateRootError::IdentityMismatch(repair)) => *repair,
                other => panic!("expected a mismatch, got {other:?}"),
            }
        }

        fn repair(&self, repair: &StateRootRepair) -> Result<PreparedRoot, StateRootError> {
            block_on(repair_state_root(repair, &self.namespace))
        }

        /// Where Maka takes the marker lock for the directory as it is now.
        fn lock_path(&self) -> PathBuf {
            use sha2::{Digest as _, Sha256};

            let identity = Self::identity_of(&self.root, 0);
            let digest = Sha256::digest(format!("{}:{}", identity.dev, identity.ino));
            let name: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
            self.namespace.join(BOOTSTRAP_LOCK_DIRECTORY).join(format!("{name}.lock"))
        }

        fn entries(&self) -> Vec<String> {
            let mut names: Vec<String> = fs::read_dir(&self.root)
                .expect("entries")
                .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for Remounted {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.dir).ok();
        }
    }

    #[test]
    fn a_marker_from_before_a_remount_is_offered_for_repair_and_left_alone() {
        let remounted = Remounted::new("offer");
        let before = remounted.marker();
        let repair = remounted.offered();
        assert_eq!(repair.path(), remounted.root);
        assert_eq!(repair.root_id(), ROOT_ID);
        assert_eq!(repair.marker, before, "the bytes as read");
        assert_eq!(repair.identity, Remounted::identity_of(&remounted.root, 0));
        assert_eq!(remounted.marker(), before, "preparing never rewrites a marker");
        let text = StateRootError::IdentityMismatch(Box::new(repair.clone())).to_string();
        assert!(text.contains(&remounted.root.display().to_string()), "{text}");

        // The window hears of it as a blocker that carries the repair.
        let error = AttemptError::from(LaunchError::StateRoot(StateRootError::IdentityMismatch(
            Box::new(repair.clone()),
        )));
        assert!(error.is_permanent());
        assert_eq!(error.blocker(), Some(HostBlocker::StateRoot(repair)));
    }

    #[test]
    fn a_repair_replaces_only_the_identity_and_the_root_then_prepares() {
        let remounted = Remounted::new("repair");
        let repair = remounted.offered();
        let prepared = remounted.repair(&repair).expect("repaired");
        assert_eq!(prepared.root_id, ROOT_ID);
        assert_eq!(prepared.canonical_path, remounted.root);
        assert!(!prepared.created);

        let text = String::from_utf8(remounted.marker()).expect("utf-8");
        let current = Remounted::identity_of(&remounted.root, 0);
        assert_eq!(text, marker_contents(ROOT_ID, &current), "the form prepare_state_root writes");
        let json: Value = serde_json::from_str(&text).expect("json");
        assert_eq!(
            json,
            serde_json::json!({
                "schemaVersion": 1,
                "kind": "interactive",
                "rootId": ROOT_ID,
                "rootIdentity": { "dev": current.dev, "ino": current.ino }
            })
        );
        let marker = fs::metadata(remounted.marker_path()).expect("marker");
        assert_eq!(marker.permissions().mode() & 0o777, 0o600);
        assert_eq!(remounted.entries(), [STORAGE_ROOT_MARKER_FILE], "no temporary file left");

        // Maka's lock file, in private directories.
        let lock = fs::metadata(remounted.lock_path()).expect("the marker lock file");
        assert!(lock.is_file());
        assert_eq!(lock.permissions().mode() & 0o777, 0o600);
        for directory in
            [remounted.namespace.clone(), remounted.namespace.join(BOOTSTRAP_LOCK_DIRECTORY)]
        {
            let mode = fs::metadata(&directory).expect("directory").permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "{}", directory.display());
        }

        let again = block_on(prepare_state_root(&remounted.root)).expect("prepared");
        assert_eq!(again.root_id, ROOT_ID);
        assert!(!again.created);
    }

    #[test]
    fn a_marker_changed_after_the_offer_is_refused_and_kept() {
        let remounted = Remounted::new("changed");
        let repair = remounted.offered();
        // Another rootId, and the same rootId with another stale identity:
        // neither is the marker the person was asked about.
        let other_id = "f".repeat(64);
        let stale = Remounted::identity_of(&remounted.root, 1);
        let staler = Remounted::identity_of(&remounted.root, 2);
        for contents in [marker_contents(&other_id, &stale), marker_contents(ROOT_ID, &staler)] {
            fs::write(remounted.marker_path(), &contents).expect("changed marker");
            let error = remounted.repair(&repair).expect_err("refused");
            assert!(matches!(error, StateRootError::MarkerChanged(_)), "{error:?}");
            assert_eq!(remounted.marker(), contents.as_bytes(), "nothing written");
        }
        assert_eq!(remounted.entries(), [STORAGE_ROOT_MARKER_FILE]);
    }

    #[test]
    fn a_marker_that_already_names_the_directory_is_not_written() {
        use std::os::unix::fs::MetadataExt as _;

        let remounted = Remounted::new("matched");
        let repair = remounted.offered();
        // Repaired meanwhile, by another window or by Maka.
        let current = Remounted::identity_of(&remounted.root, 0);
        fs::write(remounted.marker_path(), marker_contents(ROOT_ID, &current)).expect("marker");
        let before = fs::metadata(remounted.marker_path()).expect("marker");
        let prepared = remounted.repair(&repair).expect("already repaired");
        assert_eq!(prepared.root_id, ROOT_ID);
        let after = fs::metadata(remounted.marker_path()).expect("marker");
        // A write renames a new file over the marker, so its inode changes.
        assert_eq!(after.ino(), before.ino(), "not written");
        assert_eq!(after.mtime_nsec(), before.mtime_nsec());
        assert_eq!(remounted.marker(), marker_contents(ROOT_ID, &current).as_bytes());
    }

    #[test]
    fn a_directory_replaced_after_the_offer_is_not_repaired() {
        let remounted = Remounted::new("replaced");
        let repair = remounted.offered();
        let marker = remounted.marker();
        let moved = remounted.dir.join("moved");
        fs::rename(&remounted.root, &moved).expect("move the root away");
        fs::create_dir(&remounted.root).expect("another directory at the path");
        fs::write(remounted.marker_path(), &marker).expect("the same marker");
        let error = remounted.repair(&repair).expect_err("refused");
        assert!(matches!(error, StateRootError::IdentityChanged(_)), "{error:?}");
        assert_eq!(remounted.marker(), marker);
        assert_eq!(fs::read(moved.join(STORAGE_ROOT_MARKER_FILE)).expect("moved"), marker);
        assert!(!remounted.namespace.exists(), "no lock taken for a stale identity");
    }

    #[test]
    fn a_marker_with_fields_this_client_does_not_write_is_not_rewritten() {
        let remounted = Remounted::new("unknown");
        let stale = Remounted::identity_of(&remounted.root, 1);
        let contents = format!(
            r#"{{"schemaVersion":1,"kind":"interactive","rootId":"{ROOT_ID}","rootIdentity":{{"dev":"{}","ino":"{}"}},"note":"kept"}}"#,
            stale.dev, stale.ino
        );
        fs::write(remounted.marker_path(), &contents).expect("marker");
        let repair = remounted.offered();
        let error = remounted.repair(&repair).expect_err("refused");
        assert!(matches!(error, StateRootError::MarkerNotRepairable(_)), "{error:?}");
        assert_eq!(remounted.marker(), contents.as_bytes());
    }

    #[test]
    fn a_repair_waits_for_makas_marker_lock() {
        let remounted = Remounted::new("locked");
        let repair = remounted.offered();
        // Maka holds the lock, as `withExclusiveRootMarker` does.
        let lock_path = remounted.lock_path();
        fs::create_dir_all(lock_path.parent().expect("lock directory")).expect("directories");
        let held = fs::File::create(&lock_path).expect("lock file");
        held.lock().expect("locked");

        let worker = {
            let (repair, namespace) = (repair.clone(), remounted.namespace.clone());
            std::thread::spawn(move || block_on(repair_state_root(&repair, &namespace)))
        };
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(!worker.is_finished(), "the repair waits for the lock");
        assert_eq!(
            remounted.marker(),
            marker_contents(ROOT_ID, &Remounted::identity_of(&remounted.root, 1)).as_bytes()
        );
        drop(held);
        let prepared = worker.join().expect("worker").expect("repaired");
        assert_eq!(prepared.root_id, ROOT_ID);
    }
}
