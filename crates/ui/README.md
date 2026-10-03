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
Callbacks return `ComponentOutput { frame, events, actions }`; events and actions are
one-shot transaction intents, separate from retained state. A declared `capture`
callback is also captured before publication. Renderers read committed
`Frame` values; successful, bounded, validated batches replace a frame atomically.
Each mount has independent state and a revision, so old view events cannot invoke
new handlers. A realm failure retires every registration and mount in that realm.

`NativeRegistry<V>` supplies compiled implementation factories; only selected
composition entries activate them and publish their actual contracts through the
same runtime. Native components share props/state/revision and the transaction
with JavaScript. `Presentation<V>` contains either the bounded tree or an opaque,
typed, immutable native recipe; the kernel has no GPUI dependency. Native raw
inputs have scene-specific schemas. Calls are bounded to 1 MiB before dispatch.
Rust extensions are trusted synchronous code, not watchdog-isolated JavaScript;
the common commit deadline can reject their late result but cannot interrupt them.
The 4 MiB update and 32 MiB retained limits cover serialized metadata/tree/input
schemas, not native renderer allocations, which the native owner must bound.
Recipes must not retain entities, input bindings, subscriptions or pending native
work. Those belong to renderer resource owners and retain mount resource leases
until actual destruction, independently of the retired frame's recipe.


`ComponentSlot { key, binding, member, props, on }` is shared by `Node::Component`
and `NativeView::new(recipe, input_schemas, slots)`. Native slots are ordered,
limited to 64 and resolved against the parent's own imports. They use the same
child mounts, props transactions, event routes and cleanup as tree references.
Hidden slots keep their state. Complete callback metadata, including slots, state,
events and actions, shares the 1 MiB budget for both presentation variants.
Arguments that cannot be projected exactly into JavaScript are rejected before
calling that consumer, without retiring its healthy realm; Rust retains exact integers.

The tree protocol contains stacks, text, buttons and imported components.
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
cargo run --locked -p maka-desktop -- --root /tmp/maka-counter/root \
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
Synchronous event consume/default decisions remain pending.

Use `editor` instead of `native` to persist a composition importing the native Kit
Editor. Its Rust contract describes initial text, explicit
revision-based replace/reset commands, metadata observations and command receipts.
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
subscribes to Editor `changed` and displays its observed native draft revision.
The generator persists the selected packages. This exercises public delivery,
not text-resource reads, business actions, or a finished product design.

Use `resource` to compose a dynamic Composer with a private native Editor and an
explicitly imported native UTF-8 Reader:

```sh
cargo run --locked -p maka-ui --example counter -- /tmp/maka-resource resource
cargo run --locked -p maka-desktop -- --root /tmp/maka-resource/root \
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
`at_revision` rejects a changed revision. Only the trusted request sidecar carries
the actual resource. Metadata, JSON slot names and UUIDs are not read authority.
The snapshot retains no live input, entity or mount resource lease.

`ReadResource` and `Attachments` live in `maka_plugins::call`, shared by UI and
Host execution adapters. The UI capture path owns its snapshots and capacity;
local Host queries accept actual payloads with independent admission and cleanup.
Cross-process resource transfer still requires a transport executor.

`Application::new` takes explicit `RemoteServices`: static declarations are separate
from actual `RemoteService` endpoint registrations. Both local and remote services
support Query and resource-free Mutation methods. Subscription and resource
mutations still require their own executors. `NativeService::call` receives a
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

Each output allows 16 actions and each request 16 resource slots; queued requests,
preparations, accepted work and pending feedback total at most 64 entries,
captured resource bytes are limited to 64 MiB in aggregate,
and UTF-8 reads to 64 KiB with valid byte boundaries. Complete capture arguments
are limited to 1 MiB before cloning or callbacks, and share the transaction's
callback, deadline and cumulative work limits. The Reader is available only
through explicit package composition. Authenticated cross-process transport,
subscriptions
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
registration cleanup. Values are immutable for an activation. Live reconcile and
window/local theme projection are separate work; this path alone does not implement
live theme switching or a finished product.
