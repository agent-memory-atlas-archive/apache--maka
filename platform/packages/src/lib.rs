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

mod bundle;
mod io;
mod manifest;
mod path;
pub use io::PackageIoError;
pub use manifest::Manifest;
pub use path::validate_path;

use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub const MAX_FILES: usize = 256;
pub const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_PACKAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MANIFEST_FILE: &str = "maka.extension.json";

/// Validated immutable bytes. Retained instances keep their exact package alive.
#[derive(Clone, Debug)]
pub struct Package {
    manifest: Arc<Manifest>,
    files: Arc<BTreeMap<String, Vec<u8>>>,
    digest: Arc<str>,
}

impl Package {
    pub fn new(files: BTreeMap<String, Vec<u8>>) -> Result<Self, Error> {
        if files.is_empty() || files.len() > MAX_FILES {
            return Err(Error::Invalid("package must contain 1..=256 files".into()));
        }
        let mut total = 0;
        let mut paths = BTreeSet::new();
        for (path, content) in &files {
            validate_path(path)?;
            if !paths.insert(path.to_lowercase()) {
                return Err(Error::Invalid("package has case-colliding paths".into()));
            }
            total += content.len();
            if content.len() > MAX_FILE_BYTES || total > MAX_PACKAGE_BYTES {
                return Err(Error::Invalid("package exceeds byte limits".into()));
            }
        }
        for path in &paths {
            for (index, _) in path.match_indices('/') {
                if paths.contains(&path[..index]) {
                    return Err(Error::Invalid("package file is also a directory".into()));
                }
            }
        }
        let bytes = files
            .get(MANIFEST_FILE)
            .ok_or_else(|| Error::Invalid("package manifest is missing".into()))?;
        let manifest: Manifest = serde_json::from_slice(bytes)
            .map_err(|error| Error::Invalid(format!("manifest: {error}")))?;
        manifest.validate()?;
        for entry in manifest
            .application
            .iter()
            .flat_map(|application| application.entries.values())
        {
            if let maka_plugin_core::package::Implementation::JavaScript { module, .. } =
                &entry.implementation
                && !files.contains_key(module)
            {
                return Err(Error::Invalid(format!(
                    "package entry is missing: {module}"
                )));
            }
        }
        let digest = content_digest(
            files
                .iter()
                .map(|(path, bytes)| (path.as_str(), bytes.as_slice())),
        );
        Ok(Self {
            manifest: Arc::new(manifest),
            files: Arc::new(files),
            digest: digest.into(),
        })
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn application_ref(&self) -> maka_plugin_core::package::PackageRef {
        maka_plugin_core::package::PackageRef {
            id: self.manifest.id.clone(),
            revision: self.digest.to_string(),
        }
    }
    pub fn files(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.files
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn file(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }
}

/// Exact bundle identity, independent of declaration parsing or execution admission.
pub fn content_digest<'a>(files: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> String {
    // The existing bundle format sorts paths as JavaScript strings (UTF-16).
    let mut files: Vec<_> = files.into_iter().collect();
    files.sort_unstable_by(|(left, _), (right, _)| left.encode_utf16().cmp(right.encode_utf16()));
    let mut hash = Sha256::new();
    for (path, content) in files {
        hash.update((path.len() as u64).to_be_bytes());
        hash.update(path.as_bytes());
        hash.update((content.len() as u64).to_be_bytes());
        hash.update(content);
    }
    format!("sha256-{:x}", hash.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn author_descriptions_round_trip_with_localized_copy() {
        let manifest = json!({"schemaVersion":2,"id":"example.notes", "displayName":"Notes", "description":"Write and find your notes.", "localizations":{"zh-CN":{"displayName":"笔记","description":"记录和查找笔记。"}}, "application":{"entries":{"host":{"implementation":{"kind":"native","implementation":"example.notes"},"configuration":{"type":"object"}}}}});
        let package = Package::new(BTreeMap::from([(
            MANIFEST_FILE.into(),
            serde_json::to_vec(&manifest).unwrap(),
        )]))
        .unwrap();
        let restored = Package::from_bundle(&package.to_bundle()).unwrap();
        let presentation = restored.manifest().presentation();
        assert_eq!(presentation.display_name.as_str(), "Notes");
        assert_eq!(
            presentation.localizations["zh-CN"].description.as_str(),
            "记录和查找笔记。"
        );
    }

    #[test]
    fn manifest_uses_the_package_file_budget() {
        // Multi-presentation applications carry complete nominal contracts.
        // Their manifest has the same bounded byte budget as every other file.
        let manifest = json!({
            "schemaVersion":2,"id":"large.application",
            "application":{"entries":{"host":{
                "implementation":{"kind":"native","implementation":"example.host"},
                "configuration":{"description":"a".repeat(300 * 1024)}
            }}}
        });
        let bytes = serde_json::to_vec(&manifest).unwrap();
        assert!(bytes.len() > 256 * 1024);
        let files = BTreeMap::from([(MANIFEST_FILE.into(), bytes)]);
        let package = Package::new(files).unwrap();
        assert_eq!(
            Package::from_bundle(&package.to_bundle()).unwrap().digest(),
            package.digest()
        );
        let too_large = BTreeMap::from([(MANIFEST_FILE.into(), vec![b' '; MAX_FILE_BYTES + 1])]);
        assert!(
            matches!(Package::new(too_large), Err(Error::Invalid(reason)) if reason == "package exceeds byte limits")
        );
    }

    #[test]
    fn application_packages_validate_variants_and_allow_pure_composition() {
        let declaration = json!({
            "entries": {"desktop": {
                "implementation": {"kind":"javascript", "module":"desktop.mjs", "realm":"dedicated"},
                "configuration": {"type":"object"}, "imports": {}, "exports": {}
            }}
        });
        let files = BTreeMap::from([
            (
                MANIFEST_FILE.into(),
                serde_json::to_vec(
                    &json!({"schemaVersion":2,"id":"example","application":declaration}),
                )
                .unwrap(),
            ),
            ("desktop.mjs".into(), b"export default {}".to_vec()),
        ]);
        let package = Package::new(files.clone()).unwrap();
        let reference = package.application_ref();
        reference.validate().unwrap();
        assert_eq!(reference.revision, package.digest());
        assert_eq!(
            Package::from_bundle(&package.to_bundle())
                .unwrap()
                .application_ref(),
            reference
        );
        let mut missing = files.clone();
        missing.remove("desktop.mjs");
        assert!(Package::new(missing).is_err());
        let mut old = files.clone();
        let mut manifest: serde_json::Value = serde_json::from_slice(&old[MANIFEST_FILE]).unwrap();
        manifest["schemaVersion"] = json!(1);
        old.insert(MANIFEST_FILE.into(), serde_json::to_vec(&manifest).unwrap());
        assert!(Package::new(old).is_err());
        let mut mixed = files;
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&mixed[MANIFEST_FILE]).unwrap();
        manifest["runtime"] = json!({"entry":"desktop.mjs","sdkVersion":3});
        mixed.insert(MANIFEST_FILE.into(), serde_json::to_vec(&manifest).unwrap());
        assert!(Package::new(mixed).is_err());

        let structural = Package::new(BTreeMap::from([(
            MANIFEST_FILE.into(),
            serde_json::to_vec(&json!({
                "schemaVersion":2,"id":"product","application":{"composition":{"children":{}}}
            }))
            .unwrap(),
        )]))
        .unwrap();
        assert!(
            structural
                .manifest()
                .application
                .as_ref()
                .unwrap()
                .composition
                .is_some()
        );
    }

    #[test]
    fn package_pins_exact_bytes_and_rejects_cross_platform_path_aliases() {
        let manifest = serde_json::to_vec(&json!({
            "schemaVersion":2,"id":"graph","application":{"entries":{"host":{"implementation":{"kind":"native","implementation":"example.host"},"configuration":{"type":"object"}}}}
        })).unwrap();
        let files = BTreeMap::from([
            (MANIFEST_FILE.into(), manifest),
            ("index.mjs".into(), b"export default {}".to_vec()),
        ]);
        let package = Package::new(files.clone()).unwrap();
        let bundle = package.to_bundle();
        assert_eq!(
            Package::from_bundle(&bundle).unwrap().digest(),
            package.digest()
        );
        let mut corrupt: serde_json::Value = serde_json::from_slice(&bundle).unwrap();
        corrupt["files"][0]["content"] = json!("Y29ycnVwdA==");
        assert!(Package::from_bundle(&serde_json::to_vec(&corrupt).unwrap()).is_err());
        let mut updated = files.clone();
        updated.insert(
            "index.mjs".into(),
            b"export default { updated: true }".to_vec(),
        );
        assert_ne!(package.digest(), Package::new(updated).unwrap().digest());
        assert_eq!(package.file("index.mjs").unwrap(), b"export default {}");
        for path in [
            "../escape",
            "INDEX.MJS",
            "index.mjs/child",
            "dir/CON.txt",
            "C:payload",
            "trailing.",
        ] {
            let mut invalid = files.clone();
            invalid.insert(path.into(), vec![]);
            assert!(Package::new(invalid).is_err(), "{path}");
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("invalid package data: {0}")]
    Invalid(String),
}
