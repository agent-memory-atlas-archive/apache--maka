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

# Plugins

[简体中文](README.zh-CN.md)

A plugin is declared by a leaf `package.ts`. It lists executable entries, configuration, imports and exports. A Cargo crate is a compilation unit: one plugin may use several crates, and related plugins may share an implementation crate.

| Family | What it owns |
| --- | --- |
| [agent](agent/README.md) | Conversations, model execution, tools, goals and scheduling |
| [workspace](workspace/README.md) | Files, processes, Web and Jev services |
| [settings](settings/native/README.md) | Connections, credentials and preferences |
| [skills](skills/native/README.md) | Skill discovery, reads and publication |
| [interactions](interactions/native/README.md) | Questions and forms |
| [workhub](workhub/core/README.md) | Task coordination and worker results |
| `ui/` | Declarations for native UI capabilities |
| `shared/` | Shared TypeScript contracts and authoring helpers |

For an existing plugin, start with its declaration and follow its selected implementation. JavaScript entries name a local module such as `tui.js`, built from `tui.ts`. Native entries select an implementation supplied by the executable. Neither a family directory nor a linked crate activates a plugin.

Build a single package from the repository root:

```sh
node scripts/plugins/build.mjs   --plugin plugins/workspace/workspace-web --output /tmp/maka-web-bundle
```

The builder replaces its output directory. It emits immutable-package inputs under `packages/`; inspect one with `maka application inspect-package --package /tmp/maka-web-bundle/packages/workspace-web`.

To include a plugin in Maka, add its path to [the product selection](../products/maka/bundle/package-inputs.json), then add instances and bindings in [the composition](../products/maka/bundle/src/composition.ts). A new native implementation also needs a factory in [the product registry](../products/maka/native/README.md). An import identifies a dependency; Host grants determine which calls are allowed.

The [plugin-authoring skill](../skills/maka-plugin-authoring/SKILL.md) includes a runnable standalone starter and guides for services, UI, state and native extensions. The [SDK](../packages/plugin-sdk/README.md) supplies the exact contracts.
