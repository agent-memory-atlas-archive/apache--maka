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

use super::{ComponentOutput, Error, Node, Result, surface_contract};
use maka_plugin_core::{
    composition::PresentationTarget,
    contract::{Contract, Interface},
};
use serde_json::Value;
use std::{collections::BTreeMap, rc::Rc};

/// Native render payload stays on the presenter thread and is never serialized.
/// The embedding chooses V (e.g. a GPUI view recipe or a terminal renderer).
/// V is immutable presentation data, not a native resource owner. It must not
/// retain entities, input bindings, subscriptions or pending native work. Those
/// belong to renderer resource owners and keep the mount resource lease until
/// actual destruction. Retired frames can retain this recipe during cleanup.
pub struct NativeView<V: ?Sized> {
    pub view: Rc<V>,
    schemas: BTreeMap<String, Value>,
    inputs: BTreeMap<String, jsonschema::Validator>,
}
impl<V: ?Sized> NativeView<V> {
    pub fn new(view: Rc<V>, schemas: BTreeMap<String, Value>) -> Result<Self> {
        if schemas.len() > 64 {
            return Err(Error::Invalid("native component input limit".into()));
        }
        let mut inputs = BTreeMap::new();
        for (key, schema) in &schemas {
            if key.is_empty() || key.len() > 128 {
                return Err(Error::Invalid("invalid native input key".into()));
            }
            inputs.insert(
                key.clone(),
                jsonschema::validator_for(schema)
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            );
        }
        Ok(Self {
            view,
            schemas,
            inputs,
        })
    }
    pub(crate) fn schemas(&self) -> &BTreeMap<String, Value> {
        &self.schemas
    }
    fn validate_input(&self, key: &str, input: &Value) -> Result<()> {
        self.inputs
            .get(key)
            .ok_or_else(|| Error::Invalid("undeclared native input".into()))?
            .validate(input)
            .map_err(|error| Error::Invalid(error.to_string()))
    }
}

/// A committed tree or an opaque, typed native render recipe. Both belong to
/// the same mount, retain the same identity and use the same publication path.
pub enum Presentation<V: ?Sized> {
    Tree(Node),
    Native(NativeView<V>),
}
impl<V: ?Sized> Presentation<V> {
    pub(crate) fn validate(&self) -> std::result::Result<(), String> {
        match self {
            Self::Tree(node) => node.validate_root(),
            Self::Native(_) => Ok(()),
        }
    }
    pub(crate) fn validate_input(&self, key: &str, input: &Value) -> Result<()> {
        match self {
            Self::Tree(node) if node.has_action(key) => Ok(()),
            Self::Native(view) => view.validate_input(key, input),
            _ => Err(Error::Invalid("undeclared UI event".into())),
        }
    }
    pub(crate) fn components<'a>(&'a self, output: &mut Vec<super::frame::ComponentRef<'a>>) {
        if let Self::Tree(node) = self {
            node.components(output);
        }
    }
}

/// Native callbacks prepare an immutable scene. They do not receive a GPUI
/// context/editor handle: observable native effects execute after scene commit.
pub enum ComponentCall<'a> {
    Mount {
        props: &'a Value,
    },
    Update {
        state: &'a Value,
        props: &'a Value,
    },
    Event {
        state: &'a Value,
        props: &'a Value,
        key: &'a str,
        input: &'a Value,
        origin: EventOrigin<'a>,
    },
}
/// The adapter constructs this origin; an arbitrary raw input payload cannot
/// impersonate a child component notification.
pub enum EventOrigin<'a> {
    Raw,
    Action,
    Component { child: &'a str, name: &'a str },
}
pub trait NativeComponent<V: ?Sized>: 'static {
    fn capture(&self, _: super::actions::CaptureCall<'_>) -> Result<super::actions::CapturePlan> {
        Err(Error::Invalid("native capture method unavailable".into()))
    }
    fn call(&self, call: ComponentCall<'_>) -> Result<ComponentOutput<Presentation<V>>>;
}
pub enum NativeCapability<V: ?Sized> {
    Component(Rc<dyn NativeComponent<V>>),
    Surface(PresentationTarget),
    Service(std::sync::Arc<dyn super::actions::NativeService>),
}
pub struct NativeExport<V: ?Sized> {
    pub contract: Contract,
    pub value: NativeCapability<V>,
}
type Factory<V> = dyn Fn(&Value) -> Result<BTreeMap<String, NativeExport<V>>>;

/// Build-time implementation availability. The composition selects instances;
/// this map does not register capabilities or automatically activate products.
pub struct NativeRegistry<V: ?Sized> {
    factories: BTreeMap<String, Box<Factory<V>>>,
}
impl<V: ?Sized + 'static> Default for NativeRegistry<V> {
    fn default() -> Self {
        Self {
            factories: BTreeMap::new(),
        }
    }
}
impl<V: ?Sized + 'static> NativeRegistry<V> {
    pub fn register(
        &mut self,
        implementation: &str,
        factory: impl Fn(&Value) -> Result<BTreeMap<String, NativeExport<V>>> + 'static,
    ) -> Result<()> {
        if implementation.is_empty()
            || implementation.len() > 128
            || self.factories.contains_key(implementation)
        {
            return Err(Error::Invalid(
                "invalid or duplicate native implementation".into(),
            ));
        }
        self.factories
            .insert(implementation.into(), Box::new(factory));
        Ok(())
    }
    pub fn with_surface(target: PresentationTarget) -> Self {
        let mut registry = Self::default();
        let name = match target {
            PresentationTarget::Desktop => "maka.ui.surface.desktop",
            PresentationTarget::Tui => "maka.ui.surface.tui",
        };
        registry
            .register(name, move |_| {
                Ok(BTreeMap::from([(
                    "Surface".into(),
                    NativeExport {
                        contract: surface_contract(target),
                        value: NativeCapability::Surface(target),
                    },
                )]))
            })
            .expect("unique built-in surface");
        registry
            .register("maka.text.reader", |_| {
                Ok(BTreeMap::from([(
                    "Reader".into(),
                    NativeExport {
                        contract: super::text::reader_contract(),
                        value: NativeCapability::Service(std::sync::Arc::new(super::text::Reader)),
                    },
                )]))
            })
            .expect("unique native text reader");
        registry
    }
    pub(crate) fn validate_implementation(&self, implementation: &str) -> Result<()> {
        if !self.factories.contains_key(implementation) {
            return Err(Error::Invalid(format!(
                "native implementation not provided: {implementation}"
            )));
        }
        Ok(())
    }
    pub(crate) fn activate(
        &self,
        implementation: &str,
        configuration: &Value,
    ) -> Result<BTreeMap<String, NativeExport<V>>> {
        let actual = self.factories.get(implementation).ok_or_else(|| {
            Error::Invalid(format!(
                "native implementation not provided: {implementation}"
            ))
        })?(configuration)?;
        for export in actual.values() {
            export.contract.validate()?;
            match (&export.contract.interface, &export.value) {
                (Interface::Component { .. }, NativeCapability::Component(_)) => {}
                (Interface::Service { .. }, NativeCapability::Service(_)) => {}
                (_, NativeCapability::Surface(target))
                    if export.contract == surface_contract(*target) => {}
                _ => {
                    return Err(Error::Invalid(
                        "native export shape does not match its actual value".into(),
                    ));
                }
            }
        }
        Ok(actual)
    }
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LabelProps {
    #[schemars(length(max = 4096))]
    pub text: String,
    #[serde(default)]
    pub emphasis: bool,
}
pub fn label_contract() -> Contract {
    Contract {
        name: "maka.ui.Label".into(),
        version: 1,
        transport: maka_plugin_core::contract::Transport::Local,
        interface: Interface::Component {
            props: serde_json::to_value(schemars::schema_for!(LabelProps)).expect("label schema"),
            events: BTreeMap::new(),
            captures: BTreeMap::new(),
        },
    }
}
