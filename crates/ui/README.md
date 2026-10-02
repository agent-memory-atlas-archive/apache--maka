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
Renderers read committed
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
windows. Action imports, outward component events
and recovery surfaces are still pending. `close` requires an actual
native cleanup acknowledgement; `retire` only stops new input. `shutdown` retains
owners while mounts/work/resources remain unconfirmed.

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
The native buffer remains outside serialized component state. Observation inputs
update local state; outward delivery and public plugin resource reads remain
pending. Native component events receive the current mounted props as well as
state, key and input; JavaScript `event` receives props as its third argument.

The example's component contract uses the same Rust declaration types as runtime
validation. It does not depend on the previous plugin SDK or global service catalog.
