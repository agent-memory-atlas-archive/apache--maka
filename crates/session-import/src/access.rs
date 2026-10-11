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

//! The converters need only explicitly authorized source reads. They do not
//! receive execution, persistence, or authorization capabilities.
use crate::Error;
use futures_util::future::BoxFuture;
use maka_plugins::{
    filesystem::{ReadDirectory, database},
    remote::{Views, WorkspaceViewInput},
};
use maka_runtime::execution::{CollaborationMode, SandboxMode, WorkspaceTarget};

pub trait ReadSource: Send + Sync {
    fn directory(&self, path: String) -> BoxFuture<'_, Result<ReadDirectory, Error>>;
    fn query_database(
        &self,
        input: database::Read,
    ) -> BoxFuture<'_, Result<Vec<database::Table>, database::Error>>;
}

impl<T: Views + ?Sized> ReadSource for T {
    fn directory(&self, path: String) -> BoxFuture<'_, Result<ReadDirectory, Error>> {
        Box::pin(async move {
            Ok(self
                .workspace(WorkspaceViewInput {
                    workspace: WorkspaceTarget::HostPath { path },
                    sandbox_mode: SandboxMode::ReadOnly,
                    collaboration_mode: CollaborationMode::Agent,
                })
                .await?
                .files)
        })
    }
    fn query_database(
        &self,
        input: database::Read,
    ) -> BoxFuture<'_, Result<Vec<database::Table>, database::Error>> {
        Views::query_database(self, input)
    }
}
