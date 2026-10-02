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

# Plugin core

Shared declarations and composition planning for Host, TUI and Desktop. This crate
has no dependency on Host business plugins, a JavaScript engine or a renderer.

`composition::Input` is the editable deployment input. `plan::Plan::prepare` expands
immutable package templates, applies instance-owned overrides and checks explicit
capability bindings before a caller commits new input. A plan is a projection,
not another configuration store. Stable source keys are independent of aliases;
package revisions pin exact bytes. State schema compatibility and effect
authorization are separate runtime checks.

Contract declarations derive their JSON schemas through `schemars`. Entry export
batches must pass `EntryDeclaration::check_exports` before publication. Current
compatibility is conservative: nominal name, version and full shape must match.
Remote declarations prove compatibility only, never readiness or authority.

Application packages use manifest schema version 2 and an `application` declaration.
The CLI checks them without activating a deployment:

```sh
maka application check --input deployment.json --package /absolute/package/path
```

The checker does not execute JavaScript or create native UI resources.
