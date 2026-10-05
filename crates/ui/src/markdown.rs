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

//! Read-only Markdown presentation. Source is content, never a resource grant.
use crate::appearance::{Color, TextAppearance};
use maka_plugin_core::contract::{Contract, Interface, Transport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarkdownAppearance {
    pub body: TextAppearance,
    pub code: TextAppearance,
    /// Typography for heading levels 1 through 6, in source order.
    pub headings: [TextAppearance; 6],
    pub muted: Color,
    pub link: Color,
    pub selection: Color,
    pub code_background: Color,
    pub table_background: Color,
    pub border: Color,
    pub dark: bool,
    #[schemars(range(min = 0, max = 128))]
    /// Paragraph spacing in logical pixels.
    pub paragraph_gap: f32,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarkdownProps {
    #[schemars(length(max = 65536))]
    pub source: String,
    pub appearance: MarkdownAppearance,
    pub selectable: bool,
    pub disabled: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarkdownLink {
    #[schemars(length(max = 65536))]
    pub url: String,
}
pub fn link_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(MarkdownLink)).expect("Markdown link schema")
}
pub fn markdown_contract() -> Contract {
    Contract {
        name: "maka.ui.Markdown".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Component {
            props: serde_json::to_value(schemars::schema_for!(MarkdownProps))
                .expect("Markdown props"),
            events: BTreeMap::from([("link".into(), link_schema())]),
            captures: BTreeMap::new(),
        },
    }
}
