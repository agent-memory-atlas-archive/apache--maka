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

# Architecture

[简体中文](ARCHITECTURE.zh-CN.md)

Maka is a plugin application runtime. Host and UI instantiate the shared plugin kernel with explicit capability bindings. A committed composition selects executable entries, configuration and its launch component; terminal and Desktop are native presentation adapters.

- **Admission and settlement:** `crates/runtime-host` admits operations, captures execution boundaries and coordinates resource cleanup. `crates/event-log` owns durable execution facts and state-root ownership.
- **Agent execution:** `crates/agent`, `crates/runtime` and `crates/model` build model requests and settle tool effects. `crates/providers` owns provider-specific authentication, discovery and policy; `crates/responses` owns native Responses transport.
- **Capabilities:** business plugins use `crates/plugins` capabilities for storage, models, execution, credentials, HTTP and presentation. Host owns authority and lifetimes; each plugin owns its domain policy.
- **JavaScript:** `crates/js-runtime` embeds V8 for Code Mode and JavaScript plugins. Provider SDKs are bundled at build time. Guest code reaches host capabilities through explicit bindings.
- **Computer Use:** `crates/computer-use` owns its own REPL and session lifetime. It works independently of Code Mode and supports native application and browser surfaces.
- **Application kernel:** `crates/plugin-core` owns checked compositions, binding identity and lifecycle rules. `crates/ui` adds mounted components, accepted actions, native capture and deployment reconciliation without business page routing.
- **Presentation:** `crates/tui` provides native terminal layout, input, editors, scrolling and selection. `crates/ui-gpui` and `crates/desktop` provide GPUI Fast/Kit presentation. `packages/agent-product` supplies the ordinary agent workspace, navigation, themes and settings as replaceable plugins.

A plugin may provide separate terminal and desktop entries, or support one target. Dynamic JavaScript packages define new layouts and workflows; Rust native implementations are supplied with the executable. Installation stores immutable package bytes without changing activation or permissions. Replacing the launch root, configuration or bindings is an explicit checked source update. Native owners retain editing and reading state; plugins hold workflow state and observations, rather than copies of native buffers or pixel positions.

Launch a committed terminal product with `maka application run --root PATH --deployment UUID --connections FILE`, or a desktop source with `maka-desktop run`. Connections carry explicit method and recovery grants. No-command CLI shows help and acquires no State Root. The retired business TUI and its automatic agent startup are removed; the canonical client protocol remains available to CLI/ACP and Host business services.

The plugin library discovers linked definitions independently of active instances. Each plugin owns its localizable description; the Host publishes its registered defaults. Adding a linked plugin applies those defaults through the existing revision-checked composition operation. Removing an instance preserves discovery and plugin data; installed package files remain a separate resource.

The npm package in `packages/cli` only chooses and launches a matching executable. It does not add a second runtime, configuration store or tool implementation.

See the owning crate READMEs for detailed contracts, and [the SDK](packages/plugin-sdk/README.md) for JavaScript plugin authoring.
