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

# Maka plugin SDK

[简体中文](README.zh-CN.md)

TypeScript contracts and helpers for Maka Host services and native UI components. Import from `@maka-agent/plugin-sdk`; bundle guest code before giving it to the runtime.

| Source | Purpose |
| --- | --- |
| [index.ts](src/index.ts) | `HostService`, `RequestContext`, `Component` and public exports |
| [ui.ts](src/ui.ts) | Terminal and desktop scene constructors |
| [generated/runtime.ts](src/generated/runtime.ts) | Rust wire types and native component props |
| [contracts.ts](src/contracts.ts) | Service and component contracts |
| [configurations.ts](src/configurations.ts) | Native configuration schemas |
| [native.ts](src/native.ts) | Native implementation definitions for package declarations |

Rust declarations are the source for generated files. After changing them, run from the repository root:

```sh
npm --workspace @maka-agent/plugin-sdk run schema:generate
npm --workspace @maka-agent/plugin-sdk run schema:check
npm --workspace @maka-agent/plugin-sdk run typecheck
```

Generation uses [maka-schema](../../tools/schema/README.md) and does not build the product or V8. The runtime also exports its catalog through `maka application schema`.

Components have synchronous callbacks that propose a frame and actions. Terminal and desktop use their own scene types. Native controls own editor buffers, selection and IME; capture resources through their declared contracts. Host services receive request-scoped imports and attachments, without ambient Node APIs.

Guest realms provide `TextEncoder`, `TextDecoder`, `atob`, `URL` and `URLSearchParams` for pure text and byte conversion. Decode Base64 to bytes with `Uint8Array.from(atob(value), c => c.charCodeAt(0))`; these APIs grant no filesystem or network access.

TypeScript checks authoring mistakes. The runtime separately validates values, contract compatibility, resource slots and permissions. See the [plugin guide](../../plugins/README.md) and [runnable starter](../../skills/maka-plugin-authoring/references/quickstart.md).
