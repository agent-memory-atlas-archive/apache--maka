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

Five crates are copied from [GPUI Fast](https://github.com/longbridge/gpui-fast/tree/e4aabe752665baabe330464892cfe6882ed3283b), revision `e4aabe752665baabe330464892cfe6882ed3283b`: `gpui`, `gpui_macos`, `gpui_windows`, `gpui_linux`, `scheduler`. Other helpers remain dependencies from that exact official Git revision. Root Cargo source patches give Kit and every backend the same core types. This is not a replacement UI framework.

`upstream.json` records the finite original file inventory and SHA-256 hashes before changes. Original license texts and upstream attribution are retained. New Maka source is not included in this inventory. Workspace dependencies are normalized for the isolated five-crate workspace, with explicit upstream license-file paths.

The patch adds a one-shot platform cleanup receiver. Success requires native destruction acknowledgement and release of all backend-owned renderer/input/callback resources. macOS extends a final state guard across both NSWindow/NSView superclass destructors. Windows revokes OLE drag ownership during WM_DESTROY and waits for WM_NCDESTROY/DestroyWindow completion. X11 uses checked DestroyWindow and drops mapped state/callback owners. Wayland uses a distinct display-sync callback after destroy requests; it survives surface map removal. The test backend waits for actual state destruction. Other unsupported backends return an explicit error. No frame scheduler or editor refresh changes are made.

The receipt does not promise that a GPU driver has reclaimed physical memory. It does not retain a window; the application's closing owner remains alive until this receipt and its own real resource leases settle. Keep this patch bounded and re-evaluate it against upstream before changing the pin.

Upstream example executables/assets are omitted except the JPEG and two regular fonts used by core tests; automatic example targets are disabled. The unmodified IBM Plex Sans and Lilex test fonts retain their adjacent OFL 1.1 license texts and finite original hashes in `upstream.json`; production does not embed these `cfg(test)` fixtures. This keeps unrelated demonstration binaries out of the source patch.

macOS teardown also removes the AccessKit adapter from state and releases it in the foreground teardown task before native close. The adapter retains the content view, whose GPUI child owns a state Arc; leaving the adapter until final state destruction forms a cycle and prevents any honest completion receipt.

The embedding can install one quit admission gate, separate from timed shutdown observers. `App::quit` consults it before stopping any event loop. macOS consults the same gate through `applicationShouldTerminate:` and returns NSTerminateCancel until actual embedding cleanup completes. Windows checks native WM_QUIT while GetMessage can still process cleanup, instead of ignoring on_quit after the loop has stopped. Forced OS termination or a lost message/display connection is not a clean receipt; applications retain their independent foreground execution owner after a platform return. Linux's normal stop remains protected at App::quit; its callback after a lost client loop is not claimed to restore that loop.

Semantic placement claims live only in prepaint, so layout measurements may build the same component repeatedly. The per-frame ordered trace and derived uniqueness set also replay in ordinary retained reuse, spliced prepaint segments and composited layer/list rows. Prepaint range offsets include claims; `Window::transact` discards claims when an attempt returns Err. Paint reuse does not register twice. A duplicate successful placement is a fatal trusted native invariant violation, not an isolation/recovery promise.

Native scheduler timers now directly own async-io's timer future, with cancellation propagated by Drop. The test scheduler unregisters an exact reservation id on timer cancellation, with sender destruction outside its queue lock. The browser backend retains its existing scheduling path and is not claimed to support this native cancellation guarantee. The adapted async-io reactor wakes on timer removal; cancellation is a request, and real Waker release follows reactor drain. Kit retains only one owned idle task per scrollbar, using Weak state in its future. No new timer registry or dispatcher authority is added.


Accessibility presentation can mark disabled/read-only on the actual AccessKit node. A facade and its native child may track the same FocusId; TabStopMap retains the full insertion log for replay but removes the previous traversal entry before registering that id again. This preserves one native focus owner and prevents duplicate Tab stops. Candidate admission also reads the existing FocusMap through current handles, so a revoked stop is skipped before the next paint; painted order and live admission remain distinct. The test backend now records the actual platform accessibility callbacks and latest delivered TreeUpdate, and test-context helpers deliver activation/actions through the normal asynchronous channels. These are headless behavioral probes, not OS screen-reader or IME acceptance.

ScrollHandle exposes `invalidate()` for custom scroll adapters that change a deferred request outside its own offset/active-item fields. It increments the existing handle version recorded by `track_scroll`; it neither invents movement nor schedules a platform frame. Callers retain the normal view-notification/draw responsibility. Kit VirtualList uses this same mechanical owner to make a queued request visible to retained dependency checks.

Tracked Div scrolling resolves a requested active item after publishing this prepaint's overflow, bounds and maximum offset, and before child prepaint consumes the offset. Previously the request used old geometry (or initial Visible overflow) and could be consumed without revealing its target. This preserves the original handle, request and bottom-scroll precedence; it does not add animation scheduling or a second scroll owner. Maka's headless native regression checks first prepaint and one resized prepaint explicitly.

The test context can pause automatic test-mode drawing within a scoped callback while real accessibility action channels, effects and observers continue. Explicit drawing still works; an RAII guard restores the original automatic-drawing setting, including nested calls and unwind. This lets regression tests retain and assert the actual old platform tree/listeners before delivering an asynchronous action against changed live native facts. Production drawing is unchanged. The helper neither suppresses action delivery nor establishes OS accessibility acceptance.

App exposes a read-only query of the original current event propagation flag. Native post-dispatch observers can reject already consumed inputs and earlier stopped observations without copying event state or adding a keyboard registry. This supports the existing Kit Root defaults for both focused and unfocused windows; native bindings and raw plugin decisions still run first.

Native focus reveal is derived from the original Window focus generation and committed prepaint geometry. Focused subtrees mark the original autoscroll request with its generation; exact child caret requests take precedence over whole-control fallback, and unrelated sibling requests cannot replace the focus target. Scroll containers plan minimal clamped native offsets and propagate bounds clipped only to their own viewport. A normal subsequent frame rebuilds paint, hitboxes and accessibility together; serving a generation never forces later manual scrolling back. Pointer-driven focus preserves down/up and selection geometry. List retries choose one request after prepainting the full visible pass; failed transactions discard reveal geometry facts while preserving actual offset plans. Pending reveal bypasses retained/layer replay.


The original TestWindow records platform decoration facts, resize requests and client insets. TestAppContext can change those facts and request a redraw, then observe the actual platform calls. This bounded native harness extension verifies Base Root client-frame behavior without a second window/frame owner, and is not Linux OS acceptance.


Native Hitbox accepts optional exact regions relative to its original bounds. One native interaction id and dispatch scope covers disjoint glyph fragments; bounds/mask translation preserves those regions in retained layers. Empty regions never hit, and tooltip prepaint reads the same masked geometry without claiming occlusion. The weak ClickHandle cancels the original element's pending press and active styling; it does not create a gesture owner. Actual mouse/key presses retain button/key, focus generation, original Window input sequence and cancellation mark. Every physical down/up advances the sequence before capture/bindings; replay masks its scope and cannot arm. A scoped optional current serial distinguishes outside dispatch from replay and restores nested context, while nested inputs still interrupt an outer clean press. The original Window inactive callback invalidates admission in constant time before observers, then its normal refreshed layout cleans pending visual state; no per-control activation subscription is added. Moves discard stale releases before drag and hover decisions. Dirty layout cannot consume an in-flight release; nested input preserves its outer committed listener frame and lets the existing scheduled redraw handle dirtiness. Native assistive-technology actions enter the same causal input scope, interrupt other targets' pending or captured presses, and start fresh propagation/default flags, and default Div click activation goes directly through the original click listeners as Accessibility rather than fabricated centre-pointer input; an explicit AX Click listener wins. These generic prerequisites do not yet integrate TextView occurrence identity, exact original glyph fragments, or product Markdown links, and headless tests do not establish OS accessibility acceptance.

A custom canvas may install the original focused IME handler without tracked reveal geometry. The committed frame now completes that focus generation when the handler exists and its focus has no dispatch-tree geometry, so retained reuse can resume. The original focus/generation and scroll guards remain; an unmounted focus without an input handler stays pending. The dispatcher cancellation test now queues an actual `dispatch_after` runnable owning a completion sender and retains an executor waiting for that sender's cancellation. Owned async-io timers remain governed by their existing Drop contract.
