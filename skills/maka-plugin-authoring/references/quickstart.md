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

# A checked application package

The starter contains an Echo Host service and independent TUI/Desktop counters.
The authoring helper reads native Surface contracts from the actual CLI, writes a
new directory, obtains package digests from `inspect-package`, and checks the graph.
It creates no State Root, commits no deployment and starts no execution.

Copy `assets/starter/create.mjs`, `host.mjs`, `tui.mjs` and `desktop.mjs` together to
an authoring directory. Then run:

```sh
node create.mjs --output /tmp/my-plugin --target tui --maka /path/to/maka
```

`--target host` creates a service graph; `--target desktop` creates the Desktop
counter graph. The output directory must not already exist. Use the emitted
`input` and `packages` paths in subsequent CLI commands. Edit the generated files
for the requested product, then obtain new package references after each byte change.
Retain existing instance keys when updating an installed graph.

The generated `plugin/maka.extension.json` has `schemaVersion: 2` and
`application.entries`. Entries select `host`, `tui` or `desktop`, declare configuration
schemas and full import/export contracts, and choose a bundled JavaScript module
with `realm: "dedicated"`. Component entries additionally declare the native scene
language. The package exports named `activate` functions; service/component paths
refer to exports in the same module.

`composition.json` selects exact package references through
`{ source: "pinned", package: { id, revision } }`, stable instance UUIDs,
`usePackage: "entry"`, explicit bindings and the UI launch export. The helper
writes compositions outside the package so their digests cannot refer to themselves.
A package may instead publish a composition template; use the generated
`PackageDeclaration` and `CompositionInput` types for that larger product graph.

For TypeScript, author against the SDK root import and bundle the entry with
`packages/plugin-sdk/scripts/bundle.mjs`. Runtime bundles contain their dependencies,
with no unresolved imports or top-level await. Native implementations are supplied
by the executable, rather than loaded from JavaScript package bytes.

See [Testing and shipping](testing-and-shipping.md) before installing or running it.
