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

use super::*;

/// Trusted native initialization receives facts from its actual Host owner,
/// after the initialization resource fence has been acquired. There is no guest
/// constructor, serialized capability, or implicit service activation here.
/// Any acquired native work remains the returned NativeOwner's responsibility.
pub struct NativeContext {
    pub(super) activation: Activation,
    pub(super) persistence: Option<NativePersistence>,
}
impl NativeContext {
    pub fn activation(&self) -> &Activation {
        &self.activation
    }
    /// Ephemeral applications have no persistence. A native service requiring
    /// persistence must reject initialization rather than open a fallback Root.
    pub fn persistence(&self) -> Option<&NativePersistence> {
        self.persistence.as_ref()
    }
}

/// Exact applied source and its actual store travel together. Stable native
/// data ownership uses the Root domain, deployment UUID and instance identity;
/// source revision and activation epoch identify execution, not a new data owner.
/// Cloning the store retains its lease; it does not acknowledge native cleanup.
pub struct NativePersistence {
    pub(super) source: DeploymentRef,
    pub(super) store: Arc<maka_event_log::EventLog>,
}
impl NativePersistence {
    pub fn source(&self) -> &DeploymentRef {
        &self.source
    }
    pub fn store(&self) -> &Arc<maka_event_log::EventLog> {
        &self.store
    }
}
