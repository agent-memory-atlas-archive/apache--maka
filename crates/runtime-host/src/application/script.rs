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
use maka_js_runtime::plugin::Module;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Actual {
    contract: Contract,
    service: Vec<String>,
}

pub(super) async fn activate(
    module: Module,
    entry: maka_plugin_core::package::EntryDeclaration,
    configuration: Value,
) -> Result<BTreeMap<String, Published<Service>>> {
    let actual: BTreeMap<String, Actual> = serde_json::from_value(
        module
            .call(vec!["activate".into()], vec![configuration])
            .await
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    entry
        .check_exports(
            &actual
                .iter()
                .map(|(n, a)| (n.clone(), a.contract.clone()))
                .collect(),
        )
        .map_err(|e| e.to_string())?;
    let mut batch = BTreeMap::new();
    for (name, actual) in actual {
        let Interface::Service { methods } = &actual.contract.interface else {
            return Err("Host export must declare a service".into());
        };
        if actual.service.is_empty() {
            return Err("service namespace is missing".into());
        }
        let callbacks = module
            .capture(actual.service, methods.keys().cloned().collect())
            .await
            .map_err(|e| e.to_string())?;
        batch.insert(
            name,
            Published {
                value: Service::new(
                    actual.contract.clone(),
                    ImplementationValue::JavaScript(callbacks),
                )?,
                contract: actual.contract,
            },
        );
    }
    Ok(batch)
}
