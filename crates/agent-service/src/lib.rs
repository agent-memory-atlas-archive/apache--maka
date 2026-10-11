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

//! Agent execution as an ordinary native application plugin. Product composition
//! chooses whether to activate it; installing this crate does not start a product.
/// Public wire types and generated contracts. 公开协议类型与生成契约。
pub mod contract;
pub mod executor;
pub mod files;
pub mod forms;
mod interaction_wait;
mod model;
mod owner;
mod permissions;
pub mod processes;
pub mod questions;
mod service;
pub mod skills;
mod source_path;
mod tools;
pub mod workhub;
mod workspace;
pub use contract::{Configuration, ModelSelection, configuration_schema, contract};
use futures_util::future::BoxFuture;
use maka_runtime_host::application::{
    NativeActivation, NativeContext, NativeOwner, NativeRegistry,
};
use std::sync::Arc;

pub const IMPLEMENTATION: &str = "maka.agent.api";
pub const EXPORT: &str = "agent";

pub fn provide(registry: &mut NativeRegistry) -> Result<(), String> {
    files::provide(registry)?;
    skills::provide(registry)?;
    processes::provide(registry)?;
    questions::provide(registry)?;
    forms::provide(registry)?;
    workhub::provide(registry)?;
    registry.insert(
        IMPLEMENTATION.into(),
        Arc::new(|context, configuration| {
            Box::pin(async move {
                match owner::Owner::open(context, configuration).await {
                    Ok(owner) => {
                        let service = Arc::new(service::Agent(owner));
                        NativeActivation {
                            exports: Ok(std::collections::BTreeMap::from([(
                                EXPORT.into(),
                                maka_plugin_core::runtime::Published {
                                    contract: contract(),
                                    value: service.clone()
                                        as Arc<dyn maka_runtime_host::application::Handler>,
                                },
                            )])),
                            owner: service,
                        }
                    }
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
    fn close(&self) -> BoxFuture<'static, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}
