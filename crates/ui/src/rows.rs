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

//! Compact native rows with local selection, scrolling and hover controls.
//! Products supply row/action identities; this component knows no business model.
use crate::{appearance::Color, shortcut::Shortcut};
use maka_plugin_core::contract::{Contract, Interface, Transport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RowsProps {
    pub revision: u64,
    #[schemars(length(max = 64))]
    pub rows: Vec<Row>,
    #[schemars(length(max = 32))]
    pub shortcuts: Vec<Shortcut>,
    #[schemars(range(min = 1, max = 64))]
    pub max_rows: u16,
    #[schemars(range(min = -1, max = 32767))]
    pub tab_index: i16,
    pub foreground: Color,
    pub muted: Color,
    pub selected: Color,
    pub hovered: Color,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Row {
    #[schemars(length(min = 1, max = 128))]
    pub key: String,
    #[schemars(length(max = 1024))]
    pub text: String,
    #[schemars(length(max = 8))]
    pub actions: Vec<RowAction>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RowAction {
    #[schemars(length(min = 1, max = 128))]
    pub key: String,
    #[schemars(length(max = 64))]
    pub label: String,
    pub enabled: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RowActivated {
    pub revision: u64,
    pub row: String,
    pub action: String,
}
impl RowsProps {
    pub fn validate(&self) -> crate::Result<()> {
        let mut rows = BTreeSet::new();
        for row in &self.rows {
            if !rows.insert(&row.key) {
                return Err(crate::Error::Invalid("duplicate row identity".into()));
            }
            let mut actions = BTreeSet::new();
            if row
                .actions
                .iter()
                .any(|action| !actions.insert(&action.key))
            {
                return Err(crate::Error::Invalid("duplicate row action".into()));
            }
        }
        let mut chords = BTreeSet::new();
        // Unlike bubbling command scopes, a native row selector owns printable
        // keys. Shortcut shape is validated by the original component contract.
        if self
            .shortcuts
            .iter()
            .any(|shortcut| !chords.insert((&shortcut.key, shortcut.physical(false))))
        {
            return Err(crate::Error::Invalid("duplicate row shortcut".into()));
        }
        Ok(())
    }
}
pub fn activated_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(RowActivated)).expect("row activation")
}
pub fn rows_contract() -> Contract {
    Contract {
        name: "maka.ui.Rows".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Component {
            props: serde_json::to_value(schemars::schema_for!(RowsProps)).expect("row props"),
            events: BTreeMap::from([("activated".into(), activated_schema())]),
            captures: BTreeMap::new(),
        },
    }
}
