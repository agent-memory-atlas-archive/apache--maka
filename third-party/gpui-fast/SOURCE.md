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

# GPUI Fast platform cleanup patch

Four crates are copied from [GPUI Fast](https://github.com/longbridge/gpui-fast/tree/e4aabe752665baabe330464892cfe6882ed3283b), revision `e4aabe752665baabe330464892cfe6882ed3283b`: `gpui`, `gpui_macos`, `gpui_windows`, `gpui_linux`. Other helpers remain dependencies from that exact official Git revision. Root Cargo source patches give Kit and every backend the same core types. This is not a replacement UI framework.

`upstream.json` records the finite original file inventory and SHA-256 hashes before changes. Original license texts and upstream attribution are retained. New Maka source is not included in this inventory. Workspace dependencies are normalized for the isolated four-crate workspace, with explicit upstream license-file paths.

The patch adds a one-shot platform cleanup receiver. Success requires native destruction acknowledgement and release of all backend-owned renderer/input/callback resources. macOS extends a final state guard across both NSWindow/NSView superclass destructors. Windows revokes OLE drag ownership during WM_DESTROY and waits for WM_NCDESTROY/DestroyWindow completion. X11 uses checked DestroyWindow and drops mapped state/callback owners. Wayland uses a distinct display-sync callback after destroy requests; it survives surface map removal. The test backend waits for actual state destruction. Other unsupported backends return an explicit error. No frame scheduler or editor refresh changes are made.

The receipt does not promise that a GPU driver has reclaimed physical memory. It does not retain a window; the application's closing owner remains alive until this receipt and its own real resource leases settle. Keep this patch bounded and re-evaluate it against upstream before changing the pin.

Upstream example executables/assets are omitted except the JPEG used by core tests; automatic example targets are disabled. This keeps unrelated demonstration binaries out of the source patch.

macOS teardown also removes the AccessKit adapter from state and releases it in the foreground teardown task before native close. The adapter retains the content view, whose GPUI child owns a state Arc; leaving the adapter until final state destruction forms a cycle and prevents any honest completion receipt.

The embedding can install one quit admission gate, separate from timed shutdown observers. `App::quit` consults it before stopping any event loop. macOS consults the same gate through `applicationShouldTerminate:` and returns NSTerminateCancel until actual embedding cleanup completes. Windows checks native WM_QUIT while GetMessage can still process cleanup, instead of ignoring on_quit after the loop has stopped. Forced OS termination or a lost message/display connection is not a clean receipt; applications retain their independent foreground execution owner after a platform return. Linux's normal stop remains protected at App::quit; its callback after a lost client loop is not claimed to restore that loop.
