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

# Component execution

`maka-ui` activates checked UI entries from immutable packages and publishes actual
component registrations through `maka-plugin-core`. Foreground JavaScript executes
only during activation, mount initialization, props updates and input. Required
`mount`, `event` and `update` functions and their receiver are captured before
publication; later namespace mutation cannot replace a registered handler.
Renderers read committed
`Frame` values; successful, bounded, validated batches replace a frame atomically.
Each mount has independent state and a revision, so old view events cannot invoke
new handlers. A realm failure retires every registration and mount in that realm.

The renderer protocol contains stacks, text, buttons and imported components.
A component reference names an explicit binding and its selected member; stable
keys retain child mounts and their local state across parent props updates.
All changed parents and children share one deadline and commit together. A failed
child leaves every prior frame unchanged. Removed mounts retire until real native
cleanup is confirmed; uncommitted mounts have no native resources to acknowledge.
Window configuration is fixed for a root mount; embedded components cannot create
windows. Native editor/IME commands, action imports, outward component events
and recovery surfaces are still pending. `close` requires an actual
native cleanup acknowledgement; `retire` only stops new input. `shutdown` retains
owners while mounts/work/resources remain unconfirmed.

Generate a package-backed desktop example (the output directory must be new):

```sh
cargo run --locked -p maka-ui --example counter -- /tmp/maka-counter
cargo run --locked -p maka-desktop -- --input /tmp/maka-counter/input.json \
  --package /tmp/maka-counter/native.surface.maka-plugin \
  --package /tmp/maka-counter/dynamic.app.maka-plugin
```

Add `composed` after the output directory when generating the example to include
a separately packaged counter component, then pass its
`dynamic.counter.maka-plugin` as another `--package` to desktop.

The example's component contract uses the same Rust declaration types as runtime
validation. It does not depend on the previous plugin SDK or global service catalog.
