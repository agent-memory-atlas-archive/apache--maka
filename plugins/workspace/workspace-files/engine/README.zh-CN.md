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

# maka-fs-tools

[English](README.md)

实现文件读取、搜索、编辑、写入、补丁和 worktree 操作。调用方提供已授权的文件系统作用域。`ReadExecutor` 处理有界读取，`MutationExecutor` 与 `WriteCoordinator` 准备并串行化写入。补丁解析委托相邻的 `patch` crate。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
