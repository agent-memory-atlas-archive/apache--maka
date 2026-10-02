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

# Native desktop

`maka-desktop` presents package-backed components with Longbridge GPUI Fast and
GPUI Kit. It selects only Desktop deployments, reads immutable package bytes and
runs the shared component adapter on the UI owner thread. Layout/paint read Rust
frames; button callbacks dispatch revision-bound input before notifying GPUI.
No business shell or default product is embedded in this binary. Compiled native
components are selected through `NativeRegistry<dyn Widget>` and the ordinary
package binding path. Widget recipes render native GPUI elements; common input
bindings retain the exact mount lease/revision and dispatch outside paint.
Recipes contain immutable presentation data. Renderer owners hold entities and
subscriptions separately; GPUI retained state/arena may also hold resource leases.
Resources must never be cached in a recipe retained during cleanup. Native owners
prepare resources, capture request-local snapshots, then consume committed intents
before rendering, outside layout/paint.

Each component's element arena, retained GPUI state and input callbacks hold a
native resource lease. A detached, retired child closes only after its last lease
has actually been destroyed; the owner-thread receipt task does not wait for more
input. A visible retired scene stays fenced. Cleanup follows the original mount
parent chain, so one surface cannot acknowledge another surface's resources.
The receipt task belongs to the surface, uses weak application references and
is cancelled when the surface is destroyed. GPUI Kit editor entities retain their
lease through actual release processing; dropping a handle alone is insufficient.

The imported Editor uses Kit's buffer, selection, undo and platform text-input
engine. Parent updates preserve it. Explicit replace/reset intents carry a native
revision and sequence; conflicting revisions or active composition reject them.
Only one unresolved command is admitted, with an applied/conflict receipt after
native execution. Reset outside composition clears undo; it does not claim to
cancel an operating-system IME session. Observations contain resource metadata,
not full text. Reads against the bound owner check identity/revision and UTF-8
boundaries and are limited to 64 KiB. Platform UTF-16 queries use this read path.

Run the [component example](../ui/README.md). This is an initial renderer, not the
completed desktop product: real OS IME acceptance, child surfaces, recovery UI,
dynamic in-process installation and full Host/TUI integration remain pending. Counter/editor fixtures validate the
runtime, not the default product's final visual design.

GPUI window removal starts retirement before platform teardown. The independent
window owner waits for the patched backend destruction receipt, actual entity and
retained-frame release, and child cleanup before acknowledging the root. Last-window
removal alone cannot terminate the application. Failed cleanup keeps its fence.
Accepted asynchronous service work holds immutable text snapshots independently
of UI resources; the application lifetime owner retains unresolved work after
window and executor-task exit. Only successful cleanup allows shutdown. See the
[resource example and bounds](../ui/README.md) for the explicit Composer/Reader path.

Dependencies are pinned to GPUI Fast `e4aabe75` and GPUI Kit `f89c5cc9`; later Fast
text-wrapping API changes are incompatible with this Kit version. Upstream
`gpui-pre-*` reexports select Fast without a second GPUI implementation.

Native dependencies include MPL-2.0 [dwrote](https://github.com/servo/dwrote)
(Windows) and build-time [cbindgen](https://github.com/mozilla/cbindgen).
Their source is not vendored. Binary distribution must include licenses and
source references; this crate does not yet provide release packaging.
