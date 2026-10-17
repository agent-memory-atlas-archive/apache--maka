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

`diagnostics` 在固定的 `through` 边界内，从新到旧分页读取执行、模型步骤及自动标题失败。`watchDiagnostics` 在订阅时及变化后发送当前诊断版本；重连后重新读取最新页。这些会话范围内的运行记录保留原 invocation/event 标识和安全的类型化原因，不包含提供方响应正文或复制来源会话的失败。诊断版本独立于对话历史，因此回答结束后才发生的标题失败仍可发现。

`gitStatus` 和 `gitDiff` 通过同一个 Agent 服务读取所选 Session 的仓库，也适用于 WorkHub 协调者会话。状态区分已暂存、工作区和未跟踪的变化；同一路径可以同时出现在已暂存和工作区部分。diff 分别比较 HEAD 与索引、索引与工作文件，或空文件与未跟踪文件。尚无提交的仓库返回 null `head`；没有仓库的工作区返回 null。路径以 `bytesBase64` 保留 Git 文件名的原始字节，重命名呈现为删除加新增。

这些查询提供当前的只读观察，不是仓库快照。状态结果不完整时明确标记 `truncated`；单文件 diff 区分文本、二进制、未变化、不支持及输入或输出过大的情况。响应不超过 512 KiB，diff 每侧输入不超过 1 MiB。所选工作区必须是仓库根目录；外部过滤器、稀疏检出、子模块等不支持的仓库特性会明确失败，不执行配置中的命令。工作区或比较基准变化后应重新查询。协议类型见 [Git 声明](src/contract_git.rs)；已完成工作的不可变补丁仍使用 `snapshotPatch` 和 `readPatch`。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
