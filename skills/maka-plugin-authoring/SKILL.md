---
name: maka-plugin-authoring
description: Build, extend, debug, or package Maka application plugins with Host services, native TUI or Desktop components, and explicit product compositions. Use for Maka plugin development and SDK integration.
license: Apache-2.0
---
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

# Maka application plugin authoring

Use the application runtime described by `@maka-agent/plugin-sdk` and the schemas
from the actual `maka application schema` command. The package declaration,
composition, execution grants and runtime activation each describe different facts.

Start with [Quickstart](references/quickstart.md) for a runnable Host/TUI/Desktop
package. For existing work, inspect its declared entry, imports and exports before
changing code. Keep instance UUIDs stable when changing configuration; changing an
identity replaces its state owner.

Read only the relevant guide:

- [Services and model tools](references/tools-and-input.md): HostService, requests,
  mutations, subscriptions and explicitly selected Agent tools.
- [State and authority](references/state-and-authority.md): ownership, cancellation,
  receipt recovery and retained resources.
- [Native presentations](references/terminal-apps.md): Component callbacks, platform
  scenes, native editors and input routing.
- [Models and execution](references/models-and-execution.md): existing Agent/model
  contracts and where native execution extensions belong.
- [Native Rust](references/native-rust.md): compiled factories and actual Call custody.
- [Testing and shipping](references/testing-and-shipping.md): meaningful validation,
  immutable package references and installation.
- [API map](references/api-map.md): exact source contracts included in this build.

When reading through Maka's selected skill loader, use the exact skill reference
and catalog revision from its current search or preparation result. Read a relative
resource with `resource: { path: "references/quickstart.md" }`; copy each returned
`next` object unchanged. Use the configured tool name shown in the current metadata.
In a checkout these are ordinary files; `references/sdk/` is embedded from
`packages/plugin-sdk/src/` during the build.

Deliver the package or change with evidence from its actual declared runtime.
A valid manifest alone does not prove the service, native scene or recovery behavior.
