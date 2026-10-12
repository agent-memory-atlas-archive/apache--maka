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

use crate::{ProviderKind, sdk};
use maka_js_runtime::trusted::TrustedRuntime;
use maka_plugins::model::{Binding, ProviderAdapter};
use maka_runtime::composition::{SourceKind, SourceRevision};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) fn builtin(runtime: TrustedRuntime, owner: SourceRevision) -> BTreeMap<String, Binding> {
    let sdk = Arc::new(sdk::Adapter(runtime));
    let providers: [(&str, Arc<dyn ProviderAdapter>); 4] = [
        ("responses", Arc::new(maka_responses::Adapter::default())),
        ("chat-completions", sdk.clone()),
        ("anthropic-messages", sdk),
        (
            maka_providers::codex::ADAPTER,
            Arc::new(maka_providers::codex::Codex::default()),
        ),
    ];
    providers
        .into_iter()
        .map(|(name, provider)| {
            let mut source = owner.clone();
            source.kind = SourceKind::ModelAdapter;
            source.name = name.into();
            (name.into(), Binding::new(provider, source))
        })
        .collect()
}

pub fn name(kind: &ProviderKind) -> &str {
    match kind {
        ProviderKind::OpenaiResponses | ProviderKind::OpenResponses(_) => "responses",
        ProviderKind::Anthropic => "anthropic-messages",
        ProviderKind::OpenaiChat | ProviderKind::OpenaiCompatible { .. } => "chat-completions",
    }
}
