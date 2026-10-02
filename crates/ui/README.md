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

The tree protocol contains stacks, text, buttons and imported components.
A component reference names an explicit binding and its selected member; stable
keys retain child mounts and their local state across parent props updates.
All changed parents and children share one deadline and commit together. A failed
child leaves every prior frame unchanged. Removed mounts retire until real native
cleanup is confirmed; uncommitted mounts have no native resources to acknowledge.
Window configuration is fixed for a root mount; embedded components cannot create
windows. Recovery surfaces remain pending. `close` requires an actual
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
cargo run --locked -p maka-desktop -- --input /tmp/maka-counter/input.json \
  --package /tmp/maka-counter/native.surface.maka-plugin \
  --package /tmp/maka-counter/dynamic.app.maka-plugin
```

Add `composed` after the output directory when generating the example to include
a separately packaged counter component, then pass its
`dynamic.counter.maka-plugin` as another `--package` to desktop. Use `native`
instead of `composed` to include an explicitly imported compiled Label, and also
pass `native.label.maka-plugin`. The label's styling is rendered directly by GPUI,
outside the tree primitive set; its placement and props belong to the product.
Opaque native child slots and synchronous event consume/default decisions remain
pending.

Use `editor` instead of `native` and additionally pass `native.editor.maka-plugin`
to import the native Kit Editor. Its Rust contract describes initial text, explicit
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
Pass the same five packages as the editor example. This exercises public delivery,
not text-resource reads, business actions, or a finished product design.

Use `resource` to compose a dynamic Composer with a private native Editor and an
explicitly imported native UTF-8 Reader:

```sh
cargo run --locked -p maka-ui --example counter -- /tmp/maka-resource resource
cargo run --locked -p maka-desktop -- --input /tmp/maka-resource/input.json \
  --package /tmp/maka-resource/native.surface.maka-plugin \
  --package /tmp/maka-resource/dynamic.app.maka-plugin \
  --package /tmp/maka-resource/dynamic.composer.maka-plugin \
  --package /tmp/maka-resource/native.editor.maka-plugin \
  --package /tmp/maka-resource/native.reader.maka-plugin
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

Final admission rechecks the exact consumer mount and service registration.
Native service futures run on the background executor, independently of the
window. Result events use `{ kind: "action", key, event }`, which raw input cannot
impersonate. Settlement happens before result delivery; retired consumers receive
no result. Failed result handlers retire the consumer rather than silently leaving
it waiting. Business outcome and cleanup confirmation are separate: failed or
unknown cleanup retains accepted work and its snapshots until a real successful
confirmation. The application lifetime owner survives all windows and worker tasks.

Each output allows 16 actions and each request 16 resource slots; queued plus
accepted work is limited to 64, captured resource bytes to 64 MiB in aggregate,
and UTF-8 reads to 64 KiB with valid byte boundaries. Complete capture arguments
are limited to 1 MiB before cloning or callbacks, and share the transaction's
callback, deadline and cumulative work limits. The Reader is available only
through explicit package composition. Host transfer/durable admission, the
JavaScript binding bridge and UI integration, TUI integration and product recovery
UI remain pending. The standalone Host query executor is described in
[Host application services](../runtime-host/README.md). The
resource fixture proves this native asynchronous path, not a finished product.
