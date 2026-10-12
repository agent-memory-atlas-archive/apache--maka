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

[English](README.md)

导出用于生成 JavaScript SDK 的 Rust 契约目录。它依赖声明 crate 与平台接口，无需构建产品可执行文件或 V8。运行 `cargo run -p maka-schema` 输出目录。修改 Rust 契约后，通过 SDK 脚本重新生成，并用 `schema:check` 检查漂移。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
