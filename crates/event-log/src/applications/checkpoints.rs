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

//! Opaque native UI snapshots, scoped to an actual persisted UI instance.
use super::{encode, invalid};
use crate::{EventLog, StoreError};
use maka_plugin_core::{
    composition::{ExecutionLocation, InstanceId},
    deployment::DeploymentRef,
    package::Packages,
    plan::{Plan, RemoteExports},
};
use serde_json::Value;
use sqlx::{Connection, Row};
use std::sync::Arc;

pub const MAX_CONTENT: usize = 1024 * 1024;
pub const MAX_METADATA: usize = 32 * 1024;
pub const MAX_SOURCE: usize = 4096;
const MAX_NAMESPACE: usize = 40 * 1024 * 1024;
const MAX_REVISION: u64 = (1 << 53) - 1;

#[derive(Clone, Debug, PartialEq)]
pub struct Checkpoint {
    pub revision: u64,
    pub format: String,
    pub data_source: Value,
    pub metadata: Value,
    pub content: Vec<u8>,
}
/// This capability comes from a checked original source, never guest JSON.
/// Holding it retains the original EventLog and its actual Root custody.
#[derive(Clone)]
pub struct Checkpoints {
    store: Arc<EventLog>,
    deployment: String,
    instance: String,
}
fn key_valid(key: &str) -> Result<(), StoreError> {
    if key.is_empty() || key.len() > 512 || key.chars().any(char::is_control) {
        return Err(invalid("invalid UI checkpoint key"));
    }
    Ok(())
}
impl EventLog {
    pub async fn ui_checkpoints(
        self: &Arc<Self>,
        source: &DeploymentRef,
        instance: &InstanceId,
    ) -> Result<Checkpoints, StoreError> {
        let loaded = self
            .load_application_at(source)
            .await?
            .ok_or_else(|| invalid("original UI checkpoint source missing"))?;
        if loaded.source.input.environment.location != ExecutionLocation::Ui {
            return Err(invalid("UI checkpoints require a UI deployment"));
        }
        let mut packages = Packages::default();
        for package in loaded.packages {
            packages
                .insert(
                    package.application_ref(),
                    package
                        .manifest()
                        .application
                        .clone()
                        .ok_or_else(|| invalid("application declaration required"))?,
                )
                .map_err(|error| invalid(error.to_string()))?;
        }
        let remotes: RemoteExports = loaded
            .source
            .remote_exports
            .into_iter()
            .map(|remote| (remote.capability, remote.declaration))
            .collect();
        let plan = Plan::prepare(&loaded.source.input, &packages, &remotes)
            .map_err(|error| invalid(error.to_string()))?;
        if plan
            .instances()
            .get(instance)
            .is_none_or(|entry| entry.disabled || entry.entry.is_none())
        {
            return Err(invalid(
                "UI checkpoint owner must be an executable original instance",
            ));
        }
        Ok(Checkpoints {
            store: self.clone(),
            deployment: source.deployment.to_string(),
            instance: encode(instance, 4096)?,
        })
    }
}
impl Checkpoints {
    pub async fn read(&self, key: &str) -> Result<Option<Checkpoint>, StoreError> {
        key_valid(key)?;
        self.store.validate_root()?;
        let deployment = self.deployment.clone();
        let instance = self.instance.clone();
        let key = key.to_owned();
        self.store.connection.run(move |connection| Box::pin(async move {
            let row = sqlx::query("SELECT revision,
                CASE WHEN length(CAST(format AS BLOB)) BETWEEN 1 AND 128 THEN format END AS format,
                CASE WHEN length(CAST(data_source_json AS BLOB)) <= 4096 THEN data_source_json END AS source,
                CASE WHEN length(CAST(metadata_json AS BLOB)) <= 32768 THEN metadata_json END AS metadata,
                CASE WHEN length(content) <= 1048576 THEN content END AS content
                FROM application_ui_checkpoints WHERE deployment=? AND instance=? AND key=?")
                .bind(deployment).bind(instance).bind(key).fetch_optional(connection).await?;
            row.map(|row| {
                let revision: i64 = row.try_get("revision")?;
                if !(1..=MAX_REVISION as i64).contains(&revision) { return Err(invalid("invalid UI checkpoint revision")); }
                let format = row.try_get::<Option<String>, _>("format")?.ok_or_else(|| invalid("invalid UI checkpoint format"))?;
                let data_source = serde_json::from_str(&row.try_get::<Option<String>, _>("source")?.ok_or_else(|| invalid("UI checkpoint source exceeds budget"))?)?;
                let metadata = serde_json::from_str(&row.try_get::<Option<String>, _>("metadata")?.ok_or_else(|| invalid("UI checkpoint metadata exceeds budget"))?)?;
                let content = row.try_get::<Option<Vec<u8>>, _>("content")?.ok_or_else(|| invalid("UI checkpoint content exceeds budget"))?;
                Ok(Checkpoint { revision: revision as u64, format, data_source, metadata, content })
            }).transpose()
        })).await
    }

    /// Empty snapshots remain CAS tombstones; a stale writer cannot recreate a
    /// deleted key after another mount has already saved and cleared it.
    pub async fn write(
        &self,
        key: &str,
        expected: Option<u64>,
        format: &str,
        data_source: &Value,
        metadata: &Value,
        content: Vec<u8>,
    ) -> Result<u64, StoreError> {
        key_valid(key)?;
        self.store.validate_root()?;
        if format.is_empty()
            || format.len() > 128
            || content.len() > MAX_CONTENT
            || expected.is_some_and(|value| value == 0 || value >= MAX_REVISION)
        {
            return Err(invalid("invalid or oversized UI checkpoint"));
        }
        let source = encode(data_source, MAX_SOURCE)?;
        let metadata = encode(metadata, MAX_METADATA)?;
        let deployment = self.deployment.clone();
        let instance = self.instance.clone();
        let key = key.to_owned();
        let format = format.to_owned();
        let size = 256 + key.len() + format.len() + source.len() + metadata.len() + content.len();
        self.store.connection.run(move |connection| Box::pin(async move {
            let mut tx = connection.begin_with("BEGIN IMMEDIATE").await?;
            let current: Option<(i64, Option<String>, Option<String>)> = sqlx::query_as("SELECT revision, CASE WHEN length(CAST(format AS BLOB)) BETWEEN 1 AND 128 THEN format END, CASE WHEN length(CAST(data_source_json AS BLOB)) <= 4096 THEN data_source_json END FROM application_ui_checkpoints WHERE deployment=? AND instance=? AND key=?")
                .bind(&deployment).bind(&instance).bind(&key).fetch_optional(&mut *tx).await?;
            let actual = current.as_ref().map(|row| row.0 as u64);
            if actual != expected { return Err(StoreError::RevisionConflict {
                expected: expected.map_or_else(|| "absent".into(), |revision| revision.to_string()),
                actual: actual.map_or_else(|| "absent".into(), |revision| revision.to_string()),
            }); }
            if current.is_some_and(|(_, previous_format, previous_source)| previous_format.as_deref() != Some(format.as_str()) || previous_source.as_deref() != Some(source.as_str())) {
                return Err(invalid("UI checkpoint format or data source changed"));
            }
            let occupied: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(256 + length(CAST(key AS BLOB)) + length(CAST(format AS BLOB)) + length(CAST(data_source_json AS BLOB)) + length(CAST(metadata_json AS BLOB)) + length(content)), 0) FROM application_ui_checkpoints WHERE deployment=? AND instance=? AND key<>?")
                .bind(&deployment).bind(&instance).bind(&key).fetch_one(&mut *tx).await?;
            if occupied < 0 || size > MAX_NAMESPACE.saturating_sub(occupied as usize) {
                return Err(invalid("UI checkpoint namespace exceeds 40 MiB"));
            }
            let revision = actual.unwrap_or(0) + 1;
            sqlx::query("INSERT INTO application_ui_checkpoints (deployment,instance,key,revision,format,data_source_json,metadata_json,content) VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(deployment,instance,key) DO UPDATE SET revision=excluded.revision,metadata_json=excluded.metadata_json,content=excluded.content")
                .bind(deployment).bind(instance).bind(key).bind(revision as i64).bind(format).bind(source).bind(metadata).bind(content).execute(&mut *tx).await?;
            tx.commit().await.map_err(StoreError::CommitUnknown)?;
            Ok(revision)
        })).await
    }
}

#[cfg(test)]
mod tests;
