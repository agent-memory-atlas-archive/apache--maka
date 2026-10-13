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

//! A file on this machine that a reply names (the conversation's paths):
//! what a click on it does, decided from the file as it is when clicked,
//! never from what the reply wrote of it, and the reads the Files face's
//! preview of it makes.
//!
//! The path is resolved to what it names, symlinks followed, and that
//! decides ([`classify`]):
//! - nothing there: the face says the file is no longer there;
//! - a folder, a bundle such as an `.app` among them: shown in Finder;
//! - a file the preview draws ([`policy::local_kind`]: Markdown, text,
//!   source, an HTML page, an image, within the preview's limits): the
//!   Files face previews it;
//! - a document on [`policy::is_openable_document`]'s list: the default
//!   app opens it;
//! - anything else (a script, a program, a kind this does not know):
//!   shown in Finder. Nothing off that list is opened or launched.
//!
//! [`policy::local_kind`]: crate::policy::local_kind
//! [`policy::is_openable_document`]: crate::policy::is_openable_document

use std::fs::File;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use host_protocol::ArtifactKind;

use crate::policy::{Format, is_openable_document, local_kind};

/// What a click on a path does with the file there now.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathOpening {
    /// Nothing is there.
    Missing,
    /// Show this folder or file in Finder, selected in its folder.
    Reveal(PathBuf),
    /// Hand this document to the default app.
    OpenInDefaultApp(PathBuf),
    /// Preview this file in the Files face.
    Preview(LocalFile),
}

/// A file on this machine the Files face previews.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LocalFile {
    /// Where it is, symlinks resolved.
    pub path: PathBuf,
    /// Its name, the last part of `path`.
    pub name: String,
    /// How it previews: `file` (text), `html` or `image`.
    pub kind: ArtifactKind,
    pub size_bytes: u64,
    /// When it last changed, Unix milliseconds; 0 when the system does not
    /// say.
    pub modified_at: u64,
}

impl<'a> From<&'a LocalFile> for Format<'a> {
    fn from(file: &'a LocalFile) -> Self {
        Format::new(&file.kind, &file.name, None)
    }
}

/// What a click on `path` does, from what is there now. Looks at the
/// disk: run it off the main thread.
pub fn classify(path: &Path) -> PathOpening {
    let Ok(path) = std::fs::canonicalize(path) else { return PathOpening::Missing };
    let Ok(metadata) = std::fs::metadata(&path) else { return PathOpening::Missing };
    if !metadata.is_file() {
        return PathOpening::Reveal(path);
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()).map(str::to_owned) else {
        return PathOpening::Reveal(path);
    };
    if let Some(kind) = local_kind(&name, metadata.len()) {
        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX));
        return PathOpening::Preview(LocalFile {
            path,
            name,
            kind,
            size_bytes: metadata.len(),
            modified_at,
        });
    }
    if is_openable_document(&name) {
        return PathOpening::OpenInDefaultApp(path);
    }
    PathOpening::Reveal(path)
}

/// The bytes of the file at `path`, at most `limit` of them; a file grown
/// past `limit` since it was classified is an error. Blocking: run it off
/// the main thread.
pub(crate) fn read(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(io::Error::new(io::ErrorKind::FileTooLarge, "larger than the preview reads"));
    }
    Ok(bytes)
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)] // Tests write their scratch files.
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        let root = std::env::temp_dir()
            .join(format!("maka-files-local-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&root).expect("scratch");
        // Symlinks resolve to the real directory (`/var` is `/private/var`).
        std::fs::canonicalize(&root).expect("canonical")
    }

    #[test]
    fn a_path_is_decided_by_what_is_there() {
        let root = scratch();
        std::fs::write(root.join("report.docx"), b"PK").expect("docx");
        std::fs::write(root.join("notes.md"), b"# Notes").expect("md");
        std::fs::write(root.join("setup.command"), b"#!/bin/sh\n").expect("command");
        std::fs::create_dir_all(root.join("Tool.app/Contents")).expect("app");
        std::fs::create_dir_all(root.join("folder")).expect("folder");
        assert_eq!(
            classify(&root.join("report.docx")),
            PathOpening::OpenInDefaultApp(root.join("report.docx"))
        );
        for revealed in ["setup.command", "Tool.app", "folder"] {
            assert_eq!(
                classify(&root.join(revealed)),
                PathOpening::Reveal(root.join(revealed)),
                "{revealed}"
            );
        }
        let PathOpening::Preview(notes) = classify(&root.join("./notes.md")) else {
            panic!("a preview");
        };
        assert_eq!((notes.path, notes.name.as_str()), (root.join("notes.md"), "notes.md"));
        assert_eq!((notes.kind, notes.size_bytes), (ArtifactKind::File, 7));
        assert!(notes.modified_at > 0);
        assert_eq!(classify(&root.join("gone.docx")), PathOpening::Missing);
        std::fs::remove_dir_all(&root).ok();
    }

    /// A symlink named like a document is decided by what it points at.
    #[cfg(unix)]
    #[test]
    fn a_symlink_is_decided_by_its_target() {
        let root = scratch();
        std::fs::create_dir_all(root.join("folder")).expect("folder");
        std::fs::write(root.join("tool"), b"\x7fELF").expect("tool");
        std::fs::write(root.join("real.docx"), b"PK").expect("docx");
        std::os::unix::fs::symlink(root.join("folder"), root.join("x.docx")).expect("link");
        std::os::unix::fs::symlink(root.join("tool"), root.join("y.docx")).expect("link");
        std::os::unix::fs::symlink(root.join("real.docx"), root.join("z.md")).expect("link");
        std::os::unix::fs::symlink(root.join("nowhere"), root.join("w.docx")).expect("link");
        assert_eq!(classify(&root.join("x.docx")), PathOpening::Reveal(root.join("folder")));
        assert_eq!(classify(&root.join("y.docx")), PathOpening::Reveal(root.join("tool")));
        assert_eq!(
            classify(&root.join("z.md")),
            PathOpening::OpenInDefaultApp(root.join("real.docx"))
        );
        assert_eq!(classify(&root.join("w.docx")), PathOpening::Missing);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_read_stops_at_its_limit() {
        let root = scratch();
        std::fs::write(root.join("a.txt"), b"12345").expect("a");
        assert_eq!(read(&root.join("a.txt"), 5).expect("read"), b"12345");
        assert!(read(&root.join("a.txt"), 4).is_err());
        assert!(read(&root.join("gone.txt"), 4).is_err());
        std::fs::remove_dir_all(&root).ok();
    }
}
