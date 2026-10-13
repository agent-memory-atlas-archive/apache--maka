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

# Product bundle

[简体中文](README.zh-CN.md)

This directory selects the packages shipped with Maka and binds them into Agent, WorkHub or Studio compositions. Plugin declarations and implementations live under [plugins](../../../plugins/README.md).

- [package-inputs.json](package-inputs.json) selects authored packages.
- [src/composition.ts](src/composition.ts) builds instances, bindings and installation identities.
- [scripts/build.mjs](scripts/build.mjs) emits package bytes and authoring code into `dist`.

## Run and develop

From the repository root:

```sh
just run
just run --root /tmp/maka-workhub --product workhub
```

The first launch confirms the workspace and installs the selected terminal product. Later launches reopen its saved configuration. The default UI Root is `~/.maka/application`, with its Host Root under `host`. Settings and Plugins edit the installed application; startup options do not overwrite an existing installation.

`just product` rebuilds the TypeScript bundle. Development binaries load `dist`, or the directory named by `MAKA_PRODUCT_BUNDLE`; TypeScript changes do not require Rust recompilation. Release binaries embed the bundle. Use a separate output directory when building an artifact alongside a running development build:

```sh
node products/maka/bundle/scripts/build.mjs --output /tmp/maka-product-bundle
```

## Terminal interaction

Tasks can be created and drafted before a model is configured. Sending without a model opens connection setup; completing setup returns to the original draft and model picker. Providers are grouped by service, with variants and advanced configuration disclosed when needed.

The task sidebar stays alongside the workspace. Drag its divider to resize it, or focus the divider with Tab and use Left/Right; Home restores the default width. Details and running terminals open in a side panel. Narrow workspaces show the active panel in place of the conversation; closing it restores the conversation and draft. Esc dismisses the nearest temporary surface first.

Model, connection, project and task lists load or reveal further rows as they scroll. Rendered windows remain bounded, and overlapping rows keep the viewport anchored when the window moves. Scrolling does not take keyboard focus; arrows and Enter provide the corresponding list actions. Approvals and questions appear with the current task, while operation history remains available for recovery.

## Author an installation

The CLI can produce checked sources and packages without Node at runtime:

```sh
maka application compose --configuration installation.json --output product-source
```

The output directory must be new. The command writes Host, management and UI sources, connection grants, resolved installation identities and `.maka-plugin` packages. It does not start or install them.

`installation.json` selects `presentation` (`tui` or `desktop`), `locale`, `scheme`, `hostRoot` and service configuration. Agent uses the generated `AgentConfiguration`; WorkHub adds a separate `coordinator`. Optional services and ACP executors are selected explicitly. See [the configuration parser](src/composition.ts) and [SDK types](../../../packages/plugin-sdk/src/generated/runtime.ts) for accepted fields.

Preserve the emitted `keys`, `hostDeployment` and `managementDeployment` when reauthoring an installation. They identify existing owners and data. Package revisions identify bytes and can change independently.

For a terminal installation, `maka application launch --root UI_ROOT --configuration installation.json` initializes and starts it. Reopen with only `--root UI_ROOT`; interrupted initialization resumes its original preparation.

For explicit installation, initialize Host and UI Roots with `maka host init`, then commit `host.json`, `management-host.json` and `ui.json` using `maka application commit`. The management source belongs to the UI Root. Supply the selected deployment IDs, expected revisions, exported packages and, for UI, `remote-exports.json`. Run the UI with `maka application run` or `maka desktop`, passing its Root, deployment and generated `connections.json`. Each command's `--help` lists the required arguments.

## Verify changes

```sh
npm --workspace @maka-agent/product run typecheck
npm --workspace @maka-agent/product test
MAKA_PRODUCT_CLI_CHECK=1 npm --workspace @maka-agent/product test
```

Tests build once, then exercise the retained component behavior. Setting `MAKA_PRODUCT_CLI_CHECK` also checks packages and compositions against the actual CLI. Native rendering, focus and input changes need verification in their intended presenter.
