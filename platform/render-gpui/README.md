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

# maka-render-gpui

[简体中文](README.zh-CN.md)

Renders committed UI frames with GPUI. `Widget` and `Slots` let native components supply elements and child mounts; `scene` compiles the desktop scene language. The adapter also owns native editors and input bindings.

Layout and paint read committed data. JavaScript component callbacks run on the background worker; editor buffers, focus and selection remain with their native owners. The original window owner publishes ready candidates on wakeups, including when no paint occurs. Required fallible native preparation precedes atomic publication; native commands execute only after it.

Each native mount owns its `LiveInput` and publication queue. Raw receipts keep their exact values until the original logical commit acknowledges them. A mount fixes its recipe implementation type; changing it requires a new child key/remount. Rejected preparation preserves the committed owner and geometry. A retired mount closes only after its entities, subscriptions and retained frame resources are released. Window startup belongs to the product desktop crate.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
