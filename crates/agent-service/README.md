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

# Native agent application service

This crate provides `maka.agent.api`, an ordinary native application implementation exporting `agent` (`maka.agent.Agent` v1). Desktop and TUI launchers register availability; immutable product composition decides activation. It does not open the legacy business Host or install legacy business plugins.

Configuration declares finite `maxActiveRuns` and `maxSteps` (both 1–16). Each `send` supplies an explicit `model` selection containing the connection ID, observed revision and model ID; its first SQL snapshot checks that revision and enabled membership before preparing any run. The selected Root owns credentials, headers, endpoint and network settings. Each logical step reads that exact connection; unavailable selections never fall back. Provider credentials require the configuration store's real authentication receipt. Required `tools` selects `{name, description, import, method, confirmation}` entries from the native entry's explicitly declared, frozen imports. Empty `tools: []` intentionally means a conversation without tools. Each selection requires exactly one binding and a query or mutation method without resource attachments; input/output schemas come from that service contract. The output schema describes the model-visible business result. Native Host grants separately authorize the exact origin/consumer/provider/export/method edge; selecting a tool does not grant it. There is no implicit exposure of settings, receipts or other imported methods.

A selected tool may also provide `title: {fallback, translations}` for a localizable card label (for example `{"fallback":"Inspect file","translations":{"zh-CN":"查看文件"}}`). The original ToolJournal records this display text at dispatch; it never replaces the configured tool name, service binding or grant. Labels are limited to 256 UTF-8 bytes, with at most eight translations, and reject control or bidi override characters. History exposes a unique matching activity title and whether its native turn is still open at the requested fence. Pending interaction summaries carry the exact tool-use ID, or null for requests unrelated to a tool; clients must let recorded results and terminal turns take precedence over older pending observations.

Required confirmation publishes a canonical service approval for that exact tool call before dispatch; `preapproved` uses the already selected service policy. Neither choice replaces native grant admission. Each admitted tool uses the original ToolJournal, then creates a fresh native request on the same application kernel. Mutations use stable IDs derived from the actual invocation and tool operation identity. Successful results keep the original service receipt as raw evidence and project only business JSON to the model. Unknown outcomes keep typed original operation evidence separately from their concise model diagnostic. Failed cleanup keeps a nonsettlement observation and retains the original run permit and native owner fence. Stop cancels the common run root, and physical tool cleanup must finish before capacity is reusable. JSON results never acquire filesystem or image authority by resemblance to resource descriptors. Resource-bearing and streaming service methods require a separate explicit tool adapter.

`create` and `send` are durable mutations. `send` authorizes a complete bounded run on the explicitly selected account/model and returns its admitted invocation ID, not a terminal verdict. The original NativeOwner retains the run handle and a nonwaiting capacity permit before replying. Losing a request or UI observer does not cancel accepted work. `stop` names the exact invocation; stopping an old invocation cannot cancel a successor. `list`, `observe`, `history` and `watch` read canonical session metadata, same-snapshot run/active-part identities, text transcript pages and bounded delivery facts. Delivery excludes provider request payloads and credentials.

`uploadAttachment` accepts one immutable binary resource in the `file` slot, with `sessionId`, name, byte count and SHA-256 digest. The owner verifies up to 50 MiB of actual bytes and commits the original SessionArtifact atomically; the returned descriptor includes its owning Session. Invalid material, capacity, closure and archived sessions return explicit rejections. The original request Scope retains the worker through SQL completion, with one materialization permit per Agent owner. Recover a lost reply through its original operation reference; recapturing a file creates a different resource identity.

`send.attachments` accepts at most eight distinct upload IDs from that same owned Session, resolves canonical metadata and permits attachment-only messages. Queue text edits preserve the original attachments. Files `readText` and `readImage` resolve attachment URLs/short prefixes and continuations after original live invocation authorization, without opening a workspace path. Text is UTF-8 without NUL, up to 10 MiB; its bounded page reserves the service envelope so `next` remains directly model-visible. Image reads reuse the existing 5 MiB image subscription; PDF decoding remains unsupported. This supplies the service path for TUI attachment input; the ordinary product's picker, clipboard, draft persistence and attachment management remain to be connected.

Session IDs belong to the actual Root/deployment/plugin instance, independent of package revision and activation epoch. Sessions retain conversation metadata; each canonical invocation records its actual selected binding. A later send in the same conversation may choose another model. The run freezes provider identity, stored configuration and resolved execution parameters; name, testing, catalog descriptions and other models do not interrupt it. Endpoint, protocol, budget or selected membership changes reject subsequent steps. Credentials, headers and network policy continue through their original configuration owner. Original operation references derive domain IDs; journal UUID conflicts remain fail-closed. Retiring the service cancels and drains its original workers and configuration SQL owner. Unconfirmed physical cleanup retains its permit and reports failed native close so the application keeps its Root fence.

`execution` reads an exact `sessionId` / `invocationId` receipt, including its
physical run, state and Run-local `highWater` event count. It never follows the
Session's latest turn. `resume` names that original invocation and count, and
explicitly selects its model, with an optional thinking override. Only sealed failed/cancelled
runs can continue. The original Engine verifies bounded lineage, workspace
identity, replay and current tools; unsettled dispatches remain blocked. Recorded
unknown service results retain their original operation references and remain
unknown. Original
prepared input and settled tool results are replayed without redispatching those
calls. The new continuation uses the existing worker/capacity owner, live service
permissions and canonical opening receipt. Busy, stale, archived and unsafe
requests return `parked`; querying a receipt grants no execution authority.
Lost replies still recover through the original operation reference. No product
resume button or WorkHub orchestration is installed by exposing these methods.

`history` pins a committed raw-event `through` fence on its first window. Preparation is bounded; retries keep the same position and returned fence. `start` scans forward and `tail` reads the end; `before` ends exclusively and `after` starts inclusively at a real sparse transcript row and UTF-8 byte offset. Both cursor positions require the original explicit fence. Ready windows always return `before` and `after`; only `null` means that direction has ended. Windows retain ascending source order, including backward reads, with at most two fragments and 8192 UTF-8 bytes in total. The complete serialized response is checked against 64 KiB; attachments and provider payloads are excluded. `event` locates an inclusive raw event-log sequence for execution bookmarks, without fabricating a transcript cursor. Active parts include original start sequences, allowing clients to replay text prefixes without reading hidden thinking as answers. Canonical transcript changes have explicit delivery invalidations. Every window rechecks the selected session namespace. Public Rust wire types feed the generated plugin SDK directly.

A selected service tool may declare an optional `presentation` key (at most 128 ASCII letters, digits, or `._-:/@`). Agent freezes it alongside the tool before dispatch; the runtime carries it as display metadata and does not use it for execution or authorization. The ordinary product recognizes `maka.workspace.Files@1/readText`, `/write`, `/edit`, and `/applyPatch` for their corresponding Files v1 result shapes. Missing and unknown keys retain the generic original-field reader. Historical display never consults current tool aliases or bindings.

The `source` query can select a value inside JSON arguments, results or receipts using optional `path`, for example `["value", "content"]`. Strings address object keys; nonnegative integers address array elements. Selectors contain at most 16 segments and each key is at most 256 UTF-8 bytes. The same fenced pagination returns JSON strings as literal UTF-8 text and other values as JSON, preserving null separately from a missing value. The response echoes the path. Omitting it reads the original encoded field; `[]` explicitly selects the JSON root. Neither a selector nor a returned path grants filesystem access.

The optional `maka.workspace.files` implementation exports `files` (`maka.workspace.Files` v1): `readText`, `glob`, `grep`, `write`, `edit`, and `applyPatch` reuse the original filesystem executors. Compositions explicitly bind its methods and native grants. Session `permissions` selects `read_only` or `workspace_write` inside the captured selected directory, plus whether additional write requests may be presented. Workspace metadata remains read-only; Host-private storage is excluded. JSON paths cannot widen the captured directory. `readText` returns paged UTF-8 text; image/media output requires a separate adapter.

Additional access uses canonical Permissions records and Once/Turn/Session grants. Session metadata changes and explicit `revokeGrants` advance the original permission boundary, atomically close pending approvals from its old revision and invalidate prior additive grants. A final canonical admission check precedes the joined original filesystem executor. Responses use the existing operation receipts; cancellation does not release capacity before resource cleanup.

The initial product composition lives in [agent-product](../../packages/agent-product/README.md). Process execution, image/media tools, form/client-capability approvals, attachment and diff workflows remain subsequent product work; this crate is the execution service, not a complete agent application.

A complete Files presentation path selected through `source` may include `filePath`
in the ready response. The Agent resolves this display string against its stored
Session workspace on the Host, preserves literal `..`, and rejects oversized,
control/bidi and URI inputs. It performs no filesystem access and grants no file
capability. Unknown presentation keys, unrelated selectors and partial excerpts
have no projected path. The ordinary product uses it only for native path copying.

`maka.workhub.api` exports the same `agent` service plus `workhub`
(`maka.workhub.Coordinator` v1). It owns one ordinary Agent executor with a
coordinator prompt and a separate stable Session namespace. Use the Agent
configuration inside `{agent, workers}`, where `workers` selects at most eight
explicit Agent import aliases. Bind tools and native grants independently.
`workhub.coordinator({})` reads the designated coordinator without starting work;
`workhub.resolve({workspace: directorySelection})` freezes its first creation
intent and resolves that original Session on subsequent calls. Later workspace
arguments do not move an existing or pending coordinator. Concurrent calls and
reactivation reuse the original operation identity. A pending intent whose
original directory changed is rejected instead of being redirected. The normal
Agent export supplies model selection, messages, history, queue, stop and resume;
its other Sessions do not change the designated coordinator. Behavior is frozen
with each accepted message, including queued messages restored after activation.
Selected workers enable the private `workhub_tasks` coordinator tool: list workers
and Sessions, route original user text plus a bounded instruction to an existing
worker Session, and inspect or advance the exact assignment. It freezes source,
worker binding, child operation and input before dispatch. The existing caller
intent journal holds the original operation reference; unknown replies never
cause redispatch, and replacement bindings cannot take over old assignments.
Read-only `workhub.tasks` returns bounded pages and `workhub.task` reads original
admission and available canonical progress. Admission is not completion. Queued
coordinator input also freezes worker bindings; changed bindings fail preparation.

`agent.work({sessionId, target, cursor})` follows the original invocation or queued
message through its canonical continuation. It returns exact execution, pending
interactions and bounded terminal answer pages from one snapshot; answer cursors
bind invocation and UTF-8 byte offset. A changed continuation returns `changed`.
Unrelated Session activity never replaces the task result. Shared delivery is
explicit and cannot authorize control of the whole Run. `execution` retains its
separate meaning as the exact physical invocation receipt.

The coordinator's `wait` tool observes an admitted task until completion, input,
pause or unavailable progress, then returns that original task to the model in the
same conversation. Cancellation closes the observation and joins its actual
cleanup without stopping independent worker work. Session discovery refreshes
from the last ID on each model turn; it is a live browse, not a catalog snapshot.

`control` freezes stop, resume or queued-message retraction before dispatch.
`controls`, `inspect_control` and `advance_control` recover that original command;
public `workhub.controls` and `workhub.control` are read-only. A delivery race
cannot turn retraction into stopping a newer Run. Requested stop does not mean
physical termination, and unknown replies never authorize redispatch.

After worker reactivation, original admission/control receipts can still be
recovered, but current bindings cannot supply the old task's progress.
Cancellation reaches the original native cleanup budget; unconfirmed cleanup
retains Host work and the Root fence. Cross-Session attachment transfer, worker
creation, feedback after the coordinator ends and the WorkHub product UI remain
unfinished.
