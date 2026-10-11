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

mod material;
mod passages;
mod rank;
mod reader;
mod remote;
pub mod search;
mod terminal;
mod tools;
mod types;

use futures_util::future::BoxFuture;
use maka_plugins::{
    composition::Scope,
    contributions::Staged,
    kernel::{Plugin, PluginContext},
    preferences::Preferences,
    session::history::History,
};
use maka_runtime::{
    tool_call::ToolRejection,
    tool_output::{DurableToolProjection, ToolOutput, ToolSuccess},
    tools::{
        PreparationFuture, PreparedEffect, ToolCallContext, ToolDefinition, ToolError, ToolHandler,
        ToolNesting, ToolPreparer, ToolRegistration, ToolSemantics,
    },
};
use maka_tool_catalog::plugins::{Binding, BindingProvider, BindingRequest, PluginTool};
use reader::failed;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use types::{More, Query, ResultSet};

pub const ID: &str = "maka.recall";
pub struct Builtin;
#[derive(Clone)]
struct Recall {
    history: Arc<dyn History>,
    executions: Arc<dyn maka_plugins::execution::Access>,
    files: Arc<dyn maka_plugins::filesystem::Files>,
    preferences: Arc<dyn Preferences>,
    workers: Arc<Semaphore>,
}
impl Plugin for Builtin {
    fn description(&self) -> Option<maka_plugins::kernel::Description> {
        Some(maka_plugins::kernel::Description {
            name: maka_plugins::terminal_ui::Text::localized("Recall", "历史检索", "歷史檢索"),
            summary: Some(maka_plugins::terminal_ui::Text::localized(
                "Find useful information from earlier conversations.",
                "查找历史会话中的相关信息。",
                "尋找歷史對話中的相關資訊。",
            )),
        })
    }

    fn supports_scope(&self, scope: &Scope) -> bool {
        matches!(scope, Scope::Profile | Scope::Session(_))
    }
    fn validate(&self, _: &Scope, config: &Value) -> Result<(), maka_plugins::Error> {
        if config.is_null() || config.as_object().is_some_and(|value| value.is_empty()) {
            Ok(())
        } else {
            Err(maka_plugins::Error::Invalid(
                "Recall takes no instance configuration".into(),
            ))
        }
    }
    fn activate(
        &self,
        context: PluginContext,
        _: Value,
    ) -> BoxFuture<'static, Result<Staged, String>> {
        Box::pin(async move {
            let package = context
                .lifecycle
                .identity()
                .map_err(|e| e.to_string())?
                .package_id;
            let host = context.host.ok_or("Recall requires Host history")?;
            let recall = Arc::new(Recall {
                history: host.history,
                executions: host.executions,
                files: host.files,
                preferences: host.preferences,
                workers: Arc::new(Semaphore::new(2)),
            });
            let mut staged = Staged::default();
            remote::publish(&mut staged, &package, recall.clone())?;
            terminal::publish(&mut staged, &package, recall.clone())?;
            for (name, description, schema) in search::definitions() {
                let tool = PluginTool::new(ToolRegistration {
                    definition: ToolDefinition {
                        freeform: None,
                        output_schema: None,
                        provider: None,
                        name: name.into(),
                        description: description.into(),
                        input_schema: schema,
                    },
                    nesting: ToolNesting::Nestable,
                    semantics: ToolSemantics::Parallel,
                    handler: ToolHandler::Prepared(recall.clone()),
                })
                .map_err(|error| error.to_string())?
                .with_binding(recall.clone());
                staged
                    .insert(name, tool)
                    .map_err(|error| error.to_string())?;
            }
            Ok(staged)
        })
    }
}
impl BindingProvider for Recall {
    fn bind(
        &self,
        _: BindingRequest,
        _: maka_plugins::filesystem::ReadDirectory,
    ) -> BoxFuture<'static, Result<Option<Binding>, ToolError>> {
        let recall = self.clone();
        Box::pin(async move {
            if recall
                .preferences
                .read()
                .await
                .map_err(failed)?
                .privacy
                .incognito_active
            {
                return Ok(None);
            }
            Ok(Some(Binding {
                provider_tools: Default::default(),
                handler: Some(Arc::new(recall)),
                context: None,
            }))
        })
    }
}
impl Recall {
    async fn check_privacy(&self) -> Result<(), ToolError> {
        if self
            .preferences
            .read()
            .await
            .map_err(failed)?
            .privacy
            .incognito_active
        {
            return Err(failed("Recall is disabled in incognito mode"));
        }
        Ok(())
    }
}

impl search::History for Recall {
    fn list(
        &self,
        call: maka_plugins::call::Scope,
        input: maka_plugins::session::catalog::List,
    ) -> BoxFuture<'_, Result<search::Catalog, ToolError>> {
        Box::pin(async move {
            let page = self.history.list(call, input).await.map_err(failed)?;
            Ok(search::Catalog {
                revision: page.revision,
                next_cursor: page.next_cursor,
                entries: page
                    .entries
                    .into_iter()
                    .map(|entry| search::Session {
                        id: entry.session.session_id,
                        title: entry.session.name,
                        last_message_at: entry.last_message_at,
                    })
                    .collect(),
            })
        })
    }
    fn read(
        &self,
        call: maka_plugins::call::Scope,
        input: maka_plugins::session::history::Read,
    ) -> BoxFuture<'_, Result<maka_plugins::session::history::Page, ToolError>> {
        Box::pin(async move { self.history.read(call, input).await.map_err(failed) })
    }
}
