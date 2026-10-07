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


The public [GPUI adapter](../ui-gpui/README.md) exposes `Widget`, `Slots`, input
bindings and compiled editor availability to other Rust crates. Native layouts may
render their committed slots in GPUI Elements or retained virtual-list callbacks.
Handles capture the exact subtree and native owner identity; they cannot retarget
a recreated owner, render a detached child or cross the original platform window.
Measurements may repeat. Prepaint permits one actual placement per mount, including
cached replay and retry rollback. Scoped handles hold only their subtree's leases.

Each component's element arena, retained GPUI state and input callbacks hold a
native resource lease. A detached, retired child closes only after its last lease
has actually been destroyed; the owner-thread receipt task does not wait for more
input. A visible retired scene stays fenced. Cleanup follows the original mount
parent chain, so one surface cannot acknowledge another surface's resources.
An independent window owner retains the application until backend destruction,
resource release and mount cleanup are confirmed, even after Surface destruction. GPUI Kit editor entities retain their
lease through actual release processing; dropping a handle alone is insufficient.

The imported Editor uses Kit's buffer, selection, undo and platform text-input
engine. Parent updates preserve it. Explicit replace/reset intents carry a native
revision and sequence; conflicting revisions or active composition reject them.
Only one unresolved command is admitted, with an applied/conflict receipt after
native execution. Reset outside composition clears undo; it does not claim to
cancel an operating-system IME session. Observations contain resource metadata,
not full text. Reads against the bound owner check identity/revision and UTF-8
boundaries and are limited to 64 KiB. Platform UTF-16 queries use this read path.

Run the [component example](../ui/README.md). Counter/editor fixtures validate
the runtime, not the default product's final visual design. Production dynamic
installation and shared Host/TUI integration are available; complete Agent UI,
real OS IME acceptance, child surfaces and recovery UI remain pending.

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

Production startup takes `run --root PATH --deployment UUID`, reads the original
stored source and exact package bytes, and checks the Desktop target before
activation. A newer desired source does not rewrite the running source. Root
custody remains with the application through pending/failed cleanup; successful
UI shutdown releases its custody separately from actual SQL connection close.

Both launchers support trusted [local Host connections](../application-bridge/README.md)
through `--connections FILE`. One foreground lifetime owner preserves actual worker
observers if the platform loop disappears. Native quit is denied until physical UI
cleanup, accepted work, caller SQL close acknowledgement and Host cleanup all
complete. Failed close keeps the store/Root/runtime held. macOS native quit has
been exercised with a real window and blocked File/SQL owners; Windows/Linux
native acceptance and complete product UI remain pending.

The launcher selects its same executable’s internal `application-host` role for
shared private IPC Hosts. Compiled business availability stays in the child,
selected by its persistent composition. Both parent and child enable Tokio IO;
UI cleanup precedes native connection revocation, same-child Closed and actual
child exit. An embedding supplies a trusted HostExecutable for its own compiled
Host role rather than passing native factory closures across processes.


The foreground native pump advances adopted committed plans outside paint, then
revokes retired native input before continuing preparation and action processing.
Unchanged Mounts retain their original editor entities and reading scopes. An
independent new plugin's initialization failure is a deployment diagnostic; it
does not close a healthy existing window. Actual native synchronization and
cleanup failures retain their original error/fence semantics. Production management
writes use the original caller operations and persisted source. The same foreground Updates pipeline approves and adopts committed sources
through the existing connections and application owner. The production launch
owner uses one window creation path for initial and replacement roots. Replacement waits for the original window
owner's platform destruction, real entity release and mount cleanup, then opens a
fresh root with the same application and accepted-work owner. Unrelated accepted
work may coexist with the new root; replacing its provider waits for its original
settlement. Failed cleanup cannot permit replacement. Explicit native quit stops
new admission and replacement before window removal. GPUI tests exercise held Kit
entities, original snapshots, provider fences and quit during handoff.

macOS acceptance has also installed an ordinary package through the OS file panel,
saved a complete source and automatically replaced Agent with Configuration Studio
in the same process, starting its previously idle Host. A connection saved from the
new product persisted only in that Host’s settings store. Normal window close
released the parent, both children and all three Roots. This does not establish
immediate release of the previous Host at handoff. IME, screen-reader, Windows/Linux
and complete product visual acceptance remain pending.

The foreground execution owner retains each original `NativeResources` object
before the platform cleanup observer is created. Only a successful original
surface receipt releases it; a failed or lost observer preserves native worker
custody and allocation permits after window/Surface destruction. `LiveInput`
supplies that original mount's work group and shared presenter budget. Cleanup
requires both real lease destruction and a clean sealed group; ticket completion
wakes the original closing owner after its tracker token is actually gone.
Connections and caller storage must be supplied when creating the execution
owner; later conflicting backend custody is rejected rather than ignored.
