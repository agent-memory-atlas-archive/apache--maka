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

# GPUI component adapter

Public native presentation ABI for Longbridge GPUI Fast and Kit. The shared
`maka-ui` kernel supplies registrations, component transactions and mount identity;
this library supplies immutable `Widget` recipes, native input bindings, scoped
`Slots` and optional compiled Label/Editor factories. It owns no application,
window, Host, product navigation or second mount registry.

Implement `NativeComponent<dyn Widget>` and register its actual contract in a
`NativeRegistry`. Return `NativeView::new(Rc::new(recipe), input_schemas, slots)`.
The product package declares imports; only its selected composition activates the
compiled implementation. `registry()` makes the built-in Label/Editor available
and may be extended before the trusted launcher receives it.

`registry()` also installs this platform's pure `scene::compile` under
`scene::language()` (`maka.desktop.scene`, version 1). Dynamic component entries
must declare that exact scene language. Dynamic JSON
scenes provide native flex layout, scoped typography/colors, pointer and keyboard
buttons, focus styling and vertical scrolling. Imported components remain slots
of the original parent. Local keys retain the same mechanical focus/scroll owners
across layout changes; removing or changing their kind closes old input admission.
The compiler validates bounded shapes, styles and numeric values against the
original JSON before narrowing to native floats, within the callback deadline.


`Canvas` adds bounded Rect/Line/Text commands and explicit button-like regions
under `contents`. Plugins supply drawing and synchronous pointer/key/drop rules;
native owners supply focus, accessibility activation and optional drag outlines.
Paint performs no plugin work. Down/AX use the original painted input; continuing
gestures and early KeyDown interception use the same owner's latest dispatcher.
Unsubscribed moves stay native. Changing a region's admission or interaction route,
removing it, losing focus or deactivating its window cancels the original gesture.
Coordinates use f64 bounds before native conversion; pointer positions follow the
last painted canvas and drag deltas remain in window coordinates. Rectangles are
RGBA; line widths and single-line text are bounded. Try the ordinary dynamic
[workflow example](../ui/examples/counter/canvas.mjs) with the `canvas` export mode.

`Input` dispatch returns a committed `InputDecision`. `bind` applies default prevention and propagation only during its current native listener; rejected batches prevent defaults and stop propagation. It cannot reverse earlier native actions or mouse-down focus. Owner observations call the dispatcher without applying those flags to an already completed event. Desktop synchronizes native effects after kernel commit; a native synchronization failure retains the committed decision and is diagnosed separately.

`Widget::create` returns renderer-owned resource state; `capture` takes bounded
request-local snapshots and `commit` applies committed commands before rendering.
`deactivate` releases native focus/input on logical retirement, before old paints
finish destruction. `render` reads that owner, receives its exact committed child
`Slots` and returns a result so placement failures reach the presenter. Recipes
must not keep Entity, Input, Slots, subscriptions or resource leases. Retained
Elements and virtual-list callbacks may clone `Slots`, holding real custody for
only the original subtree and window. Removed or recreated owners reject old
handles. Captured recipes and Slots cannot render against a replaced frame revision. Multiple layout measurements are valid; one successful display may place
one mount once. Native invariant violations are trusted-code failures, without a
sandbox or fake cleanup receipt.

See [desktop ownership](../desktop/README.md) and [shared kernel](../ui/README.md).

The fixed `TextInput` and `SecretInput` exports from `maka.ui.input` use the
shared `Editor*` command, observation and capture protocol. Single-line props
supply complete `InputAppearance`, label, placeholder, editability and tab order.
Both controls start empty; an initial metadata observation permits a revision
checked prefill command. Input rejects control characters as one complete edit,
before clipboard or IME normalization. Reset clears native undo history.

SecretInput stays masked for its lifetime. Its AccessKit password node has no
plaintext value or text runs, and platform surrounding-text queries return masks
in the original UTF-16 units. Captures retain the original immutable UTF-8 bytes;
masking does not remove an explicitly granted Reader capability. Normal settings
composition should bind secure capture to its trusted setter and omit Reader.
Native Enter is consumed before keybindings and emits `submitted` as a business
event, not a synchronous native decision. TUI emits this independently of its
retryable observation/receipt slot. OS input-method and screen-reader acceptance
requires separate platform verification.

A rejected metadata transaction stays pending in the original native owner.
The next application synchronization republishes it even when the component
frame is unchanged, without replaying an already executed command.

Retained presentation is an immutable projection of the original frame. A scene
Stack may set `visible: false`; native parents use `NativeView::with_hidden_slots`
with declared exact slot keys. All declared children remain mounted. Hidden
children measure zero and publish no focus stops or input targets; scene stacks
exclude them from layout gaps. Their original editor buffers, selections, undo
history, capture capabilities and command receipts remain owned and can settle. Showing restores current props on the same owner.
Desktop invalidates mechanical admission before the next paint and rejects retained
child renders; TUI applies geometry admission only after a successful flush.
Custom native layouts use the same public Slots boundary and cannot paint a hidden
child. This projection does not retire resources or create a second writable draft.
