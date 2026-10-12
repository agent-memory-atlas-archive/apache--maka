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

# Maka experimental plugin SDK

This directory contains small v0 SDKs and templates for the experimental
`maka:plugin/runtime@1` core-Wasm ABI. It is deliberately thin: the Host owns
JSON framing and isolation, while the guest only needs to expose a manifest,
an allocator, and one JSON-producing function.

The templates return static JSON to keep the examples dependency-free. A real
plugin can replace the response helper with its own JSON encoder. Guest memory
must declare a maximum no larger than 64 MiB; the Host rejects an unbounded or
oversized memory before instantiation.

## Build the templates

From this directory:

```sh
node build.mjs rust templates/rust-echo
node build.mjs c templates/c-echo
node build.mjs zig templates/zig-echo
```

The selected template writes `dist/plugin.wasm`. The required compiler is not
bundled: Rust needs the `wasm32-unknown-unknown` target, C needs LLVM's Wasm
backend and `wasm-ld`, and Zig needs a Wasm-capable Zig installation.

## ABI boundary

The Rust and C helpers intentionally do not expose Node, filesystem, network,
or Maka Context objects. Adding those capabilities requires a new explicit
protocol instead of extending this raw JSON call with host pointers.

`maka_plugin_reset` is an optional export. When present, the Host calls it
after it has consumed an invocation result. The Rust helper uses it to reset
its bump arena, so repeated calls do not exhaust the 64 KiB helper heap. A
guest that manages memory with another allocator can omit the export.
