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

# Local application connections

The Desktop and TUI launchers accept `--connections FILE`. This trusted input
connects separate UI and Host deployments in one process. It does not edit either
composition. Cross-process authentication and transport are separate work.

```json
{
  "hosts": [{
    "name": "primary",
    "root": "/path/to/host/root",
    "deployment": "00000000-0000-0000-0000-000000000010",
    "consumer": ["00000000-0000-0000-0000-000000000013"],
    "imports": {
      "tasks": { "methods": ["read", "create"], "recovery": false },
      "receipts": { "methods": ["read"], "recovery": true }
    }
  }]
}
```

`name` is the alias selected by the UI's remote capability observations. `consumer`
is the existing Host consumer instance; `imports` names its explicit bindings.
The Host composition must select a gateway instance with implementation
`maka.application.gateway` and its imports, or another appropriate compiled
consumer. No factory or service is automatically activated. Other Rust factories
are supplied by the trusted launcher; dynamic JavaScript comes from committed
immutable package bytes.

Mutation resource snapshots require their executor protocol and are currently
rejected before preparation; streams and authenticated network transport remain
pending.

Each method must exist in the actual captured service contract. The endpoint
must match the selected UI deployment's observation. Startup captures that exact
`DeploymentRef`; a later desired-source update cannot inherit its approval.
Ordinary query/mutation permission does not grant original-receipt recovery.
A forwarded call to a different consumer fails closed under this configuration;
other trusted embeddings can supply a different explicit Host Policy and inert
`Scopes` factory through `LocalService`.

The configuration is closed JSON, bounded to 1 MiB, 16 Hosts, 128 imports per Host
and 64 methods per import. Omission means no local connections. Relative Root
paths resolve against the launcher's working directory.

After real surface and accepted-work cleanup, the launchers await both the caller
SQL owner's actual close acknowledgement and Host cleanup. Failure retains the
store, Root and runtime. Native quit is subject to the same gate. `drain_store`
and `Connections::drain` preserve custody instead of treating owner drop as a
cleanup receipt. Local endpoints hold captured Client/Binding/Scope admission,
not Host deployment lifecycle ownership.

[中文](README.zh-CN.md)
