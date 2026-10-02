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

//! Original operation facts. Receipts never authorize a callback or a replay.
use super::{encode, invalid};
use crate::{EventLog, StoreError, root::RootOwner};
use maka_plugin_core::{
    composition::InstanceId,
    contract::Contract,
    deployment::DeploymentRef,
    operation::{
        OperationCleanup, OperationObservation, OperationOutcome, RecoveryProtocol, RecoveryRef,
    },
};
use maka_plugins::call::Identity;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Connection, Row, SqliteConnection};
use std::sync::Arc;
use uuid::Uuid;

const MAX_REQUEST: usize = 2 * 1024 * 1024;
const MAX_RESULT: usize = 1024 * 1024 + 128;
const MAX_DIAGNOSTIC: usize = 64 * 1024;

/// Trusted embedding supplies these facts after its private admission check.
/// This data is not an execution grant and contains no serialized evidence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationRequest {
    pub source: DeploymentRef,
    pub namespace: InstanceId,
    pub consumer: InstanceId,
    pub export: String,
    pub method: String,
    pub contract: Contract,
    pub operation: Uuid,
    pub input: Value,
    pub origin: Identity,
}
impl OperationRequest {
    pub fn reference(&self) -> Result<RecoveryRef, StoreError> {
        // Bound the entire record before building a canonical representation.
        encode(self, MAX_REQUEST)?;
        if self.consumer.0.is_empty()
            || self.consumer.0.len() > 64
            || self.consumer.0.iter().any(Uuid::is_nil)
        {
            return Err(invalid("invalid operation consumer"));
        }
        let mut canonical = serde_json::to_value(self)?;
        canonical
            .as_object_mut()
            .expect("request object")
            .remove("origin");
        canonical.sort_all_objects();
        let fingerprint = format!(
            "sha256-{:x}",
            Sha256::digest(encode(&canonical, MAX_REQUEST)?.as_bytes())
        );
        let reference = RecoveryRef {
            source: self.source.clone(),
            namespace: self.namespace.clone(),
            export: self.export.clone(),
            method: self.method.clone(),
            operation: self.operation,
            fingerprint,
            contract: self.contract.clone(),
            recovery: RecoveryProtocol::JournalV1,
        };
        reference.validate().map_err(|e| invalid(e.to_string()))?;
        Ok(reference)
    }
}

/// Only a successful first commit constructs this private execution claim.
/// Holding a claim also retains its original Root owner; it cannot be restored
/// from a receipt, copied, or used against another Root owner's connection.
pub struct DispatchClaim {
    reference: RecoveryRef,
    owner: Arc<RootOwner>,
}
impl DispatchClaim {
    pub fn reference(&self) -> &RecoveryRef {
        &self.reference
    }
}
pub enum Dispatch {
    New(DispatchClaim),
    Existing(OperationObservation),
}

struct Record {
    request: OperationRequest,
    observation: OperationObservation,
}
fn validate_settlement(
    outcome: &OperationOutcome,
    cleanup: &OperationCleanup,
) -> Result<(), StoreError> {
    if matches!(outcome, OperationOutcome::Unresolved)
        || matches!(cleanup, OperationCleanup::Unconfirmed)
    {
        return Err(invalid(
            "operation settlement must report result knowledge and real cleanup",
        ));
    }
    if matches!(outcome, OperationOutcome::Unknown { diagnostic } if diagnostic.len() > MAX_DIAGNOSTIC)
        || matches!(cleanup, OperationCleanup::Failed { diagnostic } if diagnostic.len() > MAX_DIAGNOSTIC)
    {
        return Err(invalid("operation diagnostic exceeds 64 KiB"));
    }
    encode(outcome, MAX_RESULT)?;
    encode(cleanup, MAX_DIAGNOSTIC + 128)?;
    Ok(())
}
async fn read(
    connection: &mut SqliteConnection,
    reference: &RecoveryRef,
) -> Result<Option<Record>, StoreError> {
    let namespace = encode(&reference.namespace, 4096)?;
    let Some(row) = sqlx::query("SELECT source_revision,
        CASE WHEN length(CAST(request_json AS BLOB)) <= 2097152 THEN request_json END AS request_json,
        CASE WHEN length(CAST(outcome_json AS BLOB)) <= 1048704 THEN outcome_json END AS outcome_json,
        CASE WHEN length(CAST(cleanup_json AS BLOB)) <= 65664 THEN cleanup_json END AS cleanup_json
        FROM application_operations WHERE deployment=? AND namespace=? AND operation=?")
        .bind(reference.source.deployment.to_string()).bind(namespace).bind(reference.operation.to_string())
        .fetch_optional(connection).await? else { return Ok(None); };
    let request: OperationRequest = serde_json::from_str(
        &row.try_get::<Option<String>, _>("request_json")?
            .ok_or_else(|| invalid("stored operation request exceeds limit"))?,
    )?;
    let outcome: OperationOutcome = serde_json::from_str(
        &row.try_get::<Option<String>, _>("outcome_json")?
            .ok_or_else(|| invalid("stored operation outcome exceeds limit"))?,
    )?;
    let cleanup: OperationCleanup = serde_json::from_str(
        &row.try_get::<Option<String>, _>("cleanup_json")?
            .ok_or_else(|| invalid("stored operation cleanup exceeds limit"))?,
    )?;
    if request.reference()? != *reference
        || row.try_get::<i64, _>("source_revision")? != reference.source.revision as i64
    {
        return Err(StoreError::EventConflict);
    }
    match (&outcome, &cleanup) {
        (OperationOutcome::Unresolved, OperationCleanup::Unconfirmed) => {}
        _ => validate_settlement(&outcome, &cleanup)?,
    }
    Ok(Some(Record {
        request,
        observation: OperationObservation {
            reference: reference.clone(),
            outcome,
            cleanup,
        },
    }))
}
impl EventLog {
    fn operation_domain(&self, reference: &RecoveryRef) -> Result<(), StoreError> {
        reference.validate().map_err(|e| invalid(e.to_string()))?;
        if reference.source.state_domain != self.application_domain()? {
            return Err(invalid("foreign operation state domain"));
        }
        Ok(())
    }
    /// Commit before calling the handler. An uncertain commit returns no claim;
    /// the caller may only read its original receipt, never dispatch again.
    pub async fn dispatch_operation(
        &self,
        request: OperationRequest,
    ) -> Result<Dispatch, StoreError> {
        let reference = request.reference()?;
        self.operation_domain(&reference)?;
        let owner = self.root_owner.as_ref().expect("owned Root").clone();
        let request_json = encode(&request, MAX_REQUEST)?;
        let commits = self.commits.clone();
        self.connection
            .run(move |connection| {
                Box::pin(async move {
                    let mut tx = connection.begin_with("BEGIN IMMEDIATE").await?;
                    if let Some(record) = read(&mut tx, &reference).await? {
                        // The original origin remains historical observation. New request
                        // authorization must be checked again by the trusted embedding.
                        if record.request.consumer != request.consumer
                            || record.request.input != request.input
                        {
                            return Err(StoreError::EventConflict);
                        }
                        tx.commit().await.map_err(StoreError::CommitUnknown)?;
                        return Ok(Dispatch::Existing(record.observation));
                    }
                    sqlx::query("INSERT INTO application_operations VALUES (?, ?, ?, ?, ?, ?, ?)")
                        .bind(reference.source.deployment.to_string())
                        .bind(encode(&reference.namespace, 4096)?)
                        .bind(reference.operation.to_string())
                        .bind(reference.source.revision as i64)
                        .bind(request_json)
                        .bind(encode(&OperationOutcome::Unresolved, MAX_RESULT)?)
                        .bind(encode(
                            &OperationCleanup::Unconfirmed,
                            MAX_DIAGNOSTIC + 128,
                        )?)
                        .execute(&mut *tx)
                        .await?;
                    tx.commit().await.map_err(StoreError::CommitUnknown)?;
                    commits.send_modify(|_| {});
                    Ok(Dispatch::New(DispatchClaim { reference, owner }))
                })
            })
            .await
    }

    /// Settles only the first committed dispatch. Repeated identical settlement
    /// is harmless; a duplicate request cannot overwrite result or cleanup.
    pub async fn finish_operation(
        &self,
        claim: &DispatchClaim,
        outcome: OperationOutcome,
        cleanup: OperationCleanup,
    ) -> Result<OperationObservation, StoreError> {
        self.operation_domain(&claim.reference)?;
        if !Arc::ptr_eq(self.root_owner.as_ref().expect("owned Root"), &claim.owner) {
            return Err(invalid("foreign operation dispatch claim"));
        }
        validate_settlement(&outcome, &cleanup)?;
        let reference = claim.reference.clone();
        let commits = self.commits.clone();
        self.connection.run(move |connection| Box::pin(async move {
            let mut tx = connection.begin_with("BEGIN IMMEDIATE").await?;
            let old = read(&mut tx, &reference).await?.ok_or_else(|| invalid("original dispatch missing"))?.observation;
            let observation = OperationObservation { reference: reference.clone(), outcome, cleanup };
            if !matches!(old.outcome, OperationOutcome::Unresolved) {
                if old != observation { return Err(StoreError::EventConflict); }
            } else {
                sqlx::query("UPDATE application_operations SET outcome_json=?, cleanup_json=? WHERE deployment=? AND namespace=? AND operation=?")
                    .bind(encode(&observation.outcome, MAX_RESULT)?).bind(encode(&observation.cleanup, MAX_DIAGNOSTIC + 128)?)
                    .bind(reference.source.deployment.to_string()).bind(encode(&reference.namespace, 4096)?)
                    .bind(reference.operation.to_string()).execute(&mut *tx).await?;
            }
            tx.commit().await.map_err(StoreError::CommitUnknown)?;
            commits.send_modify(|_| {});
            Ok(observation)
        })).await
    }

    /// Reads one fixed original namespace. This lookup performs no effect and
    /// cannot issue a claim. Its embedding must independently authorize recovery.
    pub async fn operation_observation(
        &self,
        reference: &RecoveryRef,
    ) -> Result<Option<OperationObservation>, StoreError> {
        self.operation_domain(reference)?;
        let reference = reference.clone();
        self.connection
            .run(move |connection| {
                Box::pin(async move {
                    read(connection, &reference)
                        .await
                        .map(|record| record.map(|record| record.observation))
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests;
