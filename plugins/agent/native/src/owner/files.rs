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

//! Session-bound directory observations; the worker owns watcher settlement.
use super::*;
use maka_capabilities::call::Scope;
use maka_fs_tools::files::{Directory, Watch};
use maka_host_runtime::application::Context;
use std::{io, path::Path};

impl Owner {
    pub async fn list_files(
        self: &Arc<Self>,
        input: FileListRead,
        scope: &Scope,
    ) -> Result<FilePage, String> {
        self.resource_work(scope, move |owner, scope| {
            Box::pin(async move {
                let (original, directory) = owner
                    .file_directory(&input.session_id, input.path, &scope)
                    .await?;
                let result = worktrees::blocking(&scope, &owner.stop, move |cancel| {
                    directory
                        .list(input.after.as_ref(), &cancel)
                        .map_err(file_failure)
                })
                .await;
                owner
                    .check_file_session(&input.session_id, &original, &scope)
                    .await?;
                result
            })
        })
        .await
    }

    pub async fn watch_files(
        self: &Arc<Self>,
        input: FileWatch,
        context: Context,
    ) -> Result<Value, String> {
        let scope = context.scope().clone();
        self.resource_work(&scope, move |owner, scope| {
            Box::pin(async move {
                let (original, directory) = owner
                    .file_directory(&input.session_id, input.path, &scope)
                    .await?;
                let capture = directory.clone();
                let mut watch = worktrees::blocking(&scope, &owner.stop, move |_| {
                    capture.watch().map_err(file_failure)
                })
                .await?;
                let result = owner
                    .observe_files(
                        &input.session_id,
                        &original,
                        &directory,
                        &mut watch,
                        &context,
                    )
                    .await;
                // Never abandon cleanup when the observer or Owner stops. On
                // supported backends this waits for normal watch/callback
                // quiescence; it does not claim a portable native thread join.
                watch.close().await.map_err(file_failure)?;
                result
            })
        })
        .await
    }

    async fn file_directory(
        &self,
        session: &str,
        path: Option<WorkspacePath>,
        scope: &Scope,
    ) -> Result<(SessionConfiguration, Arc<Directory>), String> {
        if let Some(path) = &path {
            path.validate().map_err(|_| "agent_files_invalid_path")?;
        }
        let original = self.session(session).await?.configuration;
        self.check_file_session(session, &original, scope).await?;
        let root = self
            .store
            .owned_root()
            .map_err(|_| "agent_root_unavailable")?;
        let private = root.canonical_path().to_owned();
        let control = root.control_directory().to_owned();
        let selected = original.workspace.clone();
        let directory = worktrees::blocking(scope, &self.stop, move |cancel| {
            Directory::open(
                Path::new(&selected.path),
                &selected.directory_identity,
                &private,
                &control,
                selected.worktree.as_deref(),
                path.as_ref(),
                &cancel,
            )
            .map(Arc::new)
            .map_err(file_failure)
        })
        .await?;
        self.check_file_session(session, &original, scope).await?;
        Ok((original, directory))
    }

    async fn check_file_session(
        &self,
        session: &str,
        original: &SessionConfiguration,
        scope: &Scope,
    ) -> Result<(), String> {
        if scope.cancellation.is_cancelled() || self.stop.is_cancelled() {
            return Err("agent_files_cancelled".into());
        }
        workspace::validate(
            &self.workspace_roots,
            &original.workspace,
            &self.managed_worktrees,
        )
        .await?;
        let current = self.session(session).await?.configuration;
        if current.workspace != original.workspace
            || current.boundary_revision != original.boundary_revision
        {
            return Err("agent_files_changed".into());
        }
        if scope.cancellation.is_cancelled() || self.stop.is_cancelled() {
            return Err("agent_files_cancelled".into());
        }
        Ok(())
    }

    async fn observe_files(
        &self,
        session: &str,
        original: &SessionConfiguration,
        directory: &Arc<Directory>,
        watch: &mut Watch,
        context: &Context,
    ) -> Result<Value, String> {
        // Subscribe before revalidating the captured Session. Earlier changes
        // are caught by the read; later changes remain pending on this receiver.
        let mut commits = self.store.subscribe_commits();
        commits.borrow_and_update();
        let mut change = Some(FileChange::Ready {
            directory_identity: directory.directory_identity().clone(),
        });
        loop {
            let capture = directory.clone();
            let valid = worktrees::blocking(context.scope(), &self.stop, move |_| {
                capture.validate().map_err(file_failure)
            })
            .await;
            self.check_file_session(session, original, context.scope())
                .await?;
            match valid {
                Ok(()) => {}
                Err(error) if error == "agent_files_changed" => {
                    let terminal = FileChange::Unavailable {
                        reason: FileUnavailable::DirectoryChanged,
                    };
                    return self.emit_file_change(terminal, context).await;
                }
                Err(error) => return Err(error),
            }
            if let Some(change) = change {
                self.emit_file_change(change, context).await?;
            }
            change = tokio::select! {
                biased;
                _ = context.scope().cancellation.cancelled() => return Err("agent_files_cancelled".into()),
                _ = self.stop.cancelled() => return Err("agent_files_cancelled".into()),
                result = watch.changed() => {
                    // A native observation failure ends the stream. Reconnect
                    // starts with a fresh read, never an invented replay cursor.
                    result.map_err(file_failure)?;
                    Some(FileChange::Changed)
                },
                result = commits.changed() => {
                    result.map_err(|_| "agent_observation_closed")?;
                    commits.borrow_and_update();
                    None
                },
            };
        }
    }

    async fn emit_file_change(
        &self,
        change: FileChange,
        context: &Context,
    ) -> Result<Value, String> {
        let value = serde_json::to_value(change).map_err(|_| "agent_files_output_unavailable")?;
        tokio::select! {
            biased;
            _ = context.scope().cancellation.cancelled() => Err("agent_files_cancelled".into()),
            _ = self.stop.cancelled() => Err("agent_files_cancelled".into()),
            result = context.emit(value.clone()) => {
                result?;
                Ok(value)
            }
        }
    }
}

// Only backend-owned static causes cross the service boundary. OS paths and
// arbitrary error Display strings never become a client error or log payload.
fn file_failure(error: io::Error) -> String {
    let cause = match error.to_string().as_str() {
        "files_invalid_path" => "agent_files_invalid_path",
        "files_invalid_cursor" => "agent_files_invalid_cursor",
        "files_private" => "agent_files_private",
        "files_too_large" => "agent_files_too_large",
        "files_changed" => "agent_files_changed",
        "files_cancelled" => "agent_files_cancelled",
        "files_watch_unavailable" => "agent_files_watch_unavailable",
        _ => "agent_files_unavailable",
    };
    if cause != "agent_files_cancelled" {
        tracing::warn!(stage = "files_read", cause, io_kind = ?error.kind(),
            os_code = error.raw_os_error(), "Workspace file observation failed");
    }
    cause.into()
}
