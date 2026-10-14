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

//! Root-owned native launcher selection. Opaque preparation is discarded on completion.
use super::{encode, invalid};
use crate::{EventLog, StoreError};
use serde_json::Value;
use sqlx::{Connection, Row};
use uuid::Uuid;

const MAX_CONFIGURATION: usize = 1024 * 1024;
const MAX_PRODUCT: usize = 4 * MAX_CONFIGURATION;
/// Exact source and base64 package bytes for at most one unfinished initialization.
const MAX_PENDING: usize = 192 * 1024 * 1024;

pub struct Launch {
    pub deployment: Uuid,
    pub configuration: Value,
    pub pending: Option<Vec<u8>>,
}
impl EventLog {
    /// Native product authoring inputs and the last installed default sources.
    pub async fn application_product(&self) -> Result<Option<Value>, StoreError> {
        self.application_domain()?;
        self.connection.run(|connection| Box::pin(async move {
            let row = sqlx::query("SELECT product IS NOT NULL AS present, CASE WHEN length(CAST(product AS BLOB)) <= ? THEN product END AS product FROM application_launch WHERE singleton=1")
                .bind(MAX_PRODUCT as i64).fetch_optional(connection).await?;
            let Some(row) = row else { return Ok(None); };
            let value = row.try_get::<Option<String>, _>("product")?;
            if row.try_get::<bool, _>("present")? && value.is_none() {
                return Err(invalid("launcher product exceeds budget"));
            }
            value.map(|value| serde_json::from_str(&value).map_err(StoreError::from)).transpose()
        })).await
    }

    /// Refresh derived product metadata only while the observed baseline is still current.
    pub async fn compare_and_set_application_product(
        &self,
        expected: Value,
        product: Value,
    ) -> Result<bool, StoreError> {
        self.application_domain()?;
        let expected = encode(&expected, MAX_PRODUCT)?;
        let product = encode(&product, MAX_PRODUCT)?;
        self.connection.run(move |connection| Box::pin(async move {
            let mut tx = connection.begin_with("BEGIN IMMEDIATE").await?;
            let changed = sqlx::query("UPDATE application_launch SET product=? WHERE singleton=1 AND product=? AND pending IS NULL")
                .bind(product).bind(expected).execute(&mut *tx).await?.rows_affected() == 1;
            tx.commit().await.map_err(StoreError::CommitUnknown)?;
            Ok(changed)
        })).await
    }

    /// Publish an already prepared default upgrade only after every source commit.
    /// Selection, baseline and preparation completion change atomically.
    pub async fn complete_application_product(
        &self,
        deployment: Uuid,
        configuration: Value,
        product: Value,
    ) -> Result<(), StoreError> {
        self.application_domain()?;
        let configuration = encode(&configuration, MAX_CONFIGURATION)?;
        let product = encode(&product, MAX_PRODUCT)?;
        self.connection.run(move |connection| Box::pin(async move {
            let mut tx = connection.begin_with("BEGIN IMMEDIATE").await?;
            let changed = sqlx::query("UPDATE application_launch SET configuration=?, product=?, pending=NULL WHERE singleton=1 AND deployment=? AND pending IS NOT NULL")
                .bind(configuration).bind(product).bind(deployment.to_string()).execute(&mut *tx).await?.rows_affected();
            if changed != 1 { return Err(invalid("launcher selection or preparation changed")); }
            tx.commit().await.map_err(StoreError::CommitUnknown)
        })).await
    }

    pub async fn application_launch(&self) -> Result<Option<Launch>, StoreError> {
        self.application_domain()?;
        self.connection.run(|connection| Box::pin(async move {
            let row = sqlx::query("SELECT deployment, CASE WHEN length(CAST(configuration AS BLOB)) <= ? THEN configuration END AS configuration, pending IS NOT NULL AS preparing, CASE WHEN length(pending) <= ? THEN pending END AS pending FROM application_launch WHERE singleton=1")
                .bind(MAX_CONFIGURATION as i64).bind(MAX_PENDING as i64).fetch_optional(connection).await?;
            let Some(row) = row else { return Ok(None); };
            let deployment = Uuid::parse_str(&row.try_get::<String, _>("deployment")?).map_err(|_| invalid("invalid launcher deployment"))?;
            let configuration = row.try_get::<Option<String>, _>("configuration")?.ok_or_else(|| invalid("launcher configuration exceeds budget"))?;
            let pending = row.try_get::<Option<Vec<u8>>, _>("pending")?;
            if deployment.is_nil() || pending.as_ref().is_some_and(Vec::is_empty) || (row.try_get::<bool, _>("preparing")? && pending.is_none()) {
                return Err(invalid("invalid or oversized launcher preparation"));
            }
            Ok(Some(Launch { deployment, configuration: serde_json::from_str(&configuration)?, pending }))
        })).await
    }

    /// Create once, before any cross-Root source effects. An unknown COMMIT
    /// remains unknown; reopening reads this same record instead of reauthoring.
    pub async fn begin_application_launch(&self, launch: Launch) -> Result<(), StoreError> {
        self.application_domain()?;
        let configuration = encode(&launch.configuration, MAX_CONFIGURATION)?;
        let pending = launch
            .pending
            .ok_or_else(|| invalid("launcher preparation required"))?;
        if launch.deployment.is_nil() || pending.is_empty() || pending.len() > MAX_PENDING {
            return Err(invalid("invalid or oversized launcher preparation"));
        }
        self.connection.run(move |connection| Box::pin(async move {
            let mut tx = connection.begin_with("BEGIN IMMEDIATE").await?;
            sqlx::query("INSERT INTO application_launch (singleton,deployment,configuration,pending) VALUES (1,?,?,?)")
                .bind(launch.deployment.to_string()).bind(configuration).bind(pending).execute(&mut *tx).await?;
            tx.commit().await.map_err(StoreError::CommitUnknown)
        })).await
    }

    /// Prepare restoration while holding the UI Root. The original connection
    /// authority stays fixed; reopening resumes the exact saved source writes.
    pub async fn begin_application_restore(
        &self,
        deployment: Uuid,
        pending: Vec<u8>,
    ) -> Result<(), StoreError> {
        self.application_domain()?;
        if pending.is_empty() || pending.len() > MAX_PENDING {
            return Err(invalid("invalid or oversized restoration preparation"));
        }
        self.connection.run(move |connection| Box::pin(async move {
            let mut tx = connection.begin_with("BEGIN IMMEDIATE").await?;
            let changed = sqlx::query("UPDATE application_launch SET pending=? WHERE singleton=1 AND deployment=? AND pending IS NULL")
                .bind(pending).bind(deployment.to_string()).execute(&mut *tx).await?.rows_affected();
            if changed != 1 { return Err(invalid("launcher changed or already has unfinished work")); }
            tx.commit().await.map_err(StoreError::CommitUnknown)
        })).await
    }

    /// Called by the trusted launcher after its original source commits have
    /// been confirmed. Never changes an application head or connection input.
    pub async fn complete_application_launch(&self, deployment: Uuid) -> Result<(), StoreError> {
        self.application_domain()?;
        self.connection.run(move |connection| Box::pin(async move {
            let mut tx = connection.begin_with("BEGIN IMMEDIATE").await?;
            let changed = sqlx::query("UPDATE application_launch SET pending=NULL WHERE singleton=1 AND deployment=?")
                .bind(deployment.to_string()).execute(&mut *tx).await?.rows_affected();
            if changed != 1 { return Err(invalid("launcher selection changed")); }
            tx.commit().await.map_err(StoreError::CommitUnknown)
        })).await
    }
}

#[cfg(test)]
mod tests;
