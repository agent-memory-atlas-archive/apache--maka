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

`git` 提供原生的只读仓库状态和单文件比较。调用方传入原目录身份、私有 State Root 及经过验证的受管 worktree 绑定，并持有阻塞 worker 直到工作结束。库遍历前先检查 Git 元数据及 ignore/attributes 控制文件，不执行外部命令或使用替代对象库。取消采用协作方式；响应大小限制不代表固定执行时限或原子仓库快照。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
