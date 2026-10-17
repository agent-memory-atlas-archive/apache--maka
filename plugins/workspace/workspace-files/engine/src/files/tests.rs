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
use std::sync::Arc;

struct Fixture {
    temp: tempfile::TempDir,
    root: PathBuf,
    private: PathBuf,
    control: PathBuf,
    identity: DirectoryIdentity,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir(&root).unwrap();
        let private = root.join("business-root");
        let control = root.join("control");
        fs::create_dir(&private).unwrap();
        fs::create_dir(&control).unwrap();
        let (root, identity) = crate::directory::capture(&root).unwrap();
        Self {
            temp,
            root,
            private,
            control,
            identity,
        }
    }
    fn open(&self, path: Option<&WorkspacePath>) -> io::Result<Directory> {
        Directory::open(
            &self.root,
            &self.identity,
            &self.private,
            &self.control,
            None,
            path,
            &AtomicBool::new(false),
        )
    }
}
fn names(page: &FilePage) -> Vec<Vec<u8>> {
    page.entries
        .iter()
        .map(|e| e.path.bytes().unwrap())
        .collect()
}

#[test]
fn bounded_live_pages_preserve_names_and_reject_foreign_directory_cursors() {
    let f = Fixture::new();
    for i in 0..130 {
        fs::write(f.root.join(format!("{i:03}")), "").unwrap();
    }
    let root = f.open(None).unwrap();
    let first = root.list(None, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        names(&first),
        (0..128)
            .map(|i| format!("{i:03}").into_bytes())
            .collect::<Vec<_>>()
    );
    let second = root
        .list(first.next.as_ref(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(names(&second), vec![b"128".to_vec(), b"129".to_vec()]);
    assert_eq!(second.next, None);
    fs::create_dir(f.root.join("nested")).unwrap();
    let path = WorkspacePath::from_bytes(b"nested").unwrap();
    let nested = f.open(Some(&path)).unwrap();
    assert_eq!(
        nested
            .list(first.next.as_ref(), &AtomicBool::new(false))
            .unwrap_err()
            .to_string(),
        "files_invalid_cursor"
    );
    assert!(serde_json::to_vec(&first).unwrap().len() <= MAX_PAGE_BYTES);
    assert_eq!(
        root.list(None, &AtomicBool::new(true))
            .unwrap_err()
            .to_string(),
        "files_cancelled"
    );
}

#[cfg(unix)]
#[test]
fn listing_keeps_hidden_raw_names_and_dangling_links_but_prunes_original_private_roots() {
    use std::os::unix::{ffi::OsStringExt, fs::symlink};
    let f = Fixture::new();
    fs::write(f.root.join(".hidden"), "").unwrap();
    fs::create_dir(f.root.join(".git")).unwrap();
    fs::write(f.root.join(crate::workspace::MARKER_FILE), "private marker").unwrap();
    symlink("missing", f.root.join("dangling")).unwrap();
    #[cfg(target_os = "macos")]
    let raw = b"raw-\\name";
    #[cfg(not(target_os = "macos"))]
    let raw = b"raw-\xff\\name";
    fs::write(f.root.join(std::ffi::OsString::from_vec(raw.to_vec())), "").unwrap();
    let directory = f.open(None).unwrap();
    let page = directory.list(None, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        names(&page),
        vec![b".hidden".to_vec(), b"dangling".to_vec(), raw.to_vec()]
    );
    assert_eq!(
        page.entries.iter().map(|e| e.kind).collect::<Vec<_>>(),
        vec![FileKind::File, FileKind::Symlink, FileKind::File]
    );
    let link = WorkspacePath::from_bytes(b"dangling").unwrap();
    assert!(f.open(Some(&link)).is_err());
    let private = WorkspacePath::from_bytes(b"business-root").unwrap();
    assert_eq!(
        f.open(Some(&private)).err().unwrap().to_string(),
        "files_private"
    );
}

#[tokio::test]
async fn native_watch_wakes_for_create_and_closes_before_original_custody_is_released() {
    let f = Fixture::new();
    let directory = Arc::new(f.open(None).unwrap());
    let setup = directory.clone();
    let mut watch = tokio::task::spawn_blocking(move || setup.watch())
        .await
        .unwrap()
        .unwrap();
    fs::write(f.root.join("created"), "new\n").unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), watch.changed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        names(&directory.list(None, &AtomicBool::new(false)).unwrap()),
        vec![b"created".to_vec()]
    );
    watch.close().await.unwrap();
    fs::write(f.root.join("after-close"), "closed\n").unwrap();
    assert_eq!(
        names(&directory.list(None, &AtomicBool::new(false)).unwrap()),
        vec![b"after-close".to_vec(), b"created".to_vec()]
    );
}

#[tokio::test]
async fn ancestor_move_wakes_without_reading_the_moved_outside_directory() {
    let f = Fixture::new();
    fs::create_dir_all(f.root.join("parent/target")).unwrap();
    let path = WorkspacePath::from_bytes(b"parent/target").unwrap();
    let directory = Arc::new(f.open(Some(&path)).unwrap());
    let setup = directory.clone();
    let mut watch = tokio::task::spawn_blocking(move || setup.watch())
        .await
        .unwrap()
        .unwrap();
    let moved = f.temp.path().join("outside");
    fs::rename(f.root.join("parent"), &moved).unwrap();
    fs::write(moved.join("target/private-outside"), "outside sentinel\n").unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), watch.changed())
        .await
        .unwrap()
        .unwrap();
    let result = directory.list(None, &AtomicBool::new(false));
    watch.close().await.unwrap();
    assert_eq!(result.unwrap_err().to_string(), "files_changed");
}
