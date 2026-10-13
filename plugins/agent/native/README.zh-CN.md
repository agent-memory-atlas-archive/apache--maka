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

# maka-agent-native

[English](README.md)

将 Agent 执行暴露为原生应用服务。`provide` 注册工厂，`executor` 和 `workhub` 连接执行与协调领域。包声明和产品组合决定激活哪些工厂，以及如何绑定导入。

服务管理自己的激活，并委托 `maka-agent-engine` 执行运行。会话历史、控制和工具调用使用获准作用域与原始操作回执。文件、进程、技能等服务都是通过组合显式选择的依赖。

Agent 接收消息时保存模型的 Code Mode 默认值或显式 `codeMode` 覆盖设置；排队消息和恢复运行保留原模式。服务工具可以通过 `nestable: true` 允许代码内调用，仍执行与直接调用相同的权限检查、操作回执和清理流程。JavaScript 收到声明的业务结果，持久日志保留原生操作的原始回执。省略该字段时仅允许直接调用。Maka 默认组合为文件、进程、Web 和 Skills 工具启用嵌套调用。

Maka 默认组合启用 `toolDiscovery`：可选工具通过 `tool_search` 加载说明，并在同一 Host 会话内跨轮保留。Code Mode 在说明尚未加载时仍可调用已授权的嵌套工具。消息接收时保存发现策略，恢复运行时沿用原策略。已加载工具说明和 Code Mode 的 `store`/`load` 值按会话隔离、保存在内存中，Host 重启后清空；上下文压缩成功提交后重置已加载的工具说明。

独立的模型覆盖项 `applyPatch` 选择补丁编辑或 Write/Edit 结构化编辑。与 Code Mode 一样，GPT、DeepSeek 模型及 DeepSeek 端点默认启用；排队消息和恢复运行保留接收时的选择。筛选依据是实际绑定的 Files 契约和方法，工具改名不影响策略。读取工具保留，文件操作仍检查实时权限。连接设置的模型覆盖 JSON 可分别设置 `codeMode` 与 `applyPatch`。

Maka 默认组合启用 `todo`：`todo_read` 和 `todo_write` 维护当前会话的持久清单，也可用于 Code Mode 和只读工作区。写入会校验版本并替换整个列表。UI 显示已报告的进度和只读「清单」面板，以有界分页浏览最多 200 项。清单完成状态是模型报告的进度，不代表操作已成功的执行证据。

Maka 默认组合启用 `automaticTitles`。通过 `automaticTitle: true` 创建的会话在首条消息获准后，使用该消息选定的模型发起一次有界、独立计费的标题请求。显式命名、手动改名（包括保留原占位名称）、归档及托管会话保留原名称。重试和 Host 重启不会重复已接收的标题尝试；Host 关闭会等待请求实际清理完成。

模型上下文总容量、独立输入上限与压缩阈值随模型选择一起保存，排队和重启后继续使用接收时的值。未显式设置压缩阈值时，使用已知输入容量的 85%；输出预留仍由引擎单独计算。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
