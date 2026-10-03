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

Native factories receive `(NativeContext, configuration)`. The context is built
by that owner after acquiring the initialization fence, and carries the actual
activation plus optional, paired applied source and `Arc<EventLog>`. Ephemeral
applications have no persistence; services requiring a Root must reject them.
`EventLog::owned_root()` validates and shares its existing lease with trusted
native stores such as `ConfigurationStore`; their real shutdown belongs to the
returned `NativeOwner`. Stable data ownership uses Root domain, deployment UUID
and instance identity, while source revision and activation epoch remain execution
facts. These native dependencies do not replace per-request Policy admission.

Bindings capture the exact consumer and registration. Final admission checks the
Host `Issuer`, current Scope cancellation, method input, exact attachment slots/types
and explicit Host `Policy`,
then accepts work through the core without waiting between those checks. Forwarded
child Scopes retain the original private evidence. A package name, source identity
in JSON, or binding is not effect authorization.

Native `Handler::call` receives method, input and a request `Context`. Its Scope,
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
The public application SDK types come from the actual Rust ABI; initialization grants remain required work.

Policy receives a borrowed `Invocation` with named consumer, provider, Scope,
contract, method, input and attachment fields; this observation is not a new grant.

A child request carries a parent resource Ticket through its queue, executor and
actual cleanup settlement. Child cleanup completes the Ticket before publishing its
business result. Ordinary child rejection or business failure does not mark cleanup
failure; unconfirmed child cleanup retains both child and parent work and capacity,
even when JavaScript catches the error or returns without awaiting the child.
Context and owner hold only weak dispatch endpoints; observer futures retain no
attachment map or endpoint.

`Accepted::completion` observes separate business and actual executor-cleanup results
from the original owner; losing the owner reports cleanup as unconfirmed.
`Accepted::result` observes business only and cannot confirm cleanup. The owner
retains execution, Scope,
actual attachments and capacity until the query and its resources settle. Ordinary business
errors and result validation errors settle cleanly when resource cleanup succeeds.
Unconfirmed cleanup or an executor panic preserves work, Scope and capacity, and
retires affected registrations. Dropping the application root requests retirement;
client handles do not keep the root alive or cancel accepted work. Shutdown success
requires native/module cleanup and actual VM shutdown. Shared VM failure retires
its actual registrations and consumers, leaving unrelated domains independent.

Subscription methods use the same handler, binding, policy and execution owner.
`Client::subscribe` creates an independently cancellable reader; native handlers
publish through `Context::emit`, and JavaScript uses
`await context.invoke({operation:'emit', value})`. Each item and the final return
value must satisfy the method's output schema. A stream has 16 queued items and
16 pending emits, with a 64 KiB JSON limit per item; the Host admits at most 16
stream buffers, including completed streams with unread data. Producers wait for
capacity. Producer return, error or panic closes publication even if a native
handler retains a Context or send future; readers can drain accepted items to EOF.
`Subscription::cancel` and Drop request stop only. `completion` closes demand
before awaiting the original owner's separate business and cleanup results.
Cancellation never propagates to the caller's parent Scope or sibling streams;
failed cleanup still retains the original execution and its resources.

Calls have a 1 MiB JSON limit, with 128-call and 32 MiB in-flight input budgets.
Attachments allow 16 exact slots and 64 MiB across queued and accepted requests;
unconfirmed cleanup retains their payloads and capacity. This adapter currently
executes native and JavaScript queries and subscriptions with attached resources,
and resource-free durable mutations. Mutation resource snapshots,
trusted Host transport, live composition reconciliation, business recovery hooks,
and server/TUI/UI integration remain required work. This foundation is not a
completed Host migration or a compatibility facade for the previous catalog.

Durable mutations require `Application::start_from_store`, a declared mutation
method and an explicit non-nil operation UUID. Native callers first use
`Client::prepare_mutation`, commit its original reference through their own
`EventLog::record_application_intent`, then pass the prepared invocation and private
`RecordedIntent` to `Client::mutate`. Preparation holds input capacity but creates
no accepted execution or business journal entry. Dispatch rechecks the captured
binding, Scope, execution domain and policy; it never redirects to a current target.
`Context::mutate` performs the same caller commit and final admission sequence in
the original Host caller source. Native and JavaScript mutations share execution
with queries. JavaScript uses
`context.invoke({operation:'mutate', import, provider, method, input, operationId})`;
the callback's actual `HostObservation.operation` contains its original receipt
reference. Mutation resource attachments still need an executor.

Caller history retains the first source and original reference for each stable
consumer and operation, across source revisions and target changes. A different
reference conflicts. Caller records do not assert business outcomes or grant Host
authority. Failed or uncertain caller commits cannot dispatch effects; confirmed
tokens keep the caller Root owned through accepted execution and real cleanup,
including failed cleanup. Readable history cannot mint an execution token.

The original Input, package bytes and contract remain in immutable deployment
revision history. Dispatch commits before any callback; only its private first
commit claim can execute. Duplicate requests never execute or overwrite original
cleanup. Fingerprints canonicalize object key order but conservatively retain
actual JSON number representations; changed input is a conflict. A cross-VM
representation change must read the original reference rather than resubmit input.
`Mutation::reference` is observation, and acceptance alone is not dispatch confirmation.
`Mutation::result` returns an `OperationReply` retaining that reference even when the
owner is unavailable; only a recorded receipt proves original journal facts.
Native presentation adapters use `Mutation::completion` to observe this request's
actual executor cleanup separately from receipt knowledge. An unavailable receipt
does not imply failed cleanup; a historical receipt's cleanup does not describe the
new lookup. Losing the completion channel preserves the reference and reports
cleanup as unconfirmed.

Returned values, unknown results and real resource cleanup are separate journal
facts. Errors, invalid output and process loss do not prove rollback. SQL settlement
uncertainty does not imply failed physical cleanup. Observer or application Drop
does not cancel already accepted work; failed physical cleanup keeps actual work,
capacity and the Root lease. No uncertain operation is automatically replayed.

`Client::recover` reads a fixed original Root/reference without resolving a current
provider. Current consumer lifetime, the original issuer, revocation and explicit
`Policy::recover` authorization are checked before and after lookup; recovery is
denied by default, including duplicate result release. References grant no access.
Recovery remains available after replacing the deployment or removing its provider,
provided the new consumer has a real recovery grant in the original State Root.
