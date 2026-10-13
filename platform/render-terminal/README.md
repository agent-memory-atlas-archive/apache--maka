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

# maka-render-terminal

[简体中文](README.zh-CN.md)

Presents checked UI compositions in a terminal using Ratatui and Crossterm. `application` drives input, committed frames, native editors and teardown; `markdown` renders text content. The product supplies its composition and services. This renderer does not select Agent pages or embed JavaScript execution.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)

The foreground loop polls accepted work independently of drawing. Input, changed recipes and native observations request a frame; visible animations and elapsed labels schedule their next frame. An unchanged view does not repeat layout, buffer diffing or terminal output. Lifecycle polling reads phase/closing state directly instead of constructing diagnostic snapshots.

A local before/after sample on 2026-10-13 used an optimized debug build, a 120×40 PTY, the same empty WorkHub home and 22 Unicode keystrokes in the command palette. The baseline was `e21d70dc3`; each binary used its matching bundled contract versions. No compiler or sampler ran during the measurements. Over five idle seconds, synchronized terminal frames fell from 237 to zero and foreground-process CPU fell from 11.6% to 6.2% (one core = 100%). Key-to-completed-frame median was 3.4 ms before and 5.4 ms after; this single sample demonstrates reduced idle work, not an input-latency improvement or parity with the historical TUI under long streaming conversations.
