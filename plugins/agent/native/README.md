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

# maka-agent-native

[简体中文](README.zh-CN.md)

Exposes Agent execution as native application services. `provide` registers factories; `executor` and `workhub` connect the service to the execution and coordination domains. Packages and product compositions choose which factories to activate and how to bind their imports.

The service owns its activation and delegates runs to `maka-agent-engine`. Session history, controls and tool calls use admitted scopes and original operation receipts. Files, processes, skills and other services are explicit dependencies selected by composition.

The Agent captures the model's Code Mode default or explicit `codeMode` override when it accepts a message. Queued work and resumed runs retain their original mode. A service tool can opt into Code Mode with `nestable: true`; nested calls use the same permissions, operation receipts and cleanup as direct calls. JavaScript receives the declared business result; the durable journal retains the original native receipt. Omitted `nestable` keeps the tool callable only directly. The Maka default composition enables nesting for files, processes, Web and Skills.

The default Maka composition enables `toolDiscovery`: optional tool schemas load through `tool_search`, with loaded schemas retained across turns in the same active Host session. Code Mode can still call authorized nested tools before their descriptions are loaded. Discovery policy is captured when a message is accepted and retained on continuation. Loaded schemas and Code Mode `store`/`load` values are session-local, in-memory state; restarting the Host clears them. Committed context compaction resets the loaded schema set.

The independent `applyPatch` model override selects either patch editing or structured Write/Edit tools. Like Code Mode, it defaults on for GPT and DeepSeek models and the DeepSeek endpoint, and is frozen for queued work and continuations. Selection follows the bound Files contract and method even when a composition renames its tools. Read tools remain available, and all file operations retain their live permission checks. The connection settings model-overrides JSON accepts `codeMode` and `applyPatch` separately.

The default Maka composition enables `todo`: `todo_read` and `todo_write` maintain a persistent checklist for the current session, including in Code Mode and read-only workspaces. Writes replace the whole list with revision checks. The UI shows reported progress and a read-only Checklist panel, with bounded pages covering up to 200 items. Checklist completion is model-reported progress, not proof that an operation succeeded.

The default Maka composition enables `automaticTitles`. Conversations created with `automaticTitle: true` receive one title attempt after their first admitted message, using the accepted model with a bounded, separately accounted request. Explicit titles, manual renames (including the unchanged placeholder), archived and managed sessions retain their names. Retries and Host restarts do not repeat an admitted title attempt; Host shutdown waits for its physical cleanup.

Model context capacity, any independent input ceiling and the compaction threshold are captured with the accepted model selection and retained across queueing and restart. Without an explicit threshold, known input capacity sets the proactive threshold at 85%; output reserve remains a separate engine calculation.

The default composition enables `recaps`. The Session recap command reads cached text; generating a new recap requires explicit confirmation of the selected model and possible charges. Each request reads at most 256 bounded history pages, keeps at most 32 KiB of history text, and limits output to 1,024 tokens and 30 seconds. Incognito mode and session access are checked even for cached reads. Refresh and original operation recovery never regenerate; an unknown request remains pending across restart. Auxiliary usage belongs to the original application operation, and shutdown waits for physical model cleanup.

The default composition enables `goals`. The Goal command creates or saves an objective with 1–100 iterations and an optional retrospective token threshold. Confirmation captures the selected model and the exact session workspace and permission boundary; a changed boundary requires renewed approval. Each iteration enters the original Agent message queue and retains its original admission and operation receipts across restart. `GoalStatus` is always discoverable and callable from Direct and Code Mode, but can report only for its own active Goal execution. A report takes effect only after normal completion and physical cleanup. Pause allows an accepted iteration to finish; Cancel targets only that Goal's original execution. Missing token counters stop budgeted continuation. Incognito mode, archived sessions and revoked authority prevent new admission; paused Host handoff requires explicit session recovery before resuming the Goal.

The default composition enables `plans`. The Plan command switches an idle conversation into Planning mode, whose tool selection permits inspection, questions and `SubmitPlan` but no workspace mutation or autonomous delegation. Submission ends that Turn without executing. Approving the displayed proposal freezes its artifact, selected model, workspace and permission boundary; implementation enters the original Agent queue. `update_plan` also works in Code Mode and reports every frozen step, but completion requires both all steps completed/skipped and successful execution with physical cleanup. Explicit resume preserves the artifact and progress while obtaining a new approval. Cancel targets only the original Plan invocation. Durable decisions, bounded read-only history and original approval receipts survive restart; stale boundaries and Incognito prevent new execution.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
