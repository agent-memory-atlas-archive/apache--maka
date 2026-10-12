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

Directories identify ownership, `package.ts` identifies a plugin, and Cargo crates identify compilation units. These boundaries need not be one-to-one. Family folders group related plugins; they do not grant capabilities or create runtime identities. Existing package IDs, entry IDs and persisted namespaces remain independent of source paths.

- **Platform:** `platform/plugin-core` checks composition and binding identity; `platform/packages` loads immutable package bytes; `platform/capabilities` owns scoped resources. `platform/event-log` is the single durable writer and performs startup recovery before activation, including facts left by plugins no longer installed.
- **Host and UI:** `platform/host-runtime` owns admission and service settlement. `platform/ui-runtime` owns mounted components, captured input, accepted actions and cleanup. Products inject `platform/host-js` and `platform/ui-js` when JavaScript is needed. Native Host/UI and the terminal/GPUI renderers compile without V8 or Agent execution; their persistence and public business-fact dependencies remain explicit.
- **Plugins:** leaf `plugins/<family>/<plugin>/package.ts` files declare their own entries and imports. Native implementations live beside their plugin or in a shared family crate. `plugins/agent/engine` owns Agent execution; `facts`, `ports`, `api` and `authority` separate durable facts, business ports, public service contracts and original-invocation admission. Web, Files, Processes, Skills and Interactions register independently of Agent.
- **Product:** `products/maka/native` selects compiled native factories. `products/maka/bundle/package-inputs.json` selects authored packages, and `src/composition.ts` binds them. CLI and desktop supply the execution backends and start the selected product. Factory availability alone neither activates a plugin nor grants access.
- **Authoring:** native definitions come from the actual Rust implementation identifiers, contracts and configuration schemas; `tools/schema` generates SDK facts without the product or V8. The generic plugin builder accepts one leaf or the product selection. Shared components under `plugins/shared` do not depend on a product.
- **Computer Use:** `platform/computer-use` owns its REPL and session lifetime independently of Code Mode.

A plugin may provide separate terminal and desktop entries, or support one target. Dynamic JavaScript packages define new layouts and workflows; Rust native implementations are supplied with the executable. Installation stores immutable package bytes without changing activation or permissions. Replacing the launch root, configuration or bindings is an explicit checked source update. Native owners retain editing and reading state; plugins hold workflow state and observations, rather than copies of native buffers or pixel positions.

Launch a committed terminal product with `maka application run --root PATH --deployment UUID --connections FILE`, or a desktop source with `maka-desktop run`. Connections carry explicit method and recovery grants. The no-command CLI starts the ordinary product described in `products/maka/bundle`; `--help` acquires no State Root. CLI/ACP and native business services use the same canonical client protocol.

The plugin library discovers linked definitions independently of active instances. Each plugin owns its localizable description; the Host publishes its registered defaults. Adding a linked plugin applies those defaults through the existing revision-checked composition operation. Removing an instance preserves discovery and plugin data; installed package files remain a separate resource.

The npm package in `packages/cli` only chooses and launches a matching executable. It does not add a second runtime, configuration store or tool implementation.

See the owning crate READMEs for detailed contracts, and [the SDK](packages/plugin-sdk/README.md) for JavaScript plugin authoring.

Development builds read `products/maka/bundle/dist` (or `MAKA_PRODUCT_BUNDLE`) through the same validated package loader and commit pipeline. `just run` rebuilds the TypeScript bundle before starting Cargo; TypeScript inputs do not invalidate Rust compilation. Release builds embed the bundle. Installing or updating packages in an existing Root remains an explicit source operation.
