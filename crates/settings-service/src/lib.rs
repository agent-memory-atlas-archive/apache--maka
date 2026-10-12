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

//! Root-owned settings as an ordinary compiled capability, activated only by composition.
mod connections;
pub mod contract;
mod credential;
pub mod network;
pub mod preferences;
mod service;
pub mod usage;
use futures_util::future::BoxFuture;
use maka_runtime_host::application::{NativeActivation, NativeOwner, NativeRegistry};
use std::sync::Arc;
pub const IMPLEMENTATION: &str = "maka.settings";
pub const EXPORT: &str = "Settings";
pub fn provide(registry: &mut NativeRegistry) -> Result<(), String> {
    registry.insert(
        IMPLEMENTATION.into(),
        Arc::new(|context, input| {
            Box::pin(async move {
                let opened = async {
                    if input != serde_json::json!({}) {
                        return Err("settings_configuration_invalid".to_owned());
                    }
                    let persistence = context
                        .persistence()
                        .ok_or("settings_requires_persistence")?;
                    let root = persistence
                        .store()
                        .owned_root()
                        .map_err(|_| "settings_root_unavailable")?;
                    let store = maka_config::ConfigurationStore::for_root(root)
                        .await
                        .map_err(|_| "settings_store_unavailable")?;
                    Ok::<_, String>(Arc::new(service::Settings {
                        configuration: Arc::new(store),
                        log: persistence.store().clone(),
                    }))
                }
                .await;
                match opened {
                    Ok(owner) => NativeActivation {
                        exports: Ok(std::collections::BTreeMap::from([(
                            EXPORT.into(),
                            maka_plugin_core::runtime::Published {
                                contract: contract::contract(),
                                value: owner.clone()
                                    as Arc<dyn maka_runtime_host::application::Handler>,
                            },
                        )])),
                        owner,
                    },
                    Err(error) => NativeActivation {
                        exports: Err(error),
                        owner: Arc::new(Uninitialized),
                    },
                }
            })
        }),
    )
}
struct Uninitialized;
impl NativeOwner for Uninitialized {
    fn prepare_retirement(
        &self,
    ) -> Option<Box<dyn maka_runtime_host::application::NativeRetirement>> {
        Some(Box::new(()))
    }
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}
