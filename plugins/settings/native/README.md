<!--
  Licensed to the Apache Software Foundation (ASF) under one
  or more contributor license agreements.  See the NOTICE file
  distributed with this work for additional information
  regarding copyright ownership.  The ASF licenses this file
  to you under the Apache License, Version 2.0 (the
  "License"); you may not use this file except in compliance
  with the License.  You may obtain a copy of the License at

      http://www.apache.org/licenses/LICENSE-2.0

  Unless required by applicable law or agreed to in writing,
  software distributed under the License is distributed on an
  "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
  KIND, either express or implied.  See the License for the
  specific language governing permissions and limitations
  under the License.
-->

# Root settings capability

`models` Start accepts an optional `query` of at most 256 Unicode characters; omission or empty text browses normally. All whitespace-separated terms must match the model ID, display label or connection name, case-insensitively. Each call examines at most 256 enabled models. An empty result may have a continuation, whose original cursor carries the exact query and physical position; the default projection remains independent of search.

`maka.settings` is a compiled native extension exporting `Settings` (`maka.settings.Settings` v1). Launchers register availability; an ordinary composition must activate and bind it. The actual `NativeContext` supplies persistence and its owned Root. A retained configuration SQL owner is drained on native close; cleanup failure keeps the Host owner and Root fence.

`models` returns at most sixteen public choices per page, including the exact observed connection revision. Its cursor remains bound to the original catalog revision; a changed catalog returns `changed` instead of mixing snapshots. Each ready page includes one `defaultModel` projection from that same snapshot: `none`, `unavailable` with its saved target, or `ready` with its exact choice. It is independent of pagination and uses the same eligibility and labels as the choices. Ready means selectable from this catalog, not authenticated or reachable. Choices contain labels and model IDs, with no credentials, endpoint, or native execution grant. Selecting a choice does not authorize execution.

`setDefault` is a durable mutation using the original catalog CAS. Its required nullable `target` explicitly sets or clears the default. SQL uncertainty remains the original operation's Unknown result; recovery never repeats a write. The settings contract uses the actual Rust deserialization shape for input and serialization shape for output; domain validation remains in the configuration owner.

`connections` returns at most sixteen public summaries per catalog revision. `connection` compares an exact connection basis and returns its public configuration and editable basic fields, or `stale`; hidden model inventories and overrides are not copied into the detail. `providers` pages the compiled API provider descriptors, eight per page within a 128 KiB encoded response budget. These descriptors describe availability, not activation or authority.

`create`, `update` and `remove` use the original configuration owner and durable mutation journal. Creation materializes provider defaults once; changing configuration validates the supplied object without rematerializing defaults. Updates merge only supplied fields after the original connection CAS. Omission preserves a field; explicit false and empty arrays remain updates; null clears only the original `modelOverrides` and `requestBodyOverlay` patches. Large hidden model collections therefore do not have to fit in a basic edit request.

Confirmed validation failures return `rejected` with a closed reason code, allowing a form to retain and correct its draft. This includes single-transaction configuration validation only after confirmed rollback. SQL, rollback, commit and owner uncertainty remain Unknown. Credential input validation is rejected before enrollment; errors after preparation or claim stay Unknown unless the original enrollment ticket confirms a terminal result. Neither result exposes raw diagnostics or input.

`credentialStatus` exposes public vault metadata; `deleteCredential` checks the original connection and credential generation together. `saveCredential` requires the observed connection target, an explicitly nullable credential basis, and an immutable `secret` attachment of type `maka.text.utf8`. It reads that request-local attachment only in the admitted mutation callback, authenticates with the compiled noninteractive API-key provider, and completes the original enrollment transaction. It acquires no network authority. Secrets and resource metadata do not enter public operation, caller intent or enrollment receipts. Its attempt identity derives from the complete original operation reference; recovery reads that operation rather than exchanging or writing again.

The default agent product binds this capability through ordinary model selection, default setting, connection forms and secure credential components. They remain replaceable through explicit contracts.
