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

//! One captured endpoint set; declarations never become executable bindings.
use super::{Description, Endpoint};
use crate::{Grant, Result};
use maka_runtime_host::application::CapturedImport;
use std::collections::BTreeMap;

pub struct Endpoints(BTreeMap<String, Entry>);
enum Entry {
    Ready(Endpoint),
    Unavailable(Description),
}
impl From<BTreeMap<String, Endpoint>> for Endpoints {
    fn from(endpoints: BTreeMap<String, Endpoint>) -> Self {
        Self(
            endpoints
                .into_iter()
                .map(|(name, endpoint)| (name, Entry::Ready(endpoint)))
                .collect(),
        )
    }
}
impl Endpoints {
    pub(crate) fn empty() -> Self {
        Self(BTreeMap::new())
    }
    pub(crate) fn insert(
        &mut self,
        name: String,
        import: CapturedImport,
        grant: Grant,
    ) -> Result<()> {
        let entry = match import {
            CapturedImport::Ready(binding) => Entry::Ready(Endpoint::new(binding, grant)?),
            CapturedImport::Unavailable {
                provider,
                export,
                contract,
            } => {
                super::policy::check_grant(&contract, &grant)?;
                Entry::Unavailable(Description {
                    provider,
                    export,
                    contract,
                    registration_epoch: None,
                })
            }
        };
        if self.0.insert(name, entry).is_some() {
            return Err("duplicate Application endpoint".into());
        }
        Ok(())
    }
    pub(super) fn validate(&self) -> Result<()> {
        if self.0.is_empty()
            || self.0.len() > 128
            || self
                .0
                .keys()
                .any(|name| name.is_empty() || name.len() > 256)
        {
            return Err("invalid Application endpoints".into());
        }
        Ok(())
    }
    pub(super) fn get(&self, name: &str) -> Option<&Endpoint> {
        match self.0.get(name)? {
            Entry::Ready(endpoint) => Some(endpoint),
            Entry::Unavailable(_) => None,
        }
    }
    pub(crate) fn epochs(&self) -> BTreeMap<String, Option<uuid::Uuid>> {
        self.0
            .iter()
            .map(|(name, entry)| {
                (
                    name.clone(),
                    match entry {
                        Entry::Ready(endpoint) => Some(endpoint.binding.registration_epoch()),
                        Entry::Unavailable(_) => None,
                    },
                )
            })
            .collect()
    }
    pub(super) fn descriptions(&self) -> BTreeMap<String, Description> {
        self.0
            .iter()
            .map(|(name, entry)| {
                (
                    name.clone(),
                    match entry {
                        Entry::Ready(endpoint) => Description {
                            provider: endpoint.binding.provider().instance().clone(),
                            export: endpoint.binding.export().into(),
                            contract: endpoint.binding.contract().clone(),
                            registration_epoch: Some(endpoint.binding.registration_epoch()),
                        },
                        Entry::Unavailable(description) => description.clone(),
                    },
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_declarations_still_require_the_original_method_grant() {
        let captured = || CapturedImport::Unavailable {
            provider: maka_plugin_core::composition::InstanceId(vec![uuid::Uuid::new_v4()]),
            export: "Receipts".into(),
            contract: maka_plugin_core::operation::receipt_reader_contract(),
        };
        let mut endpoints = Endpoints::empty();
        assert!(
            endpoints
                .insert(
                    "receipts".into(),
                    captured(),
                    Grant {
                        methods: ["write".into()].into(),
                        recovery: true
                    }
                )
                .is_err()
        );
        endpoints
            .insert(
                "receipts".into(),
                captured(),
                Grant {
                    methods: ["read".into()].into(),
                    recovery: true,
                },
            )
            .unwrap();
        let catalogue = super::super::Catalogue::captured(&endpoints);
        assert!(
            catalogue
                .describe("receipts")
                .unwrap()
                .registration_epoch
                .is_none()
        );
        assert!(catalogue.select("receipts").is_err());
        assert!(endpoints.get("receipts").is_none());
        assert!(
            catalogue
                .check_names(["missing".to_owned()].iter())
                .is_err()
        );
    }
}
