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

# maka-ui-runtime

[简体中文](README.zh-CN.md)

Owns component activation, state, input dispatch and atomic frame updates. `Application` runs a checked UI composition; `NativeRegistry` supplies compiled factories and a scene compiler. Products inject JavaScript execution through `backend` when needed.

A callback proposes state, a frame and one-shot actions. The runtime validates the whole batch before committing it. Renderers consume committed frames and own platform resources; they do not execute guest code during layout or paint. Terminal and desktop scenes have separate languages.

Native editors keep their buffers and expose bounded captures. Asynchronous mutations retain their original receipt and caller identity. Replacing a component waits for its accepted work and native resources to settle; dropping a frame or result observer is insufficient. See `actions`, `operations`, `editor` and `checkpoint` for these paths.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
