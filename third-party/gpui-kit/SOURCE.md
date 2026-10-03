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

# GPUI Kit local presentation projection

The `gpui-base` crate is copied from [GPUI Kit](https://github.com/longbridge/gpui-kit/tree/f89c5cc9576d823d253b4563806e5f842e9d69f9), revision `f89c5cc9576d823d253b4563806e5f842e9d69f9`, under Apache-2.0. Other Kit crates remain dependencies from that exact revision. `upstream.json` records the finite imported file inventory and original SHA-256 hashes before modifications; licenses are retained byte-for-byte. The root README is retained because upstream library tests include it. Examples, benchmarks and website sources are omitted. Benchmark-only dependencies and GPUI profiler features are omitted; library tests keep their original test support. The crate manifest is normalized to explicit upstream dependencies/lints and disables automatic example targets; this repository includes the adapted Base crate as a workspace member so its actual unit tests can run with the same source patches as the application.

The bounded changes preserve the single native editing engine:

- `InputBaseState` distinguishes inherited editor styles from complete projections. Complete styles preserve explicit transparent colours without evaluating an App Global.
- `Scrollbar` accepts an immutable local `Rc<Theme>` and reads that same snapshot for its cascade, default foreground, mode, geometry, motion and radius. `EditorScrollbar` forwards its state owner's selected snapshot to the actual internal scrollbar. System reduced-motion policy still wins.
- Existing scrollbar interaction state re-arms its idle deadline after timing changes; obsolete timer generations cannot clear a newer deadline or notify its view. Hover, drag and scroll offset remain owned by the original state.

Idle waits are owned tasks; replacement or state release cancels the old wait, and the future retains only Weak state. The shared native scheduler and async-io removal wake are adapted separately so cancellation also drains backend timer custody. Added tests cover local projection, retained draws, built-in editor scrollbars and actual cancellation. Two imported tests explicitly settle the initial geometry frame or await the existing 120 Hz drag notification before checking retained painting. Their original forms fail identically with the pinned Kit source under this repository's Fast backend; the idle cache and painted grab-point assertions remain intact, and painting must not change the already updated drag offset.

Imported dock JSON fixtures are formatted to the repository's existing formatter policy. Their data, string values and numeric tokens are preserved; provenance hashes still identify their original upstream bytes.

The theme snapshot is a derived presentation cache, not an application registry or a writable source of plugin configuration. Application adapters must build it from their selected declarative values without entities, resource leases or custom animation closures, project body text colour/font/size/line-height explicitly, and preserve the existing EditorState identity. This patch alone does not implement Maka's public theme contract, live reconciliation, product UI, accessibility or OS IME acceptance.


InputBaseState can update tab index/stop metadata on its existing FocusId, notifying only when that metadata changes. This does not recreate its focus or editing identity. Maka's plain multiline adapter uses TextareaState; the source-code EditorState remains a separate upstream mode, with no extra code-editor protocol implied by this seam.

Two imported dock renderer-contract tests explicitly request a fresh draw before checking the original hook-order assertions. Their helper already settles and then clears the call record; a normal retained draw can legally skip those hooks. This adjustment does not change production retention or weaken the composition assertions, and no original-HEAD baseline is claimed for that discovery.

VirtualList invalidates its original base scroll handle when queuing its existing deferred scroll request. The offset changes only when prepaint consumes the request. Axis tests keep their original range/offset assertions and additionally verify that admission does not move the offset, and that idle drawing still reuses retained rendering. No second scroll state or invalidation registry is introduced.

Seven imported animation/deferred-layout tests explicitly deliver GPUI's existing next-frame callbacks before drawing, matching the platform's callback-before-draw sequence. Merely advancing the test clock or drawing does not deliver those callbacks. The TextView selection fixture includes the actual TextSelectionLayer, which installs its selection listeners; its double/triple-click and clipping assertions remain intact. These test corrections and the VirtualList fix address failures reproduced at the pre-slice local HEAD, and do not claim OS accessibility, IME or product acceptance.
