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

//! Bounded asynchronous UTF-8 reads through explicit request-local resources.
use super::actions::{NativeService, ServiceCompletion, ServiceFuture, ServiceRequest};
use maka_plugin_core::contract::{
    Contract, Interface, Method, MethodMode, ResourceType, Transport,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadText {
    pub start: usize,
    pub end: usize,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextChunk {
    pub text: String,
    pub metadata: Value,
}
pub fn text_resource_type() -> ResourceType {
    ResourceType {
        name: "maka.text.utf8".into(),
        version: 1,
    }
}
pub fn reader_contract() -> Contract {
    Contract {
        name: "maka.text.Reader".into(),
        version: 1,
        transport: Transport::Local,
        interface: Interface::Service {
            methods: BTreeMap::from([(
                "read".into(),
                Method {
                    mode: MethodMode::Query,
                    input: serde_json::to_value(schemars::schema_for!(ReadText))
                        .expect("read input schema"),
                    output: serde_json::to_value(schemars::schema_for!(TextChunk))
                        .expect("chunk schema"),
                    resources: BTreeMap::from([("text".into(), text_resource_type())]),
                },
            )]),
        },
    }
}
pub(super) struct Reader;
impl NativeService for Reader {
    fn call(&self, request: ServiceRequest) -> ServiceFuture {
        let ServiceRequest {
            method,
            input,
            mut resources,
            ..
        } = request;
        Box::pin(async move {
            let result = (|| {
                if method != "read" {
                    return Err("unknown text reader method".into());
                }
                let range: ReadText = serde_json::from_value(input).map_err(|e| e.to_string())?;
                if range
                    .end
                    .checked_sub(range.start)
                    .is_none_or(|n| n > 64 * 1024)
                {
                    return Err("text read exceeds 64 KiB or range invalid".into());
                }
                let resource = resources.remove("text").ok_or("missing text resource")?;
                if resource.resource_type() != text_resource_type() {
                    return Err("text resource type mismatch".into());
                }
                let bytes = resource.read(range.start..range.end)?;
                if bytes.len() > 64 * 1024 {
                    return Err("text resource returned oversized chunk".into());
                }
                let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
                serde_json::to_value(TextChunk {
                    text,
                    metadata: resource.metadata(),
                })
                .map_err(|e| e.to_string())
            })();
            ServiceCompletion {
                result,
                cleanup: Ok(()),
            }
        })
    }
}
