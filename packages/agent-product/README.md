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

# Plugin product compositions

[简体中文](README.zh-CN.md)

This is an ordinary plugin composition for a text agent, with separate desktop and terminal entries. The launchers contain no product pages. Workspace, composer, navigation, timeline and appearance are individually replaceable packages, connected through explicit public contracts. Native packages provide surfaces, the editor, bounded text capture, operation history and the Host agent service.

The initial product supports conversation creation, selection, canonical transcript pages, streaming replies, exact-run stop and recovery of original mutation receipts. It uses typed English, Simplified Chinese and Traditional Chinese catalogs. The desktop design uses scoped light/dark appearance, system typography, quiet navigation and a native multiline composer. Connection browsing, creation, public field editing and native API-key saving/removal are ordinary plugins. Default model setting and clearing are ordinary plugins with explicit confirmation and receipt recovery. Tool workflows, attachments, history virtualization and platform visual/IME/accessibility acceptance remain unfinished.

## Configuration Studio

An independent ordinary `maka.studio.workbench` package provides desktop and terminal
roots for configuring connections, default models and plugin composition. It uses
quiet top navigation and retains hidden page mounts and native drafts. It has no
conversation, composer or Agent binding; its separate Host source contains only
Settings, Receipts and a gateway with their explicit grants. Shared settings
components remain replaceable. Model selection is local to the workbench; saving a
default still uses the original confirmed Settings operation.

Select it at authoring time with `--product studio` (the default is `agent`). The
Studio installation configuration uses the same fields and keys below, except
`agent` is unnecessary. Both products use the same generic launch commands and
checked source replacement; neither requires a launcher product-name branch.
Installing a root package does not switch products. Changing a running product
requires a separately confirmed source update and original surface cleanup. A new
root does not inherit the previous product’s native drafts. Changing Host location
also requires explicit trusted connection configuration; source declarations do
not grant access to a different Host.

## Build and author

From the repository root:

```sh
just setup
npm --workspace @maka-agent/product run build
node packages/agent-product/scripts/compose.mjs \
  --configuration installation.json --output product-source \
  --maka /absolute/path/to/maka
```

`installation.json` supplies `presentation` (`desktop` or `tui`), the full `locale` tag, `scheme` (`light` or `dark`), `hostRoot`, `hostDeployment`, `managementDeployment`, `agent` and `keys`. `agent` is the SDK's generated `AgentConfiguration`: explicit `workspaceRoots` (`[{ label, path }]`, absolute directories), finite `maxActiveRuns`/`maxSteps` and explicit `tools` (use `[]` for a text-only composition). Credentials and endpoint settings belong to that Host Root; they do not belong in the product source.

`keys` maps every name exported by `src/composition.ts` to a nonnil UUID. Allocate these once for an installation and keep them across rebuilds. `managementDeployment` is a stable Host UUID in the UI Root, distinct from the UI and business Host deployment. New package revisions do not imply new data namespaces.

The authoring command asks the actual Rust CLI to validate every package and compute its pinned reference. It writes and checks `host.json`, `management-host.json`, `ui.json`, `remote-exports.json`, `connections.json` and `packages.json`. It does not initialize a Root, commit a deployment or start execution. `connections.json` is trusted launcher configuration; inspect its explicit method and recovery grants before use.

For installation, initialize separate owned Host/UI Roots with `maka host init --root PATH`, then commit each source with `maka application commit`. Supply the selected deployment UUID, the current `--expected-revision`, and each built directory with `--package PATH`; UI also takes `--remote-exports product-source/remote-exports.json`. A first commit uses expected revision zero. Commit `management-host.json` into the **same UI Root**, using `managementDeployment`; it contains the explicitly selected UI manager, receipt service and gateway. Its connection is a cohost, so it borrows that launcher's original Store. The business Host remains a separate owned Root. Launch the committed UI with:

```sh
maka application run --root UI_ROOT --deployment UI_DEPLOYMENT \
  --connections product-source/connections.json
# For a desktop source:
maka desktop --root UI_ROOT --deployment UI_DEPLOYMENT \
  --connections product-source/connections.json
```

The trusted connections file may declare additional Hosts for later compositions.
Unselected declarations acquire no Root; a checked committed source starts a newly
selected Host using its original explicit configuration and grants.

Sending requires an authenticated, enabled connection and an explicitly selected model from the selected Host Root. The existing [trusted local connection](../../crates/application-bridge/README.md) owns Host startup and cleanup. Product code neither chooses a fallback model nor imports private execution objects.

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
# Actual foreground install and automatic Agent -> Studio surface/backend switch:
cargo nextest run -p maka-cli --test integration --run-ignored only --locked \
  -E 'test(agent_installs_studio)'
```

The opt-in Rust integration uses the actual packaged guest, native TUI editor, persisted caller intents, authenticated native service and a local HTTP provider. It exercises creation, input, send, canonical response and reopening the same deployment. The separate CLI PTY test installs Studio through native file input and commits its
composition through the original manager. The production pipeline replaces the
surface and opens a previously idle, explicitly configured settings Host. It checks
the persisted source/package, final Root release and real terminal restoration.
Passing protocol tests does not establish the final desktop visual or three-platform input quality.

The ordinary `models` component reads bounded choices from the explicitly bound Root settings capability. Its default projection is independent of the choice page, so a default beyond the first sixteen choices can still be suggested. It offers the saved default only when no model is selected; choosing another model captures its observed connection revision for that send. Desktop presents a scrollable list; TUI pages three choices at a time with visible navigation. No host-selected model fallback or unknown-result resend is performed. Catalog refresh preserves the chosen connection/model identity and rechecks its actual revision; it never switches to an unrelated default.

`connection-form` creates connections from observed provider descriptors and edits only explicitly selected public fields. Unread model collections, overrides and body overlays remain unchanged unless the user chooses complete replacement. Endpoint editing preserves other configuration keys; complete configuration editing is a separate choice. Native editors retain all drafts across TUI field pages and confirmations. Saving captures every selected original owner/revision and validates the complete typed input against build-time compiled Rust schemas before issuing a mutation. Invalid JSON types and CAS rejection leave the form editable. A conflict requires explicit review and confirmation of the current connection/catalog version; an unknown result checks the original operation without retrying it. This component has Reader access for public fields and no SecretInput import. Edited fields are bounded to 64 KiB of UTF-8; JSON rejects nonfinite numbers and more than 64 nested containers before dispatch. Removing a saved connection uses the same form, its exact observed version and explicit confirmation of the saved identity. It removes associated saved keys and clears that connection’s default model while keeping conversations. Cancelling preserves all native drafts; conflicts require review and a new confirmation. Unknown removal checks its original receipt without issuing another deletion. Earlier removal receipts are independent of the current draft.

The ordinary `connections` component presents bounded catalog pages and opens a separate `credential` component. Credential has no Reader import and exposes no capture to its parent. It sends the original native SecretInput resource directly to Settings, together with the observed connection and credential versions. Confirmed saving resets only that captured editor version; unknown results query the original reference, and conflicts keep the native draft. An explicit connection/key review lets a retained draft use newly observed versions only after confirmation. Removing a saved key also requires confirmation and both versions; it leaves the draft untouched.

Earlier operations have a separate bounded pane because caller history is shared by mounts of the same consumer instance and its references do not contain original connection inputs. Checking those facts never adopts a current target or clears a new draft. The settings route hides the conversation presentation while retaining its native composer owner.

`model-default` is a separate component bound explicitly by the model selector. It distinguishes an unset default from an unavailable saved target. Setting or clearing freezes the observed catalog revision and target for confirmation; a conflict requires refresh and a new confirmation. Unknown results query the original receipt without another write. Earlier receipts have a separate bounded pane and never adopt a current target. Saving refreshes the catalog without changing the model selected for the current conversation. Hiding the selector retains the same child slot and pending workflow. A selectable default describes catalog eligibility, not authenticated execution or network availability.

The desktop model selector uses the public anchored Popup scene.

Both model selectors retain one native search input and search the complete enabled catalog through bounded pages. Plain Up/Down changes the active choice while keeping the query focused; Enter or a pointer choice verifies the original native text revision before selecting. Desktop reveals the active row through the existing native ReadingArea; TUI keeps its three-choice presentation. Caret movement acknowledges text without restarting search. Closing, defaults and busy scopes stop automatic scanning; defaults refresh reads its independent metadata even with an invalid search draft.

Opening its model/default/history workflows keeps the composer in place; Escape or an
outside click asks the plugin to close. The default child stays mounted across
close/reopen. Terminal entries keep paged in-flow presentation.

Desktop navigation can be collapsed without unmounting its component or the composer. Context and model controls use separate rows; long titles and model triggers are measured and truncated by the native renderer, with complete accessible values. Terminal navigation labels remain bounded by its own presentation.


The `command-palette` package exports ordinary `CommandPalette` content for explicit compositions, with parent-controlled `open`, current command guards and `invoked`/`selected`/`closed` events. It imports TextInput, TextReader, ReadingArea and Agent. Search requires all whitespace-separated terms in a command's label/id/hint or a session's id/title; it scans the original revision-bound directory and keeps at most 32 matching sessions, with an explicit More window. Choice confirmation captures the original native query revision. The parent handles command ids and re-observes selected sessions through Agent; presentation and modal input isolation belong to its scene.

## Plugin controls

The ordinary Deployment component provides Desktop/TUI variants for fixed-source,
eight-instance pages and confirmed enable/disable intents. The management Host changes
the original input on CAS, preserving template parameters and unrelated overrides;
required connections must remain valid. Desired input is distinct from runtime readiness.
Conflicts require refresh and a new confirmation. Unknown results expose only original
operation checks and durable earlier intents, never automatic replay. The current screen
covers existing instances. Its separate advanced composition editor loads the complete
committed source into one native buffer through bounded UTF-8 chunks, waiting for each
original editor receipt. It edits configuration, explicit bindings, package selections
and the launch root without flattening template provenance. Saving freezes the native
owner/revision for confirmation and passes an immutable UTF-8 document resource to Host;
Host checks installed revisions, the original execution environment, the full plan and
source budget before CAS. Validation diagnostics retain the draft. Conflicts require
explicit discard/reload; unknown replacement checks its original receipt and is included
in durable caller history. Closing this page retains its native editor. Source documents
are limited to 1 MiB; the committed input and remote observations still share their
512 KiB limit. Full product/runtime acceptance remains required.

The Plugins page installs `.maka-plugin` files through a retained native FilePicker.
One confirmation captures immutable selected bytes at the observed owner/revision
and submits the saved source revision to Host validation. Installing stores the
package without changing component enablement or permissions. Unknown results
check only their original operation; conflicts require refresh and a new confirmation.
The advanced composition draft remains intact; explicitly reload it to include newly
saved package references. Export an authored directory with
`maka application export-package --package DIRECTORY --output FILE.maka-plugin`;
an existing destination is never overwritten.

The native Agent captures published workspace roots without creating markers. New conversations reuse the selected conversation’s immutable workspace; the first conversation queries a published directory selection before creation. Sessions project canonical workspace path/identities and labels. `metadata` changes only title/labels with the original Session revision CAS; `create` and `metadata` report confirmed refusals as ordinary closed results, while uncertain persistence still uses the original operation journal. The command palette searches title, ID, workspace path and labels. A directory picker and metadata editor are still pending product UI work.
