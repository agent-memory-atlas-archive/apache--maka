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

# maka-schema

[简体中文](README.zh-CN.md)

Exports the Rust contract catalog used to generate the JavaScript SDK. It depends on declaration crates and platform interfaces without building the product executable or V8. Run `cargo run -p maka-schema` to print the catalog. After changing Rust contracts, regenerate through the SDK scripts and use `schema:check` to detect drift.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
