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
connects separate UI and Host deployments in one process. It does not edit either
composition. Private cross-process invocation channels are described below.
Network authentication and the default UI connector remain separate work.

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
are supplied by the trusted launcher; dynamic JavaScript comes from committed
immutable package bytes.

Mutations carry their original immutable snapshots through preparation, caller
persistence and final Host admission. The session moves its request resources
into the original queued/prepared custody, and Scope cleanup waits for actual
release even if prepare and close run concurrently. Snapshot bytes and metadata
are never operation-log input. Authenticated network transport remains pending.
Local subscriptions execute the actual Host `Client.subscribe` with the original
Binding/Scopes/Policy. UI cancellation cancels the Host execution scope and then
awaits its original admission observer and actual completion, including scope cleanup.
Dropping an admission observer cannot stand in for a cleanup receipt.

Each method must exist in the actual captured service contract. The endpoint
must match the selected UI deployment's observation. Startup captures that exact
`DeploymentRef`; a later desired-source update cannot inherit its approval.
Ordinary query/mutation permission does not grant original-receipt recovery.
A forwarded call to a different consumer fails closed under this configuration;
other trusted embeddings can supply a different explicit Host Policy and inert
`Scopes` factory through `LocalService`.

The configuration is closed JSON, bounded to 1 MiB, 16 Hosts, 128 imports per Host
and 64 methods per import. Omission means no local connections. Relative Root
paths resolve against the launcher's working directory.

After real surface and accepted-work cleanup, the launchers await both the caller
SQL owner's actual close acknowledgement and Host cleanup. Failure retains the
store, Root and runtime. Native quit is subject to the same gate. `drain_store`
and `Connections::drain` preserve custody instead of treating owner drop as a
cleanup receipt. Local endpoints hold captured Client/Binding/Scope admission,
not Host deployment lifecycle ownership.

[中文](README.zh-CN.md)

## Private invocation channels

`transport::Server` serves independently owned invocation streams on the existing
Application. Its embedding authenticates the physical channel and supplies a
`Peer` approving an exact caller source and consumer set. `PeerPolicy` issues
private request evidence and checks the original Binding/method grant at final
admission; mutation confirmation also checks the original caller and operation.
Caller JSON, endpoint aliases and descriptor metadata are observations.

`maka application serve --root ROOT --deployment UUID --peer FILE` serves one
invocation on launcher-provided stdin/stdout pipes. The closed approval file is
bounded to 512 KiB and contains `source` (DeploymentRef), `consumers` (UI instance
IDs), `consumer` (Host instance ID), and `imports` (method/recovery grants as
above). Each import selects exactly one existing provider. The launcher chooses
the process, owns its private pipe handles, and provides that approval file.
The command registers compiled gateway/agent/settings availability; the persisted
Host composition still selects every executable instance.

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
Desktop/TUI `--connections` path still uses `LocalService`. Shared authenticated
listeners, the default launcher connector and native platform acceptance remain work.

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
