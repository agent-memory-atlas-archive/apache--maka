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
built-in business plugins by default. CLI Host serving, the TUI and ACP use this
application runtime through the public Host endpoint; Agent and Settings are
explicit native services. The former Operation/Kernel server has been removed.

`Application::adopt_committed` lets the trusted root owner adopt an exact committed
revision in its existing State Root. It validates the full candidate before kernel
installation, rejects foreign or older sources, and bounds source loading to one
in-flight snapshot. An acknowledgement means desired input was adopted, not that
activation or cleanup succeeded. `Client::snapshot` separates `desired` from
`effective` (the last fully converged source), and reports current instance phases,
startup failure and cleanup failure. `Application::source` is the original startup
source. `ready` waits for convergence and returns an observed failure.
`Client::ready_instance` observes one actual active instance without waiting for
unrelated initializers; its activation still needs normal binding and admission.

Unchanged activations retain their original source, native context and captured
bindings. Prepared and later mutations from those consumers keep that original
receipt reference. Changed instances revoke new admission; already accepted work
settles under its original owner. Replacement waits for actual call resources,
module/native close and final VM shutdown. A shared VM remains while any module
owns it. Initial startup and update failures retire only the failed instance and
its dependent consumers. Independent pending and loading instances continue.
`initialization_failures` identifies failed initializers and blocked dependents for
that desired source; their cleanup remains separately owned. The source cannot
become effective, and only a later committed revision permits another attempt.
Package storage does not imply activation or grant remote management access.

The owner task publishes an entire validated export batch. JavaScript services
retain actual receiver/functions through `Captured`, in `Vm::scoped` realms
without ambient Host routing or Deno globals. Native activation supplies an independent `NativeOwner` whose
real close completion covers its resources. Initialization is fenced before any
asynchronous work starts. Rejected or late exports still close under that original
owner; failed close retains the actual native owner and lifecycle fence.

Native factories receive `(NativeContext, configuration)`. The context is built
by that owner after acquiring the initialization fence, and carries the actual
activation, exact immutable selected package, plus optional paired applied source
and `Arc<EventLog>`. The package identifies execution provenance, not data ownership. Ephemeral
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
The public application SDK types come from the actual Rust ABI.

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

`Application::shutdown_if_idle` serializes the idle decision with invocation
admission and source adoption. It requires settled calls and a nonblocking
`NativeOwner::prepare_retirement` fence from every native owner; unsupported
owners report busy. Dropping preparation guards restores admission. Committing
them stops independent producers, and success still awaits ordinary physical
cleanup. Sleeping Agent timers do not count as active work; their current
iterations and accepted runs do. Lost retirement observers never release custody.

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

Calls have a 1 MiB JSON limit, with 128-call and 32 MiB in-flight input/descriptor
budgets. `Client::start`, `subscribe` and `prepare_mutation` consume `CallResources`;
native attachment maps enter through `From<Attachments>`. Attachments allow 16 exact
slots and 64 MiB across queued and accepted requests; unconfirmed cleanup retains
payloads and capacity. The final owner rechecks actual descriptors against the
original reservations before policy admission.

`Client::capture_resources` reserves these same call, JSON and payload budgets
before allocating immutable transport snapshots. Native producers use
`Context::capture_resources` (also available through `NativeRequest`) with that
context’s actual Scope; `Context::mutate` consumes the resulting `CallResources`.
Both entry points reserve the same Host capacities and retain the original request
until physical capture or dispatch cleanup completes. Descriptors preserve nominal
type, length, metadata and byte/UTF-8 encoding; these facts grant no authority.
Zero-byte receptions still own a call slot. `ResourceCapture::write` accepts only
contiguous chunks of at most 64 KiB. `finish` requires exact completion, validates
UTF-8 where declared, and moves the original buffers without a whole-payload copy.
The private frozen input transfers the same permits and original Scope ticket into
the existing executor; another Application or request Scope cannot consume it.
UTF-8 snapshots preserve character-boundary reads. The receiving adapter owns
partial/frozen input before dispatch and must drain it before server shutdown;
`Application::shutdown` alone does not acknowledge external reception cleanup.
An EOF or timeout cannot acknowledge accepted work or failed physical cleanup.

This adapter executes native and JavaScript queries/subscriptions with resources,
and durable mutations with request-local snapshots. The public Host supplies
authenticated IPC/stdio/WebSocket ingress and follows committed composition updates.

Durable mutations require `Application::start_from_store`, a declared mutation
method and an explicit non-nil operation UUID. Native callers first use
`Client::prepare_mutation`, commit its original reference through their own
`EventLog::record_application_intent`, then pass the prepared invocation and private
`RecordedIntent` to `Client::mutate`. Preparation holds the original input and attachment capacity but creates
no accepted execution or business journal entry. Dispatch rechecks the captured
binding, Scope, execution domain and policy; it never redirects to a current target.
`Client::mutate_from_peer` instead receives a bounded `CallerIntent` observation
from an authenticated caller adapter. It cannot reconstruct `RecordedIntent`,
prove another process's SQL commit, or cause a substitute Host caller commit.
The final owner checks the original reference, ordinary `Policy::admit` and the
additional `Policy::confirm_peer_mutation` before accepting execution. Confirmation
defaults to denial; a trusted policy must compare the actual peer's approved caller
source, UI consumer and original request against private Scope evidence. The UI
consumer and Host import consumer are distinct execution identities. The original
binding, cancellation and independent historical recovery checks still apply.
Confirmation panics use the existing isolated admission failure path. Accepted work
retains its local token or peer observation through actual cleanup and sticky failures.
The remote caller adapter must retain its **actual** SQL token/Root lease until
remote cleanup is confirmed; a peer observation cannot own that Root. Disconnect
or a business receipt is not cleanup confirmation.

`application::host::Host` exposes a persisted application over current-account IPC.
The embedding supplies checked endpoint and native grants. Each authenticated
session owns its exact peer approval and transport; closing it revokes admission
and drains its calls without affecting other sessions. Public mutations use
`Call::confirm` and `Client::mutate_owned`: the Host gateway records its own caller
intent before final admission. Private UI channels require their original caller
commit and reject Host confirmation. Public status counts are observations;
retirement delegates to `Application::shutdown_if_idle` and physical cleanup.
The same application wire runs over bounded binary WebSocket messages after HTTP
credential and Origin checks. Pairing grants the exact persisted source and
published endpoint selection, expires before finalization, and binds the importing
client identity. Session admission and credential revocation share one gate;
revocation closes only that credential's original transports. Native lifecycle and
credential controls remain local-owner operations. Legacy operation grants do not
authorize application calls. CLI serve, managed/ephemeral candidates, discovery,
credential controls and SSH stdio forwarding use this application Host. Initial
startup persists the selected product source and package bytes; reopening keeps
that source instead of reauthoring it. Managed directory changes update the saved
Agent, Skills and process scopes while preserving user permission restrictions.
An empty published directory list grants no workspace. Discovery is only a hint:
clients verify Root, epoch, source, generation and lifecycle against live status.
Candidate expiry requires both physical connection inactivity and the Application's
atomic idle fence. Explicit interrupt commits retirement before physical cleanup;
the original Root remains held until drain. Legacy cooperative connection handoff
is no longer exposed. The ACP entrypoint uses this same public Host and the
Agent/Settings service contracts; its original invocation receipts and physical
cleanup remain authoritative.

`Context::mutate` performs the same caller commit and final admission sequence in
the original Host caller source. Native and JavaScript mutations share execution
with queries. JavaScript uses
`context.invoke({operation:'mutate', import, provider, method, input, operationId, attachments})`;
the callback's actual `HostObservation.operation` contains its original receipt
reference. The `attachments` map explicitly forwards actual current request slots.

`prepare_mutation` takes actual `Attachments` and queues their custody synchronously.
An original Scope ticket follows the same map and capacity through the queued call,
prepared reply and executor. Dropping preparation releases actual snapshots before
acknowledging its ticket; close waits for physical custody, including unreceived
replies. No resource bytes or metadata enter the operation journal: a private
Host-minted bundle nonce binds the original fingerprint. A fresh bundle conflicts
with a prior operation UUID; a recovery reference never grants resource lookup.

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


A persisted application now observes committed heads in its same owned Root and
adopts exact immutable revisions without restart. `EventLog::subscribe_application`
returns a coalescing cursor; commit wakes cause a head-only SQL read, not package
copies or grants. The original SQL worker's real close result ends idle observation.
The owner retains one observation future across business turns and ignores delayed
heads already superseded by its adopted source. Actual candidate errors remain
visible without retiring unrelated healthy activations.

Only `Application` owns the bounded adoption ingress. `Client::retire` uses a
separate bounded lifecycle ingress and remains immediate while a source SQL load
is pending. `Snapshot.loading` reports the exact source being read/preflighted and grants no
access. Explicit adoption acknowledges its actual load/preflight/install;
activation convergence and resource cleanup remain independent. Accepted work and
original recovery references keep their original source. The application bridge
coordinates committed Host and UI sources with the foreground runtime.

## Deployment management

Persisted applications make `maka.application.management` available as a native
implementation. A product must explicitly compose an instance exporting
`Management` and select its bindings; availability grants no permission. The
default factory uses its original `NativePersistence` Root and deployment.
A trusted launcher can instead construct `ManagementTarget::from_source` and
register it with `provide_target`. The target captures an exact committed
revision's deployment and environment; activation requires the same original
EventLog owner as the management Host. This supports a distinct UI deployment in
the owned Root without opening it again. Requests cannot select a filesystem path,
another deployment, a different execution role or another presentation target.

`read` returns the committed input and exact available PackageRefs from a bounded
SQL metadata snapshot. It never loads package payloads or claims execution
readiness. `install` validates the actual immutable `package` resource using the
existing digest-checked bundle format, then CAS-commits the unchanged input with
those bytes available. It does not execute the new package. `replace` selects
already installed exact revisions and explicitly submits a new input with its
expected revision. Templates, bindings and overrides retain the ordinary core
validation; the manager does not add business routes or a second editable plan.

Both writes are ordinary journaled mutations. `ManagementResult` records
`committed`, `conflict`, or `rejected` as closed business outcomes. Known bundle,
input, role and package-reference rejection occurs before SQL effects. SQL,
COMMIT, worker and cleanup uncertainty retain the original operation's unknown or
failed-cleanup state. A commit acknowledgement is desired input, not convergence.
Default manager instances in the original application share a source-materialization
lane; fixed-target instances share the lane of their trusted target handle.
CAS remains the original SQL authority. Blocking bundle work holds its
actual Scope ticket until its resource drops and worker completes.

The contract and types live in plugin-core and generate the TS SDK. Host automatic
head adoption applies committed sources. A fixed UI target uses the management
Host's original operation namespace and journal; its returned deployment reference
identifies the distinct UI desired source. The application bridge owns cohosting
and foreground source coordination.
