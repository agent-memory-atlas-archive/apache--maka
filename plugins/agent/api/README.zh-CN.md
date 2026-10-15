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

# maka-agent-api

[English](README.md)

包含 Agent、Session、Executor、WorkHub 的协议声明以及 Agent 配置 schema。原生服务与 SDK 生成共用这些定义。调用方只需要公开契约时，可以依赖这里，无需引入 Agent 执行引擎。

独立 Desktop 通过同一个 Root 绑定的[公共 Host client](../../../platform/host-runtime/src/application/host/client.rs)使用持久 Agent 服务，不直接打开 Agent 存储，也不嵌入执行引擎。字段及 schema 以 [Agent 契约](src/contract.rs)和[交互契约](src/contract_interactions.rs)为准。

`history` 分页保持同一个 `through` 边界，`watch` 从最后的 `nextAfter` 继续，并发布队列与待处理交互更新。`send` 接受工作并返回其身份，`stop` 指定该次 invocation。重连后通过 `pendingInteractions`、`interaction` 和 `respond` 重建并回答原请求。修改保留原操作回执；丢失响应不代表可以重新发送。参见[原生接入验收](../native/README.zh-CN.md)及[传输所有权与退出方式](../../../platform/transport/README.zh-CN.md)。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
