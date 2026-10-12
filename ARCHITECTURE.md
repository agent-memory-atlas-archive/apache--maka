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

Maka assembles applications from plugins. A package declares what it provides and what it needs. A composition chooses instances and connects their imports to exports. Host services perform work; UI components present it through native terminal or desktop renderers.

## Code ownership

[Platform](platform/README.md) contains the package loader, composition kernel, execution owners and renderers. [Plugins](plugins/README.md) contain business features and their declarations. [The Maka product](products/maka/README.md) selects packages, native factories and startup behavior.

A `package.ts` identifies an authored plugin. A Cargo crate identifies code compiled together. They need not correspond one-to-one: an Agent service can use several domain crates, while related plugin entries can share one native implementation crate.

The Host and UI owners use the same lifecycle kernel. The `host-js` and `ui-js` adapters execute JavaScript plugins; native renderers and owners can compile without V8 or the Agent engine. Shared fact types remain explicit dependencies where persistence or presentation needs them.

## From package to running application

1. The package loader validates immutable bytes and computes their reference.
2. The composition planner checks entry selection, configuration and bindings.
3. A Root stores the desired source and its package bytes at a revision.
4. Host or UI activates the selected implementations. The source becomes effective when its instances converge.

Installation does not grant permissions. A binding makes a compatible provider available to a consumer; the Host checks the caller, method, input and actual resources when admitting each call. Local IPC connections carry explicit method and recovery grants supplied by the launcher.

Updating a source preserves unchanged activations and their captured bindings. Changed instances retire before replacement. Already accepted work remains with its original owner, including when the caller disconnects or a component disappears.

## State and effects

The EventLog owns the durable Root and canonical records. Stable plugin data belongs to a Root, deployment and instance. Package revisions and activation epochs describe code and execution; neither creates a new data namespace by itself.

Mutations have receipts. Business completion and physical cleanup are recorded separately. If a reply is lost, the caller resolves the original receipt instead of issuing a replacement operation. Cancellation asks work to stop; resources remain owned until their actual cleanup completes.

UI callbacks propose a state/frame update and actions. A validated batch commits atomically. Renderers read committed frames; they do not execute plugin code during paint. Native editors own buffers, selection and IME, while plugins use bounded captures and commands.

## Development and distribution

`just run` bundles TypeScript before starting Cargo. Development binaries read that bundle from disk; release binaries embed it. Both use the same package validation and commit path. Rebuilding code does not replace an existing installation's saved composition.

Rust contracts generate the [JavaScript SDK](packages/plugin-sdk/README.md) through [maka-schema](tools/schema/README.md), without building the product. The npm package is a [native executable launcher](packages/cli/README.md).
