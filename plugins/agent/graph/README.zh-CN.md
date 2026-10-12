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

# maka-graph

[English](README.md)

实现 Agent Graph 的协调与调度策略。`coordinator`、`decision` 和 `schedule` 选择工作，`repository` 与 `store` 持久化图状态，`projection` 和 `view` 向客户端提供视图。执行与权限检查仍由 Agent 和 Host 所有者负责。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
