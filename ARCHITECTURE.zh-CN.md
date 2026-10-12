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

目录表示代码归属，`package.ts` 表示插件身份，Cargo crate 表示编译单元，三者无需一一对应。family 目录只分组，不授予能力或创建运行时身份；既有包 ID、入口 ID 与持久化 namespace 不取决于源码路径。

- **平台：** `platform/plugin-core` 校验组合与绑定身份，`platform/packages` 加载不可变包，`platform/capabilities` 管理调用作用域与资源。`platform/event-log` 保持唯一持久写入者，并在激活前恢复遗留事实，包括已卸载插件留下的事实。
- **Host 与 UI：** `platform/host-runtime` 拥有准入与服务结算；`platform/ui-runtime` 拥有挂载组件、输入捕获、已接受动作和清理。需要 JavaScript 时，产品注入 `platform/host-js`、`platform/ui-js`。原生 Host/UI 与终端、GPUI 渲染器的编译不包含 V8 或 Agent 执行实现，持久化和公开业务事实依赖仍显式保留。
- **插件：** 叶子目录 `plugins/<family>/<plugin>/package.ts` 声明自己的入口与导入；原生实现位于插件自身或共享 family crate。`plugins/agent/engine` 管理 Agent 执行，`facts`、`ports`、`api`、`authority` 分别承载持久事实、业务接口、公开服务契约和原调用准入。Web、Files、Processes、Skills、Interactions 均可独立注册。
- **产品：** `products/maka/native` 选择编入的原生工厂；`products/maka/bundle/package-inputs.json` 选择插件包，`src/composition.ts` 完成绑定。CLI 与桌面入口注入执行后端并启动组合。工厂可用不意味着插件已激活或已获得授权。
- **开发：** 原生描述信息取自 Rust 的实际实现标识、契约和配置 schema，`tools/schema` 无需产品或 V8 即可生成 SDK。通用插件构建器接受单个叶子或产品选择；`plugins/shared` 的共享组件不依赖某个产品。
- **Computer Use：** `platform/computer-use` 独立于 Code Mode，拥有自己的 REPL 和会话生命周期。

同一插件可以提供独立终端和桌面入口，也可以只支持一端。动态 JavaScript 包定义新布局与工作流，Rust 原生实现随二进制提供。安装只保存不可变包字节，不改变启用状态或权限；替换启动根、配置或绑定需显式校验来源变更。原生 owner 保有编辑与阅读状态，插件保存工作流状态和观察，不复制原生缓冲区或像素位置。

用 `maka application run --root PATH --deployment UUID --connections FILE` 启动终端产品，桌面来源用 `maka-desktop run`。连接配置提供显式方法与恢复授权。不带命令启动 `products/maka/bundle` 中的普通产品；`--help` 不获取 State Root。CLI/ACP 与原生业务服务沿用同一规范客户端协议。

插件资料库独立于活动实例发现内置定义。插件拥有自己的本地化介绍，Host 发布其已注册的默认组合。添加内置插件通过现有的组合操作与版本校验应用默认设置。移除实例保留目录发现能力和插件数据；已安装的包文件仍是独立资源。

`packages/cli` 的 npm 包只负责选择并启动匹配的可执行文件，不实现运行时、配置存储或工具。

详细契约见各 crate 的 README；JavaScript 插件开发见 [SDK](packages/plugin-sdk/README.zh-CN.md)。

开发版从 `products/maka/bundle/dist` 或 `MAKA_PRODUCT_BUNDLE` 读取产物，继续走同一包校验与提交管线。`just run` 先构建 TypeScript 产物再启动 Cargo，修改 TypeScript 不触发 Rust 重编译。发布版内嵌产物。已有 Root 中的包安装和更新仍需显式提交来源变更。
