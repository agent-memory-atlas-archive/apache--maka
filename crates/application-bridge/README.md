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

# Application Host connections

The Desktop and TUI launchers accept `--connections FILE`. This trusted input
explicitly chooses `process` or `cohost` for each selected deployment without
editing either composition. Process Hosts own a separate Application and Root;
cohosts run a distinct Host deployment in the UI's already owned EventLog. Both use
the same Server, PeerPolicy and native invocation adapter. Network authentication
remains separate work.

```json
{
  "hosts": [{
    "name": "primary",
    "location": { "kind": "process", "root": "/path/to/host/root" },
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
Plan. A forwarded call to a different Host consumer fails closed under this
configuration. Compiled Rust extensions run within their selected Host; all UI
remote services use the same physical transport adapter.

The configuration is closed JSON, bounded to 1 MiB, 16 Hosts, 128 imports per Host
and 64 methods per import. Omission means no local connections. Relative Root
paths resolve against the launcher's working directory.

After real surface and accepted-work cleanup, the launchers await both the caller
SQL owner's actual close acknowledgement and Host cleanup. Failure retains the
store, Root and runtime. Native quit is subject to the same gate. `drain_store`
and `Connections::drain` preserve custody instead of treating owner drop as a
cleanup receipt. Native endpoints retain the exact Host catalogue selection,
registration epoch and original physical-call custody; sessions freeze caller approval. They never
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
`Peers` holding exact caller-source and consumer approvals. `PeerPolicy` issues
private request evidence and checks the original Binding/method grant at final
admission; mutation confirmation also checks the original caller and operation.
Caller JSON, endpoint aliases and descriptor metadata are observations.

`maka application serve` owns one shared Host for its entire lifetime. The selected
child receives `process::Bootstrap` on inherited stdin: Root/deployment/namespaces,
exact approved UI source/consumers, Host consumer/import grants, private IPC address
and a nonnil incarnation nonce. The input is closed JSON, bounded to 512 KiB.
Stdin/stdout carry Bootstrap/Ready, Capture/Captured, Approve/Approved and Close/Closed. Each import selects
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
settles a lost dispatch; no mutation is automatically resent. Desktop/TUI
`--connections` use the shared child supervisor and this physical transport.
Complete native platform and product acceptance remain work.

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
original Connection. Both launchers use this adapter; there is no separate
in-process UI service/session implementation.

## Exact-source updates

The trusted Root manager may call `Connections::approve_source` with an exact
committed UI source and the revisions whose original consumers remain live. This
explicit operation delegates the original configuration's method/recovery grants;
watching a SQL head alone grants nothing. It cannot change the selected Host Root,
deployment, executable, gateway consumer or native grants.

Before publication, all participating children capture actual Bindings over their
original inherited control channels. The parent validates the entire candidate's
targets, contracts, unique registrations, approvals and wire bounds. A child then
publishes those same captured Bindings; it does not rebind an alias between
validation and approval. Unused Hosts apply retention without binding endpoints.
Known validation errors preserve the previous approvals. Cross-child physical I/O
failure is not a transaction or a rollback receipt: the original child/control
owners remain responsible for their actual state and cleanup.

Unchanged Host registration epochs reuse the original service implementation.
Sessions and issued Scopes freeze their original approval ticket. Revocation
rejects subsequent admission, including prepared but unaccepted mutations;
already accepted work retains its original execution, resources and cleanup.
Within the original controller incarnation, a revoked or superseded revision cannot
be newly approved. Reinstalling its contents requires a fresh committed revision;
retained live revisions keep their original ticket. Retained actors
continue using their original source, so the manager must retain those revisions
until their last executable consumer finishes actual retirement cleanup.


## Same-Root cohosts

`location: { "kind": "cohost" }` explicitly borrows the launcher's original UI
Store. It accepts no Root path, never opens a Root and requires a Host deployment
UUID distinct from the UI and other cohosts. Compiled native availability comes
from the embedding; composition, bindings, method and recovery grants still
select every activation and call. Generic Duplex channels use the same tracked
Server/native adapter as process Hosts, without sockets or a second call protocol.

Cohost availability includes `maka.application.ui-management`, bound to the exact
UI deployment and presentation. The product must explicitly compose and bind it.
Install/replace operations retain the cohost's original Host operation namespace
and journal; business results identify the UI desired source. Committing or
watching that source does not approve endpoints or claim foreground convergence.

Capture preserves actual Bindings for whole-batch approval; unchanged epochs keep
the same service implementation. Startup observers may be dropped while the same
owner retains original initialization and shutdown. The launcher drains UI work,
then every original connection, Server and Host Application, and only then shuts
down the shared SQL Store. Failed cleanup retains the original owners.


## Foreground updates

The production TUI and Desktop launchers own one `Updates` pipeline on the original
Store and Connections. It watches a coalescing deployment cursor, loads one exact
snapshot off the event loop, asks the original foreground Application to preflight
native implementations and scene languages, then explicitly delegates the original
connection configuration. `PreparedUpdate` is opaque and tied to that foreground
owner; final adoption rechecks provenance, revision and closing state.

Unchanged actors, registration epochs and physical surfaces retain their identity.
Replacement still waits for the original terminal/window cleanup acknowledgements.
Retention comes from the current desired revision and actual executable activation
origins, including retiring owners; structural origins confer no consumer grant.
Retention never binds endpoints or delegates new consumers. Local revocation does
not depend on a healthy child controller; every original controller receives an
independent retention attempt, and physical failures retain their original owners.

Only one load, approval or retention exchange runs at a time. Known preflight errors
leave the current frontend usable. Exit stops new adoption and waits for the original
in-flight worker before Connections and SQL drain; it never aborts an exchange or
turns an observer failure into a cleanup receipt. Product plugin-management screens
and end-to-end visual acceptance remain separate product work.
