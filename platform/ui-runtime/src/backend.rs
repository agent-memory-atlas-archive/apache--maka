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

//! Component execution backends return inert declarations and batches. The UI
//! runtime alone owns input admission, transaction commit and native cleanup.
use crate::{
    ComponentCall, ComponentOutput, Result, abi, actions::CaptureCall, actions::CapturePlan,
};
use serde_json::Value;
use std::{collections::BTreeMap, rc::Rc, time::Instant};

pub trait Factory: 'static {
    fn create(&self, scheduler: tokio::runtime::Handle) -> Result<Rc<dyn Realm>>;
}

/// Shared modules retain the same realm object and therefore the same failure fate.
pub trait Realm: 'static {
    fn load(&self, name: &str, source: &str) -> Result<Rc<dyn Module>>;
    fn failure(&self) -> Option<String>;
}

/// An original module remains owned until close confirms its handles are released.
pub trait Module: 'static {
    fn activate(
        &self,
        configuration: &Value,
        values: &abi::ValueBindings,
    ) -> Result<BTreeMap<String, abi::UiExport>>;
    fn component(&self, namespace: &[String], captures: bool) -> Result<Rc<dyn Component>>;
    fn close(&self) -> Result<()>;
}

/// Captured callbacks retain their original receiver and realm. Returned batches
/// contain no native resource authority and cannot commit effects themselves.
pub trait Component: 'static {
    fn call(
        &self,
        call: ComponentCall<'_>,
        values: &abi::ValueBindings,
        deadline: Instant,
    ) -> Result<ComponentOutput>;
    fn capture(&self, call: CaptureCall<'_>, deadline: Instant) -> Result<CapturePlan>;
}
