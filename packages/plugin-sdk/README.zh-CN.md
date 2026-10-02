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

# Maka 插件 SDK

[English](README.md)

根导入 `@maka-agent/plugin-sdk` 对应新的插件应用运行时。
Wire 类型和内置契约由 Host/UI 实际执行所用的 Rust 类型生成。
`Component` 定义同步回调；`RequestContext` 使用当前调用的资源槽，服务结果为
`unknown`，作者需要按服务契约检查结果。TypeScript 泛型不能代替验证；
数值范围、JSON 限制和权限由 Rust 检查。

```sh
npm run schema:generate -w @maka-agent/plugin-sdk
npm run schema:check -w @maka-agent/plugin-sdk
node packages/plugin-sdk/scripts/bundle.mjs packages/plugin-sdk/examples/counter/desktop.ts /tmp/counter.mjs
```

[计数器](examples/counter/) 提供独立的 Desktop/TUI 入口；
[Host 转发器](examples/forwarder.ts) 显式映射当前调用的资源槽。
打包后的模块在实际应用集成测试中执行。使用包导出前先构建 workspace。
Bundle 必须没有未解析 import 和顶层 await；它不包含原生扩展，也不授予权限。

Host 应用服务支持 query 和不含资源附件的持久 mutation。
`context.invoke({ operation: 'mutate', ..., operationId })` 返回 `OperationReply`，
即使结果未知或无法交付，也保留原 `reference`。只有 `receipt.kind === 'recorded'`
描述 journal 中的事实；应分别检查 `outcome` 与 `cleanup`。
`recover` 使用原引用，不依赖当前 import，并需要独立的恢复授权。
`not_recorded` 表示经过授权的精确读取没有找到原记录；`unavailable` 不断言记录内容，
两者均不会自动重做。保留原引用用于恢复；新的 operation identity 可能再次执行效果。
Subscription、副作用资源快照和旧业务迁移仍待实现。

可以通过 `maka application commit --root ROOT --deployment UUID --expected-revision 0 --input input.json --package bundle.maka-plugin`
将部署来源存入实际持锁的 State Root；替换时必须提供已观察的修订号。
`maka application read --root ROOT --deployment UUID` 会检查已保存的输入与精确包字节。
返回的 `DeploymentRef` 只标识数据域与来源修订，不授予权限，也不表示运行就绪。
Host 可以从该快照启动，并在已接受的工作及资源实际收尾期间保留存储所有权。
原回执可以跨来源替换和进程重启读取；未知外部副作用的权威恢复、
副作用资源快照与在线组合收敛仍待实现。

`./host` API 及下方指南属于仍待迁移和删除的旧实现，不是新应用 API 的适配层。

Rust Runtime Host 的公开 TypeScript 契约。当前为 **Host SDK 3**、
**终端 View 9**，与 Maka 应用版本及 npm workspace 版本分别演进。
此 workspace 仍为私有包，不应假设能从 npm 仓库安装。

插件开发文档统一维护在
[Maka 插件编写 skill](../../skills/maka-plugin-authoring/SKILL.md) 中。
它随 Maka 内置分发，包含可执行模板和完整 SDK 源码契约。
每个 Profile 会自动安装，启动时将完整目录同步到随 Maka 提供的版本，清除过时资源。
启用、禁用和固定设置会保留；自定义内容请另建技能保存。
可用 `/skill:maka-plugin-authoring`，也可由模型按任务发现。
`Skill` 按需读取附带资源，无需向普通文件工具开放 Host 私有目录。

先读[快速开始](../../skills/maka-plugin-authoring/references/quickstart.md)，
使用[可运行插件模板](../../skills/maka-plugin-authoring/assets/starter/)。
[API 索引](../../skills/maka-plugin-authoring/references/api-map.md) 对应完整
[SDK 源码](src/host.ts)；专题指南分别说明工具与输入、权限与状态、终端界面、
模型与执行器、原生 Rust 插件，以及测试和交付。

```sh
npm --workspace @maka-agent/plugin-sdk run build
npm --workspace @maka-agent/plugin-sdk run typecheck
node --test packages/plugin-sdk/tests/*.test.mjs
```

Runtime/UI 入口必须是无未解析 import、无顶层 await 的独立 ESM。
开发时使用 SDK 类型导入，运行时通过实际调用上下文的 Host 能力执行操作。
Manifest、生命周期和实例激活方法见统一指南。
