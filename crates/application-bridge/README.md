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

# Local application connections

The Desktop and TUI launchers accept `--connections FILE`. This trusted input
starts a separate shared Host child for each selected deployment. It does not edit
either composition. Each child owns its original Application and Root; independent
private IPC invocation streams share that same Host state. Network authentication
remains separate work.

```json
{
  "hosts": [{
    "name": "primary",
    "root": "/path/to/host/root",
    "deployment": "00000000-0000-0000-0000-000000000010",
    "consumer": ["00000000-0000-0000-0000-000000000013"],
    "imports": {
      "tasks": { "methods": ["read", "create"], "recovery": false },
      "receipts": { "methods": ["read"], "recovery": true }
    }
  }]
}
```

`name` is the alias selected by the UI's remote capability observations. `consumer`
is the existing Host consumer instance; `imports` names its explicit bindings.
The Host composition must select a gateway instance with implementation
`maka.application.gateway` and its imports, or another appropriate compiled
consumer. No factory or service is automatically activated. Other Rust factories
are supplied by the selected executable’s compiled Host role; dynamic JavaScript comes from committed
immutable package bytes.

Mutations carry their original immutable snapshots through preparation, caller
persistence and final Host admission. The session moves its request resources
into the original queued/prepared custody, and Scope cleanup waits for actual
release even if prepare and close run concurrently. Snapshot bytes and metadata
are never operation-log input. Authenticated network transport remains pending.
Subscriptions cross private IPC and execute the actual Host `Client.subscribe`
with the original Binding/Peer Policy. UI cancellation cancels the Host execution scope and then
awaits its original admission observer and actual completion, including scope cleanup.
Dropping an admission observer cannot stand in for a cleanup receipt.

Each method must exist in the actual captured service contract. The endpoint
must match the selected UI deployment's observation. Startup captures that exact
`DeploymentRef`; a later desired-source update cannot inherit its approval.
Ordinary query/mutation permission does not grant original-receipt recovery.
The UI consumer set comes from enabled executable instances in the exact resolved
Plan. A forwarded call to a different Host consumer fails closed under this configuration;
other trusted embeddings can supply a different explicit Host Policy and inert
`Scopes` factory through `LocalService`.

The configuration is closed JSON, bounded to 1 MiB, 16 Hosts, 128 imports per Host
and 64 methods per import. Omission means no local connections. Relative Root
paths resolve against the launcher's working directory.

After real surface and accepted-work cleanup, the launchers await both the caller
SQL owner's actual close acknowledgement and Host cleanup. Failure retains the
store, Root and runtime. Native quit is subject to the same gate. `drain_store`
and `Connections::drain` preserve custody instead of treating owner drop as a
cleanup receipt. Native endpoints retain the exact Host catalogue selection,
registration epoch, caller source and original physical-call custody. They never
retarget a replacement Host or own its deployment lifecycle.

`Connections::prepare` validates the requested presentation and immutable source
before any Host starts. The launcher retains Connections before observing
`start(&mut self)`; cancelling that observer leaves the actual Child/Control in
that same owner. Close revokes every connection before polling its observer and
drains all children. Each requires its same-child Closed and an actual Child.wait
observation. Aggregate cleanup cannot settle an old Lost Call or caller SQL token.

SQL startup errors return only after real connection close. Typed
`StartupCleanupFailed`, a crashed worker or an unconfirmed reader close keeps the
original SQL worker and Root authority fenced. Launchers keep their runtime alive
on this boundary, including failures before Host Ready. The finite SQLx SQLite
[owned-startup seam](../../third-party/sqlx-sqlite/SOURCE.md) splits establishment
from configuration without adding another SQL owner.

[中文](README.zh-CN.md)

## Private invocation channels

`transport::Server` serves independently owned invocation streams on the existing
Application. Its embedding authenticates the physical channel and supplies a
`Peer` approving an exact caller source and consumer set. `PeerPolicy` issues
private request evidence and checks the original Binding/method grant at final
admission; mutation confirmation also checks the original caller and operation.
Caller JSON, endpoint aliases and descriptor metadata are observations.

`maka application serve` owns one shared Host for its entire lifetime. The selected
child receives `process::Bootstrap` on inherited stdin: Root/deployment/namespaces,
exact approved UI source/consumers, Host consumer/import grants, private IPC address
and a nonnil incarnation nonce. The input is closed JSON, bounded to 512 KiB.
Stdin/stdout are only for Bootstrap, Ready, Close and Closed. Each import selects
exactly one existing provider. Compiled gateway/agent/settings availability comes
from this executable; the persisted Host composition selects executed instances.

`runtime_host::ipc` is the canonical physical listener/connector used by both Host
paths. Unix sockets require a private current-account parent and use mode 0600;
Windows pipes reject remote clients and retain the current-account/SYSTEM ACL.
`transport::ipc::Connector` pairs each fresh stream with the original child nonce.
Hello uses the original Server's sixteen wire leases, tracked task and close fence,
before any Scope/Open. Pairing does not grant business permission. Independent
calls share the same original Host application and JavaScript state.

`process::Control` captures this child's actual Catalogue and retains its original
stdout across interrupted control reads/writes. It never appends a second command
after an uncertain write. Child shutdown revokes the listener, drains the original
Server and Application, then closes SQL/Root before publishing matching Closed.
Startup failure after opening the store follows the same cleanup. Start preserves
a matching Closed even when no Ready was published, while reporting startup failure.
The launcher must own the actual Child and independently observe its terminal exit;
normal shutdown requires success. A failed start can return a nonzero exit despite
confirmed cleanup. Neither aggregate Closed nor process exit settles a Lost Call.

Input and resource descriptors use separate bounded JSON frames. Raw chunks are
at most 64 KiB; the original Host slot/JSON/payload reservations precede reception,
including empty bundles. Sixteen connection leases bound wire buffering. They do
not create another resource budget or execution owner. Subscriptions acknowledge
admission, then receive only in response to the caller's next Pull.

Cancellation/EOF revokes the original Scope while the connection task keeps the
same admission and completion observers. Terminal messages report business and
actual Scope cleanup separately. Server shutdown drains reception/connections
before the embedding shuts down Application and SQL; failed or panicking cleanup
retains its original fence. Interrupted control writes forbid further frames;
half-close may still observe a complete terminal acknowledgement on the same
original channel. Cancel delivery failure does not erase an already sent reply.

`transport::Call` owns the caller's immutable inputs and actual `RecordedIntent`.
I/O failure retains this custody. `disconnect()` closes both physical halves while
retaining unconfirmed inputs/token; EOF is a cancellation request, not a remote
cleanup acknowledgement. A successful fresh original-receipt query never
settles a lost dispatch; no mutation is automatically resent. The default
Desktop/TUI `--connections` path still uses `LocalService`. Default launcher process supervision and native platform acceptance remain work.

Before accepting the invocation, the CLI publishes a bounded `Catalogue` from its
actual captured Bindings. A `Selection` freezes the endpoint alias and registration
epoch. Every Open carries that epoch; another Host incarnation cannot retarget an
old selection, even if it publishes the same alias. Provider/contract descriptions
are read-only facts, not grants or constructors of Host Bindings. A fresh recovery
query selects the new Host registration and reads only the original operation Ref.

`transport::native::{Connection, Service}` adapts this channel to the original UI
kernel's query, subscription and prepared-mutation interfaces. The trusted
`Connector` opens channels to the same authenticated Host incarnation that supplied
the catalogue. Cancellation awaits that original opening's explicit completion;
constructor, poll or destructor panic retains its opening fence. The adapter
checks the UI kernel's private source against the approved caller and transfers
actual SQL tokens synchronously when dispatch observers are created. Stop requests
precede the session mutex; cleanup waits for the same invocation and actual local
resource destructors before releasing capacity. Failed custody stays in its
original Connection. This adapter is not yet the default Desktop/TUI connector.
