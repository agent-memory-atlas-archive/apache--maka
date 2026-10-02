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

# Maka plugin SDK

[简体中文](README.zh-CN.md)

The root import `@maka-agent/plugin-sdk` describes the new application runtime.
Its wire types and built-in contracts are generated from the Rust types used by
Host and UI execution. `Component` describes synchronous callbacks; `RequestContext`
uses actual invocation-local slots and returns `unknown` for service results.
Validate those results against the service contract; a TypeScript generic is not
validation. Numeric bounds, JSON limits and authority are checked by Rust.

```sh
npm run schema:generate -w @maka-agent/plugin-sdk
npm run schema:check -w @maka-agent/plugin-sdk
node packages/plugin-sdk/scripts/bundle.mjs packages/plugin-sdk/examples/counter/desktop.ts /tmp/counter.mjs
```

The [counter](examples/counter/) has independent Desktop and TUI entrypoints;
the [Host forwarder](examples/forwarder.ts) maps request-local slots explicitly.
Their bundled modules execute in the real application integration tests. Build the
workspace before importing its package exports. Bundles must contain no unresolved
imports or top-level await; they do not embed native extensions or confer authority.

Host application services support queries and durable, resource-free mutations.
`context.invoke({ operation: 'mutate', ..., operationId })` returns an `OperationReply`
with the original `reference` even when the result is unknown or cannot be delivered.
Only `receipt.kind === 'recorded'` describes journal facts; inspect both `outcome`
and `cleanup`. `recover` takes that exact reference, independently of current imports,
and requires a separate recovery grant. `not_recorded` means an authorized lookup
found no original row; `unavailable` makes no journal claim. Neither triggers replay.
Keep the original reference for recovery; a new operation identity can execute a new effect.
Persisted UI components submit resource-free mutations through their ordinary
`actions`. The kernel generates the operation identity and records the original
caller intent before dispatch. Feedback uses `ActionOutcome` with
`status: 'operation'` and the original `OperationReply`; components must handle
unknown/unavailable receipts without automatically repeating the action.
Subscriptions, effectful resource snapshots and business migration remain unfinished.

Deployment sources can be stored in an owned State Root with
`maka application commit --root ROOT --deployment UUID --expected-revision 0 --input input.json --package bundle.maka-plugin`.
A replacement requires the observed revision. `maka application read --root ROOT --deployment UUID`
validates the persisted source and exact package bytes. The returned `DeploymentRef` identifies
that state domain and source revision; it grants no access and makes no readiness claim.
The Host executor can start from this snapshot and retains storage ownership through accepted
work and cleanup. Original receipts survive source replacement and restart. Resolving unknown external effects,
effectful resource snapshots and live reconciliation remain unfinished.

The `./host` API and the guide below belong to the implementation still awaiting
migration and removal. They are not an adapter for the application API.

Public TypeScript contracts for the Rust Runtime Host. **Host SDK 3** and
**terminal View 9** are independent of Maka's application version and this npm
workspace's version. The workspace is private and is not an assumed registry package.

Plugin development has one maintained guide: the
[Maka plugin authoring skill](../../skills/maka-plugin-authoring/SKILL.md).
It is bundled with Maka, including executable starter files and the complete SDK
source contracts. It is installed automatically for each Profile and its complete directory is
replaced with the shipped version at startup, removing obsolete resources. Enable/disable
and pinning preferences are preserved; keep custom changes in a separate skill. Load
`/skill:maka-plugin-authoring` or let the model discover it. `Skill` reads its linked
resources without exposing Host-private directories to ordinary file tools.

Start with the [quickstart](../../skills/maka-plugin-authoring/references/quickstart.md)
and [runnable package](../../skills/maka-plugin-authoring/assets/starter/).
The [API map](../../skills/maka-plugin-authoring/references/api-map.md) routes to
exact [SDK sources](src/host.ts); separate guides cover tools/input, authority/state,
terminal apps, providers/executors, native Rust, and testing/shipping.

```sh
npm --workspace @maka-agent/plugin-sdk run build
npm --workspace @maka-agent/plugin-sdk run typecheck
node --test packages/plugin-sdk/tests/*.test.mjs
```

Package runtime/UI entries are self-contained ESM with no unresolved imports or
top-level await. Use type-only SDK imports during development and the issued Host
capabilities for runtime effects. See the guide for manifests, lifecycle and activation.
