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
