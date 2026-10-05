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

//! Caller-owned original references. These records do not assert business journal facts.
use super::{encode, invalid};
use crate::{EventLog, StoreError, root::RootOwner};
use maka_plugin_core::{
    composition::InstanceId,
    deployment::DeploymentRef,
    operation::RecoveryRef,
    package::Packages,
    plan::{Plan, RemoteExports},
};
use serde::{Deserialize, Serialize};
use sqlx::{Connection, Row, SqliteConnection};
use std::sync::Arc;
use uuid::Uuid;

const MAX_REFERENCE: usize = 512 * 1024;
const MAX_PAGE: usize = 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CallerIntent {
    pub cursor: u64,
    pub caller: DeploymentRef,
    pub consumer: InstanceId,
    pub reference: RecoveryRef,
}
impl CallerIntent {
    /// Checks a bounded observation's shape, not whether a caller committed it.
    /// Deserialized observations are never `RecordedIntent` or effect authority.
    pub fn validate(&self) -> Result<(), StoreError> {
        if self.cursor == 0 || self.cursor > i64::MAX as u64 {
            return Err(invalid("invalid caller intent cursor"));
        }
        self.caller.validate().map_err(invalid)?;
        consumer_valid(&self.consumer)?;
        encode(&self.reference, MAX_REFERENCE)?;
        encode(self, MAX_PAGE)?;
        self.reference
            .validate()
            .map_err(|e| invalid(e.to_string()))?;
        Ok(())
    }
}
/// Only a confirmed caller commit constructs this token. It grants no Host authority.
/// Accepted executors retain it through real cleanup, independently of the waiter.
pub struct RecordedIntent {
    intent: CallerIntent,
    _owner: Arc<RootOwner>,
}
impl RecordedIntent {
    pub fn intent(&self) -> &CallerIntent {
        &self.intent
    }
}
fn consumer_valid(consumer: &InstanceId) -> Result<(), StoreError> {
    if consumer.0.is_empty() || consumer.0.len() > 64 || consumer.0.iter().any(Uuid::is_nil) {
        return Err(invalid("invalid caller consumer"));
    }
    Ok(())
}
async fn read(
    connection: &mut SqliteConnection,
    domain: &str,
    deployment: Uuid,
    consumer: &InstanceId,
    operation: Uuid,
) -> Result<Option<CallerIntent>, StoreError> {
    let row = sqlx::query("SELECT cursor, source_revision, CASE WHEN length(CAST(reference_json AS BLOB)) <= 524288 THEN reference_json END AS reference_json FROM application_intents WHERE deployment=? AND consumer=? AND operation=?")
        .bind(deployment.to_string()).bind(encode(consumer,4096)?).bind(operation.to_string()).fetch_optional(connection).await?;
    row.map(|row| {
        let reference: RecoveryRef = serde_json::from_str(
            &row.try_get::<Option<String>, _>("reference_json")?
                .ok_or_else(|| invalid("caller reference exceeds 512 KiB"))?,
        )?;
        reference.validate().map_err(|e| invalid(e.to_string()))?;
        if reference.operation != operation {
            return Err(StoreError::EventConflict);
        }
        let caller = DeploymentRef {
            state_domain: domain.into(),
            deployment,
            revision: row.try_get::<i64, _>("source_revision")? as u64,
        };
        caller.validate().map_err(invalid)?;
        let cursor = row.try_get::<i64, _>("cursor")?;
        if cursor <= 0 {
            return Err(invalid("invalid caller intent cursor"));
        }
        Ok(CallerIntent {
            cursor: cursor as u64,
            caller,
            consumer: consumer.clone(),
            reference,
        })
    })
    .transpose()
}
impl EventLog {
    /// Commit the exact original reference before dispatch. An unknown commit gives
    /// no token and cannot authorize sending; lookup only returns observation data.
    pub async fn record_application_intent(
        &self,
        caller: DeploymentRef,
        consumer: InstanceId,
        reference: RecoveryRef,
    ) -> Result<RecordedIntent, StoreError> {
        consumer_valid(&consumer)?;
        reference.validate().map_err(|e| invalid(e.to_string()))?;
        let encoded = encode(&reference, MAX_REFERENCE)?;
        let loaded = self
            .load_application_at(&caller)
            .await?
            .ok_or_else(|| invalid("original caller source missing"))?;
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
                .map_err(|e| invalid(e.to_string()))?;
        }
        let remotes: RemoteExports = loaded
            .source
            .remote_exports
            .into_iter()
            .map(|r| (r.capability, r.declaration))
            .collect();
        let plan = Plan::prepare(&loaded.source.input, &packages, &remotes)
            .map_err(|e| invalid(e.to_string()))?;
        if plan
            .instances()
            .get(&consumer)
            .is_none_or(|i| i.disabled || i.entry.is_none())
        {
            return Err(invalid(
                "caller must belong to an executable original source instance",
            ));
        }
        let domain = self.application_domain()?;
        let owner = self.root_owner.as_ref().expect("owned caller Root").clone();
        let commits = self.commits.clone();
        self.connection.run(move |connection| Box::pin(async move {
            let mut tx = connection.begin_with("BEGIN IMMEDIATE").await?;
            let old = read(&mut tx,&domain,caller.deployment,&consumer,reference.operation).await?;
            let intent = if let Some(old) = old {
                if old.reference != reference { return Err(StoreError::EventConflict); }
                // Keep the first source and original reference, even after a caller update.
                old
            } else {
                let result = sqlx::query("INSERT INTO application_intents (deployment,consumer,operation,source_revision,reference_json) VALUES (?,?,?,?,?)")
                    .bind(caller.deployment.to_string()).bind(encode(&consumer,4096)?).bind(reference.operation.to_string()).bind(caller.revision as i64).bind(encoded).execute(&mut *tx).await?;
                CallerIntent {cursor:result.last_insert_rowid() as u64,caller,consumer,reference}
            };
            tx.commit().await.map_err(StoreError::CommitUnknown)?;
            commits.send_modify(|_| {});
            Ok(RecordedIntent {intent,_owner:owner})
        })).await
    }
    /// Read original caller history across source revisions. The count limit is
    /// an upper bound; the complete JSON array also fits 1 MiB. Continue after
    /// the last returned cursor. Never returns an execution token.
    pub async fn application_intents(
        &self,
        deployment: Uuid,
        consumer: InstanceId,
        after: u64,
        limit: usize,
    ) -> Result<Vec<CallerIntent>, StoreError> {
        consumer_valid(&consumer)?;
        let domain = self.application_domain()?;
        if deployment.is_nil() || after > i64::MAX as u64 || !(1..=128).contains(&limit) {
            return Err(invalid("invalid caller intent page"));
        }
        self.connection.run(move |connection| Box::pin(async move {
            let mut tx = connection.begin().await?;
            let operations: Vec<Option<String>> = sqlx::query_scalar("SELECT CASE WHEN length(CAST(operation AS BLOB)) = 36 THEN operation END FROM application_intents WHERE deployment=? AND consumer=? AND cursor>? ORDER BY cursor LIMIT ?")
                .bind(deployment.to_string()).bind(encode(&consumer,4096)?).bind(after as i64).bind(limit as i64).fetch_all(&mut *tx).await?;
            let mut intents = Vec::with_capacity(operations.len());
            let mut bytes = 2; // The complete array, including brackets and commas.
            for operation in operations {
                let operation = operation.ok_or_else(|| invalid("invalid caller operation"))?;
                let intent = read(&mut tx,&domain,deployment,&consumer,Uuid::parse_str(&operation).map_err(|_| invalid("invalid caller operation"))?).await?.ok_or_else(|| invalid("caller intent disappeared"))?;
                let encoded = encode(&intent, MAX_PAGE)?;
                let size = encoded.len() + usize::from(!intents.is_empty());
                if size > MAX_PAGE - bytes { break; }
                bytes += size;
                intents.push(intent);
            }
            tx.commit().await?;
            Ok(intents)
        })).await
    }
}

#[cfg(test)]
mod tests;
