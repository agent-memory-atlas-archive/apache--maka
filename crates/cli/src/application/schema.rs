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

use maka_plugin_core::{composition, contract, package};
use maka_runtime_host::application::abi::{
    HostActivation, HostObservation, HostRequest, ResourceMetadata,
};
use maka_ui::{abi, actions, editor, input, markdown, text, textarea};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::{Value, json};

fn root<T: JsonSchema>(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<T>()
}

pub(super) fn catalog() -> Value {
    use composition::PresentationTarget;
    let mut g = SchemaGenerator::default();
    let roots = vec![
        root::<composition::Input>(&mut g),
        root::<maka_plugin_core::deployment::Deployment>(&mut g),
        root::<maka_plugin_core::deployment::management::DeploymentRead>(&mut g),
        root::<maka_plugin_core::deployment::management::PackageInstall>(&mut g),
        root::<maka_plugin_core::deployment::management::PackageInstalled>(&mut g),
        root::<maka_plugin_core::deployment::management::DeploymentReplace>(&mut g),
        root::<maka_plugin_core::deployment::management::ManagedDeployment>(&mut g),
        root::<maka_plugin_core::deployment::management::DeploymentInstances>(&mut g),
        root::<maka_plugin_core::deployment::management::ManagedInstancesPage>(&mut g),
        root::<maka_plugin_core::deployment::management::InstanceSetEnabled>(&mut g),
        root::<maka_plugin_core::deployment::management::DeploymentDocument>(&mut g),
        root::<maka_plugin_core::deployment::management::DeploymentDocumentRead>(&mut g),
        root::<maka_plugin_core::deployment::management::DeploymentDocumentChunk>(&mut g),
        root::<
            maka_plugin_core::deployment::management::ManagementResult<
                maka_plugin_core::deployment::management::PackageInstalled,
            >,
        >(&mut g),
        root::<
            maka_plugin_core::deployment::management::ManagementResult<
                maka_plugin_core::deployment::DeploymentRef,
            >,
        >(&mut g),
        root::<package::Declaration>(&mut g),
        root::<package::PackageRef>(&mut g),
        root::<contract::Contract>(&mut g),
        root::<contract::Import>(&mut g),
        root::<contract::Export>(&mut g),
        root::<abi::UiExport>(&mut g),
        root::<abi::ValueBindings>(&mut g),
        root::<abi::ComponentInput>(&mut g),
        root::<maka_ui::ComponentOutput>(&mut g),
        root::<maka_ui::ComponentSlot>(&mut g),
        root::<maka_ui::scene::terminal::Node>(&mut g),
        root::<maka_ui::scene::desktop::Node>(&mut g),
        root::<maka_ui::scene::desktop::canvas::CanvasInput>(&mut g),
        root::<actions::CapturePlan>(&mut g),
        root::<actions::ActionOutcome>(&mut g),
        root::<maka_ui::operations::CallerHistoryPage>(&mut g),
        root::<maka_event_log::applications::intents::CallerIntent>(&mut g),
        root::<maka_ui::LabelProps>(&mut g),
        root::<textarea::TextareaProps>(&mut g),
        root::<textarea::FilePasteObservation>(&mut g),
        root::<markdown::MarkdownProps>(&mut g),
        root::<maka_ui::diff::DiffProps>(&mut g),
        root::<markdown::MarkdownLink>(&mut g),
        root::<maka_ui::file::FilePickerProps>(&mut g),
        root::<maka_ui::file::FileObservation>(&mut g),
        root::<maka_ui::file::FileCapture>(&mut g),
        root::<maka_ui::image::ImageProps>(&mut g),
        root::<maka_ui::image::ImageObservation>(&mut g),
        root::<maka_ui::source::SourceTextProps>(&mut g),
        root::<maka_ui::terminal::TerminalViewProps>(&mut g),
        root::<maka_ui::terminal::TerminalViewObservation>(&mut g),
        root::<maka_ui::terminal::TerminalInputBatch>(&mut g),
        root::<maka_ui::terminal::TerminalInputAcknowledgement>(&mut g),
        root::<maka_ui::selection_menu::SelectionMenuProps>(&mut g),
        root::<maka_ui::reading::ReadingProps>(&mut g),
        root::<maka_ui::reading::ReadingObservation>(&mut g),
        root::<maka_ui::rows::RowsProps>(&mut g),
        root::<maka_ui::rows::RowActivated>(&mut g),
        root::<input::InputProps>(&mut g),
        root::<editor::EditorObservation>(&mut g),
        root::<maka_ui::checkpoint::EditorCheckpointObservation>(&mut g),
        root::<editor::EditorCapture>(&mut g),
        root::<text::ReadText>(&mut g),
        root::<text::TextChunk>(&mut g),
        root::<maka_agent_service::contract::Configuration>(&mut g),
        root::<maka_runtime::configuration::ModelSelection>(&mut g),
        root::<maka_settings_service::contract::ModelsRead>(&mut g),
        root::<maka_settings_service::contract::ModelsPage>(&mut g),
        root::<maka_settings_service::contract::ConnectionsRead>(&mut g),
        root::<maka_settings_service::contract::ConnectionsPage>(&mut g),
        root::<maka_settings_service::contract::ConnectionDetail>(&mut g),
        root::<maka_settings_service::contract::ProvidersRead>(&mut g),
        root::<maka_settings_service::contract::ProvidersPage>(&mut g),
        root::<maka_settings_service::contract::SaveCredential>(&mut g),
        root::<maka_settings_service::contract::CredentialSaveResult>(&mut g),
        root::<maka_runtime::configuration::CreateCatalogConnectionInput>(&mut g),
        root::<maka_runtime::configuration::UpdateCatalogConnectionInput>(&mut g),
        root::<maka_runtime::configuration::RemoveCatalogConnectionInput>(&mut g),
        root::<maka_runtime::configuration::CredentialVaultQueryInput>(&mut g),
        root::<maka_runtime::configuration::CredentialVaultQueryResult>(&mut g),
        root::<maka_runtime::configuration::DeleteCredentialInput>(&mut g),
        root::<maka_runtime::configuration::CredentialMutationResult>(&mut g),
        root::<maka_runtime::configuration::SetDefaultConnectionTargetInput>(&mut g),
        root::<maka_runtime::configuration::CatalogMutationResult>(&mut g),
        root::<maka_agent_service::contract::List>(&mut g),
        root::<maka_agent_service::contract::Create>(&mut g),
        root::<maka_agent_service::contract::Created>(&mut g),
        root::<maka_agent_service::contract::Directories>(&mut g),
        root::<maka_agent_service::contract::DirectoryPage>(&mut g),
        root::<maka_agent_service::contract::Metadata>(&mut g),
        root::<maka_agent_service::contract::MetadataResult>(&mut g),
        root::<maka_agent_service::contract::Send>(&mut g),
        root::<maka_agent_service::contract::Stop>(&mut g),
        root::<maka_agent_service::contract::UploadAttachment>(&mut g),
        root::<maka_agent_service::contract::Attachment>(&mut g),
        root::<maka_agent_service::contract::AttachmentUploadResult>(&mut g),
        root::<maka_agent_service::contract::MarkRead>(&mut g),
        root::<maka_agent_service::contract::MarkReadResult>(&mut g),
        root::<maka_agent_service::contract::Watch>(&mut g),
        root::<maka_agent_service::contract::PendingInteractions>(&mut g),
        root::<maka_agent_service::contract::InteractionRead>(&mut g),
        root::<maka_agent_service::contract::InteractionRespond>(&mut g),
        root::<maka_agent_service::contract::InteractionResult>(&mut g),
        root::<maka_agent_service::questions::Ask>(&mut g),
        root::<maka_agent_service::forms::Ask>(&mut g),
        root::<maka_agent_service::forms::Answer>(&mut g),
        root::<maka_agent_service::processes::Configuration>(&mut g),
        root::<maka_agent_service::processes::terminal::List>(&mut g),
        root::<maka_agent_service::processes::terminal::Target>(&mut g),
        root::<maka_agent_service::processes::terminal::Start>(&mut g),
        root::<maka_agent_service::processes::terminal::Controller>(&mut g),
        root::<maka_agent_service::processes::terminal::Control>(&mut g),
        root::<maka_agent_service::processes::terminal::Page>(&mut g),
        root::<maka_agent_service::processes::terminal::LeaseUpdate>(&mut g),
        root::<maka_agent_service::processes::terminal::ControlReceipt>(&mut g),
        root::<maka_agent_service::processes::terminal::Released>(&mut g),
        root::<maka_agent_service::processes::terminal::Stopped>(&mut g),
        root::<
            maka_agent_service::processes::terminal::Outcome<
                maka_agent_service::processes::terminal::Page,
            >,
        >(&mut g),
        root::<
            maka_agent_service::processes::terminal::Outcome<
                maka_agent_service::processes::terminal::ResourceUpdate,
            >,
        >(&mut g),
        root::<
            maka_agent_service::processes::terminal::Outcome<
                maka_agent_service::processes::terminal::LeaseUpdate,
            >,
        >(&mut g),
        root::<
            maka_agent_service::processes::terminal::Outcome<
                maka_agent_service::processes::terminal::ControlReceipt,
            >,
        >(&mut g),
        root::<
            maka_agent_service::processes::terminal::Outcome<
                maka_agent_service::processes::terminal::Released,
            >,
        >(&mut g),
        root::<
            maka_agent_service::processes::terminal::Outcome<
                maka_agent_service::processes::terminal::Stopped,
            >,
        >(&mut g),
        root::<maka_agent_service::questions::Answer>(&mut g),
        root::<maka_runtime::interaction::InteractionRecord>(&mut g),
        root::<maka_agent_service::contract::Session>(&mut g),
        root::<maka_agent_service::contract::SessionPage>(&mut g),
        root::<maka_agent_service::contract::SendResult>(&mut g),
        root::<maka_agent_service::contract::Stopped>(&mut g),
        root::<maka_agent_service::contract::Delivery>(&mut g),
        root::<maka_agent_service::contract::DeliveryFact>(&mut g),
        root::<maka_agent_service::contract::ContentKind>(&mut g),
        root::<maka_agent_service::contract::Observation>(&mut g),
        root::<maka_agent_service::contract::SessionRead>(&mut g),
        root::<maka_agent_service::contract::History>(&mut g),
        root::<maka_agent_service::contract::DeliveryPage>(&mut g),
        root::<maka_agent_service::contract::Queue>(&mut g),
        root::<maka_agent_service::contract::QueueMessage>(&mut g),
        root::<maka_agent_service::contract::QueueMessageResult>(&mut g),
        root::<maka_agent_service::contract::QueueMutation>(&mut g),
        root::<maka_agent_service::contract::QueueMutationResult>(&mut g),
        root::<maka_agent_service::contract::HistoryChunk>(&mut g),
        root::<maka_agent_service::contract::HistoryPage>(&mut g),
        root::<maka_agent_service::contract::HistorySearch>(&mut g),
        root::<maka_agent_service::contract::HistorySearchPage>(&mut g),
        root::<maka_agent_service::contract::HistorySourceRead>(&mut g),
        root::<maka_agent_service::contract::HistorySourcePage>(&mut g),
        root::<maka_agent_service::contract::ImageFrame>(&mut g),
        root::<maka_agent_service::contract::ToolImageRead>(&mut g),
        root::<maka_agent_service::contract::ToolImagePage>(&mut g),
        root::<maka_plugins::session::history::Read>(&mut g),
        root::<HostActivation>(&mut g),
        root::<HostRequest>(&mut g),
        root::<HostObservation>(&mut g),
        root::<maka_plugin_core::operation::OperationObservation>(&mut g),
        root::<maka_plugin_core::operation::OperationReply>(&mut g),
        root::<ResourceMetadata>(&mut g),
    ];
    // The union makes actual ABI types reachable to the generator. It is not
    // an additional message format or admission contract.
    json!({
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "RuntimeProtocol", "$defs": g.definitions(), "anyOf": roots,
        },
        "configurations": {
            "agent": maka_agent_service::configuration_schema(),
            "workspaceProcesses": maka_agent_service::processes::configuration_schema(),
        },
        "contracts": {
            "markdown": markdown::markdown_contract(),
            "diff": maka_ui::diff::diff_contract(),
            "image": maka_ui::image::image_contract(),
            "filePicker": maka_ui::file::file_picker_contract(),
            "sourceText": maka_ui::source::source_contract(),
            "terminalView": maka_ui::terminal::terminal_view_contract(),
            "selectionMenu": maka_ui::selection_menu::selection_menu_contract(),
            "rows": maka_ui::rows::rows_contract(),
            "readingArea": maka_ui::reading::reading_contract(),
            "label": maka_ui::label_contract(), "textarea": textarea::textarea_contract(),
            "textInput": input::text_input_contract(), "secretInput": input::secret_input_contract(),
            // Bundled availability is data, never an activated service or grant.
            "agent": maka_agent_service::contract(),
            "questions": maka_agent_service::questions::contract(),
            "forms": maka_agent_service::forms::contract(),
            "workspaceFiles": maka_agent_service::files::contract(),
            "workspaceProcesses": maka_agent_service::processes::contract(),
            "terminal": maka_agent_service::processes::terminal::contract(),
            "settings": maka_settings_service::contract::contract(),
            "textReader": text::reader_contract(),
            "callerHistory": maka_ui::operations::caller_history_contract(),
            "deploymentManager": maka_plugin_core::deployment::management::deployment_manager_contract(),
            "receiptReader": maka_plugin_core::operation::receipt_reader_contract(),
            "desktopSurface": maka_ui::surface_contract(PresentationTarget::Desktop),
            "terminalSurface": maka_ui::surface_contract(PresentationTarget::Tui),
        },
    })
}
