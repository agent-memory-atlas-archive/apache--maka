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

# Component execution

`maka-ui` activates checked UI entries from immutable packages and publishes actual
component registrations through `maka-plugin-core`. Foreground JavaScript executes
only during activation, mount initialization, props updates and input. Required
`mount`, `event` and `update` functions and their receiver are captured before
publication; later namespace mutation cannot replace a registered handler.
Callbacks return `ComponentOutput { frame, events, actions, subscriptions, decision? }`; events and actions are
one-shot transaction intents, separate from retained state. A declared `capture`
callback is also captured before publication. Renderers read committed
`Frame` values; successful, bounded, validated batches replace a frame atomically.
Each mount has independent state and a revision, so old view events cannot invoke
new handlers. A realm failure retires every registration and mount in that realm.

Persisted UI roots can load an exact same-Root committed revision with
`load_committed_update` without borrowing the foreground application. The embedding
can continue input/rendering while awaiting that storage future, then call
`adopt_update` in its owner turn. `adopt_committed` combines these operations.
Final adoption rechecks deployment identity and monotonic revision, validates
packages, target and trusted endpoint shapes, then installs the plan in the same
kernel. Remote observations do not create registrations or permission grants.

`advance_deployment` runs after original task settlement and renderer close
acknowledgements. It keeps unchanged activation, frame, module and mount identity;
changed instances wait for original calls, native mount cleanup and module close.
Changed remote registration epochs retire the original attachment before replacement.
Optional missing endpoints remain optional. The endpoint vector is the trusted
owner's complete registration selection for this plan.

`deployment_status` separates desired input, last completely converged activation
plan, pending convergence and observed failure; it does not assert window, IME or
accessibility readiness. Convergence checks every enabled desired instance, including
ones absent after a failure. A current foreground failure remains an error; subsequent
explicit committed input can recover. Normal failure of an already retiring original
domain does not cancel a replacement. The embedding still owns surface replacement
and physical resource acknowledgements; production launcher live-update integration
remains required work.

Each prepared action freezes the original source of its exact consumer activation.
Retained consumers keep that source for later requests too; newly activated consumers
use the adopted source. The local history implementation fixes its original Root and
deployment identity, accepts exact original request references within that identity,
and never permits guest JSON to select another caller namespace.

A raw input callback may return `decision: { preventDefault, stopPropagation }`; omitted flags are false. The kernel returns this decision only after the entire causal batch commits. Lifecycle callbacks, component notifications and asynchronous action results cannot supply a decision. Decisions are bounded transaction data, never retained in `Frame`. A platform applies them at its actual listener phase; they cannot cancel native behavior that already happened.

`NativeRegistry<V>` supplies compiled implementation factories; only selected
composition entries activate them and publish their actual contracts through the
same runtime. Native components share props/state/revision and the transaction
with JavaScript. Every committed frame contains a `NativeView<V>`: an opaque,
typed, immutable native recipe; the kernel has no layout vocabulary or GPUI dependency. Native raw
inputs have scene-specific schemas. Calls are bounded to 1 MiB before dispatch.
Rust extensions are trusted synchronous code, not watchdog-isolated JavaScript;
the common commit deadline can reject their late result but cannot interrupt them.
The 4 MiB update and 32 MiB retained limits cover serialized metadata/source/input
schemas, not native renderer allocations, which the native owner must bound.
Recipes must not retain entities, input bindings, subscriptions or pending native
work. Those belong to renderer resource owners and retain mount resource leases
until actual destruction, independently of the retired frame's recipe.


JavaScript returns platform-owned JSON scenes. The embedding installs a pure
`NativeRegistry::with_scene_compiler(language, compiler)` before activating
JavaScript components. Their entries declare `scene: { name, version }`; every
enabled entry must exactly match the installed language before any factory or
guest runs. Pure Value/Service entries and native implementations do not declare
scenes. The language and compiler share one availability object, with no routing
or fallback. The compiler
receives only JSON and the current deadline, and returns a `NativeView`. The kernel
retains the original scene JSON for accounting, independently of the compiler's
metadata. Compilation failure rolls back the batch while leaving the realm healthy.
TUI and Desktop can define different scene languages for the same plugin.

`ComponentSlot { key, binding, member, props, on }` belongs to
`NativeView::new(recipe, input_schemas, slots)`. Slots are ordered,
limited to 64 and resolved against the parent's own imports. They use the same
child mounts, props transactions, event routes and cleanup. Hidden slots keep their
state. Complete callback metadata, including original scene JSON, slots, state,
events, actions and subscriptions, shares the 1 MiB budget.
Arguments that cannot be projected exactly into JavaScript are rejected before
calling that consumer, without retiring its healthy realm; Rust retains exact integers.

A component reference names an explicit binding and its selected member; stable
keys retain child mounts and their local state across parent props updates.
All changed parents and children share one deadline and commit together. A failed
child leaves every prior frame unchanged. Removed mounts retire until real native
cleanup is confirmed; uncommitted mounts have no native resources to acknowledge.
`Frame.surface` is a closed declaration: `Window { binding, title, width, height }`
for Desktop, `Terminal { binding }` for TUI. Its variant and exact surface capability
must match the deployment. Surface configuration is fixed for a root mount;
embedded components cannot create surfaces. Recovery surfaces remain pending. `close` requires an actual
native cleanup acknowledgement; `retire` only stops new input. `shutdown` retains
owners while mounts/work/resources remain unconfirmed.

Imported references declare `on`, mapping a public event name to the parent's
local handler key. Names and every emitted payload are checked against the actual
export's public contract, including unobserved events. JavaScript `event` receives
`{ kind: "raw", key, event }` or `{ kind: "component", key, event, child, name }`,
plus mounted props as its third argument. Native callbacks receive the equivalent
`EventOrigin`. A raw payload cannot impersonate a component origin.

Events capture the exact parent/child mount and handler, then run FIFO within the
same transaction. Every callback reads the latest staged state. Detaching or
changing a subscription permanently removes its queued events; restoring the
same name cannot revive them. All callbacks share the deadline, 64-callback and
64-emission limits, and a 4 MiB cumulative work budget. The 32 MiB retained budget
counts only final scenes. Multiple causal callbacks may advance one revision
several times, but renderers observe only the final scene. Exceptions or budget
failure roll back the whole batch. Temporary subtrees created and removed within
it are never published and close without a renderer acknowledgement. Root and
unobserved events are validated then discarded; there is no global event bus.

Desktop owns each window's cleanup separately from its visual Surface. Logical
removal revokes inputs and drops renderer owners; root completion also waits for
the platform destruction receipt, every real resource lease and child cleanup.
Known failures retain a failed fence. The bounded [GPUI source patch](../../third-party/gpui-fast/SOURCE.md)
provides backend receipts without equating logical close or a queued destroy
request with resource release. This does not promise driver-internal GPU memory
reclamation.

Generate a package-backed desktop example (the output directory must be new):

```sh
cargo run --locked -p maka-ui --example counter -- /tmp/maka-counter
cargo build --locked -p maka-cli -p maka-desktop
target/debug/maka desktop --root /tmp/maka-counter/root \
  --deployment 00000000-0000-0000-0000-000000000001
```

The generator commits exact bytes and deployment sources into `root` and also
exports JSON and package bundles for inspection/import. Desktop uses deployment
`00000000-0000-0000-0000-000000000001`; `dual` additionally commits the TUI source
as `00000000-0000-0000-0000-000000000002`. Startup accepts only Root/deployment.

Add `composed` after the output directory for a separately packaged counter, or
`native` for an explicitly imported compiled Label; the generator persists all
selected packages. The label's styling is rendered directly by GPUI,
outside the tree primitive set; its placement and props belong to the product.
The platform applies synchronous input decisions at the native listener phase.

Use `canvas` for a dynamically packaged workflow board with plugin-defined
snapping/drop/key rules and native drawing, focus and drag tracking.

Use `editor` instead of `native` to persist a composition importing the native Kit
Textarea. Its Rust contract describes initial text, explicit
owner/revision-based replace/reset commands, metadata observations and command receipts.
The native buffer remains outside serialized component state. The public `changed`
event contains typed metadata observations and receipts;
receiving a TextRef does not grant a global lookup or text-read authority. Public
captures produce request-local immutable text resources. `command: null` proposes no new command;
an accepted request remains pending until its matching receipt commits, even if
later props clear the command. The final native recipe consumes that request once.
Native component events receive the current mounted props as well as
state, key and input; JavaScript `event` receives props as its third argument.

The example's component contract uses the same Rust declaration types as runtime
validation. It does not depend on the previous plugin SDK or global service catalog.

Use `events` instead of `editor` to generate a product component that explicitly
subscribes to Textarea `changed` and displays its observed native draft revision.
The generator persists the selected packages. This exercises public delivery,
not text-resource reads, business actions, or a finished product design.

Use `resource` to compose a dynamic Composer with a private native Textarea and an
explicitly imported native UTF-8 Reader:

```sh
cargo run --locked -p maka-ui --example counter -- /tmp/maka-resource resource
cargo build --locked -p maka-cli -p maka-desktop
target/debug/maka desktop --root /tmp/maka-resource/root \
  --deployment 00000000-0000-0000-0000-000000000001
```

An action names an explicit service binding/member/method and typed resource
slots. Each slot names a direct child's public capture method. JavaScript capture
callbacks return a pure forwarding plan to their own direct child; they cannot
mint native resources. The runtime captures exact mounts, source edges and service
registrations during the same bounded transaction. Later removal or rebinding
invalidates pending requests permanently, including same-name replacements.

Desktop publishes the scene, prepares native owners, captures an immutable Rope
at the native read barrier, then applies native commands and collects retired
owners. A capture conflict produces `not_accepted`; it does not roll back a
published scene. `current` explicitly selects the barrier's current text;
`at_revision` checks the original native owner and revision. Commands and receipts
also correlate by owner and sequence; a replacement editor cannot consume an old
command even when its local revision is equal. Snapshot metadata retains that
original owner for correlation, not as read authority. Only the trusted request sidecar carries
the actual resource. Metadata, JSON slot names and UUIDs are not read authority.
The snapshot retains no live input, entity or mount resource lease.

`ReadResource` and `Attachments` live in `maka_plugins::call`, shared by UI and
Host execution adapters. The UI capture path owns its snapshots and capacity;
local Host queries accept actual payloads with independent admission and cleanup.
Cross-process resource transfer still requires a transport executor.

`Application::new` takes explicit `RemoteServices`: static declarations are separate
from actual `RemoteService` endpoint registrations. Both local and remote services
support Query, Subscription and resource-free Mutation methods. Resource mutations
still require their own executor. `NativeService::call` receives a
`ServiceRequest` with checked input, actual attachments and a private `ServiceSource`
constructed by the kernel. Consumer, mount and request observations grant no Host
access. A transport callback captures `remote_activation` and passes that exact token
to `retire_remote`; a late disconnect cannot retire a replacement kernel.

Persisted UI mutations construct an inert `NativeMutation` owner, prepare the
original native request and confirm the caller intent in the UI's own Root before
accepting Work. `Application::finish_task` returns dispatch or close successors
after rechecking the exact original mount/registration. Presenters must execute
every successor even when the window or terminal is gone. Preparation is fenced
by a provider-owned Core resource, then accepted Work takes over its protection.
Unsaved previews reject mutations. Native authorization remains independent.

A mutation may include `Action.recovery`: up to 32 KiB of encoded, caller-local
correlation data. It commits atomically with the original caller intent before
dispatch and is returned by caller history. Queries and subscriptions reject it.
It is neither a replay grant nor a business result; the peer confirmation omits
it entirely. Products must validate their own format, data source and editing
revision before using it to restore local state.

Mutation feedback is `{ status: "operation", reply: OperationReply }`, preserving
the actual original target reference. Receipt knowledge and actual executor cleanup
are independent. Undeliverable results become `unavailable` with that same reference;
the original journal is unchanged. No uncertain operation is replayed. Failed or
interrupted preparation remains owned and appears in `failed_action_ids`;
`complete_action_cleanup` requires the embedding's actual cleanup confirmation,
not merely cancellation or an observer disappearing.

Final admission rechecks the exact consumer mount and service registration.
Native service futures run on the background executor, independently of the
window. Result events use `{ kind: "action", key, event }`, which raw input cannot
impersonate. Settlement happens before result delivery; retired consumers receive
no result. Failed result handlers retire the consumer rather than silently leaving
it waiting. Business outcome and cleanup confirmation are separate: failed or
unknown cleanup retains accepted work and its snapshots until a real successful
confirmation. The application lifetime owner survives all windows and worker tasks.

`ComponentOutput.subscriptions` declares the complete current `Action[]` demand;
omission means no streams. Actions accept only Query/Mutation, subscriptions only
Subscription. Equal key and complete request preserve the private request identity
across commits, including natural completion; change/remove operates only at successful
commit. Resource capture is once per request. Old feedback cannot reach a same-key
replacement. Each stream delivers `Updated { value }` using the existing internal
component result event and waits for its successful transaction commit before polling
its next item. Invalid items stop demand; item JSON is limited to 64 KiB and checked
against the method output schema. Failed result delivery also cancels the stream.

`NativeService::subscribe(request, CancellationToken)` returns `StreamFuture` with
`StreamCompletion::Item { value, next }` or `Finished(ServiceCompletion)`. The successor
moves the original executor, and presenters must run every successor after retirement
or surface loss. Cancellation bypasses the feedback ACK wait, then polls actual source
cleanup; the native implementation must honor cancellation and report its real cleanup.
Failure/panic retains the existing Core Work fence until actual confirmation. No
separate event bus, authority registry or cleanup owner is introduced.

Each output allows 16 actions and 16 subscriptions; unchanged declared streams retain
their original captured resources. One pending update of a live stream does not count
as a second request. Each request allows 16 resource slots; queued requests,
preparations, accepted work and pending feedback total at most 64 entries,
captured resource bytes are limited to 64 MiB in aggregate,
and UTF-8 reads to 64 KiB with valid byte boundaries. Complete capture arguments
are limited to 1 MiB before cloning or callbacks, and share the transaction's
callback, deadline and cumulative work limits. The Reader is available only
through explicit package composition. Authenticated cross-process transport
and product recovery UI remain pending. Direct remote registration to an actual
Host query is exercised with native Kit snapshots across the two kernels.
The standalone Host query executor is described in
[Host application services](../runtime-host/README.md). The
resource fixture proves this native asynchronous path, not a finished product.

Generate with `dual` to package both presentation entries in the same immutable
revision. The generator persists separate Desktop and TUI deployments; exported
`input.json` and `input.tui.json` describe them for inspection. The terminal uses
cell spacing and no pixel dimensions:

```sh
cargo run --locked -p maka-ui --example counter -- /tmp/maka-dual dual
cargo run --locked -p maka-cli -- application run --root /tmp/maka-dual/root \
  --deployment 00000000-0000-0000-0000-000000000002
```

The terminal presenter reads committed scenes and uses exact mount/revision input.
It publishes hit geometry only after drawing and flushing successfully. Raw terminal
mouse coordinates have no screen epoch; they resolve against the last flushed scene
at delivery. This counter verifies the runtime path, not finished product UX.

Persisted applications make the native implementation `maka.operations.history`
available before activation. A product must explicitly select a provider instance,
its `History` export and a binding with `caller_history_contract()`; availability
does not activate a plugin. Unsaved previews do not provide it. Factory configuration
is an empty object. Its ordinary `page` Query takes `{ after, limit }` and returns
`CallerIntent[]` for the kernel's actual calling instance and deployment. Guest
JSON cannot choose another namespace or Root.

History preserves the original caller source and `RecoveryRef` across source
replacement and Root reopening. These are observations, not execution tokens or
Host recovery grants. Each page contains at most 128 entries and its complete JSON
array fits 1 MiB; continue after the last returned cursor. The count is an upper
bound. The actual callback also includes current state, props and the action
envelope. An oversized successful query becomes `failed` without retiring a healthy
reader; a product can reduce its page size and retry the query without repeating
a mutation. This service leaves the recovery interface and receipt lookup to
explicitly composed plugins.
All consumers share the byte budget; JavaScript integer and codec restrictions
apply at the actual JavaScript callback boundary. Native callbacks preserve exact
Rust JSON integers, including when they return a portable tree.


Immutable values are ordinary `Interface::Value` exports. JavaScript activation
returns tagged `UiExport` entries (`kind: "component"` with a captured path, or
`kind: "value"` with schema-validated data); native factories return
`NativeCapability::Value`. Factories receive `(configuration, values)`. JavaScript
`activate`, `mount`, `update`, `event`, and `capture` receive `ValueBindings` as
their final argument; native `call` receives the equivalent second argument and
`CaptureCall.values` supplies it during capture. Keys come only from the consumer's
explicit Value imports, members preserve checked order, and empty optional imports
remain empty. These detached JSON snapshots carry no service grants. Guest changes
to a snapshot cannot modify its provider.

Imports capture actual capabilities during loading. Each callback inspects those
original registrations instead of rebinding names. The complete borrowed input,
including repeated values, is counted before payload copying against 1 MiB; JS
precision checks precede guest entry. Native consumers preserve exact integers.
Registered values and committed frames share the application's 32 MiB serialized
retained budget; retiring a provider does not reclaim its quota before real
registration cleanup. Values are immutable for an activation. Local appearance
projection is implemented for Label and Textarea; live provider reconciliation
and finished product interfaces remain separate work.

The `maka.ui.Textarea` contract publishes the `Textarea` export, backed by Kit's
ordinary multiline TextareaState. Props require complete `appearance` data plus
`accessibilityLabel`, `readOnly`, `disabled`, `softWrap` and `tabIndex`. Typography,
RGBA colours (including alpha zero), padding, borders and actual scrollbar
geometry/motion are projected into the existing native owner. Appearance updates
do not replace the text, selections, composition or undo history. Label also takes
explicit text appearance. The example brand is an ordinary JavaScript Value
provider selected by composition; native rendering reads no global brand palette.

Textarea exposes its actual focus, label, value and editability to AccessKit, and
supports Focus and editable SetValue requests. Headless platform-channel tests
cover those requests and focus navigation. The ordinary editing context menu and
OS accessibility/IME acceptance remain pending. The TUI Label projects colour and
weight to RGB/bold terminal styling; desktop fonts and alpha compositing do not
imply equivalent terminal rendering.

The pure `editor::reduce` command/receipt protocol is shared by GPUI and TUI.
Each presenter keeps its own original native editor as the sole buffer, selection
and undo owner. TUI prepares native owners, captures immutable resources, then
applies staged commands. Successfully flushed geometry alone becomes interactive;
unpublished observations retain their original receipt without replaying commands.
TUI physical Copy reads the focused visible textarea's native selection through
the bounded terminal writer; it does not expose a second JavaScript text buffer.

Textarea optionally accepts `persistence: {key, format, dataSource, metadata,
restore, retry}`. The actual mounted provider supplies a weak scope for its owned
UI Root and stable instance; props supply no filesystem path or storage authority.
Key, format and business source stay fixed for that mount. Native workers save a
raw UTF-8 blob (up to 1 MiB), native revision/selection and bounded opaque metadata
with revision CAS. Empty records remain tombstones. Each namespace has a 40 MiB
limit, and snapshots share the presenter's allocation budget. The worker coalesces
pending snapshots and drains its last write before native resource cleanup; a
failed write never produces a successful cleanup receipt.

The `checkpoint` event exposes Loading, Restore, Ready or Failed metadata without
body text. Restore requires a command for the current native owner and loaded
version; the product decides whether original business receipts supersede the
saved body. Both presenters validate exact grapheme selection boundaries and
create a new native owner, without restoring undo or geometry. Editing and send
stay gated until that new lineage has a durable checkpoint. `Ready.saved` reports
whether the latest captured state has settled; abrupt process loss can still lose
unacknowledged edits. A later `retry` retries a failure, or re-emits held Restore
metadata when the product revisits that draft. Recipes and validation caches keep
only weak database capabilities; live workers retain the original Root.

The fixed `TextInput` and `SecretInput` exports from `maka.ui.input` use the
shared `Editor*` command, observation and capture protocol. Single-line props
supply complete `InputAppearance`, label, placeholder, editability and tab order.
Both controls start empty; an initial metadata observation permits a revision
checked prefill command. Input rejects control characters as one complete edit,
before clipboard or IME normalization. Reset clears native undo history.

SecretInput stays masked for its lifetime. Its AccessKit password node has no
plaintext value or text runs, and platform surrounding-text queries return masks
in the original UTF-16 units. Captures retain the original immutable UTF-8 bytes;
masking does not remove an explicitly granted Reader capability. Normal settings
composition should bind secure capture to its trusted setter and omit Reader.
Native Enter is consumed before keybindings and emits `submitted` as a business
event, not a synchronous native decision. TUI emits this independently of its
retryable observation/receipt slot. OS input-method and screen-reader acceptance
requires separate platform verification.

Retained presentation is an immutable projection of the original frame. A scene
Stack may set `visible: false`; native parents use `NativeView::with_hidden_slots`
with declared exact slot keys. All declared children remain mounted. Hidden
children measure zero and publish no focus stops or input targets; scene stacks
exclude them from layout gaps. Their original editor buffers, selections, undo
history, capture capabilities and command receipts remain owned and can settle. Showing restores current props on the same owner.
Desktop invalidates mechanical admission before the next paint and rejects retained
child renders; TUI applies geometry admission only after a successful flush.
Custom native layouts use the same public Slots boundary and cannot paint a hidden
child. This projection does not retire resources or create a second writable draft.

An embedding may set `NativeRegistry::with_render_depth_limit` for bounded
native rendering. Its scene compiler and trusted native factories describe the
maximum local cost and each slot's enclosing cost with
`NativeView::with_render_depth`. The transaction checks complete proposed visible
surface paths, merging staged frames and retained ancestors/children, before any
publication or native effect. Hidden slots still mount and settle observations,
but consume no drawing depth. Rejection preserves the original frames, mounts
and resource custody. These numbers describe renderer work; the kernel neither
interprets layout nor lets guest code choose its budget.

`maka.ui.Markdown` is a local read-only component contract. Source, selectable /
disabled state and complete typography/color inputs are explicit. Link events
participate in the original synchronous component batch; their committed input
decision controls native defaults. Literal image URLs grant no resource access.
Desktop provides this implementation; other presentations declare their own
implementation or compose an explicit alternative.

`file::file_picker_contract()` supplies a bundled `maka.ui.FilePicker` component
with the same public contract on Desktop and TUI. Props contain appearance and
labels, never a path or a programmatic open command. Actual native selection
stages a private intent; only its matching committed marker may start a file read.
Observations expose owner/revision, basename, SHA-256, byte count and a bounded
phase/error code. `snapshot` captures that exact ready selection as immutable
`maka.bytes` chunks; changing the original file does not change captured bytes.
Consumers still validate content, including the DeploymentManager's complete
package bundle checks. Selection permits at most 50 MiB of regular-file bytes,
uses no-follow opens and charges actual 64 KiB chunks to the presenter's shared
`budget::NativeBudget`. Byte permits follow snapshots; job permits and original
resource tickets follow actual dialog/read settlement. Lost dialog observation
keeps original budget custody and a failed cleanup fence.
The terminal picker also browses local directories through that committed native
intent and worker. It reads at most 2,048 directory entries, orders directories
before files and retains paths/listings only in the native owner, charged to its
shared budget. Browse starts at the typed absolute path (or local process working
directory when empty); Parent, arrows, Home/End, PageUp/PageDown, Enter, wheel and
scrollbar dragging navigate the presented list. Clipped/resized lists use actual
visible rows. Props supply localized browse, parent, empty and truncation labels;
observations never expose local paths or directory contents.

TextInput, SecretInput and Textarea accept optional `autoFocus` (default false). A structurally visible, enabled input offers its original native focus once on reveal. Stable updates and viewport clipping do not repeat the offer. Hidden or disabled inputs cancel it; showing or enabling them can offer again. Actual native presentation admits the first eligible offer and consumes the other painted offers. Desktop keeps active native composition in its original editor. These props do not replace native text, selection or undo ownership.

Textarea can opt into terminal file-path paste with `filePaste: {context, maxFiles}`
(`maxFiles` is 0–8; zero permits only clipboard text). A physical whole-path paste stages a native intent; committed
admission starts serial reads under the existing file budget and resource owner.
`files` returns the original context and current selection metadata; `file` captures
the exact owner/revision as immutable bytes. `fileCommand` accepts only `next`,
`retry` or `cancel` for that selection, never a path. Text snapshots keep their
original contract. An admitted batch retains its context across navigation; hiding
the editor cancels only an uncommitted intent. Desktop rejects these optional props
until it implements the same capability.

Opted-in terminal Textarea also admits Ctrl+V through the same physical marker.
`FilePasteObservation.retryable` distinguishes original paths from a one-shot
clipboard read; guest commands cannot reread a clipboard. Text completion requires
the original context and text revision, preserving the captured selection even if
the caret moved. Clipboard images use `FileSnapshotWriter`: its chunk permits follow
the immutable capture and Host upload, with no temporary-file custody. Decode checks
dimensions/33,554,432-pixel and 128-MiB pixel limits before allocating output pixels.
Native clipboard API responses are size-checked after the platform getter returns.
Independent native observation keys retain their latest pending values separately,
so an editor update and file progress cannot overwrite one another before publication.
