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

# Native agent application service

This crate provides `maka.agent.api`, an ordinary native application implementation exporting `agent` (`maka.agent.Agent` v1). Desktop and TUI launchers register availability; immutable product composition decides activation. It does not open the legacy business Host or install legacy business plugins.

Configuration selects a nonsecret connection ID, slug, model and compiled API provider, with finite `maxActiveRuns` and `maxSteps` (both 1–16). The selected Root owns credentials, headers, endpoint and network settings. Each logical step reads that exact connection; unavailable selections never fall back. Provider credentials require the configuration store's real authentication receipt. This first service executes text conversations with an empty tool catalog.

`create` and `send` are durable mutations. `send` authorizes a complete bounded run on the configured account/model and returns its admitted invocation ID, not a terminal verdict. The original NativeOwner retains the run handle and a nonwaiting capacity permit before replying. Losing a request or UI observer does not cancel accepted work. `stop` names the exact invocation; stopping an old invocation cannot cancel a successor. `list`, `observe`, `history` and `watch` read canonical session metadata, same-snapshot run/active-part identities, text transcript pages and bounded delivery facts. Delivery excludes provider request payloads and credentials.

Session IDs belong to the actual Root/deployment/plugin instance, independent of package revision and activation epoch. Session metadata freezes the model/provider identity, not mutable launcher limits. Original operation references derive domain IDs; journal UUID conflicts remain fail-closed. Retiring the service cancels and drains its original workers and configuration SQL owner. Unconfirmed physical cleanup retains its permit and reports failed native close so the application keeps its Root fence.

`history` pins a committed `through` fence on its first page. Preparation is bounded; clients keep that fence while polling or following its UTF-8 byte cursor. Pages contain at most two 4096-byte text chunks, with enough headroom for JSON escaping; attachments and provider payloads are excluded. A `start` position begins a scan; an `event` position is an inclusive raw event-log sequence mapped by the Host to sparse transcript rows. A returned `cursor` position requires its original explicit fence. Active parts include original start sequences, allowing clients to replay text prefixes without reading hidden thinking as answers. Canonical transcript changes have explicit delivery invalidations. Every page rechecks the selected session namespace. Public Rust wire types feed the generated plugin SDK directly.

The initial product composition lives in [agent-product](../../packages/agent-product/README.md). Complete tool and connection-settings workflows remain subsequent product work; this crate is the execution service, not a complete agent application.
