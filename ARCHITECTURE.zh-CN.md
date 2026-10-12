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

Maka 通过插件组装应用。包声明自己提供和需要的能力，组合选择实例并将导入连接到导出。Host 服务执行工作，UI 组件通过原生终端或桌面渲染器呈现结果。

## 代码归属

[平台](platform/README.zh-CN.md)包含包加载器、组合内核、执行所有者和渲染器。[插件](plugins/README.zh-CN.md)包含业务功能及其声明。[Maka 产品](products/maka/README.zh-CN.md)选择包、原生工厂和启动行为。

`package.ts` 标识一个开发者声明的插件，Cargo crate 标识一起编译的代码。两者不必一一对应：Agent 服务可以使用多个领域 crate，相关插件入口也可以共用一个原生实现 crate。

Host 与 UI 所有者使用相同的生命周期内核，通过 `host-js` 和 `ui-js` 执行适配器运行 JavaScript 插件。原生渲染器和所有者无需 V8 或 Agent 引擎即可编译；存储与呈现所需的共用事实类型仍作为显式依赖存在。

## 从包到运行中的应用

1. 包加载器校验不可变内容并计算引用。
2. 组合规划器检查入口选择、配置和绑定。
3. Root 按修订保存期望配置及包内容。
4. Host 或 UI 激活所选实现，实例完成收敛后，该配置才成为有效状态。

安装不授予权限。绑定让调用方能够找到符合契约的提供方；每次调用准入时，Host 都会检查调用方、方法、输入和实际资源。本地 IPC 连接携带启动器提供的显式方法与回执恢复权限。

更新配置会保留未改变的激活及其绑定。改变的实例先退役再替换。已接受的工作始终归原所有者管理，即使调用方断开或组件消失也是如此。

## 状态与副作用

EventLog 持有持久 Root 和规范记录。稳定的插件数据归属于 Root、部署和实例。包修订与激活 epoch 描述代码和执行，本身不会创建新的数据命名空间。

变更操作具有回执，业务完成和实际清理分别记录。响应丢失时，调用方查询原回执，不重新发起替代操作。取消表示请求停止工作；资源仍被持有，直到实际清理完成。

UI 回调提出状态、帧和动作的候选更新，整个批次校验后原子提交。渲染器读取已提交帧，不在绘制时执行插件代码。原生编辑器管理缓冲区、选择和 IME，插件通过有界捕获与命令操作它们。

## 开发与分发

`just run` 先打包 TypeScript，再启动 Cargo。开发二进制从磁盘读取 bundle，发布二进制将其嵌入程序；两者使用相同的包校验与提交路径。重建代码不会替换已有安装保存的组合。

Rust 契约通过 [maka-schema](tools/schema/README.zh-CN.md)生成 [JavaScript SDK](packages/plugin-sdk/README.zh-CN.md)，无需构建产品。npm 包只负责[启动原生可执行文件](packages/cli/README.zh-CN.md)。
