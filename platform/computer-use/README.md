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

# Computer Use

[简体中文](README.zh-CN.md)

Provides Maka's native app and browser API through an independent JavaScript REPL. `cua_repl` and `cua_reset` work with Code Mode disabled. The pinned Cua driver is selected in the [workspace manifest](../../Cargo.toml); the object API is documented in [src/api.md](src/api.md).

A Session owns its REPL variables, target bindings and observations. The Host owns permission checks and the native driver. Each operation uses current authority; keeping a JavaScript object does not keep a grant. The REPL has no ambient Node, filesystem or arbitrary network API.

For a fresh REPL, select an entrypoint and read its emitted state before acting:

```js
const app = await cua.getApp("Calculator");
```

Subsequent calls use indices from that observation. `cua.getState()` lists available surfaces. App and tab observations emit automatically. The bundled [maka-cua skill](skills/maka-cua/SKILL.md) explains target selection, screenshots and input.

## Desktop and browser configuration

The `desktop` setting defaults to `auto`: the Windows host in WSL, otherwise the local desktop. `local` selects Linux inside WSL; `windows` requires the Windows helper. Native releases prepare their matching Windows helper through the verified distribution cache. WSL interoperability and an interactive Windows desktop are required.

Browser providers are explicitly configured CDP connections to existing profiles. When no provider is configured, use a native browser window. Configuration and platform-specific methods are described by the API reference; a missing provider does not trigger a browser restart or enable debugging.

## Lifetime

Reset clears this Session's bindings without closing user apps or tabs. Navigation, window changes and fresh observations can invalidate previous input targets. Cancellation or timeout requires a reset; an uncertain action is never replayed automatically. Shutdown waits for accepted native work to settle.

Native dependencies require Xcode Command Line Tools on macOS and X11 development libraries on Linux. Verify input and capture behavior on the target desktop; a REPL unit test cannot establish native focus or accessibility behavior.
