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

//! Atomic durable application sources; no legacy composition translation.
mod changes;
mod source;
pub use source::SourceSnapshot;
pub mod checkpoints;
pub mod intents;
pub mod operations;
use crate::{EventLog, StoreError};
pub use changes::DeploymentChanges;
use maka_plugin_core::{
    composition::Input,
    deployment::{Deployment, DeploymentRef, RemoteObservation, management::ManagedDeployment},
    package::{PackageRef, Packages},
    plan::{Plan, RemoteExports},
};
use maka_plugins::package::Package;
use sqlx::{Connection, Row, SqliteConnection};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};

use uuid::Uuid;

const MAX_JSON: usize = 512 * 1024;
const MAX_PACKAGES: usize = 128;
const MAX_BYTES: usize = 128 * 1024 * 1024;

/// One read transaction captures the source and every immutable byte it selects.
pub struct LoadedDeployment {
    pub source: Deployment,
    pub packages: Vec<Package>,
    snapshot: SourceSnapshot,
}
impl LoadedDeployment {
    pub fn snapshot(&self) -> SourceSnapshot {
        self.snapshot.clone()
    }
}

fn invalid(message: impl Into<String>) -> StoreError {
    StoreError::InvalidTransition(message.into())
}
struct LimitedJson {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for LimitedJson {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("JSON exceeds byte limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn encode<T: serde::Serialize + ?Sized>(value: &T, limit: usize) -> Result<String, StoreError> {
    let mut writer = LimitedJson {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut writer, value)?;
    String::from_utf8(writer.bytes).map_err(|_| invalid("invalid JSON encoding"))
}
fn validate(
    input: &Input,
    observations: &[RemoteObservation],
    bytes: &[Package],
) -> Result<Plan, StoreError> {
    if bytes.len() > MAX_PACKAGES {
        return Err(invalid("application package count exceeds 128"));
    }
    let mut packages = Packages::default();
    let mut references = BTreeSet::new();
    let mut size = 0usize;
    for package in bytes {
        if !references.insert(package.application_ref()) {
            return Err(invalid("duplicate application package revision"));
        }
        size = size
            .checked_add(package.files().values().map(Vec::len).sum::<usize>())
            .filter(|n| *n <= MAX_BYTES)
            .ok_or_else(|| invalid("application packages exceed 128 MiB"))?;
        let declaration = package
            .manifest()
            .application
            .as_ref()
            .ok_or_else(|| invalid("application declaration required"))?;
        packages
            .insert(package.application_ref(), declaration.clone())
            .map_err(|e| invalid(e.to_string()))?;
    }
    let mut remote = RemoteExports::new();
    for observation in observations {
        if remote
            .insert(
                observation.capability.clone(),
                observation.declaration.clone(),
            )
            .is_some()
        {
            return Err(invalid("duplicate remote capability observation"));
        }
    }
    Plan::prepare(input, &packages, &remote).map_err(|e| invalid(e.to_string()))
}
/// Shared deterministic source checks, before any SQL effect. This validates
/// metadata and immutable bytes; it grants no deployment management authority.
pub fn validate_application(
    input: &Input,
    observations: &[RemoteObservation],
    bytes: &[Package],
) -> Result<(), StoreError> {
    let encoded = encode(input, MAX_JSON)?;
    encode(observations, MAX_JSON - encoded.len())?;
    validate(input, observations, bytes).map(|_| ())
}
impl EventLog {
    fn application_domain(&self) -> Result<String, StoreError> {
        self.validate_root()?;
        self.root_owner
            .as_ref()
            .map(|owner| owner.root_id().to_owned())
            .ok_or_else(|| invalid("application sources require an owned State Root"))
    }
    /// Commits desired input and exact bytes, not live readiness or convergence.
    /// Revision zero creates a deployment; every replacement is an explicit CAS.
    pub async fn commit_application(
        &self,
        deployment: Uuid,
        expected_revision: u64,
        input: Input,
        observations: Vec<RemoteObservation>,
        bytes: Vec<Package>,
    ) -> Result<DeploymentRef, StoreError> {
        let domain = self.application_domain()?;
        if deployment.is_nil() || expected_revision >= 9_007_199_254_740_991 {
            return Err(invalid("invalid deployment or exhausted revision"));
        }
        let encoded = encode(&input, MAX_JSON)?;
        let remote = encode(&observations, MAX_JSON - encoded.len())?;
        validate(&input, &observations, &bytes)?;
        let commits = self.commits.clone();
        self.connection.run(move |connection| Box::pin(async move {
            let mut tx = connection.begin_with("BEGIN IMMEDIATE").await?;
            let id = deployment.to_string();
            let current: Option<i64> = sqlx::query_scalar("SELECT revision FROM application_deployments WHERE id = ?")
                .bind(&id).fetch_optional(&mut *tx).await?;
            let current = current.unwrap_or(0) as u64;
            if current != expected_revision { return Err(StoreError::RevisionConflict {
                expected: expected_revision.to_string(), actual: current.to_string(),
            }); }
            let revision = expected_revision + 1;
            sqlx::query("INSERT INTO application_sources VALUES (?, ?, ?, ?)")
                .bind(&id).bind(revision as i64).bind(encoded).bind(remote).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO application_deployments VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision")
                .bind(&id).bind(revision as i64).execute(&mut *tx).await?;
            for package in bytes {
                let reference = package.application_ref();
                // Existing immutable content must match as well; never repair corruption by an overwrite.
                let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM application_packages WHERE id = ? AND revision = ?)")
                    .bind(&reference.id).bind(&reference.revision).fetch_one(&mut *tx).await?;
                if exists {
                    if read_package(&mut tx, &reference, MAX_BYTES).await?.files() != package.files() {
                        return Err(StoreError::EventConflict);
                    }
                } else {
                    sqlx::query("INSERT INTO application_packages VALUES (?, ?)").bind(&reference.id).bind(&reference.revision).execute(&mut *tx).await?;
                    for (path, payload) in package.files() {
                        sqlx::query("INSERT INTO application_package_files VALUES (?, ?, ?, ?)")
                            .bind(&reference.id).bind(&reference.revision).bind(path).bind(payload).execute(&mut *tx).await?;
                    }
                }
                sqlx::query("INSERT INTO application_source_packages VALUES (?, ?, ?, ?)")
                    .bind(&id).bind(revision as i64).bind(&reference.id).bind(&reference.revision).execute(&mut *tx).await?;
            }
            tx.commit().await.map_err(StoreError::CommitUnknown)?;
            commits.send_modify(|_| {});
            Ok(DeploymentRef { state_domain: domain, deployment, revision })
        })).await
    }

    pub async fn load_application(
        &self,
        deployment: Uuid,
    ) -> Result<Option<LoadedDeployment>, StoreError> {
        self.read_application(deployment, None).await
    }

    /// Reads a committed original revision, independent of the current head.
    pub async fn load_application_at(
        &self,
        reference: &DeploymentRef,
    ) -> Result<Option<LoadedDeployment>, StoreError> {
        reference.validate().map_err(invalid)?;
        if reference.state_domain != self.application_domain()? {
            return Err(invalid("foreign application state domain"));
        }
        self.read_application(reference.deployment, Some(reference.revision))
            .await
    }

    /// Bounded source and references from one original SQL snapshot. No package
    /// payload is read or claimed validated; executors still load exact bytes.
    pub async fn read_application_metadata(
        &self,
        deployment: Uuid,
    ) -> Result<Option<ManagedDeployment>, StoreError> {
        self.read_metadata(deployment, None).await
    }

    /// Bounded metadata of the exact committed revision, independent of head.
    pub async fn read_application_metadata_at(
        &self,
        reference: &DeploymentRef,
    ) -> Result<Option<ManagedDeployment>, StoreError> {
        reference.validate().map_err(invalid)?;
        if reference.state_domain != self.application_domain()? {
            return Err(invalid("foreign application state domain"));
        }
        self.read_metadata(reference.deployment, Some(reference.revision))
            .await
    }

    async fn read_metadata(
        &self,
        deployment: Uuid,
        revision: Option<u64>,
    ) -> Result<Option<ManagedDeployment>, StoreError> {
        let domain = self.application_domain()?;
        if deployment.is_nil() {
            return Err(invalid("invalid deployment"));
        }
        self.connection
            .run(move |connection| {
                Box::pin(async move {
                    let mut tx = connection.begin().await?;
                    let metadata = read_metadata(&mut tx, domain, deployment, revision).await?;
                    tx.commit().await?;
                    Ok(metadata)
                })
            })
            .await
    }

    async fn read_application(
        &self,
        deployment: Uuid,
        revision: Option<u64>,
    ) -> Result<Option<LoadedDeployment>, StoreError> {
        let domain = self.application_domain()?;
        if deployment.is_nil() {
            return Err(invalid("invalid deployment"));
        }
        let owner = std::sync::Arc::downgrade(self.root_owner.as_ref().expect("owned source Root"));
        self.connection
            .run(move |connection| {
                Box::pin(async move {
                    let mut tx = connection.begin().await?;
                    let Some(metadata) =
                        read_metadata(&mut tx, domain, deployment, revision).await?
                    else {
                        return Ok(None);
                    };
                    let mut packages = Vec::new();
                    let mut size = 0;
                    for reference in metadata.packages {
                        let package = read_package(&mut tx, &reference, MAX_BYTES - size).await?;
                        size += package.files().values().map(Vec::len).sum::<usize>();
                        if size > MAX_BYTES {
                            return Err(invalid("application packages exceed 128 MiB"));
                        }
                        packages.push(package);
                    }
                    let plan = validate(
                        &metadata.source.input,
                        &metadata.source.remote_exports,
                        &packages,
                    )?;
                    tx.commit().await?;
                    let snapshot = SourceSnapshot::new(
                        owner,
                        metadata.source.reference.clone(),
                        metadata.source.input.environment.location,
                        &plan,
                    );
                    Ok(Some(LoadedDeployment {
                        snapshot,
                        source: metadata.source,
                        packages,
                    }))
                })
            })
            .await
    }
}

async fn read_metadata(
    connection: &mut SqliteConnection,
    domain: String,
    deployment: Uuid,
    revision: Option<u64>,
) -> Result<Option<ManagedDeployment>, StoreError> {
    let id = deployment.to_string();
    let revision = match revision {
        Some(revision) => revision,
        None => match sqlx::query_scalar::<_, i64>(
            "SELECT revision FROM application_deployments WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(&mut *connection)
        .await?
        {
            Some(revision) => revision as u64,
            None => return Ok(None),
        },
    };
    let Some(row) = sqlx::query("SELECT revision,
                CASE WHEN length(CAST(input_json AS BLOB)) + length(CAST(remote_exports_json AS BLOB)) <= 524288 THEN input_json END AS input_json,
                CASE WHEN length(CAST(input_json AS BLOB)) + length(CAST(remote_exports_json AS BLOB)) <= 524288 THEN remote_exports_json END AS remote_exports_json
                FROM application_sources WHERE deployment = ? AND revision = ?")
                .bind(&id).bind(revision as i64).fetch_optional(&mut *connection).await? else { return Ok(None); };
    let input_json: String = row
        .try_get::<Option<String>, _>("input_json")?
        .ok_or_else(|| invalid("application source exceeds 512 KiB"))?;
    let remote_json: String = row
        .try_get::<Option<String>, _>("remote_exports_json")?
        .ok_or_else(|| invalid("application source exceeds 512 KiB"))?;
    let input: Input = serde_json::from_str(&input_json)?;
    let observations: Vec<RemoteObservation> = serde_json::from_str(&remote_json)?;
    let reference = DeploymentRef {
        state_domain: domain,
        deployment,
        revision: row.try_get::<i64, _>("revision")? as u64,
    };
    reference.validate().map_err(invalid)?;
    let refs = sqlx::query("SELECT CASE WHEN length(CAST(id AS BLOB)) <= 128 THEN id END AS id,
                CASE WHEN length(CAST(revision AS BLOB)) = 71 THEN revision END AS revision
                FROM application_source_packages WHERE deployment = ? AND source_revision = ? ORDER BY id, revision LIMIT 129")
                .bind(&id).bind(revision as i64).fetch_all(&mut *connection).await?;
    if refs.len() > MAX_PACKAGES {
        return Err(invalid("application package count exceeds 128"));
    }
    let mut packages = Vec::new();
    for row in refs {
        let reference = PackageRef {
            id: row
                .try_get::<Option<String>, _>("id")?
                .ok_or_else(|| invalid("invalid stored package id"))?,
            revision: row
                .try_get::<Option<String>, _>("revision")?
                .ok_or_else(|| invalid("invalid stored package revision"))?,
        };
        reference
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        packages.push(reference);
    }
    Ok(Some(ManagedDeployment {
        source: Deployment {
            reference,
            input,
            remote_exports: observations,
        },
        packages,
    }))
}

async fn read_package(
    connection: &mut SqliteConnection,
    reference: &PackageRef,
    budget: usize,
) -> Result<Package, StoreError> {
    use maka_plugins::package::{MAX_FILE_BYTES, MAX_FILES, MAX_PACKAGE_BYTES};
    let rows = sqlx::query("SELECT CASE WHEN length(CAST(path AS BLOB)) <= 512 THEN path END AS path,
        CASE WHEN typeof(payload) = 'blob' THEN length(payload) END AS bytes FROM application_package_files WHERE id = ? AND revision = ? ORDER BY path LIMIT 257")
        .bind(&reference.id).bind(&reference.revision).fetch_all(&mut *connection).await?;
    if rows.is_empty() || rows.len() > MAX_FILES {
        return Err(invalid("invalid stored application package file count"));
    }
    let mut size = 0;
    for row in &rows {
        let length = usize::try_from(
            row.try_get::<Option<i64>, _>("bytes")?
                .ok_or_else(|| invalid("stored package payload must be a BLOB"))?,
        )
        .map_err(|_| invalid("invalid package payload length"))?;
        size += length;
        if length > MAX_FILE_BYTES || size > MAX_PACKAGE_BYTES || size > budget {
            return Err(invalid("stored application package exceeds byte limits"));
        }
        let path: Option<&str> = row.try_get("path")?;
        maka_plugins::package::validate_path(
            path.ok_or_else(|| invalid("invalid stored package path"))?,
        )
        .map_err(|e| invalid(e.to_string()))?;
    }
    let mut files = BTreeMap::new();
    for row in rows {
        let path: String = row.try_get("path")?;
        let bytes: Vec<u8> = sqlx::query_scalar("SELECT payload FROM application_package_files WHERE id = ? AND revision = ? AND path = ?")
            .bind(&reference.id).bind(&reference.revision).bind(&path).fetch_one(&mut *connection).await?;
        files.insert(path, bytes);
    }
    let package = Package::new(files).map_err(|e| invalid(e.to_string()))?;
    if &package.application_ref() != reference {
        return Err(invalid("application package digest mismatch"));
    }
    Ok(package)
}

#[cfg(test)]
mod tests;
