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

//! Text authoring uses the same loader, revision checks and commit as native captures.
use super::*;
use maka_plugin_core::deployment::management::{
    DeploymentSourceReplace, PackageSourceChunk, PackageSourceInstall, PackageSourceRead,
};
impl Manager {
    pub(super) async fn install_source(&self, input: Value) -> Result<Value> {
        let request: PackageSourceInstall =
            serde_json::from_value(input).map_err(|e| e.to_string())?;
        let _write = self.writes.lock().await;
        let metadata = self.metadata().await?;
        if request.expected_revision != metadata.source.reference.revision {
            return conflict(metadata.source.reference.revision);
        }
        let package = match Package::new(
            request
                .files
                .into_iter()
                .map(|(path, text)| (path, text.into_bytes()))
                .collect(),
        ) {
            Ok(package) => package,
            Err(error) => {
                return rejected_diagnostic(ManagementRejection::InvalidBundle, error.to_string());
            }
        };
        self.install_package(request.expected_revision, package, metadata)
            .await
    }
    pub(super) async fn replace_source(&self, input: Value) -> Result<Value> {
        let request: DeploymentSourceReplace =
            serde_json::from_value(input).map_err(|e| e.to_string())?;
        let _write = self.writes.lock().await;
        let metadata = self.metadata().await?;
        if request.expected_revision != metadata.source.reference.revision {
            return conflict(metadata.source.reference.revision);
        }
        self.replace_document(request.expected_revision, request.document, metadata)
            .await
    }
    pub(super) async fn read_package(&self, input: Value) -> Result<Value> {
        let request: PackageSourceRead =
            serde_json::from_value(input).map_err(|e| e.to_string())?;
        let _read = self.writes.lock().await;
        let loaded = self.source_at(Some(request.source)).await?;
        let package = loaded
            .packages()
            .iter()
            .find(|p| p.application_ref() == request.package)
            .ok_or("package is not installed in this UI source")?;
        let paths = package.files().keys().cloned().collect();
        let (text, next) = if let Some(path) = request.path {
            let text = std::str::from_utf8(package.file(&path).ok_or("package file unavailable")?)
                .map_err(|_| "package file is not UTF-8")?;
            if request.offset > text.len() || !text.is_char_boundary(request.offset) {
                return Err("invalid package byte offset".into());
            }
            let mut end = request.offset.saturating_add(16384).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1
            }
            (
                Some(text[request.offset..end].to_owned()),
                (end < text.len()).then_some(end),
            )
        } else {
            (None, None)
        };
        serde_json::to_value(PackageSourceChunk { paths, text, next }).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maka_event_log::root::{RootNamespaces, RootOwner};
    use maka_plugin_core::composition::{Environment, Input};
    use serde_json::json;

    #[tokio::test]
    async fn text_authoring_uses_original_source_revision_and_package_validation() {
        let dir = tempfile::tempdir().unwrap();
        let owner = Arc::new(
            RootOwner::create(
                &dir.path().join("root"),
                &RootNamespaces {
                    ownership: dir.path().join("owners"),
                    control: dir.path().join("control"),
                },
            )
            .unwrap(),
        );
        let store = Arc::new(EventLog::for_root(owner).await.unwrap());
        let environment = Environment {
            location: ExecutionLocation::Host,
            presentation: None,
        };
        let input = Input {
            environment,
            instances: BTreeMap::new(),
            recovery: None,
            launch: None,
        };
        let source = store
            .commit_application(uuid::Uuid::new_v4(), 0, input.clone(), vec![], vec![])
            .await
            .unwrap();
        let manager = ManagementTarget::from_source(store.clone(), source.clone())
            .await
            .unwrap()
            .0;
        let files = json!({
            "maka.extension.json":json!({"schemaVersion":2,"id":"example.footer","application":{"entries":{"host":{"implementation":{"kind":"javascript","module":"host.mjs","realm":"shared"},"configuration":{"type":"object"},"imports":{},"exports":{}}}}}).to_string(),
            "host.mjs":"export function activate() { return {}; } // 界面"
        });
        let installed = manager
            .install_source(json!({"expectedRevision":1,"files":files}))
            .await
            .unwrap();
        let installed: ManagementResult<PackageInstalled> =
            serde_json::from_value(installed).unwrap();
        let ManagementResult::Committed { value } = installed else {
            panic!("valid package rejected")
        };
        assert_eq!(value.source.revision, 2);
        let read = manager
            .read_package(
                json!({"source":value.source,"package":value.package,"path":"host.mjs","offset":0}),
            )
            .await
            .unwrap();
        assert!(read["text"].as_str().unwrap().ends_with("界面"));
        let stale = manager
            .install_source(json!({"expectedRevision":1,"files":files}))
            .await
            .unwrap();
        assert_eq!(stale["status"], "conflict");
        let invalid = manager
            .install_source(json!({"expectedRevision":2,"files":{"../outside":"invalid"}}))
            .await
            .unwrap();
        assert_eq!(invalid["status"], "rejected");
        let mut foreign = value.source.clone();
        foreign.deployment = uuid::Uuid::new_v4();
        assert!(
            manager
                .read_package(
                    json!({"source":foreign,"package":value.package,"path":null,"offset":0})
                )
                .await
                .is_err()
        );
        let replaced=manager.replace_source(json!({"expectedRevision":2,"document":{"input":input,"remoteExports":[],"packages":[value.package]}})).await.unwrap();
        assert_eq!(replaced["status"], "committed");
        assert_eq!(
            store
                .read_application_metadata(source.deployment)
                .await
                .unwrap()
                .unwrap()
                .source
                .reference
                .revision,
            3
        );
    }
}
