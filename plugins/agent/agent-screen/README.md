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

# Agent screen

`AgentScreen` is the Agent's main UI: session navigation, conversation, composer and product pages. A session's `workspace` still means its Host working directory. The component can own a foreground surface or mount inside WorkHub.

| Module | Responsibility |
| --- | --- |
| `index.ts` | Component construction, initial state and lifecycle wiring |
| `state.ts` | The single component state, request types and navigation bookmark |
| `actions.ts` | Available commands and explicit service intents |
| `results.ts` | Query results, mutation receipts and live deliveries |
| `messages.ts` | Bounded live text and timeline display projections |
| `events.ts`, `component-events.ts`, `raw-events.ts` | Lifecycle/action routing, child callbacks and raw commands within one candidate-frame transaction |
| `frame.ts` | Retained root composition, dialogs and subscriptions |
| `conversation-view.ts` | Conversation, composer and its footer |
| `task-overview.ts`, `outputs-view.ts` | Recorded task state and bounded output/evidence views |
| `navigation-view.ts`, `header-view.ts`, `pages-view.ts` | Navigation, header and retained product pages |
| `tui.ts`, `desktop.ts`, `package.ts` | Target adapters and package declaration |

Modules share the existing `State`; they do not own separate stores or dispatch loops. View helpers return native scenes. Publication of a candidate frame and its intents still happens once at the component boundary. The default footer is a bound component, documented in the [runtime UI guide](../../../skills/maka-plugin-authoring/references/runtime-ui.md).

The Goal command opens a session-scoped panel with objective, iteration limit and optional token threshold. Native editors supply immutable snapshots for review before creation. The confirmation retains the selected model, workspace and permission boundary. Controls use the displayed Goal revision; a newer observation dismisses a stale confirmation. Original operation recovery checks the saved receipt rather than submitting another Goal.

The Plan command presents the current proposal or execution, ordered steps, risks and progress. Approval and resume review the exact artifact, selected model, workspace and access; a newer Plan revision dismisses the confirmation. Users can request revision, abandon a proposal or cancel its execution. History uses a pinned revision and bounded pages without live controls. Planning/Agent mode changes use the original session metadata mutation and are unavailable during active or queued work. Unknown approvals recover their original receipt without resubmission.

The WorkHub coordinator supports multiple conversations. Its Graph / Swarm panel selects the current conversation’s mode, displays original task results and final selections, and provides stop, restart and ordinary-conversation controls. Epoch history is pinned and read-only; full instructions and result pages retain their original identities. Starting a mode requires an idle Agent conversation without active Goal or Plan execution. A message after a closed, settled epoch starts a new epoch in the selected mode.
