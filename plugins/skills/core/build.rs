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

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

fn collect(root: &Path, directory: &Path, prefix: &str, files: &mut BTreeMap<String, PathBuf>) {
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut entries = fs::read_dir(directory)
        .expect("bundled skill directory")
        .map(|entry| entry.expect("bundled skill entry"))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type().expect("bundled skill file type");
        assert!(
            !kind.is_symlink(),
            "bundled skills contain only regular files/directories"
        );
        if kind.is_dir() {
            collect(root, &path, prefix, files);
        } else {
            assert!(kind.is_file());
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/");
            if relative == "SKILL.md" {
                continue;
            }
            let name = format!("{prefix}{relative}");
            assert!(
                files.insert(name, path).is_none(),
                "duplicate bundled resource"
            );
        }
    }
}

/// A build-time allowlist keeps artifacts, credentials and unrelated source out
/// even when the source distribution has no Git metadata.
fn collect_sources(repo: &Path, directory: &Path, files: &mut BTreeMap<String, PathBuf>) {
    println!("cargo:rerun-if-changed={}", directory.display());
    for entry in fs::read_dir(directory).expect("authoring source directory") {
        let entry = entry.expect("authoring source entry");
        let path = entry.path();
        let kind = entry.file_type().unwrap();
        assert!(
            !kind.is_symlink(),
            "authoring sources must not follow symlinks"
        );
        if kind.is_dir() {
            // Runtime implementations and generated/build output are not examples.
            continue;
        }
        if !kind.is_file() || path.extension().is_none_or(|extension| extension != "ts") {
            continue;
        }
        let relative = path
            .strip_prefix(repo)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        assert!(
            files
                .insert(format!("references/source/{relative}"), path)
                .is_none()
        );
    }
}

// Contracts are one generated object in the SDK. Publish its members separately
// so every contract remains readable through the skill's 1 MiB resource bound.
fn split_contracts(files: &mut BTreeMap<String, PathBuf>) {
    let original = files
        .remove("references/sdk/contracts.ts")
        .expect("SDK contracts");
    println!("cargo:rerun-if-changed={}", original.display());
    let source = fs::read_to_string(original).unwrap();
    let mut members = Vec::new();
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        if let Some(name) = line
            .strip_prefix("  ")
            .and_then(|line| line.strip_suffix(": {\n"))
        {
            if !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            {
                members.push((name.to_owned(), offset));
            }
        }
        offset += line.len();
    }
    assert!(!members.is_empty(), "generated contract layout changed");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("authoring-sdk");
    fs::create_dir_all(out.join("contracts")).unwrap();
    let header = source.split("// Generated from").next().unwrap();
    let mut index = header.to_owned();
    for (at, (name, start)) in members.iter().enumerate() {
        let end = members
            .get(at + 1)
            .map(|(_, offset)| *offset)
            .unwrap_or_else(|| source.rfind("} satisfies").expect("contract object ending"));
        let entry = source[*start..end].trim();
        let body = entry
            .strip_prefix(&format!("{name}: "))
            .unwrap()
            .strip_suffix(',')
            .unwrap();
        let path = out.join("contracts").join(format!("{name}.ts"));
        fs::write(&path, format!("{header}import type {{ Contract }} from '../generated/runtime.js';\nexport const {name} = {body} satisfies Contract;\n")).unwrap();
        files.insert(format!("references/sdk/contracts/{name}.ts"), path);
        index.push_str(&format!(
            "import {{ {name} }} from './contracts/{name}.js';\n"
        ));
    }
    index.push_str(&format!(
        "export const contracts = {{ {} }};\n",
        members
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    let path = out.join("contracts.ts");
    fs::write(&path, index).unwrap();
    files.insert("references/sdk/contracts.ts".into(), path);
}

fn main() {
    let crate_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let repo = crate_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut source = String::from("const BUNDLED: &[Bundled] = &[\n");
    for (id, relative) in [
        ("maka-cua", "platform/computer-use/skills/maka-cua"),
        ("maka-plugin-authoring", "skills/maka-plugin-authoring"),
    ] {
        let root = repo.join(relative);
        let mut files = BTreeMap::new();
        collect(&root, &root, "", &mut files);
        if id == "maka-plugin-authoring" {
            let sdk = repo.join("packages/plugin-sdk/src");
            collect(&sdk, &sdk, "references/sdk/", &mut files);
            split_contracts(&mut files);
            for relative in [
                "plugins/shared",
                "plugins/agent/composer-footer",
                "plugins/agent/composer",
                "plugins/agent/agent-screen",
                "plugins/application/authoring",
                "plugins/application/deployment",
                "products/maka/bundle/src",
            ] {
                collect_sources(repo, &repo.join(relative), &mut files);
            }
            let mut index = String::from(
                "# Sources from this build\n\nRead a path with the skill resource reader. Paths under `references/source/` preserve checkout paths; SDK imports resolve to the existing `references/sdk/` resources.\n\n",
            );
            for (name, path) in &files {
                if name.starts_with("references/source/") {
                    index.push_str(&format!(
                        "- `{name}` ({} bytes)\n",
                        fs::metadata(path).unwrap().len()
                    ));
                }
            }
            let index_path =
                PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("authoring-source-index.md");
            fs::write(&index_path, index).unwrap();
            files.insert("references/source-index.md".into(), index_path);
            assert!(
                files.len() <= 256,
                "authoring resources exceed runtime resource limit"
            );
            assert!(
                files
                    .values()
                    .all(|path| fs::metadata(path).unwrap().len() <= 1024 * 1024),
                "authoring resource exceeds file limit"
            );
            assert!(
                files
                    .values()
                    .map(|path| fs::metadata(path).unwrap().len())
                    .sum::<u64>()
                    <= 16 * 1024 * 1024,
                "authoring resources exceed byte budget"
            );
        }
        let document = root.join("SKILL.md");
        println!("cargo:rerun-if-changed={}", document.display());
        source.push_str(&format!(
            "Bundled {{ id: {id:?}, content: include_str!({:?}), files: &[\n",
            document.to_str().unwrap()
        ));
        for (name, path) in files {
            if path.starts_with(repo) {
                println!("cargo:rerun-if-changed={}", path.display());
            }
            source.push_str(&format!(
                "({name:?}, include_bytes!({:?})),\n",
                path.to_str().unwrap()
            ));
        }
        source.push_str("] },\n");
    }
    source.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("bundled.rs"),
        source,
    )
    .unwrap();
}
