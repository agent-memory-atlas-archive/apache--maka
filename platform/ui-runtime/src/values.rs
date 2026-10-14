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

//! Immutable values use the existing actual registration.
use super::*;
use maka_plugin_core::{contract::Import, runtime::Capability};

pub(super) type Imports<V> = BTreeMap<String, Vec<Capability<Export<V>>>>;

pub(super) fn export<V: ?Sized>(value: Value, contract: &Contract) -> Result<Export<V>> {
    contract.validate()?;
    let Interface::Value { schema } = &contract.interface else {
        return Err(Error::Invalid("value contract required".into()));
    };
    jsonschema::validator_for(schema)
        .map_err(|e| Error::Invalid(e.to_string()))?
        .validate(&value)
        .map_err(|e| Error::Invalid(e.to_string()))?;
    Ok(Export::Value(value))
}

impl<V: ?Sized + 'static> Application<V> {
    pub(super) fn capture_values(
        &mut self,
        activation: &Activation,
        imports: &BTreeMap<String, Import>,
    ) -> Result<()> {
        let mut captured = BTreeMap::new();
        for (name, import) in imports {
            if matches!(import.contract.interface, Interface::Value { .. }) {
                captured.insert(name.clone(), self.runtime.bind(activation, name)?);
            }
        }
        self.values.insert(activation.epoch(), captured);
        Ok(())
    }

    pub(super) fn values_snapshot(&self, activation: &Activation) -> Result<abi::ValueBindings> {
        let imports = self
            .values
            .get(&activation.epoch())
            .ok_or_else(|| Error::Invalid("value consumer activation unavailable".into()))?;
        let mut bindings = BTreeMap::new();
        for (name, capabilities) in imports {
            let mut members = Vec::new();
            for capability in capabilities {
                let Export::Value(value) = self.runtime.inspect(capability)? else {
                    return Err(Error::Invalid("bound value export required".into()));
                };
                members.push(value.clone());
            }
            bindings.insert(name.clone(), members);
        }
        Ok(abi::ValueBindings(bindings))
    }

    pub(super) fn mount_values(&self, mount: &Mount) -> Result<abi::ValueBindings> {
        let provider = self.provider(mount)?;
        self.values_snapshot(&provider)
    }
}
