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

Configuration declares finite `maxActiveRuns` and `maxSteps` (both 1–16). Each `send` supplies an explicit `model` selection containing the connection ID, observed revision and model ID; its first SQL snapshot checks that revision and enabled membership before preparing any run. The selected Root owns credentials, headers, endpoint and network settings. Each logical step reads that exact connection; unavailable selections never fall back. Provider credentials require the configuration store's real authentication receipt. This first service executes text conversations with an empty tool catalog.

`create` and `send` are durable mutations. `send` authorizes a complete bounded run on the explicitly selected account/model and returns its admitted invocation ID, not a terminal verdict. The original NativeOwner retains the run handle and a nonwaiting capacity permit before replying. Losing a request or UI observer does not cancel accepted work. `stop` names the exact invocation; stopping an old invocation cannot cancel a successor. `list`, `observe`, `history` and `watch` read canonical session metadata, same-snapshot run/active-part identities, text transcript pages and bounded delivery facts. Delivery excludes provider request payloads and credentials.

Session IDs belong to the actual Root/deployment/plugin instance, independent of package revision and activation epoch. Sessions retain conversation metadata; each canonical invocation records its actual selected binding. A later send in the same conversation may choose another model. The run freezes provider identity, stored configuration and resolved execution parameters; name, testing, catalog descriptions and other models do not interrupt it. Endpoint, protocol, budget or selected membership changes reject subsequent steps. Credentials, headers and network policy continue through their original configuration owner. Original operation references derive domain IDs; journal UUID conflicts remain fail-closed. Retiring the service cancels and drains its original workers and configuration SQL owner. Unconfirmed physical cleanup retains its permit and reports failed native close so the application keeps its Root fence.

`history` pins a committed raw-event `through` fence on its first window. Preparation is bounded; retries keep the same position and returned fence. `start` scans forward and `tail` reads the end; `before` ends exclusively and `after` starts inclusively at a real sparse transcript row and UTF-8 byte offset. Both cursor positions require the original explicit fence. Ready windows always return `before` and `after`; only `null` means that direction has ended. Windows retain ascending source order, including backward reads, with at most two fragments and 8192 UTF-8 bytes in total. The complete serialized response is checked against 64 KiB; attachments and provider payloads are excluded. `event` locates an inclusive raw event-log sequence for execution bookmarks, without fabricating a transcript cursor. Active parts include original start sequences, allowing clients to replay text prefixes without reading hidden thinking as answers. Canonical transcript changes have explicit delivery invalidations. Every window rechecks the selected session namespace. Public Rust wire types feed the generated plugin SDK directly.

The initial product composition lives in [agent-product](../../packages/agent-product/README.md). Complete tool and connection-settings workflows remain subsequent product work; this crate is the execution service, not a complete agent application.
