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

# Host application services

`application::Application` runs a service deployment using the same plugin-core
composition and lifecycle kernel as the UI. It accepts immutable application
packages and an explicit native implementation registry; it does not activate
built-in business plugins by default. The existing server entry point has not
yet migrated to this adapter.

The owner task publishes an entire validated export batch. JavaScript services
retain actual receiver/functions through `Captured`, in `Vm::scoped` realms
without ambient Host routing or Deno globals. Native activation supplies an independent `NativeOwner` whose
real close completion covers its resources. Initialization is fenced before any
asynchronous work starts. Rejected or late exports still close under that original
owner; failed close retains the actual native owner and lifecycle fence.

Bindings capture the exact consumer and registration. Final admission checks the
Host `Issuer`, current Scope cancellation, method input, exact attachment slots/types
and explicit Host `Policy`,
then accepts work through the core without waiting between those checks. Forwarded
child Scopes retain the original private evidence. A package name, source identity
in JSON, or binding is not effect authorization.

Native `Query::call` receives method, input and a request `Context`. Its Scope,
attachments and declared import bindings are actual capabilities; the owner captures
imports before asynchronous activation. Native and JavaScript calls use this same
Context and final admission path. `Context::call` enqueues synchronously and returns
only a result observer, preserving the original private evidence.

Queries accept `maka_plugins::call::Attachments`: actual immutable
`ReadResource` payloads, shared with UI capture adapters. Metadata and JSON slot
names cannot construct these values. Forwarders pass actual payloads explicitly;
text resources enforce their own UTF-8 ranges and chunk limits. Native descriptor
panics before admission reject that request while preserving existing work. During
an accepted parent's forwarding preparation, a panic records that parent's cleanup
failure; catching the business error cannot release its work.
JavaScript query callbacks receive `(input, observation, context)`. The frozen
context exposes request-private `invoke` operations: `metadata` with a slot,
`read` with a slot and byte `start`/`end`, and `call` with `import`, zero-based
`provider`, `method`, `input`, and an `attachments` map from target slots to current
request source slots. Calls select only the captured imports and actual payloads.
Reads return bytes, limited to 64 KiB;
slots resolve only within this request's actual attachments. Revocation blocks
new reads. Saved contexts expire before callback completion is reported, without
retaining snapshots. Native read/metadata panics record Scope cleanup failure,
even if JavaScript catches the rejection; unrelated modules remain runnable.
The public SDK and initialization grants are not yet implemented.

Policy receives a borrowed `Invocation` with named consumer, provider, Scope,
contract, method, input and attachment fields; this observation is not a new grant.

A child request carries a parent resource Ticket through its queue, executor and
actual cleanup settlement. Child cleanup completes the Ticket before publishing its
business result. Ordinary child rejection or business failure does not mark cleanup
failure; unconfirmed child cleanup retains both child and parent work and capacity,
even when JavaScript catches the error or returns without awaiting the child.
Context and owner hold only weak dispatch endpoints; observer futures retain no
attachment map or endpoint.

An `Accepted` result is only an observer. The owner retains execution, Scope,
actual attachments and capacity until the query and its resources settle. Ordinary business
errors and result validation errors settle cleanly when resource cleanup succeeds.
Unconfirmed cleanup or an executor panic preserves work, Scope and capacity, and
retires affected registrations. Dropping the application root requests retirement;
client handles do not keep the root alive or cancel accepted work. Shutdown success
requires native/module cleanup and actual VM shutdown. Shared VM failure retires
its actual registrations and consumers, leaving unrelated domains independent.

Calls have a 1 MiB JSON limit, with 128-call and 32 MiB in-flight input budgets.
Attachments allow 16 exact slots and 64 MiB across queued and accepted requests;
unconfirmed cleanup retains their payloads and capacity. This adapter currently
executes native and JavaScript queries, including attached resources. Mutation
and subscription are rejected before activation until their durable and stream
executors are implemented. Host transport, persistent composition
commit/reconciliation, operation recovery, the public SDK,
and server/TUI/UI integration remain required work. This foundation is not a
completed Host migration or a compatibility facade for the previous catalog.
