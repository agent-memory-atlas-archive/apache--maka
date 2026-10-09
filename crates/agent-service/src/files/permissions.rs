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
use crate::interaction_wait::{wait, withdraw};
use maka_runtime::interaction::{
    InteractionOutcome, InteractionRecord, InteractionRequest, PermissionRequest,
};
use maka_sandbox::grant::Decision;
use sha2::{Digest, Sha256};
use std::path::Component;

pub(super) async fn authorize(
    store: &EventLog,
    captured: Captured,
    configuration: &SessionConfiguration,
    tool: &'static str,
    input: &Value,
    context: &ToolCallContext,
    cancellation: &CancellationToken,
) -> Result<Captured, ToolError> {
    let grants = store
        .permission_grants(
            &context.invocation,
            Some(&context.tool_use_id()),
            configuration.boundary_revision,
        )
        .await
        .map_err(|_| failed("Workspace permission observation unavailable"))?;
    let input = input.clone();
    let (mut captured, required) = tokio::task::spawn_blocking(move || {
        let mut captured = captured;
        for grant in grants {
            captured.sandbox = captured
                .sandbox
                .with_grant(&grant.permissions, &captured.ceiling)
                .map_err(failed)?;
        }
        let paths = maka_fs_tools::mutation_paths(tool, input, &captured.path)?;
        let mut requested = Vec::new();
        for path in paths {
            let relative = path
                .strip_prefix(&captured.path)
                .map_err(|_| failed("File is outside the captured workspace"))?;
            if relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
            {
                return Err(failed("Write does not support parent path components"));
            }
            let relative: PathBuf = relative
                .components()
                .filter(|part| *part != Component::CurDir)
                .collect();
            let resolved = match captured.directory.canonicalize(&relative) {
                Ok(path) => path,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // A missing leaf is allowed; dangling links and absent parents are not.
                    match captured.directory.symlink_metadata(&relative) {
                        Ok(_) => return Err(failed("Write target is a dangling link")),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(failed(error)),
                    }
                    let name = relative
                        .file_name()
                        .ok_or_else(|| failed("Write requires a file name"))?;
                    let parent = relative
                        .parent()
                        .filter(|path| !path.as_os_str().is_empty())
                        .unwrap_or(Path::new("."));
                    captured
                        .directory
                        .canonicalize(parent)
                        .map_err(failed)?
                        .join(name)
                }
                Err(error) => return Err(failed(error)),
            };
            let resolved: PathBuf = resolved
                .components()
                .filter(|part| *part != Component::CurDir)
                .collect();
            if resolved != relative {
                return Err(failed("Write does not support symlink targets"));
            }
            let permissions = Permissions {
                filesystem: vec![Rule::exact(captured.path.join(resolved), Access::Write)],
                network: Network::Denied,
            };
            if !captured.ceiling.permits(&permissions).map_err(failed)? {
                return Err(failed(
                    "Workspace metadata or Host-private files cannot be granted",
                ));
            }
            if !captured.sandbox.permits(&permissions).map_err(failed)? {
                requested.extend(permissions.filesystem);
            }
        }
        let required = Permissions {
            filesystem: requested,
            network: Network::Denied,
        };
        required.validate().map_err(failed)?;
        Ok::<_, ToolError>((captured, required))
    })
    .await
    .map_err(failed)??;
    cancelled(cancellation)?;
    if required.filesystem.is_empty() {
        return Ok(captured);
    }
    if !configuration.permissions.request_permissions {
        return Err(failed(
            "Workspace policy does not allow requesting additional permissions",
        ));
    }
    let record = InteractionRecord {
        session_id: context.invocation.session_id.clone(),
        turn_id: context.invocation.turn_id.clone(),
        run_id: context.invocation.run_id.clone(),
        request_id: format!(
            "files_{:x}",
            Sha256::digest(
                serde_json::to_vec(&(
                    &context.invocation,
                    &context.operation_id,
                    configuration.boundary_revision
                ))
                .expect("tool identity")
            )
        ),
        created_at: crate::owner::now(),
        request: InteractionRequest::Permissions {
            tool_use_id: Some(context.tool_use_id()),
            base_revision: configuration.boundary_revision,
            request: PermissionRequest {
                reason: format!(
                    "{tool} needs write access to these files in the selected workspace."
                ),
                command: None,
                permissions: required.clone(),
            },
        },
        outcome: None,
    };
    let observed = async {
        let committed = store
            .establish_run_interaction(&context.invocation, &record)
            .await
            .map_err(|_| "workspace permission publication unavailable")?;
        if !committed.matches {
            return Err("workspace permission identity conflict");
        }
        wait(store, &record.request_id, cancellation).await
    }
    .await;
    let outcome = match observed {
        Ok(outcome) => outcome,
        Err(error) => match withdraw(store, &record).await {
            Ok(Some(outcome)) => outcome,
            Ok(None) => return Err(failed(error)),
            Err(cleanup) => return Err(ToolError::CleanupUnconfirmed(cleanup.into())),
        },
    };
    cancelled(cancellation)?;
    if !matches!(
        outcome,
        InteractionOutcome::PermissionsDecision {
            decision: Decision::Allow { .. },
            ..
        }
    ) {
        return Err(failed(
            "Additional workspace write permission was not granted",
        ));
    }
    let grant = store
        .permission_grants(
            &context.invocation,
            Some(&context.tool_use_id()),
            configuration.boundary_revision,
        )
        .await
        .map_err(|_| failed("Workspace permission observation unavailable"))?
        .into_iter()
        .find(|grant| grant.request_id == record.request_id)
        .ok_or_else(|| failed("Approved workspace permissions are no longer current"))?;
    captured.sandbox = captured
        .sandbox
        .with_grant(&grant.permissions, &captured.ceiling)
        .map_err(failed)?;
    if !captured.sandbox.permits(&required).map_err(failed)? {
        return Err(failed(
            "Not all target files were approved; no files were changed",
        ));
    }
    Ok(captured)
}
