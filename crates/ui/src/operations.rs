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

//! Caller-owned original references exposed through an ordinary local service.
use crate::{NativeCapability, NativeExport, NativeRegistry, actions::*};
use maka_event_log::{EventLog, applications::intents::CallerIntent};
use maka_plugin_core::{
    contract::{Contract, Interface, Method, MethodMode, Transport},
    deployment::DeploymentRef,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};

/// Available only in a persisted UI application; composition selects instances.
pub const CALLER_HISTORY: &str = "maka.operations.history";

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CallerHistoryPage {
    #[schemars(range(max = 9_007_199_254_740_991_u64))]
    pub after: u64,
    #[schemars(range(min = 1, max = 128))]
    pub limit: usize,
}

pub fn caller_history_contract() -> Contract {
    Contract {
        name: "maka.operations.CallerHistory".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Service {
            methods: BTreeMap::from([(
                "page".into(),
                Method {
                    mode: MethodMode::Query,
                    input: serde_json::to_value(schemars::schema_for!(CallerHistoryPage))
                        .expect("caller history request schema"),
                    output: serde_json::to_value(schemars::schema_for!(Vec<CallerIntent>))
                        .expect("caller history page schema"),
                    resources: BTreeMap::new(),
                },
            )]),
        },
    }
}

struct History {
    source: DeploymentRef,
    store: Arc<EventLog>,
}
impl NativeService for History {
    fn call(&self, request: ServiceRequest) -> ServiceFuture {
        let source = self.source.clone();
        let store = self.store.clone();
        Box::pin(async move {
            let result = async {
                if request.method != "page"
                    || !request.resources.is_empty()
                    || request.source.deployment().is_none_or(|caller| {
                        caller.state_domain != source.state_domain
                            || caller.deployment != source.deployment
                    })
                {
                    return Err("caller history requires its original persisted UI source".into());
                }
                let page: CallerHistoryPage =
                    serde_json::from_value(request.input).map_err(|error| error.to_string())?;
                let intents = store
                    .application_intents(
                        source.deployment,
                        request.source.consumer().instance().clone(),
                        page.after,
                        page.limit,
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                let value = serde_json::to_value(intents).map_err(|error| error.to_string())?;
                maka_js_runtime::plugin::check_json_delivery(&value, 1024 * 1024)
                    .map_err(|error| error.to_string())?;
                Ok(value)
            }
            .await;
            // No per-request native resource is created. The database owner
            // independently retains the Root until its actual SQL cleanup.
            ServiceCompletion {
                result,
                cleanup: Ok(()),
            }
        })
    }
}

/// Source and store come from the same actual loaded Root, before activation.
/// Neither caller namespace nor deployment can be selected through guest JSON.
pub(crate) fn provide<V: ?Sized + 'static>(
    native: &mut NativeRegistry<V>,
    source: DeploymentRef,
    store: Arc<EventLog>,
) -> crate::Result<()> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Configuration {}
    native.register(CALLER_HISTORY, move |configuration: &Value, _values| {
        serde_json::from_value::<Configuration>(configuration.clone())
            .map_err(|error| crate::Error::Invalid(error.to_string()))?;
        Ok(BTreeMap::from([(
            "History".into(),
            NativeExport {
                contract: caller_history_contract(),
                value: NativeCapability::Service(Arc::new(History {
                    source: source.clone(),
                    store: store.clone(),
                })),
            },
        )]))
    })
}
