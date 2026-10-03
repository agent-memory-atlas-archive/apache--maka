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

`registry()` also installs this platform's pure `scene::compile`. Dynamic JSON
scenes provide native flex layout, scoped typography/colors, pointer and keyboard
buttons, focus styling and vertical scrolling. Imported components remain slots
of the original parent. Local keys retain the same mechanical focus/scroll owners
across layout changes; removing or changing their kind closes old input admission.
The compiler validates bounded shapes, styles and numeric values against the
original JSON before narrowing to native floats, within the callback deadline.

`Widget::create` returns renderer-owned resource state; `capture` takes bounded
request-local snapshots and `commit` applies committed commands before rendering.
`deactivate` releases native focus/input on logical retirement, before old paints
finish destruction. `render` reads that owner, receives its exact committed child
`Slots` and returns a result so placement failures reach the presenter. Recipes
must not keep Entity, Input, Slots, subscriptions or resource leases. Retained
Elements and virtual-list callbacks may clone `Slots`, holding real custody for
only the original subtree and window. Removed or recreated owners reject old
handles. Multiple layout measurements are valid; one successful display may place
one mount once. Native invariant violations are trusted-code failures, without a
sandbox or fake cleanup receipt.

See [desktop ownership](../desktop/README.md) and [shared kernel](../ui/README.md).
