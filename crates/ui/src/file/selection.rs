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

use super::*;
use crate::budget::{Kind, NativeBudget, Permit};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use maka_plugins::call::ReadResource;
use sha2::{Digest, Sha256};
use std::{io::Read, ops::Range, path::Path, sync::Arc};

const CHUNK: usize = 64 * 1024;
struct Chunk {
    bytes: Vec<u8>,
    _permit: Permit,
}
/// Contains only immutable bytes, descriptors and allocation permits, no native lease.
pub struct FileSnapshot {
    chunks: Vec<Chunk>,
    descriptor: FileDescriptor,
}
impl ReadResource for FileSnapshot {
    fn resource_type(&self) -> maka_plugin_core::contract::ResourceType {
        binary_resource_type()
    }
    fn bytes(&self) -> usize {
        self.descriptor.bytes
    }
    fn metadata(&self) -> Value {
        serde_json::to_value(&self.descriptor).expect("file descriptor")
    }
    fn read(&self, range: Range<usize>) -> Result<Vec<u8>, String> {
        if range.end > self.bytes() || range.end.checked_sub(range.start).is_none_or(|n| n > CHUNK)
        {
            return Err("invalid file resource range or exceeds 64 KiB".into());
        }
        let mut out = Vec::with_capacity(range.end - range.start);
        let mut start = range.start;
        while start < range.end {
            let chunk = &self.chunks[start / CHUNK].bytes;
            let offset = start % CHUNK;
            let count = (chunk.len() - offset).min(range.end - start);
            out.extend_from_slice(&chunk[offset..offset + count]);
            start += count;
        }
        Ok(out)
    }
}
/// Runs only in an admitted original native worker, after actual user selection.
pub fn read_selected(path: &Path, budget: &NativeBudget) -> Result<Arc<FileSnapshot>, FileError> {
    if !path.is_absolute() {
        return Err(FileError::InvalidFile);
    }
    let name = path.file_name().ok_or(FileError::InvalidFile)?;
    let directory = Dir::open_ambient_dir(
        path.parent().ok_or(FileError::InvalidFile)?,
        ambient_authority(),
    )
    .map_err(|_| FileError::ReadFailed)?;
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = directory
        .open_with(name, &options)
        .map_err(|_| FileError::InvalidFile)?;
    let metadata = file.metadata().map_err(|_| FileError::ReadFailed)?;
    if !metadata.is_file() {
        return Err(FileError::InvalidFile);
    }
    if metadata.len() > MAX_FILE_BYTES as u64 {
        return Err(FileError::LimitExceeded);
    }
    let mut chunks = Vec::new();
    let mut total = 0;
    let mut hash = Sha256::new();
    loop {
        // Probe without an allocation: exact-size and empty files need no extra permit.
        let mut first = [0];
        if file.read(&mut first).map_err(|_| FileError::ReadFailed)? == 0 {
            break;
        }
        if total == MAX_FILE_BYTES {
            return Err(FileError::LimitExceeded);
        }
        let permit = budget.reserve(Kind::Encoded, CHUNK)?;
        let mut bytes = Vec::with_capacity(CHUNK);
        bytes.push(first[0]);
        (&mut file)
            .take((CHUNK - 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| FileError::ReadFailed)?;
        hash.update(&bytes);
        total += bytes.len();
        let last = bytes.len() < CHUNK;
        chunks.push(Chunk {
            bytes,
            _permit: permit,
        });
        if last {
            break;
        }
    }
    let after = file.metadata().map_err(|_| FileError::ReadFailed)?;
    if total as u64 != metadata.len()
        || after.len() != metadata.len()
        || after.modified().ok() != metadata.modified().ok()
    {
        return Err(FileError::ReadFailed);
    }
    Ok(Arc::new(FileSnapshot {
        chunks,
        descriptor: FileDescriptor {
            name: name.to_string_lossy().chars().take(512).collect(),
            bytes: total,
            digest: format!("sha256:{:x}", hash.finalize()),
        },
    }))
}

/// Original native selection owner; no path or content in its public observations.
pub struct FileSelection {
    observation: FileObservation,
    snapshot: Option<Arc<FileSnapshot>>,
    intent: Option<String>,
}
impl Default for FileSelection {
    fn default() -> Self {
        Self {
            observation: FileObservation {
                owner: uuid::Uuid::new_v4().to_string(),
                revision: 0,
                phase: FilePhase::Empty,
                file: None,
                error: None,
            },
            snapshot: None,
            intent: None,
        }
    }
}
impl FileSelection {
    pub fn observation(&self) -> FileObservation {
        self.observation.clone()
    }
    pub fn busy(&self) -> bool {
        self.observation.phase == FilePhase::Choosing
    }
    /// Called by actual renderer input, never by props or component reduction.
    pub fn stage(&mut self) -> Option<FileChoose> {
        if self.busy() || self.intent.is_some() {
            return None;
        }
        let request = uuid::Uuid::new_v4().to_string();
        self.intent = Some(request.clone());
        Some(FileChoose { request })
    }
    pub fn cancel_intent(&mut self) {
        self.intent = None;
    }
    pub fn accept(&mut self, request: Option<&str>) -> bool {
        if request.is_none() || self.intent.as_deref() != request {
            return false;
        }
        self.intent = None;
        self.observation.revision += 1;
        self.observation.phase = FilePhase::Choosing;
        self.observation.error = None;
        true
    }
    pub fn settle(&mut self, result: Result<Option<Arc<FileSnapshot>>, FileError>) {
        self.observation.revision += 1;
        match result {
            Ok(Some(snapshot)) => {
                self.observation.file = Some(snapshot.descriptor.clone());
                self.snapshot = Some(snapshot);
                self.observation.phase = FilePhase::Ready;
                self.observation.error = None;
            }
            Ok(None) => {
                self.observation.phase = if self.snapshot.is_some() {
                    FilePhase::Ready
                } else {
                    FilePhase::Empty
                };
                self.observation.error = None;
            }
            Err(error) => {
                self.observation.phase = FilePhase::Failed;
                self.observation.error = Some(error);
            }
        }
    }
    pub fn capture(&self, capture: &FileCapture) -> crate::Result<Arc<dyn ReadResource>> {
        if capture.owner != self.observation.owner
            || capture.revision != self.observation.revision
            || self.observation.phase != FilePhase::Ready
        {
            return Err(crate::Error::Invalid(
                "file selection owner or revision unavailable".into(),
            ));
        }
        self.snapshot
            .clone()
            .map(|s| s as Arc<dyn ReadResource>)
            .ok_or_else(|| crate::Error::Invalid("file selection empty".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_keeps_original_bytes_and_permits_across_filesystem_and_owner_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("资料.bin");
        let bytes: Vec<u8> = (0..CHUNK + 17).map(|n| n as u8).collect();
        std::fs::write(&path, &bytes).unwrap();
        let budget = NativeBudget::default();
        let original = read_selected(&path, &budget).unwrap();
        assert_eq!(budget.usage(), (CHUNK * 2, 0, 0));
        assert_eq!(
            original.read(CHUNK - 9..CHUNK + 17).unwrap(),
            bytes[CHUNK - 9..]
        );
        assert!(original.read(0..CHUNK + 1).is_err());
        let mut selection = FileSelection::default();
        assert!(!selection.accept(Some("forged-marker")));
        let intent = selection.stage().unwrap();
        assert!(selection.accept(Some(&intent.request)));
        assert!(!selection.accept(Some(&intent.request)));
        selection.settle(Ok(Some(original.clone())));
        let observed = selection.observation();
        let resource = selection
            .capture(&FileCapture {
                owner: observed.owner.clone(),
                revision: observed.revision,
            })
            .unwrap();
        assert_eq!(resource.resource_type(), binary_resource_type());
        assert_eq!(resource.metadata()["name"], "资料.bin");
        assert_eq!(resource.metadata().as_object().unwrap().len(), 3);
        std::fs::write(&path, b"replaced").unwrap();
        assert_eq!(resource.read(0..7).unwrap(), bytes[..7]);
        assert!(
            selection
                .capture(&FileCapture {
                    owner: observed.owner,
                    revision: observed.revision - 1
                })
                .is_err()
        );
        drop(selection);
        drop(original);
        assert_eq!(budget.usage(), (CHUNK * 2, 0, 0));
        drop(resource);
        assert_eq!(budget.usage(), (0, 0, 0));
    }
    #[test]
    fn regular_file_limits_and_failed_reads_release_original_allocations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("selected");
        std::fs::write(&path, vec![1; CHUNK + 1]).unwrap();
        let budget = NativeBudget::new(crate::budget::BudgetLimits {
            encoded_bytes: CHUNK,
            ..Default::default()
        });
        assert!(matches!(
            read_selected(&path, &budget),
            Err(FileError::BudgetExhausted)
        ));
        assert_eq!(budget.usage(), (0, 0, 0));
        assert!(matches!(
            read_selected(dir.path(), &budget),
            Err(FileError::InvalidFile)
        ));
        assert!(matches!(
            read_selected(Path::new("relative"), &budget),
            Err(FileError::InvalidFile)
        ));
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len(MAX_FILE_BYTES as u64 + 1).unwrap();
        assert!(matches!(
            read_selected(&path, &budget),
            Err(FileError::LimitExceeded)
        ));
        file.set_len(MAX_FILE_BYTES as u64).unwrap();
        let full_budget = NativeBudget::default();
        let full = read_selected(&path, &full_budget).unwrap();
        assert_eq!(full.bytes(), 50 * 1024 * 1024);
        assert_eq!(full_budget.usage(), (MAX_FILE_BYTES, 0, 0));
        drop(full);
        assert_eq!(full_budget.usage(), (0, 0, 0));
        file.set_len(0).unwrap();
        let empty = read_selected(&path, &budget).unwrap();
        assert_eq!(empty.bytes(), 0);
        assert_eq!(budget.usage(), (0, 0, 0));
        #[cfg(unix)]
        {
            let link = dir.path().join("link");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(matches!(
                read_selected(&link, &budget),
                Err(FileError::InvalidFile)
            ));
        }
    }
}
