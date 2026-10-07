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

# 架构

[English](ARCHITECTURE.md)

Maka 是插件应用运行时。Host 与 UI 以显式能力绑定实例化共用插件内核；已提交的组合选择执行入口、配置和启动组件，终端与 Desktop 是原生呈现适配器。

- **准入与结算：** `crates/runtime-host` 接纳操作、捕获执行边界并协调资源清理；`crates/event-log` 管理持久执行事实与状态根所有权。
- **Agent 执行：** `crates/agent`、`crates/runtime` 和 `crates/model` 构建模型请求并结算工具效果。`crates/providers` 管理提供商认证、模型发现和策略，`crates/responses` 实现原生 Responses 传输。
- **能力边界：** 业务插件通过 `crates/plugins` 访问存储、模型、执行、凭据、HTTP 和呈现能力。Host 管理权限和生命周期，插件管理领域策略。
- **JavaScript：** `crates/js-runtime` 内嵌 V8，承载 Code Mode 和 JavaScript 插件。提供商 SDK 在构建时打包，Guest 代码通过明确绑定访问 Host 能力。
- **Computer Use：** `crates/computer-use` 拥有独立的 REPL 与会话生命周期，独立于 Code Mode，支持原生应用和浏览器。
- **应用内核：** `crates/plugin-core` 负责校验组合、绑定身份与生命周期。`crates/ui` 提供挂载组件、已接受动作、原生捕获与部署收敛，不保存业务页面路由。
- **呈现：** `crates/tui` 提供终端布局、输入、编辑、滚动与选区；`crates/ui-gpui` 和 `crates/desktop` 提供 GPUI Fast/Kit 呈现。`packages/agent-product` 以可替换普通插件提供 Agent 工作区、导航、主题与设置。

同一插件可以提供独立终端和桌面入口，也可以只支持一端。动态 JavaScript 包定义新布局与工作流，Rust 原生实现随二进制提供。安装只保存不可变包字节，不改变启用状态或权限；替换启动根、配置或绑定需显式校验来源变更。原生 owner 保有编辑与阅读状态，插件保存工作流状态和观察，不复制原生缓冲区或像素位置。

用 `maka application run --root PATH --deployment UUID --connections FILE` 启动终端产品，桌面来源用 `maka-desktop run`。连接配置提供显式方法与恢复授权。不带命令显示帮助，不获取 State Root。旧业务 TUI 及自动 Agent 启动已删除，规范客户端协议仍提供给 CLI/ACP 和 Host 业务服务。

插件资料库独立于活动实例发现内置定义。插件拥有自己的本地化介绍，Host 发布其已注册的默认组合。添加内置插件通过现有的组合操作与版本校验应用默认设置。移除实例保留目录发现能力和插件数据；已安装的包文件仍是独立资源。

`packages/cli` 的 npm 包只负责选择并启动匹配的可执行文件，不实现运行时、配置存储或工具。

详细契约见各 crate 的 README；JavaScript 插件开发见 [SDK](packages/plugin-sdk/README.zh-CN.md)。
