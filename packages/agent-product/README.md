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

# Agent product composition

[简体中文](README.zh-CN.md)

This is an ordinary plugin composition for a text agent, with separate desktop and terminal entries. The launchers contain no product pages. Workspace, composer, navigation, timeline and appearance are individually replaceable packages, connected through explicit public contracts. Native packages provide surfaces, the editor, bounded text capture, operation history and the Host agent service.

The initial product supports conversation creation, selection, canonical transcript pages, streaming replies, exact-run stop and recovery of original mutation receipts. It uses typed English, Simplified Chinese and Traditional Chinese catalogs. The desktop design uses scoped light/dark appearance, system typography, quiet navigation and a native multiline composer. Full tool workflows, settings, attachments, history virtualization and platform visual/IME/accessibility acceptance remain unfinished.

## Build and author

From the repository root:

```sh
just setup
npm --workspace @maka-agent/product run build
node packages/agent-product/scripts/compose.mjs \
  --configuration installation.json --output product-source \
  --maka /absolute/path/to/maka
```

`installation.json` supplies `presentation` (`desktop` or `tui`), the full `locale` tag, `scheme` (`light` or `dark`), `hostRoot`, `hostDeployment`, `agent` and `keys`. `agent` is the SDK's generated `AgentConfiguration`: finite `maxActiveRuns`/`maxSteps`. Credentials and endpoint settings belong to that Host Root; they do not belong in the product source.

`keys` maps all twelve names exported by `src/composition.ts` (`workspace`, `composer`, `navigation`, `timeline`, `appearance`, `surface`, `editor`, `reader`, `history`, `agent`, `receipts`, `gateway`) to nonnil UUIDs. Allocate these once for an installation and keep them across rebuilds. Deployment UUIDs are likewise stable installation identities. New package revisions do not imply new data namespaces.

The authoring command asks the actual Rust CLI to validate every package and compute its pinned reference. It writes and checks `host.json`, `ui.json`, `remote-exports.json`, `connections.json` and `packages.json`. It does not initialize a Root, commit a deployment or start execution. `connections.json` is trusted launcher configuration; inspect its explicit method and recovery grants before use.

For installation, initialize separate owned Host/UI Roots with `maka host init --root PATH`, then commit each source with `maka application commit`. Supply the selected deployment UUID, the current `--expected-revision`, and each built directory with `--package PATH`; UI also takes `--remote-exports product-source/remote-exports.json`. A first commit uses expected revision zero. Launch the committed UI with:

```sh
maka application run --root UI_ROOT --deployment UI_DEPLOYMENT \
  --connections product-source/connections.json
# For a desktop source:
maka-desktop --root UI_ROOT --deployment UI_DEPLOYMENT \
  --connections product-source/connections.json
```

The selected Host Root must already contain an authenticated, enabled connection matching `agent.model`. The existing [trusted local connection](../../crates/application-bridge/README.md) owns Host startup and cleanup. Product code neither chooses a fallback model nor imports private execution objects.

## State and verification

The native editor owns the draft, selection and IME. JavaScript retains only observations, command receipts and workflow state. Sending captures an immutable resource at the original editor owner/revision; only a durable admission permits clearing that draft. Unknown results keep the original recovery reference and are queried, never resent automatically. New mounts read their persisted caller history and recover original receipts without clearing a new editor.

History fixes its log fence and follows native transcript cursors. Active parts replay from their original start events; thinking is excluded from answer text. Canonical transcript invalidations refresh durable messages without restarting the watch. Display excerpts are bounded and labelled; they are not a substitute for complete history pagination.

```sh
npm --workspace @maka-agent/product run typecheck
npm --workspace @maka-agent/product test
# Adds actual immutable-package and Host/UI plan checks:
MAKA_PRODUCT_CLI_CHECK=1 npm --workspace @maka-agent/product test
# Requires the normal Node build dependencies and an actual native test binary:
cargo nextest run -p maka-agent-service --run-ignored only \
  -E 'test(packaged_product_creates)'
```

The opt-in Rust integration uses the actual packaged guest, native TUI editor, persisted caller intents, authenticated native service and a local HTTP provider. It exercises creation, input, send, canonical response and reopening the same deployment. Passing protocol tests does not establish the final desktop visual or three-platform input quality.

The ordinary `models` component reads bounded choices from the explicitly bound Root settings capability. It offers the saved default only when no model is selected; choosing another model captures its observed connection revision for that send. Desktop presents a scrollable list; TUI pages three choices at a time with visible navigation. No host-selected model fallback or unknown-result resend is performed. Connection creation and secure credential editing remain product work.
