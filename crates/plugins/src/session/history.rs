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

use crate::{call::Scope, execution::CommandError};
use futures_util::future::BoxFuture;
use maka_runtime::attachment::AttachmentRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub use maka_runtime::message::EditableMessage;
pub use maka_runtime::session::CopyPurpose;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopySource {
    pub session_id: String,
    pub expected_revision: u64,
    pub purpose: CopyPurpose,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopySession {
    pub source: CopySource,
    pub root: crate::execution::CreateRoot,
}
impl CopySession {
    pub fn validate(&self) -> Result<(), CommandError> {
        self.root
            .validate()
            .map_err(|error| CommandError::Invalid(error.to_string()))?;
        crate::name(&self.source.session_id)
            .map_err(|error| CommandError::Invalid(error.to_string()))?;
        if self.source.expected_revision == 0 || self.source.expected_revision >= 1 << 53 {
            return Err(CommandError::Invalid(
                "invalid source Session revision".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CopyResult {
    Committed {
        session: crate::execution::ChildSession,
    },
    SourceRevisionConflict {
        expected_revision: u64,
        actual_revision: u64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcesRead {
    pub session_id: String,
    pub turn_id: String,
}
impl SourcesRead {
    pub fn validate(&self) -> Result<(), CommandError> {
        for id in [&self.session_id, &self.turn_id] {
            crate::name(id).map_err(|_| CommandError::Invalid("invalid message source".into()))?;
        }
        Ok(())
    }
}

/// Copy an uploaded historical material into an independently authorized Session.
/// Identifiers locate content; the history call and destination capability grant access.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyMaterial {
    pub session_id: String,
    pub artifact_id: String,
    pub target_session_id: String,
}
impl CopyMaterial {
    pub fn validate(&self) -> Result<(), CommandError> {
        for id in [&self.session_id, &self.artifact_id, &self.target_session_id] {
            crate::name(id)
                .map_err(|_| CommandError::Invalid("invalid history material".into()))?;
        }
        Ok(())
    }
}

/// A readable field in a conversation window, in presentation order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HistoryField {
    Thinking,
    #[default]
    Body,
}

/// Byte offset in one message field's UTF-8 text, not a bearer capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cursor {
    #[serde(default)]
    pub field: HistoryField,
    #[schemars(range(min = 1, max = 9_007_199_254_740_991u64))]
    pub sequence: u64,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub offset: u64,
}

/// Cursor sequence addresses a sparse transcript row; Event addresses the raw
/// event log. Before excludes its end and After includes its start. Returned
/// chunks always retain ascending source order, including a reverse window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "HistoryPosition")]
pub enum Position {
    Start,
    Tail,
    Before {
        cursor: Cursor,
    },
    After {
        cursor: Cursor,
    },
    /// Locate canonical output at an execution event, including sparse rows.
    Event {
        #[schemars(range(min = 1, max = 9_007_199_254_740_991u64))]
        sequence: u64,
    },
}

/// A stable raw-event fence prevents a window from chasing appended messages.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Read {
    pub session_id: String,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub through: Option<u64>,
    pub position: Position,
    #[serde(default)]
    pub include_thinking: bool,
}
impl Read {
    pub fn validate(&self) -> Result<(), CommandError> {
        let invalid = || CommandError::Invalid("invalid history read".into());
        crate::name(&self.session_id).map_err(|_| invalid())?;
        const MAX: u64 = 9_007_199_254_740_991;
        if self.through.is_some_and(|value| value > MAX) {
            return Err(invalid());
        }
        match self.position {
            Position::Before { cursor } | Position::After { cursor }
                if self.through.is_none()
                    || cursor.sequence == 0
                    || cursor.sequence > MAX
                    || cursor.offset > MAX
                    || (!self.include_thinking && cursor.field == HistoryField::Thinking) =>
            {
                Err(invalid())
            }
            Position::Event { sequence }
                if sequence == 0
                    || sequence > MAX
                    || self.through.is_some_and(|through| sequence > through) =>
            {
                Err(invalid())
            }
            _ => Ok(()),
        }
    }
}

/// Canonical message author/kind; the field identifies body versus thinking.
/// Tool body JSON contributes string values, never its keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    ToolCall,
    ToolResult,
}
/// Immutable canonical presentation facts, not execution or recovery authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "HistoryTool")]
pub enum Tool {
    Call {
        tool_use_id: String,
        tool_name: String,
    },
    Result {
        tool_use_id: String,
        #[schemars(with = "maka_runtime::schema::RequiredNullable<String>")]
        tool_name: Option<String>,
        /// The presentation's error flag does not distinguish failed from unknown effects.
        is_error: bool,
        /// Canonical content is an image; this does not grant artifact reads.
        image: bool,
    },
}

/// An explicitly selected canonical transcript field, never an execution grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceField {
    Body,
    Thinking,
    ToolName,
    FailureMessage,
    ErrorClass,
    Arguments,
    Result,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourcePosition {
    Start,
    Tail,
    Before {
        #[schemars(range(max = 9_007_199_254_740_991u64))]
        offset: u64,
    },
    After {
        #[schemars(range(max = 9_007_199_254_740_991u64))]
        offset: u64,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceFormat {
    Text,
    Json,
    ImageDescriptor,
}
/// Presentation role of the exact row, including diagnostics omitted from the
/// ordinary conversation window. This carries no execution authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "HistorySourceRole")]
pub enum SourceRole {
    User,
    Assistant,
    ToolCall,
    ToolResult,
    Failure,
}
/// One literal UTF-8 excerpt of an immutable field. JSON excerpts need not be
/// standalone JSON. Image descriptors carry no artifact read authorization.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistorySource {
    #[schemars(range(min = 1, max = 9_007_199_254_740_991u64))]
    pub sequence: u64,
    /// Host-recorded milliseconds of this immutable row; not a client clock.
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub timestamp: u64,
    pub role: SourceRole,
    /// Readable canonical fields of this same immutable row, in display order.
    #[schemars(length(min = 1, max = 7))]
    pub fields: Vec<SourceField>,
    pub field: SourceField,
    pub format: SourceFormat,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub offset: u64,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub total_bytes: u64,
    pub text: String,
    #[schemars(with = "maka_runtime::schema::RequiredNullable<Tool>")]
    pub tool: Option<Tool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Chunk {
    pub field: HistoryField,
    pub message_id: String,
    pub turn_id: String,
    pub timestamp: u64,
    pub role: Role,
    pub sequence: u64,
    pub offset: u64,
    pub total_bytes: u64,
    pub text: String,
    /// Present on every tool fragment, including tools with no projected text.
    pub tool: Option<Tool>,
    /// User attachment descriptors, present only on the first chunk of a message.
    pub attachments: Vec<AttachmentRef>,
}

/// Index preparation is bounded too. Repeat with the returned fence and the
/// same position. Ready windows expose exact byte boundaries in both directions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Page {
    Preparing {
        through: u64,
    },
    Ready {
        through: u64,
        chunks: Vec<Chunk>,
        before: Option<Cursor>,
        after: Option<Cursor>,
    },
}

/// Admitted Agent tools can recall this trusted Host profile's history.
/// Remote/background calls retain their actual principal's ReadHistory scope.
/// Every page rechecks current access and Session existence, including old fences.
pub trait History: Send + Sync {
    /// A new root with owned immutable history. Source read access does not grant
    /// destination creation; both capabilities and the source workspace are checked.
    fn copy_session(
        &self,
        call: Scope,
        target: Arc<dyn crate::execution::Commands>,
        input: CopySession,
    ) -> BoxFuture<'_, Result<CopyResult, CommandError>>;
    /// Ordered preparation-free opening sources of a Turn, not its aggregated UI row.
    /// Their identities and intent grant no execution authority.
    fn sources(
        &self,
        call: Scope,
        input: SourcesRead,
    ) -> BoxFuture<'_, Result<Vec<EditableMessage>, CommandError>>;
    fn list(
        &self,
        call: Scope,
        input: super::catalog::List,
    ) -> BoxFuture<'_, Result<super::catalog::Page, CommandError>>;
    fn read(&self, call: Scope, input: Read) -> BoxFuture<'_, Result<Page, CommandError>>;
    fn copy_material(
        &self,
        call: Scope,
        target: Arc<dyn crate::execution::Commands>,
        input: CopyMaterial,
    ) -> BoxFuture<'_, Result<AttachmentRef, CommandError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_positions_require_fixed_cursor_fences_and_distinguish_raw_events() {
        let cursor = Cursor {
            field: Default::default(),
            sequence: 4096,
            offset: 0,
        };
        for position in [Position::Before { cursor }, Position::After { cursor }] {
            let mut read = Read {
                include_thinking: false,
                session_id: "session".into(),
                through: None,
                position,
            };
            assert!(read.validate().is_err());
            read.through = Some(16);
            assert!(
                read.validate().is_ok(),
                "row sequence is not compared to raw fence"
            );
        }
        for position in [
            Position::Start,
            Position::Tail,
            Position::Event { sequence: 16 },
        ] {
            assert!(
                Read {
                    include_thinking: false,
                    session_id: "session".into(),
                    through: None,
                    position
                }
                .validate()
                .is_ok()
            );
        }
        assert!(
            Read {
                include_thinking: false,
                session_id: "session".into(),
                through: Some(15),
                position: Position::Event { sequence: 16 }
            }
            .validate()
            .is_err()
        );
        for position in [
            Position::Before {
                cursor: Cursor {
                    field: Default::default(),
                    sequence: 0,
                    offset: 0,
                },
            },
            Position::Event { sequence: 0 },
        ] {
            assert!(
                Read {
                    include_thinking: false,
                    session_id: "session".into(),
                    through: Some(16),
                    position
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            serde_json::from_value::<Read>(
                serde_json::json!({"sessionId":"session","cursor":null})
            )
            .is_err()
        );
        assert!(serde_json::from_value::<Read>(serde_json::json!({"sessionId":"session","through":16,"position":{"kind":"cursor","cursor":cursor}})).is_err());
        assert!(
            serde_json::from_value::<Page>(
                serde_json::json!({"kind":"ready","through":16,"chunks":[],"next":null})
            )
            .is_err()
        );
    }
}
