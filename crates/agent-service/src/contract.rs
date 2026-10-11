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

use maka_plugin_core::contract::{Contract, Interface, Method, MethodMode, Transport};
use maka_plugins::session::history::{Cursor, Role};
use maka_runtime::execution::{DirectoryIdentity, WorkspaceIdentity};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[path = "contract_projects.rs"]
mod projects;
pub use projects::*;
#[path = "contract_execution.rs"]
mod execution;
pub use execution::*;

#[path = "contract_interactions.rs"]
mod interaction;
pub use interaction::*;
#[path = "contract_permissions.rs"]
mod permissions;
pub use maka_runtime::configuration::ModelSelection;
pub use permissions::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentConfiguration")]
pub struct Configuration {
    /// Explicit service backends; discovery does not authorize execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 8))]
    pub executors: Vec<crate::executor::ServiceExecutor>,
    /// Explicit Library import for immutable skill input preparation and metadata.
    /// This does not select model tools or confer a service grant.
    #[serde(default)]
    #[schemars(length(min = 1, max = 256))]
    pub skill_library: Option<String>,
    /// Initial policy within the selected workspace capability.
    pub permissions: WorkspacePermissions,
    /// Installation-published directories, captured without creating workspace markers.
    #[schemars(length(min = 1, max = 8))]
    pub workspace_roots: Vec<maka_runtime_host::server::DirectoryRootSpec>,
    #[schemars(range(min = 1, max = 16))]
    pub max_active_runs: usize,
    #[schemars(range(min = 1, max = 16))]
    pub max_steps: usize,
    /// Only explicitly selected, frozen application service methods become tools.
    #[schemars(length(max = 128))]
    pub tools: Vec<ServiceTool>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentServiceTool")]
pub struct ServiceTool {
    /// Explicit input projection. Skill tools receive the actual Agent tool names,
    /// outside model-controlled arguments; this is compatibility data, not authority.
    #[serde(default)]
    pub input: ToolInput,
    /// Explicit versioned renderer key, frozen with the call; never inferred from its name.
    #[serde(default)]
    #[schemars(length(min = 1, max = 128))]
    pub presentation: Option<String>,
    /// Display only; the selected name, contract and binding still identify execution.
    #[serde(default)]
    pub title: Option<maka_runtime::display::Text>,
    /// Explicit model projection; JSON data never implicitly grants image access.
    pub result: ToolResult,
    /// Additional user consent for this exact call; service grants remain independent.
    pub confirmation: ToolConfirmation,
    #[schemars(length(min = 1, max = 128))]
    pub name: String,
    #[schemars(length(min = 1, max = 4096))]
    pub description: String,
    #[schemars(length(min = 1, max = 256))]
    pub import: String,
    #[schemars(length(min = 1, max = 256))]
    pub method: String,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentToolInput")]
pub enum ToolInput {
    #[default]
    Json,
    Skill,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentToolResult")]
pub enum ToolResult {
    Json,
    Image,
}

/// Bounded image bytes from a selected subscription. Complete is a producer
/// return value, never an emitted item or an executor-cleanup acknowledgement.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentToolImageFrame")]
pub enum ImageFrame {
    Chunk {
        #[schemars(regex(pattern = "^image/(png|jpeg|gif|webp)$"))]
        mime_type: String,
        #[schemars(range(min = 1, max = 5242880))]
        total_bytes: usize,
        #[schemars(range(max = 5242879))]
        offset: usize,
        #[schemars(length(min = 1, max = 43692))]
        data: String,
    },
    Complete,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentToolConfirmation")]
pub enum ToolConfirmation {
    Required,
    Preapproved,
}
pub fn configuration_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(Configuration)).expect("configuration schema")
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentList")]
pub struct List {
    pub filter: SessionFilter,
    pub position: SessionPosition,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentSessionFilter")]
pub enum SessionFilter {
    All,
    NeedsYou,
}
#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "AgentSessionPosition")]
pub enum SessionPosition {
    Start,
    At {
        #[schemars(length(min = 1, max = 128))]
        cursor: String,
    },
    After {
        revision: String,
        #[schemars(length(min = 1, max = 128))]
        cursor: String,
    },
    Before {
        revision: String,
        #[schemars(length(min = 1, max = 128))]
        cursor: String,
    },
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentCreate")]
pub struct Create {
    #[schemars(length(min = 1, max = 128))]
    pub title: String,
    #[schemars(length(max = 32))]
    pub labels: Vec<String>,
    pub workspace: WorkspaceSelection,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentWorkspaceSelection")]
pub enum WorkspaceSelection {
    Project {
        project: ProjectRef,
        selection: DirectorySelection,
    },
    Directory {
        selection: DirectorySelection,
    },
    Session {
        #[schemars(length(min = 1, max = 128))]
        session_id: String,
    },
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentDirectorySelection")]
pub struct DirectorySelection {
    #[schemars(length(min = 1, max = 64))]
    pub root_id: String,
    #[schemars(length(max = 64))]
    pub segments: Vec<String>,
    #[schemars(with = "String", regex(pattern = "^sha256:[a-f0-9]{64}$"))]
    pub directory_identity: DirectoryIdentity,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "AgentDirectories")]
pub enum Directories {
    Roots,
    Children {
        selection: DirectorySelection,
        #[schemars(with = "maka_runtime::schema::RequiredNullable<String>")]
        after: Option<String>,
    },
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentDirectory")]
pub struct Directory {
    pub name: String,
    pub path: String,
    pub selection: DirectorySelection,
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "AgentDirectoryPage")]
pub enum DirectoryPage {
    Roots {
        #[schemars(length(max = 8))]
        roots: Vec<Directory>,
    },
    Children {
        directory: Directory,
        #[schemars(length(max = 32))]
        children: Vec<Directory>,
        #[schemars(with = "maka_runtime::schema::RequiredNullable<String>")]
        after: Option<String>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentWorkspace")]
pub struct Workspace {
    /// Optional membership in this Agent's canonical Project catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<uuid::Uuid>,
    #[schemars(length(min = 1, max = 4096))]
    pub path: String,
    #[schemars(with = "String", regex(pattern = "^sha256:[a-f0-9]{64}$"))]
    pub directory_identity: DirectoryIdentity,
    #[schemars(with = "String", regex(pattern = "^workspace:v1:[A-Fa-f0-9-]{36}$"))]
    pub identity: WorkspaceIdentity,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentMetadata")]
pub struct Metadata {
    /// Omission leaves the permission boundary and its approvals unchanged.
    #[serde(default)]
    pub permissions: Option<PermissionChange>,
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(range(min = 1, max = 9_007_199_254_740_991u64))]
    pub expected_revision: u64,
    #[schemars(length(min = 1, max = 128))]
    pub title: String,
    #[schemars(length(max = 32))]
    pub labels: Vec<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentRejection")]
pub enum Rejection {
    TitleInvalid,
    LabelsInvalid,
    WorkspaceUnavailable,
    SessionUnavailable,
    Closed,
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "AgentCreated")]
pub enum Created {
    Created { session: Box<Session> },
    Rejected { reason: Rejection },
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "AgentMetadataResult")]
pub enum MetadataResult {
    Committed { session: Session },
    RevisionConflict { expected: u64, actual: u64 },
    Rejected { reason: Rejection },
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentSend")]
pub struct Send {
    /// IDs of immutable user uploads in this Session; metadata is resolved by the owner.
    #[serde(default)]
    #[schemars(length(max = 8))]
    pub attachments: Vec<String>,
    pub model: ModelSelection,
    /// Omission/null uses the configured model default; `off` is explicit.
    pub thinking_level: Option<maka_runtime::execution::ThinkingLevel>,
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(length(max = 65536))]
    pub message: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "AgentExecutorsRead")]
pub struct ExecutorsRead {}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "AgentExecutors")]
pub struct Executors {
    pub executors: Vec<String>,
    pub setup: Vec<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentExecute")]
pub struct Execute {
    /// IDs of immutable user uploads in this Session, resolved by the Agent owner.
    #[serde(default)]
    #[schemars(length(max = 8))]
    pub attachments: Vec<String>,
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(length(max = 65536))]
    pub message: String,
    pub executor: crate::executor::Selection,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentUploadAttachment")]
pub struct UploadAttachment {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(length(min = 1, max = 512))]
    pub name: String,
    #[schemars(range(max = 52428800))]
    pub bytes: usize,
    #[schemars(regex(pattern = "^sha256:[a-f0-9]{64}$"))]
    pub digest: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "AgentAttachmentUploadResult")]
pub enum AttachmentUploadResult {
    Uploaded { attachment: Attachment },
    Rejected { reason: AttachmentRejection },
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentAttachmentRejection")]
pub enum AttachmentRejection {
    InvalidFile,
    Busy,
    Closed,
    Archived,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentAttachment")]
pub struct Attachment {
    pub session_id: String,
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub bytes: u64,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentMarkRead")]
pub struct MarkRead {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(length(min = 1, max = 128))]
    pub message_id: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "AgentMarkReadResult")]
pub enum MarkReadResult {
    Observed { session: Box<Session> },
    Rejected { reason: Rejection },
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentStop")]
pub struct Stop {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(length(min = 1, max = 128))]
    pub invocation_id: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentWatch")]
pub struct Watch {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub after: u64,
}
/// Stable business-data origin, independent of package or activation revisions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentDataSource")]
pub struct DataSource {
    pub state_domain: String,
    pub deployment: uuid::Uuid,
    pub namespace: maka_plugin_core::composition::InstanceId,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentSession")]
pub struct Session {
    pub data_source: Box<DataSource>,
    pub permissions: WorkspacePermissions,
    pub boundary_revision: u64,
    pub id: String,
    pub title: String,
    pub labels: Vec<String>,
    pub workspace: Workspace,
    pub revision: u64,
    pub updated_at: u64,
    pub archived: bool,
    pub has_unread: bool,
    pub activity: SessionActivity,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentSessionActivity")]
pub enum SessionActivity {
    Idle,
    Working,
    Waiting,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentSessionPage")]
pub struct SessionPage {
    pub revision: String,
    pub needs_you_count: u64,
    #[schemars(length(max = 32))]
    pub sessions: Vec<Session>,
    #[schemars(with = "maka_runtime::schema::RequiredNullable<String>")]
    pub before: Option<String>,
    #[schemars(with = "maka_runtime::schema::RequiredNullable<String>")]
    pub after: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentSendResult")]
pub enum SendResult {
    Rejected {
        session_id: String,
        reason: SkillInputRejection,
        skills: Option<maka_skills::SkillInvocationResult>,
    },
    Started {
        session_id: String,
        invocation_id: String,
    },
    Queued {
        session_id: String,
        message_id: String,
        revision: u64,
    },
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentSkillInputRejection")]
pub enum SkillInputRejection {
    SkillsUnavailable,
    InputTooLarge,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentStopped")]
pub struct Stopped {
    pub requested: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentDelivery")]
pub struct Delivery {
    pub sequence: u64,
    pub invocation_id: String,
    pub turn_id: String,
    pub fact: DeliveryFact,
}
#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentDeliveryFact")]
pub enum DeliveryFact {
    Opened,
    /// Both external text streams share the original ExecutorStarted message.
    ExecutorStarted {
        message_id: String,
    },
    /// Canonical transcript facts changed; clients query bounded history.
    TranscriptChanged,
    StepEnded {
        step_id: String,
    },
    PartStarted {
        message_id: String,
        step_id: String,
        part_id: String,
        text_kind: ContentKind,
    },
    Delta {
        step_id: String,
        part_id: String,
        text: String,
    },
    PartFinished {
        step_id: String,
        part_id: String,
    },
    Ended {
        failed: bool,
    },
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentContentKind")]
pub enum ContentKind {
    Text,
    Thinking,
}
impl From<maka_runtime::model::TextKind> for ContentKind {
    fn from(value: maka_runtime::model::TextKind) -> Self {
        match value {
            maka_runtime::model::TextKind::Text => Self::Text,
            maka_runtime::model::TextKind::Thinking => Self::Thinking,
        }
    }
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentSessionRead")]
pub struct SessionRead {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentObservation")]
pub struct Observation {
    pub session: Session,
    pub through: u64,
    pub run: Option<Run>,
    pub parts: Vec<Part>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentRun")]
pub struct Run {
    pub invocation_id: String,
    pub turn_id: String,
    pub state: RunState,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentRunState")]
pub enum RunState {
    Admitted,
    Running,
    WaitingForUser,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentPart")]
pub struct Part {
    pub start_sequence: u64,
    pub message_id: String,
    pub step_id: String,
    pub part_id: String,
    pub text_kind: ContentKind,
}
#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentDeliveryPage")]
pub enum DeliveryPage {
    Events {
        through: u64,
        next_after: u64,
        events: Vec<Delivery>,
    },
    Queue {
        queue: Queue,
    },
    Interactions {
        pending: PendingInteractions,
    },
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentQueue")]
pub struct Queue {
    pub revision: u64,
    #[schemars(length(max = 64))]
    pub entries: Vec<QueueEntry>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentQueueEntry")]
pub struct QueueEntry {
    pub message_id: String,
    pub kind: QueueKind,
    #[schemars(length(max = 80))]
    pub preview: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentQueueKind")]
pub enum QueueKind {
    InFlight,
    Steering,
    Followup,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentQueueMessage")]
pub struct QueueMessage {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(length(min = 1, max = 128))]
    pub message_id: String,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub expected_revision: u64,
}
#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentQueueMessageResult")]
pub enum QueueMessageResult {
    Ready {
        revision: u64,
        has_attachments: bool,
        #[schemars(length(max = 65536))]
        message: String,
    },
    RevisionConflict {
        expected: u64,
        actual: u64,
    },
    Missing,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentQueueMutation")]
pub struct QueueMutation {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub expected_revision: u64,
    pub edit: QueueAction,
}
#[derive(Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentQueueAction")]
pub enum QueueAction {
    Retract {
        #[schemars(length(min = 1, max = 128))]
        message_id: String,
    },
    Promote {
        #[schemars(length(min = 1, max = 128))]
        message_id: String,
    },
    Update {
        #[schemars(length(min = 1, max = 128))]
        message_id: String,
        #[schemars(length(max = 65536))]
        message: String,
    },
    Reorder {
        #[schemars(length(min = 1, max = 64))]
        message_ids: Vec<String>,
    },
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "AgentQueueMutationResult")]
pub enum QueueMutationResult {
    Committed { revision: u64 },
    RevisionConflict { expected: u64, actual: u64 },
    Rejected { reason: QueueRejection },
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "AgentQueueRejection")]
pub enum QueueRejection {
    Closed,
    Missing,
    InFlight,
    InvalidEdit,
    ExecutionChanged,
    NotRunning,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentHistoryChunk")]
pub struct HistoryChunk {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<maka_plugins::session::history::TurnTiming>,
    pub field: maka_plugins::session::history::HistoryField,
    pub message_id: String,
    pub turn_id: String,
    pub timestamp: u64,
    pub role: Role,
    pub sequence: u64,
    pub offset: u64,
    pub total_bytes: u64,
    pub text: String,
    #[schemars(
        with = "maka_runtime::schema::RequiredNullable<maka_plugins::session::history::Tool>"
    )]
    pub tool: Option<maka_plugins::session::history::Tool>,
}
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentHistoryRead")]
pub struct History {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub through: Option<u64>,
    pub position: HistoryPosition,
    #[serde(default)]
    pub include_thinking: bool,
}
pub use maka_plugins::session::history::Position as HistoryPosition;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentHistorySearch")]
pub struct HistorySearch {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    /// Original raw-event fence, shared with history reads.
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub through: u64,
    /// Exclusive transcript-row sequence returned by the previous batch; zero starts a scan.
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub after: u64,
    /// Literal, case-sensitive query, additionally limited to 512 UTF-8 bytes.
    #[schemars(length(min = 1, max = 512))]
    pub query: String,
    pub include_internal: bool,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentHistoryMatch")]
pub struct HistoryMatch {
    /// Exact canonical field to read through the source query; history body may omit it.
    pub field: maka_plugins::session::history::SourceField,
    /// Use this exact cursor with HistoryPosition::After and the same raw-event fence.
    pub cursor: Cursor,
    #[schemars(length(max = 384))]
    pub preview: String,
}

#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentHistorySearchPage")]
pub enum HistorySearchPage {
    Preparing {
        through: u64,
    },
    Ready {
        through: u64,
        #[schemars(length(max = 16))]
        matches: Vec<HistoryMatch>,
        /// Continue even when this batch has no matches. Null is the end of the scan.
        #[schemars(with = "maka_runtime::schema::RequiredNullable<u64>")]
        next_after: Option<u64>,
    },
}

#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentHistoryPage")]
pub enum HistoryPage {
    Preparing {
        through: u64,
    },
    Ready {
        through: u64,
        chunks: Vec<HistoryChunk>,
        #[schemars(with = "maka_runtime::schema::RequiredNullable<Cursor>")]
        before: Option<Cursor>,
        #[schemars(with = "maka_runtime::schema::RequiredNullable<Cursor>")]
        after: Option<Cursor>,
    },
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentHistorySourceRead")]
pub struct HistorySourceRead {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub through: u64,
    pub source: HistorySourceReference,
    pub field: maka_plugins::session::history::SourceField,
    /// Optional JSON keys/array indices within arguments, result or receipt.
    #[serde(default)]
    #[schemars(length(max = 16))]
    pub path: Option<Vec<maka_plugins::session::history::SourceSegment>>,
    pub position: maka_plugins::session::history::SourcePosition,
}
#[derive(Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentHistorySourceReference")]
pub enum HistorySourceReference {
    Sequence {
        #[schemars(range(min = 1, max = 9_007_199_254_740_991u64))]
        sequence: u64,
    },
    ToolUse {
        #[schemars(length(min = 1, max = 256))]
        tool_use_id: String,
    },
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "AgentHistorySourcePage")]
pub enum HistorySourcePage {
    Preparing {
        through: u64,
    },
    Ready {
        through: u64,
        source: Box<maka_plugins::session::history::HistorySource>,
        /// Complete file-path evidence resolved on the Host for copying only.
        /// This is display text, never a filesystem capability or open command.
        #[serde(rename = "filePath", skip_serializing_if = "Option::is_none")]
        #[schemars(length(max = 4096))]
        file_path: Option<String>,
    },
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "AgentToolImageRead")]
pub struct ToolImageRead {
    #[schemars(length(min = 1, max = 128))]
    pub session_id: String,
    #[schemars(range(max = 9_007_199_254_740_991u64))]
    pub through: u64,
    #[schemars(range(min = 1, max = 9_007_199_254_740_991u64))]
    pub sequence: u64,
    #[schemars(range(max = 5_242_880u64))]
    pub offset: u64,
}
#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[schemars(rename = "AgentToolImagePage")]
pub enum ToolImagePage {
    Preparing {
        through: u64,
    },
    Ready {
        through: u64,
        sequence: u64,
        offset: u64,
        #[schemars(range(min = 1, max = 5_242_880u64))]
        total_bytes: u64,
        #[schemars(regex(pattern = "^image/(png|jpeg|gif|webp|avif)$"))]
        mime_type: String,
        #[schemars(regex(pattern = "^sha256:[a-f0-9]{64}$"))]
        digest: String,
        /// Standard padded Base64 of at most 32KiB of encoded image bytes.
        #[schemars(
            length(max = 43692),
            regex(pattern = "^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$")
        )]
        data: String,
    },
}

pub fn contract() -> Contract {
    fn method<I: JsonSchema, O: JsonSchema>(mode: MethodMode) -> Method {
        Method {
            progress: None,
            mode,
            input: serde_json::to_value(schemars::schema_for!(I)).expect("input schema"),
            output: serde_json::to_value(schemars::schema_for!(O)).expect("output schema"),
            resources: BTreeMap::new(),
        }
    }
    Contract {
        name: "maka.agent.Agent".into(),
        version: 1,
        transport: Transport::Remote,
        interface: Interface::Service {
            methods: BTreeMap::from([
                (
                    "list".into(),
                    method::<List, SessionPage>(MethodMode::Query),
                ),
                (
                    "catalog".into(),
                    method::<List, SessionPage>(MethodMode::Subscription),
                ),
                (
                    "projects".into(),
                    method::<ProjectsRead, ProjectsPage>(MethodMode::Query),
                ),
                (
                    "project".into(),
                    method::<ProjectChange, ProjectResult>(MethodMode::Mutation),
                ),
                (
                    "directories".into(),
                    method::<Directories, DirectoryPage>(MethodMode::Query),
                ),
                (
                    "observe".into(),
                    method::<SessionRead, Observation>(MethodMode::Query),
                ),
                (
                    "history".into(),
                    method::<History, HistoryPage>(MethodMode::Query),
                ),
                (
                    "search".into(),
                    method::<HistorySearch, HistorySearchPage>(MethodMode::Query),
                ),
                (
                    "source".into(),
                    method::<HistorySourceRead, HistorySourcePage>(MethodMode::Query),
                ),
                (
                    "toolImage".into(),
                    method::<ToolImageRead, ToolImagePage>(MethodMode::Query),
                ),
                (
                    "create".into(),
                    method::<Create, Created>(MethodMode::Mutation),
                ),
                (
                    "metadata".into(),
                    method::<Metadata, MetadataResult>(MethodMode::Mutation),
                ),
                // Authorizes the entire bounded run against the immutable selected
                // account/model. A started reply is not a terminal business verdict.
                (
                    "send".into(),
                    method::<Send, SendResult>(MethodMode::Mutation),
                ),
                (
                    "execute".into(),
                    method::<Execute, SendResult>(MethodMode::Mutation),
                ),
                (
                    "executorSetup".into(),
                    Method {
                        mode: MethodMode::Mutation,
                        input: schemars::schema_for!(crate::executor::setup::Input).into(),
                        output: schemars::schema_for!(crate::executor::setup::Outcome).into(),
                        progress: Some(
                            schemars::schema_for!(crate::executor::setup::Progress).into(),
                        ),
                        resources: BTreeMap::new(),
                    },
                ),
                (
                    "executors".into(),
                    method::<ExecutorsRead, Executors>(MethodMode::Query),
                ),
                (
                    "uploadAttachment".into(),
                    Method {
                        progress: None,
                        resources: BTreeMap::from([(
                            "file".into(),
                            maka_plugin_core::contract::binary_resource_type().required(),
                        )]),
                        ..method::<UploadAttachment, AttachmentUploadResult>(MethodMode::Mutation)
                    },
                ),
                (
                    "execution".into(),
                    method::<ExecutionRead, ExecutionResult>(MethodMode::Query),
                ),
                ("work".into(), method::<WorkRead, Work>(MethodMode::Query)),
                (
                    "resume".into(),
                    method::<Resume, ResumeResult>(MethodMode::Mutation),
                ),
                ("stop".into(), method::<Stop, Stopped>(MethodMode::Mutation)),
                (
                    "markRead".into(),
                    method::<MarkRead, MarkReadResult>(MethodMode::Mutation),
                ),
                (
                    "queue".into(),
                    method::<SessionRead, Queue>(MethodMode::Query),
                ),
                (
                    "queueMessage".into(),
                    method::<QueueMessage, QueueMessageResult>(MethodMode::Query),
                ),
                (
                    "editQueue".into(),
                    method::<QueueMutation, QueueMutationResult>(MethodMode::Mutation),
                ),
                (
                    "pendingInteractions".into(),
                    method::<SessionRead, PendingInteractions>(MethodMode::Query),
                ),
                (
                    "interaction".into(),
                    method::<InteractionRead, maka_runtime::interaction::InteractionRecord>(
                        MethodMode::Query,
                    ),
                ),
                (
                    "respond".into(),
                    method::<InteractionRespond, InteractionResult>(MethodMode::Mutation),
                ),
                (
                    "watch".into(),
                    method::<Watch, DeliveryPage>(MethodMode::Subscription),
                ),
            ]),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn history_window_contract_distinguishes_end_from_missing_boundaries() {
        let Interface::Service { methods } = contract().interface else {
            panic!("service contract");
        };
        let validator = jsonschema::validator_for(&methods["history"].output).unwrap();
        let mut page = serde_json::to_value(HistoryPage::Ready {
            through: 16,
            chunks: Vec::new(),
            before: None,
            after: Some(Cursor {
                field: Default::default(),
                sequence: 4096,
                offset: 0,
            }),
        })
        .unwrap();
        assert!(validator.is_valid(&page));
        page["chunks"] = json!([{
            "messageId":"result", "field":"body", "turnId":"turn", "timestamp":1, "role":"tool_result",
            "sequence":1, "offset":0, "totalBytes":0, "text":"",
            "tool":{"toolUseId":"call", "call":null, "result":{"sequence":1,"isError":true, "image":false,"settlement":null}}
        }]);
        assert!(validator.is_valid(&page));
        let tool = page["chunks"][0]
            .as_object_mut()
            .unwrap()
            .remove("tool")
            .unwrap();
        assert!(
            !validator.is_valid(&page),
            "tool facts are explicit, including null"
        );
        page["chunks"][0]["tool"] = tool;
        page["chunks"][0]["tool"]
            .as_object_mut()
            .unwrap()
            .remove("call");
        assert!(
            !validator.is_valid(&page),
            "missing call reference is not an observed absence"
        );
        page["chunks"][0]["tool"]["call"] = json!(null);
        assert!(validator.is_valid(&page));
        for field in ["before", "after"] {
            let value = page.as_object_mut().unwrap().remove(field).unwrap();
            assert!(!validator.is_valid(&page), "missing {field} is not EOF");
            page.as_object_mut().unwrap().insert(field.into(), value);
        }
        page["after"] = json!(null);
        assert!(validator.is_valid(&page));
        page["next"] = json!(null);
        assert!(
            !validator.is_valid(&page),
            "removed one-way API is rejected"
        );
    }
}
