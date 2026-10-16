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

# maka-transport

[简体中文](README.zh-CN.md)

Connects a foreground UI to independently owned Host deployments over local IPC. The launcher supplies trusted connection configuration and method/recovery grants. Public connections discover a Root-bound Host or ask the trusted CLI launcher to start it; cohosts borrow the UI Store. `connections` owns those connections; `Updates` coordinates approved source changes.

Remote registration data describes available services. It cannot create permission, choose a different caller or retarget an accepted operation. Closing the UI waits for its original physical calls and locally owned cohost cleanup. Normal quit asks each public Host to stop if no other session remains, atomically with session admission. Explicit UI-only exit detaches and preserves background work.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
