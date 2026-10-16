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

# maka-desktop

[简体中文](README.zh-CN.md)

Builds the `maka-desktop` executable and owns native windows. It loads a committed Desktop composition, injects the UI JavaScript execution adapter and uses `maka-render-gpui` to present frames. Pages and workflows come from plugin packages.

Build with `just build`, then launch an installed Desktop deployment with `target/debug/maka desktop --root PATH --deployment UUID --connections FILE`. The CLI starts its sibling frontend. Startup connects to the configured local Host or starts it automatically through the sibling `maka` executable. Closing the last window or quitting also stops the Host when no other client remains. Choose “Close UI and Keep Host” from the application menu to keep background tasks running. Platform-specific IME and accessibility behavior needs verification in a real window.

[Source](src/main.rs) · [Cargo manifest](Cargo.toml)
