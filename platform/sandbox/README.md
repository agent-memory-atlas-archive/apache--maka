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

# maka-sandbox

[简体中文](README.zh-CN.md)

Prepares authorized commands for operating-system isolation. The Host decides permission and owns execution records; this crate applies filesystem and network restrictions. Process I/O and lifetime belong to [maka-process](../process/README.md).

macOS uses Seatbelt, Linux uses bubblewrap and network namespaces, and Windows uses provisioned low-privilege accounts, WFP rules and Jobs. Linux requires bubblewrap 0.11 or newer at `/usr/bin/bwrap`. Unsupported policies fail before execution. Windows setup is explicit through `maka sandbox setup --root PATH`; the Host remains unelevated.

Destination-restricted traffic passes through an execution-owned gateway. Routing and credentials stay with the Host, and redirects need their own destination permission. Linux and Windows expand deny globs before launch; future matching paths are covered on a subsequent launch, not by that existing snapshot.

To exercise a policy with the production backend:

```sh
maka sandbox run --policy policy.json --cwd /absolute/workspace --command 'your command'
```

On Windows, also pass the configured `--root`. A read-only policy with no network is:

```json
{"kind":"managed","filesystem":{"default":"read","rules":[],"denyGlobs":[]},"network":"denied"}
```

Use `maka sandbox --help` for setup, status and removal. These controls constrain model-directed effects; trusted native plugin code runs in the host process.

## Source provenance
`src/seatbelt/{base,network}.sbpl` are copied unchanged from Codex's
[base policy](https://github.com/openai/codex/blob/d1e3f9dfe3d5f6105b7e4c3958b9bf2ac6c32818/codex-rs/sandboxing/src/seatbelt_base_policy.sbpl)
and [network policy](https://github.com/openai/codex/blob/d1e3f9dfe3d5f6105b7e4c3958b9bf2ac6c32818/codex-rs/sandboxing/src/seatbelt_network_policy.sbpl),
revision `d1e3f9dfe3d5f6105b7e4c3958b9bf2ac6c32818`.
Copyright OpenAI; Apache-2.0. The upstream source comments are retained.
