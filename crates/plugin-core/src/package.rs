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

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Error,
    composition::Template,
    contract::{Availability, Contract, Export, Import, Interface},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageRef {
    pub id: String,
    pub revision: String,
}

impl PackageRef {
    pub fn validate(&self) -> Result<(), Error> {
        crate::name(&self.id)?;
        if !self.revision.strip_prefix("sha256-").is_some_and(|hash| {
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        }) {
            return Err(Error::Declaration(
                "package revision must pin immutable bytes".into(),
            ));
        }
        Ok(())
    }
}

/// Templates can select an entry in their own immutable package without a
/// self-referential content hash. Deployment roots must select a pinned revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum PackageSelection {
    Pinned { package: PackageRef },
    Current,
}

impl From<PackageRef> for PackageSelection {
    fn from(package: PackageRef) -> Self {
        Self::Pinned { package }
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EntryTarget {
    Host,
    Tui,
    Desktop,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "PackageDeclaration")]
pub struct Declaration {
    #[serde(default)]
    pub entries: BTreeMap<EntryTarget, EntryDeclaration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition: Option<Template>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntryDeclaration {
    pub implementation: Implementation,
    pub configuration: Value,
    #[serde(default)]
    pub imports: BTreeMap<String, Import>,
    #[serde(default)]
    pub exports: BTreeMap<String, Export>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Implementation {
    #[serde(rename = "javascript")]
    JavaScript {
        module: String,
        realm: Realm,
    },
    Native {
        implementation: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Realm {
    Shared,
    Dedicated,
}

impl EntryDeclaration {
    pub fn validate(&self) -> Result<(), Error> {
        match &self.implementation {
            Implementation::JavaScript { module, .. } if module.is_empty() => {
                return Err(Error::Declaration("JavaScript module is missing".into()));
            }
            Implementation::Native { implementation } => crate::name(implementation)?,
            _ => {}
        }
        crate::schema(&self.configuration)?;
        validate_imports(&self.imports)?;
        for (name, export) in &self.exports {
            crate::name(name)?;
            export.contract.validate()?;
        }
        Ok(())
    }

    /// Check the entire activation batch before publishing any actual exports.
    pub fn check_exports(&self, actual: &BTreeMap<String, Contract>) -> Result<(), Error> {
        for (name, contract) in actual {
            if !self
                .exports
                .get(name)
                .is_some_and(|declared| declared.contract.accepts(contract))
            {
                return Err(Error::Contract(format!(
                    "undeclared or changed export: {name}"
                )));
            }
        }
        for (name, export) in &self.exports {
            if export.availability == Availability::Required && !actual.contains_key(name) {
                return Err(Error::Contract(format!("missing required export: {name}")));
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_imports(imports: &BTreeMap<String, Import>) -> Result<(), Error> {
    for (name, import) in imports {
        crate::name(name)?;
        import.contract.validate()?;
        import.cardinality.validate()?;
    }
    Ok(())
}

impl Declaration {
    pub fn validate(&self) -> Result<(), Error> {
        if self.entries.is_empty() && self.composition.is_none() {
            return Err(Error::Declaration(
                "package has no entries or composition".into(),
            ));
        }
        for entry in self.entries.values() {
            entry.validate()?;
        }
        if let Some(template) = &self.composition {
            template.validate()?;
        }
        if let (Some(tui), Some(desktop)) = (
            self.entries.get(&EntryTarget::Tui),
            self.entries.get(&EntryTarget::Desktop),
        ) {
            for (name, export) in &tui.exports {
                if let Some(other) = desktop.exports.get(name)
                    && (matches!(export.contract.interface, Interface::Component { .. })
                        || matches!(other.contract.interface, Interface::Component { .. }))
                    && export != other
                {
                    return Err(Error::Contract(format!(
                        "component variants disagree: {name}"
                    )));
                }
            }
        }
        Ok(())
    }
}

/// Immutable declarations indexed by their exact package revision.
#[derive(Clone, Default)]
pub struct Packages(BTreeMap<PackageRef, Declaration>);

impl Packages {
    pub fn insert(&mut self, package: PackageRef, declaration: Declaration) -> Result<(), Error> {
        package.validate()?;
        declaration.validate()?;
        if self
            .0
            .get(&package)
            .is_some_and(|current| current != &declaration)
        {
            return Err(Error::Declaration(
                "package revision was reused with different declarations".into(),
            ));
        }
        self.0.insert(package, declaration);
        Ok(())
    }

    pub fn get(&self, package: &PackageRef) -> Option<&Declaration> {
        self.0.get(package)
    }
}
