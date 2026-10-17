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

# maka-agent-api

[简体中文](README.zh-CN.md)

Contains Agent, Session, Executor and WorkHub wire declarations, plus the Agent configuration schema. Native services and SDK generation share these definitions. Use this crate when a consumer needs the public contract without pulling in the Agent execution engine.

Independent Desktop clients consume the same persisted Agent service through the Root-bound [public Host client](../../../platform/host-runtime/src/application/host/client.rs); they do not open Agent storage or embed its execution engine. [Agent contracts](src/contract.rs) and [interaction contracts](src/contract_interactions.rs) are the source of field definitions and schemas.

Read `history` pages under one fixed `through` fence and resume `watch` from its last `nextAfter`; watch also publishes queue and pending-interaction updates. `send` admits work and returns its identity; `stop` names that exact invocation. Use `pendingInteractions`, `interaction` and `respond` to reconstruct and answer the original request after reconnecting. Mutations retain their original operation receipts: a lost response does not authorize another send. See [native acceptance](../native/README.md) and [transport ownership and exit modes](../../../platform/transport/README.md).

`diagnostics` reads bounded, newest-first pages of execution, model-step and automatic-title failures under a fixed `through` fence. `watchDiagnostics` emits the current diagnostic revision on subscription and after changes; reconnect by reading the latest page again. These session-scoped operational records retain original invocation/event identities and safe typed causes. They exclude provider bodies and failures belonging to copied source sessions. Their revision is independent of conversation history, so a late title failure remains discoverable after the answer finishes.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
