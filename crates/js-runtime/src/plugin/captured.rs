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
use super::{Command, Module, Result, bridge, failed};
use serde_json::Value;
use std::{collections::BTreeSet, sync::Arc};
use tokio::sync::oneshot;

pub(super) enum Target {
    Path(Vec<String>),
    Captured { callback: u64, method: String },
}

/// Exact functions and receiver retained in their original worker module.
/// Handles are not serializable and cannot address another module or VM.
#[derive(Clone)]
pub struct Captured(Arc<Owner>);
struct Owner {
    module: Module,
    id: u64,
    methods: BTreeSet<String>,
}
impl Drop for Owner {
    fn drop(&mut self) {
        let _ = self.module.0.vm.0.commands.send(Command::ReleaseCaptured {
            module: self.module.0.id,
            callback: self.id,
        });
    }
}
impl Captured {
    pub(super) fn new(module: Module, id: u64, methods: Vec<String>) -> Self {
        Self(Arc::new(Owner {
            module,
            id,
            methods: methods.into_iter().collect(),
        }))
    }
    /// Calls may return a Promise. Dropping the waiting future does not cancel
    /// admitted work; the worker retains capacity until the actual call finishes.
    pub async fn call(&self, method: &str, args: Vec<Value>) -> Result<Value> {
        if !self.0.methods.contains(method) {
            return Err(failed("method was not captured"));
        }
        let module = &self.0.module;
        module.ready().await?;
        let size = bridge::encoded_size(&args, 1024 * 1024).map_err(failed)?;
        let slot = module
            .0
            .vm
            .0
            .calls
            .clone()
            .try_acquire_owned()
            .map_err(|_| failed("VM call capacity exhausted"))?;
        let bytes = module
            .0
            .vm
            .0
            .bytes
            .clone()
            .try_acquire_many_owned(size.max(1) as u32)
            .map_err(|_| failed("VM input budget exhausted"))?;
        let (reply, result) = oneshot::channel();
        module
            .0
            .vm
            .0
            .commands
            .send(Command::Call {
                module: module.clone(),
                target: Target::Captured {
                    callback: self.0.id,
                    method: method.into(),
                },
                args,
                reply,
                slot: Some(slot),
                bytes,
            })
            .map_err(|_| module.0.vm.0.health.error())?;
        result.await.map_err(|_| module.0.vm.0.health.error())?
    }
}
impl Module {
    /// Capture actual receiver/functions before capability publication. Lookup
    /// executes under the worker's poll watchdog; namespace mutation afterwards
    /// never redirects this registration. Last handle drop releases its globals.
    pub async fn capture(&self, namespace: Vec<String>, methods: Vec<String>) -> Result<Captured> {
        self.ready().await?;
        if namespace.len() > 32
            || namespace.iter().any(|s| s.is_empty() || s.len() > 128)
            || methods.is_empty()
            || methods.len() > 64
            || methods.iter().any(|s| s.is_empty() || s.len() > 128)
            || methods.iter().collect::<BTreeSet<_>>().len() != methods.len()
        {
            return Err(failed("invalid worker callback registration"));
        }
        let size = bridge::encoded_size(&(&namespace, &methods), 1024 * 1024).map_err(failed)?;
        let slot = self
            .0
            .vm
            .0
            .calls
            .clone()
            .try_acquire_owned()
            .map_err(|_| failed("VM call capacity exhausted"))?;
        let bytes = self
            .0
            .vm
            .0
            .bytes
            .clone()
            .try_acquire_many_owned(size.max(1) as u32)
            .map_err(|_| failed("VM input budget exhausted"))?;
        let (reply, result) = oneshot::channel();
        self.0
            .vm
            .0
            .commands
            .send(Command::Capture {
                module: self.clone(),
                namespace,
                methods,
                reply,
                slot,
                bytes,
            })
            .map_err(|_| self.0.vm.0.health.error())?;
        result.await.map_err(|_| self.0.vm.0.health.error())?
    }
}
