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

Public Session history uses `HistoryPosition`: `start`, `tail`, `before`, `after`,
and raw execution `event` bookmarks. A first read captures an immutable raw-event
`through` fence; repeat preparation with the same position and returned fence.
`before` excludes its end, `after` includes its start, and both require that original
fence. Ready windows always supply `before` and `after` as a real sparse-row/UTF-8
byte cursor or `null`; chunks retain ascending source order in either direction.
The Session Host budgets each window to 128 KiB of text and eight fragments,
with a 1 MiB complete JSON limit including all attachment descriptors. The
native Agent service uses two fragments and 8192 bytes in total, with a 64 KiB
complete source-window budget and a separate actual response check. A fragment's offset and `totalBytes` identify
whether it is a complete message; a partial message is source, not standalone
Markdown. Position and fence locate content and never grant access.

Every Agent history chunk explicitly carries `tool: HistoryTool | null`. Tool
fragments retain canonical call identity and name; results also carry `isError`
and the preceding call's name, or `null` when no prior call is known. Empty-text
tools still occupy one fragment and advance the cursor. Numeric JSON and image
results therefore do not disappear merely because their text projection is empty.
These are presentation facts: `isError` does not distinguish a failed effect from
an unknown outcome, and tool identities grant no permission to execute or retry.
Tool arguments and full result payloads are not part of this text-window API.

`configurations.agent` is the generated native agent configuration schema for
package declarations. Use it alongside `contracts.agent`; product builders do
not maintain a second copy of the native configuration or service protocol.

`Component<State, Props, View>` lets an entrypoint type its platform's scene
language. The kernel treats `View` as opaque JSON and delegates interpretation to
the selected platform's pure scene compiler; there is no shared kernel layout
language. `ComponentSlot` remains the common child declaration: its binding is
resolved from the original parent's imports, never from renderer state. Desktop
and TUI entries can therefore present the same plugin differently. A platform
must install its compiler with a nominal language identity before activating
JavaScript components. Component entries declare `scene: { name, version }`;
all enabled entries must exactly match before any guest or factory runs.
For example, Desktop declares `{ name: "maka.desktop.scene", version: 1 }` and
TUI declares `{ name: "maka.tui.scene", version: 1 }`. This requirement is separate
from public props/events/captures. Pure Value/Service and native entries omit it. Original scene
JSON counts toward callback, transaction and retained-frame budgets.

`ui.desktop` and `ui.terminal` construct strongly typed native scenes. Both provide
`text`, `button`, `stack`, `dialog`, and `component`; Desktop also provides `popup`
and `canvas`. Their `input`, `textarea`, `label`, `markdown`, `filePicker`, `image`,
`sourceText`, `selectionMenu`, and `reading` helpers check the built-in props.
Bindings and event routes remain explicit. `component` accepts the props of an
external plugin's declared contract; it does not infer that contract from a binding name.

Use `DesktopNode` or `TerminalNode` as the `Component` view type. Constructors
reject cross-target children, including variables and nested reading content.
A nonserialized symbol marks the authoring target; it changes neither the JSON
protocol nor native admission. The generated `DesktopScene` and `TerminalScene`
are the wire types used by the actual renderers, defined together in `maka-ui`.
The SDK supplies declarations; theme, defaults, and product interaction wrappers
belong to the product plugin. See the counter entries for independent typed scenes
sharing one reducer, without a portable intermediate tree.

```ts
import { ui } from '@maka-agent/plugin-sdk';

const view = ui.terminal.stack({
  key: 'root', direction: 'column', gap: 1, padding: 1, background: 0,
  children: [ui.terminal.button({ key: 'save', label: 'Save' })],
});
```

`TerminalLayout` is generated from the actual terminal protocol.
A Stack's `layout.grow` distributes remaining cells after fixed siblings;
`basis` sets its base length in the parent's main axis. Row `wrap` lays out
controls on successive rows, including per-row growth and height measurement
at each allocated width. Column Stack `scroll` selects `none`, `manual`, `anchor`
or `follow_end`. Give a reading viewport `layout.grow` or `basis`; its scrollable
content retains natural height. Offsets, wheel/Page/Home/End handling and stable
child anchors belong to the native owner. Paint is read-only; only a successful
terminal flush accepts matching viewport geometry. Hidden routes retain their
reading baseline. Hidden subtrees still require unique node keys.


A `raw` event and its synchronous component observers may return `decision: { preventDefault: true, stopPropagation: true }`. Omitted flags default to false. Private admission follows only the original checked `on` routes; public component inputs keep their `component` kind. Flags join by OR, become visible after the whole transaction commits, and are never stored in a frame. Lifecycle emissions and action/subscription feedback cannot acquire admission through a raw DTO in their payload. Decisions affect the actual platform listener phase and cannot undo earlier native behavior.

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

Host application services support queries, subscriptions and durable mutations with request-local resource snapshots.
Subscription handlers publish with `await context.invoke({operation: 'emit', value})`;
items and the final return value use the declared output schema. Each stream allows
16 queued items, 16 pending emits and 64 KiB JSON per item. Await each publish to
respect reader backpressure. The native Host client owns stream cancellation and
observes real cleanup separately.
`context.invoke({ operation: 'mutate', ..., operationId, attachments: { text: 'text' } })` returns an `OperationReply`
with the original `reference` even when the result is unknown or cannot be delivered.
Only `receipt.kind === 'recorded'` describes journal facts; inspect both `outcome`
and `cleanup`. `recover` takes that exact reference, independently of current imports,
and requires a separate recovery grant. `not_recorded` means an authorized lookup
found no original row; `unavailable` makes no journal claim. Neither triggers replay.
Keep the original reference for recovery; a new operation identity can execute a new effect.
UI components return `subscriptions: Action[]` as their complete current demand,
separately from one-shot query/mutation `actions`. Each key is local to its component.
An unchanged key and full request retain the original stream, including after natural
completion; change the request to start a new one. Removing it cancels demand after
commit. Resources are captured once at start, rather than on each frame. Feedback uses
`{ status: 'updated', value }` per item and the existing `succeeded`/`failed` terminal
outcomes. Each item waits for its component feedback transaction to commit before
the next is consumed. Replacement and retirement discard old feedback while the
original executor still completes actual cleanup. This supports both Desktop and TUI;
authenticated cross-process transport remains unfinished.

One-shot `Action.key` is the feedback correlation within its original component Mount.
Do not reuse it while queued, preparing, executing, or waiting for feedback. Overlapping
requests use different keys. A feedback transaction may atomically issue the next request
with its own key. Pending cleanup retains the original executor independently; freeing a
key does not restore revoked admission. Action keys cannot also name subscription demand.

Component captures declare `output: { kind: 'resource', resource: { name, version } }`
or `output: { kind: 'continuation' }`. Resource sources fill the service's exact declared
slots. Optional `Action.continuations: CaptureSource[]` is available only to Queries.
The same checked child/forwarding path terminates at a native capture; JavaScript cannot
supply native evidence. Continuation snapshots stay in the original foreground request,
including after execution settles. Before invoking feedback, Desktop/TUI check the original
route, native owner and current committed recipe. Invalid evidence produces `{ status: 'stale' }`
without publishing the service value, undoing execution, or confirming cleanup. Native
widgets implement the observation check; a missing capture is `not_accepted`. These mechanics
do not by themselves implement product history navigation.

Desktop and TUI provide `contracts.readingArea` (`maka.ui.reading`, export `ReadingArea`).
Its `content` uses the selected presenter's checked scene language: a scrolling column Stack,
with its own declared component bindings. The original native scroll and text
selection owners retain geometry and selection; plugins receive `observed` facts
and `input` events, never pixel offsets. Numbered `start`, `end`, and `reveal_child`
commands settle as `applied`, `cancelled`, or `rejected`; omission does not replay
an accepted command. Observations retain only the current command and latest receipt.
The native `tail` continuation admits implicit replacement only while the original
painted end, reading intent and scoped selection remain eligible. A wheel or active
selection produces `stale` feedback; it does not undo the independent service read.
Desktop also provides `contracts.sourceText` (`maka.ui.source`, export `SourceText`)
for literal, read-only source. It preserves whitespace and uses original native
selection and Copy, including across SourceText and Markdown in one ReadingArea.
Appending a prefix retains the original byte selection through wrapping; replacing
source withdraws that selection. No parsing or image/network access is implied.

The ordinary agent product retains one exact canonical history window at a fixed
fence. Earlier/Next replace that window; Latest explicitly captures a new fence.
Watch progress never automatically replaces it. Live excerpts retain a bounded
prefix after the displayed fence; complete canonical messages alone use Markdown.
The product also opens a fixed-fence, single-message source reader with bounded
beginning/previous/next/end excerpts, exact retry and Back to the retained
conversation. Both presenters use native ReadingArea and literal SourceText; complete
Desktop assistant messages additionally use Markdown. TUI provides native SourceText with
the same contract and implementation name.
It preserves literal UTF-8 selection bytes (including CRLF, tabs and controls), displays controls
safely, and copies through Ctrl+Shift+C; Copy is a bounded OSC52 write, not an OS acknowledgement.
Prefix append preserves existing byte endpoints; replacement/disabled revokes the old selection.
TUI ReadingArea shares one selection across logical SourceText participants, including clipped
source in Select All. Selection and active pointer gestures invalidate automatic tail replacement.
Copy rejects a selection over 64 KiB before allocating or writing, preserving its original range.
Actual whole-reader OS acceptance remains unfinished.

Persisted UI components submit mutations, including declared captured resources, through their ordinary
`actions`. The kernel generates the operation identity and records the original
caller intent before dispatch. Feedback uses `ActionOutcome` with
`status: 'operation'` and the original `OperationReply`; components must handle
unknown/unavailable receipts without automatically repeating the action.
Declared resource slots capture their original immutable snapshots before native
commands. The original bundle stays owned and budgeted through preparation,
caller persistence, dispatch and actual cleanup. Host persists only a private
bundle nonce in the operation fingerprint, never snapshot bytes or metadata. A
new capture with the same operation UUID is a conflicting original request;
recovery only reads the original receipt and never rebuilds or replays resources.
An explicitly composed `maka.operations.history` native instance exposes the local
`contracts.callerHistory` service (`History` export). Its `page` query accepts
`CallerHistoryPage` (`after`, `limit`) and returns `CallerIntent[]` for the actual
calling instance of a persisted deployment. There is no namespace input. Continue
after the last returned cursor; the requested count is an upper bound and the
complete array fits 1 MiB. If the full callback envelope exceeds its delivery budget,
the query returns `failed` and a healthy component can retry with fewer entries.
Original references survive source replacement and Root reopening; these observations
confer no Host grant and cannot be used as a confirmed caller execution token.
A persisted Host composition can select `maka.operations.receipts` and export
`Receipts` with `contracts.receiptReader`. Its ordinary `read` query accepts a
`RecoveryRef` and returns an `OperationReply`. Bind this reader separately from
the current business import; it reads its owner's original Root and needs an
independent recovery grant. Wrong Root or denied recovery yields `unavailable`,
never `not_recorded`. Configuration cannot select another Root or namespace.
After reading caller history, a UI component can submit this query through its
ordinary `actions`; successful Query feedback contains the reply in `value`.
Read the receipt even when Query feedback says `succeeded`: the original effect
may still be unknown or unavailable. Transport/query failures use `failed`;
keep the original reference. Large or inexact journal values produce the same
reference with `unavailable` for this portable contract, while raw native recovery
preserves them. Query cleanup does not describe the original operation's cleanup.
Cross-process resource transport and business migration remain unfinished.

Deployment sources can be stored in an owned State Root with
`maka application commit --root ROOT --deployment UUID --expected-revision 0 --input input.json --package bundle.maka-plugin`.
A replacement requires the observed revision. `maka application read --root ROOT --deployment UUID`
validates the persisted source and exact package bytes. The returned `DeploymentRef` identifies
that state domain and source revision; it grants no access and makes no readiness claim.
The Host executor can start from this snapshot and retains storage ownership through accepted
work and cleanup. Original receipts survive source replacement and restart. Resolving unknown external effects,
cross-process resource transport and live reconciliation remain unfinished.

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


UI activation returns `UiExport` entries tagged `component` or `value`. The latter
publishes immutable, schema-checked JSON through a normal Value contract. Every
component callback and `activate(configuration, values)` receives its captured
Value imports as the final argument. `ComponentValues` makes the map and member
lists read-only in authoring types; the runtime provides detached snapshots, not
provider mutation or routing authority. Optional imports contain an empty list;
undeclared imports are absent. Multiple imports to the same data each count toward
the complete 1 MiB input before copying. Values and frames share the 32 MiB retained
budget. No old untagged activation shape is accepted.
