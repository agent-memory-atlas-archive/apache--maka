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


Native Hitbox accepts optional exact regions relative to its original bounds. One native interaction id and dispatch scope covers disjoint glyph fragments; bounds/mask translation preserves those regions in retained layers. Empty regions never hit, and tooltip prepaint reads the same masked geometry without claiming occlusion. The weak ClickHandle cancels the original element's pending press and active styling; it does not create a gesture owner. Actual mouse/key presses retain button/key, focus generation, original Window input sequence and cancellation mark. Every physical down/up advances the sequence before capture/bindings; replay masks its scope and cannot arm. A scoped optional current serial distinguishes outside dispatch from replay and restores nested context, while nested inputs still interrupt an outer clean press. The original Window inactive callback invalidates admission in constant time before observers, then its normal refreshed layout cleans pending visual state; no per-control activation subscription is added. Moves discard stale releases before drag and hover decisions. Dirty layout cannot consume an in-flight release; nested input preserves its outer committed listener frame and lets the existing scheduled redraw handle dirtiness. Native assistive-technology actions enter the same causal input scope, interrupt other targets' pending or captured presses, and start fresh propagation/default flags, and default Div click activation goes directly through the original click listeners as Accessibility rather than fabricated centre-pointer input; an explicit AX Click listener wins. Kit integrates TextView occurrence identity and original glyph fragments through these native seams. Headless tests do not establish product Markdown or OS accessibility acceptance.

A custom canvas may install the original focused IME handler without tracked reveal geometry. The committed frame now completes that focus generation when the handler exists and its focus has no dispatch-tree geometry, so retained reuse can resume. The original focus/generation and scroll guards remain; an unmounted focus without an input handler stays pending. The dispatcher cancellation test now queues an actual `dispatch_after` runnable owning a completion sender and retains an executor waiting for that sender's cancellation. Owned async-io timers remain governed by their existing Drop contract.


Window's original prepaint transaction checkpoints its active accessibility node builder, focus mappings and bounds, restoring those same facts when the pass fails. This includes open parent builders and their child ids, so an original List autoscroll retry cannot publish duplicate or stale nodes from its discarded pass. Inactive accessibility does not take the snapshot; original paint-stage action listeners and the single published tree remain unchanged. Kit's actual AX-active retry regression verifies corrected final native bounds and activation; it is not OS accessibility acceptance.

原 Window prepaint transaction 在可访问性 active 时临时保存原 node builder、focus 映射及 bounds，失败时恢复同一事实，包含尚未闭合的父 builder 与 child ids，避免原 List autoscroll retry 发布废弃轮次的重复或过时节点。可访问性 inactive 不建立快照；原 paint 阶段 action listeners 与唯一发布树保持。Kit 的真实 AX-active retry 回归验证最终纠正的原生 bounds 与激活，不替代 OS 可访问性验收。


`Entity::query` exposes the existing native platform-query classification for derived frame reports: it keeps the original entity lease and read dependency, while observable changes still require the original notification. The original queried-owner scope now restores its outer classification before rethrowing an unwind; this does not introduce entity-lease rollback. A native regression catches a same-owner double-lease failure inside a valid ordinary update, then verifies later source writes remain visible.

Window exposes read-only slices of its original prepainted and completed hitboxes, and the original focus handle's insertion position in the completed dispatch tree. Kit derives retained selection geometry, touch-menu survival and document order from those native frame facts. It does not allocate a second hitbox, focus tree or frame owner.

`Entity::query` 公开原平台查询的派生报告分类，保留原 entity lease 与读取依赖；可观察变化仍须走原通知。queried-owner scope 在 unwind 复抛前恢复外层分类，不新增 entity lease rollback。原生回归在有效的普通 update 内捕获同 owner 双借用失败，再验证后续 source 写入仍可见。

Window 只读公开原 prepainted/completed hitboxes 及原 focus handle 在完成 dispatch tree 中的插入位置。Kit 据此投影 retained 选区几何、维护触控菜单存活与文档顺序，不新建 hitbox、focus tree 或 frame owner。
