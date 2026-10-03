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
use maka_ui::{abi, actions, editor, text};
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
        root::<package::Declaration>(&mut g),
        root::<package::PackageRef>(&mut g),
        root::<contract::Contract>(&mut g),
        root::<contract::Import>(&mut g),
        root::<contract::Export>(&mut g),
        root::<abi::ComponentActivation>(&mut g),
        root::<abi::ComponentInput>(&mut g),
        root::<maka_ui::ComponentOutput>(&mut g),
        root::<actions::CapturePlan>(&mut g),
        root::<actions::ActionOutcome>(&mut g),
        root::<maka_ui::operations::CallerHistoryPage>(&mut g),
        root::<maka_event_log::applications::intents::CallerIntent>(&mut g),
        root::<maka_ui::LabelProps>(&mut g),
        root::<editor::EditorProps>(&mut g),
        root::<editor::EditorObservation>(&mut g),
        root::<editor::EditorCapture>(&mut g),
        root::<text::ReadText>(&mut g),
        root::<text::TextChunk>(&mut g),
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
        "contracts": {
            "label": maka_ui::label_contract(), "editor": editor::editor_contract(),
            "textReader": text::reader_contract(),
            "callerHistory": maka_ui::operations::caller_history_contract(),
            "desktopSurface": maka_ui::surface_contract(PresentationTarget::Desktop),
            "terminalSurface": maka_ui::surface_contract(PresentationTarget::Tui),
        },
    })
}
