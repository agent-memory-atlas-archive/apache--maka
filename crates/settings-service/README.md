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

`maka.settings` is a compiled native extension exporting `Settings` (`maka.settings.Settings` v1). Launchers register availability; an ordinary composition must activate and bind it. The actual `NativeContext` supplies persistence and its owned Root. A retained configuration SQL owner is drained on native close; cleanup failure keeps the Host owner and Root fence.

`models` returns at most sixteen public choices per page, including the exact observed connection revision. Its cursor remains bound to the original catalog revision; a changed catalog returns `changed` instead of mixing snapshots. Projections contain labels, model IDs and an explicit default flag, with no credentials, endpoint, or native execution grant. Selecting a choice does not authorize execution.

`setDefault` is a durable mutation using the original catalog CAS. Its required nullable `target` explicitly sets or clears the default. SQL uncertainty remains the original operation's Unknown result; recovery never repeats a write. The settings contract is generated from the actual Rust serialization shape, while domain validation remains in the configuration owner.

`connections` returns at most sixteen public summaries per catalog revision. `connection` compares an exact connection basis and returns its public configuration and editable basic fields, or `stale`; hidden model inventories and overrides are not copied into the detail. `providers` pages the compiled API provider descriptors, eight per page within a 128 KiB encoded response budget. These descriptors describe availability, not activation or authority.

`create`, `update` and `remove` use the original configuration owner and durable mutation journal. Creation materializes provider defaults once; changing configuration validates the supplied object without rematerializing defaults. Updates merge only supplied fields after the original connection CAS. Omission preserves a field; explicit false and empty arrays remain updates; null clears only the original `modelOverrides` and `requestBodyOverlay` patches. Large hidden model collections therefore do not have to fit in a basic edit request.

`credentialStatus` exposes public vault metadata; `deleteCredential` checks the original connection and credential generation together. `saveCredential` requires the observed connection target, an explicitly nullable credential basis, and an immutable `secret` attachment of type `maka.text.utf8`. It reads that request-local attachment only in the admitted mutation callback, authenticates with the compiled noninteractive API-key provider, and completes the original enrollment transaction. It acquires no network authority. Secrets and resource metadata do not enter public operation, caller intent or enrollment receipts. Its attempt identity derives from the complete original operation reference; recovery reads that operation rather than exchanging or writing again.

The default agent product currently binds this capability through its ordinary model selector. Full connection forms and secure input workflows still require product UI integration.
