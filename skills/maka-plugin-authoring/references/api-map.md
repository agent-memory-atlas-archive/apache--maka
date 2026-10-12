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

# Exact application API

The root SDK export is the current application ABI. The `references/sdk/` files are
copied from this build's `packages/plugin-sdk/src/`; read them as source when needed.

| Need | Source |
| --- | --- |
| Component, Output, RequestContext, HostService | `sdk/index.ts` |
| Native scene constructors and target typing | `sdk/ui.ts` |
| PackageDeclaration, CompositionInput, HostRequest, OperationReply, native props | `sdk/generated/runtime.ts` |
| Exact public service/component contracts | `sdk/contracts.ts` |
| Native configuration schemas | `sdk/configurations.ts` |

Use the root definitions above and `maka application schema`.
The schema command emits contracts/configurations plus declaration schemas from the
same Rust types that validate actual runtime calls.

A TypeScript type is useful during authoring; runtime Rust still validates JSON,
resource slots, contract compatibility, budgets and permissions. Do not infer support
for an undeclared method.
