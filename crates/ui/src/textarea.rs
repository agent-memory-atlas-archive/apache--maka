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

//! Native editor protocol. Snapshots are observations, never writable buffers.
use crate::editor::*;
/// Maximum retained editable buffer; individual commands remain at 64 KiB.
pub const MAX_BUFFER_BYTES: usize = 1024 * 1024;
use maka_plugin_core::contract::{Capture, Contract, Interface, Transport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
impl crate::editor::Props for TextareaProps {
    fn command(&self) -> Option<&EditorCommand> {
        self.command.as_ref()
    }
    fn set_command(&mut self, command: Option<EditorCommand>) {
        self.command = command;
    }
}
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextareaProps {
    pub appearance: crate::appearance::TextareaAppearance,
    #[serde(default)]
    pub menu: Option<TextareaMenu>,
    #[schemars(length(min = 1, max = 512))]
    pub accessibility_label: String,
    pub read_only: bool,
    pub disabled: bool,
    pub soft_wrap: bool,
    /// Preferred terminal text rows, excluding native padding. A constrained
    /// viewport may shrink further. Desktop does not use terminal row sizing.
    #[serde(default)]
    pub terminal_rows: TerminalTextareaRows,
    /// None preserves native multiline editing without a submit shortcut.
    #[serde(default)]
    pub submit: Option<SubmitKey>,
    #[schemars(range(min = -1, max = 32767))]
    pub tab_index: i16,
    /// Offer native focus once when this original input becomes visible and
    /// enabled. Stable updates and viewport scrolling do not repeat the offer.
    #[serde(default)]
    pub auto_focus: bool,
    #[serde(default)]
    #[schemars(length(max = 65536))]
    pub initial_text: String,
    #[serde(default)]
    #[schemars(length(max = 512))]
    pub placeholder: String,
    #[serde(default)]
    /// None issues no new command; an accepted command remains pending until
    /// its matching receipt commits. Clearing props is not cancellation.
    pub command: Option<EditorCommand>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerminalTextareaRows {
    #[schemars(range(min = 1))]
    pub min: u16,
    #[schemars(range(min = 1))]
    pub max: u16,
    /// Optional cap relative to the surface viewport's height in this paint
    /// pass, not the parent container. The minimum still applies.
    #[serde(default)]
    #[schemars(range(min = 0, max = 1))]
    pub max_viewport_fraction: Option<f64>,
}
impl Default for TerminalTextareaRows {
    fn default() -> Self {
        Self {
            min: 3,
            max: 8,
            max_viewport_fraction: None,
        }
    }
}

/// Local presentation for user-initiated native editing actions. This is not
/// a command interface: clipboard access still requires native activation.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextareaMenu {
    pub appearance: crate::appearance::MenuAppearance,
    pub labels: TextareaMenuLabels,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextareaMenuLabels {
    #[schemars(length(min = 1, max = 512))]
    pub title: String,
    #[schemars(length(min = 1, max = 512))]
    pub undo: String,
    #[schemars(length(min = 1, max = 512))]
    pub redo: String,
    #[schemars(length(min = 1, max = 512))]
    pub cut: String,
    #[schemars(length(min = 1, max = 512))]
    pub copy: String,
    #[schemars(length(min = 1, max = 512))]
    pub paste: String,
    #[schemars(length(min = 1, max = 512))]
    pub select_all: String,
}
/// Product-selected native submit gesture. PrimaryEnter uses the platform
/// accelerator on Desktop and Control+Enter in a terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubmitKey {
    Enter,
    PrimaryEnter,
    ControlEnter,
}

/// Submit carries the same original editor observation as a change; it grants
/// no text read and adds no second buffer or command owner.
pub fn reduce(
    call: crate::ComponentCall<'_>,
) -> crate::Result<crate::ComponentOutput<TextareaProps>> {
    let submitted = matches!(&call, crate::ComponentCall::Event { key, .. } if *key == "submit");
    let mut output = crate::editor::reduce::<TextareaProps>(call)?;
    let rows = output.frame.view.terminal_rows;
    if rows.min == 0
        || rows.min > rows.max
        || rows
            .max_viewport_fraction
            .is_some_and(|fraction| !fraction.is_finite() || fraction <= 0.0 || fraction > 1.0)
    {
        return Err(crate::Error::Invalid(
            "invalid terminal textarea row range".into(),
        ));
    }
    if submitted {
        for event in &mut output.events {
            event.name = "submitted".into();
        }
    }
    Ok(output)
}

pub fn textarea_contract() -> Contract {
    Contract {
        name: "maka.ui.Textarea".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Component {
            props: serde_json::to_value(schemars::schema_for!(TextareaProps))
                .expect("editor props schema"),
            events: BTreeMap::from([
                ("changed".into(), observation_schema()),
                ("submitted".into(), observation_schema()),
            ]),
            captures: BTreeMap::from([(
                "snapshot".into(),
                Capture {
                    input: serde_json::to_value(schemars::schema_for!(EditorCapture))
                        .expect("capture schema"),
                    output: maka_plugin_core::contract::CaptureOutput::Resource {
                        resource: text_resource_type(),
                    },
                },
            )]),
        },
    }
}
