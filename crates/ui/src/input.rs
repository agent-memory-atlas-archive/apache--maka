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

//! Fixed native single-line exports; secrets never become observation values.
use crate::{ComponentCall, ComponentOutput, Emission, editor::*};
use maka_plugin_core::contract::{Capture, Contract, Interface, Transport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputProps {
    pub appearance: crate::appearance::InputAppearance,
    #[schemars(length(min = 1, max = 512))]
    pub accessibility_label: String,
    pub read_only: bool,
    pub disabled: bool,
    #[schemars(range(min = -1, max = 32767))]
    pub tab_index: i16,
    #[serde(default)]
    #[schemars(length(max = 512))]
    pub placeholder: String,
    #[serde(default)]
    pub command: Option<EditorCommand>,
}
impl crate::editor::Props for InputProps {
    fn command(&self) -> Option<&EditorCommand> {
        self.command.as_ref()
    }
    fn set_command(&mut self, command: Option<EditorCommand>) {
        self.command = command;
    }
}
pub fn representable(text: &str) -> bool {
    text.len() <= 65536 && !text.chars().any(char::is_control)
}
pub fn submitted_schema() -> serde_json::Value {
    json!({"type":"object","additionalProperties":false,"properties":{}})
}
pub fn reduce(call: ComponentCall<'_>) -> crate::Result<ComponentOutput<InputProps>> {
    if let ComponentCall::Event {
        state,
        props,
        key: "submit",
        ..
    } = call
    {
        let mut output =
            crate::editor::reduce::<InputProps>(ComponentCall::Update { state, props })?;
        output.events.push(Emission {
            name: "submitted".into(),
            value: json!({}),
        });
        return Ok(output);
    }
    crate::editor::reduce(call)
}
pub fn text_input_contract() -> Contract {
    contract("maka.ui.TextInput")
}
pub fn secret_input_contract() -> Contract {
    contract("maka.ui.SecretInput")
}
fn contract(name: &str) -> Contract {
    Contract {
        name: name.into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Component {
            props: serde_json::to_value(schemars::schema_for!(InputProps)).expect("input props"),
            events: BTreeMap::from([
                ("changed".into(), observation_schema()),
                ("submitted".into(), submitted_schema()),
            ]),
            captures: BTreeMap::from([(
                "snapshot".into(),
                Capture {
                    input: serde_json::to_value(schemars::schema_for!(EditorCapture))
                        .expect("input capture"),
                    resource: text_resource_type(),
                },
            )]),
        },
    }
}
