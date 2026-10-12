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
| `navigation-view.ts`, `header-view.ts`, `pages-view.ts` | Navigation, header and retained product pages |
| `tui.ts`, `desktop.ts`, `package.ts` | Target adapters and package declaration |

Modules share the existing `State`; they do not own separate stores or dispatch loops. View helpers return native scenes. Publication of a candidate frame and its intents still happens once at the component boundary. The default footer is a bound component, documented in the [runtime UI guide](../../../skills/maka-plugin-authoring/references/runtime-ui.md).
