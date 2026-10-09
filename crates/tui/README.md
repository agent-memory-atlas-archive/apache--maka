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

# Native terminal presentation

[简体中文](README.zh-CN.md)

`maka-tui` presents plugin application compositions through the shared `maka-ui`
component kernel. It contains terminal layout, input, focus, scrolling, selection,
bounded native editors and immutable resource capture. Product navigation, pages,
themes, local workflow state and business calls belong to ordinary plugins.

Run a checked committed source with
`maka application run --root PATH --deployment UUID [--connections FILE]`.
The source selects the launch component and all capability bindings; the terminal
adapter has no automatic agent startup or built-in business page registry.
[The ordinary agent product](../../packages/agent-product/README.md) and
[the dual-entry example](../ui/README.md) use the same kernel as Desktop.

Tab/Shift+Tab moves focus; Enter/Space or a left click activates buttons. Native
Unicode editors retain their text, selections and undo history across compatible
component updates and hidden routes. Product-supplied appearance controls their
styles. Terminal scenes support flexible viewports and native scroll policies;
SourceText/ReadingArea provide bounded literal selection and Copy.
The native `maka.ui.diff` implementation exports `Diff` (`maka.ui.Diff` v1).
Its tagged content supports annotated source ranges, bounded before/after line
comparison and structured create/update patches. Comparisons reuse the original
`similar` algorithm (8 KiB / 256 combined lines, 2 ms deadline); patch previews
reuse `apply-patch` validation and fall back to literal text for invalid or large
inputs. All variants reuse the original Diff layout and the same reading,
selection and find owner as SourceText/Markdown. Added/removed gutters are
presentation only and never enter copied or searched content. UTF-8 boundaries,
ordered disjoint ranges, language hints and source/row budgets are validated
before layout. This is a TUI capability; it has no filesystem access.


Ctrl+C retires the surface. The original guard checks every restoration operation
even after an earlier failure. Native workers, resource snapshots and accepted
service calls retain their original owners until actual cleanup completes. Failed
cleanup keeps the application fenced. `--connections` uses explicit
[Host method and recovery grants](../application-bridge/README.md); exit waits for
original service, caller SQL and Host close acknowledgements.

No legacy business TUI, locale/profile flags, theme store or whole-Host quit UI is
provided. Localization and themes belong to the selected product. The generic
application schema and native component contracts come from the Rust definitions
and are generated for the plugin SDK. Product feature and platform UX acceptance
are distinct from presenter tests.

Scene Stack containers can declare bounded `shortcuts` with a key, modifiers, input and enabled flag. Native editing handles keys first; the nearest matching painted scope owns an unconsumed chord, including disabled declarations. Inputs use the original mount/revision and current committed visibility. An unfocused root uses its last successfully presented declaration. `primary` means Control in the terminal; unmodified printable keys stay native.

Native editors can declare `autoFocus` to offer focus once when their committed structural route becomes visible and enabled. The successful flush consumes only offers whose original painted frame revisions and owner remain current, including unchanged children under changed ancestors. An unflushed or late old scene cannot consume a newer reveal; viewport clipping and stable repaint do not create another offer.

The native FilePicker supports local directory browsing as well as absolute file
paths. Use Browse/Parent, then arrows and Enter or a click to select. Home/End,
PageUp/PageDown, the wheel and scrollbar support bounded long directories; a
resize keeps the focused selection visible. Directory reads use the same native
commit/worker lifecycle as file capture; plugins receive only selected-file
metadata and immutable bytes.

The Composer also accepts a whole paste of absolute local file paths, including
quoted names and `file://localhost` URLs. One batch holds at most eight distinct
paths and reads one file at a time. Retry, Skip file and Cancel act on the original
batch; ordinary prose remains text. Enter cannot submit while a batch is pending.
Ctrl+V reads the local OS clipboard once, preferring files, then an image, then text.
Images are limited to 33,554,432 pixels and encoded directly into immutable PNG chunks;
no temporary image file is created. Text uses its original insertion position and
requires the same conversation and unchanged draft. Clipboard failures require a
new physical paste; Retry only rereads an original selected file. Attachment draft
restoration remains unfinished.

Conversation navigation retains up to 32 original native draft editors, including
selection and undo. Late captures and send acknowledgements still target their
original owner/revision. At capacity, only an observed empty, idle draft can be
reclaimed; otherwise send or clear an existing draft first. These drafts remain
local to the current Composer mount; restart restoration is not implemented.
