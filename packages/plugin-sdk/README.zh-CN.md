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

公开 Session history 使用 `HistoryPosition`：`start`、`tail`、`before`、`after`
以及原始执行事件 `event` 书签。首次读取固定原始事件 `through` 水位；准备重试保留
同一 position 与返回的水位。`before` 排他结束，`after` 包含开始，两者必须携带原水位。
Ready 始终返回 `before`、`after`，其值为真实 sparse row/UTF-8 字节游标或 `null`；
双向窗口都按源顺序返回。Session Host 每个窗口最多 128 KiB 文本、八块节选，
包括全部附件描述的完整 JSON 最多 1 MiB；原生
Agent 服务合计最多 8192 字节、两块节选，实际完整 JSON 还须通过 64 KiB 检查。
节选的 offset 与 `totalBytes` 表明是否为完整消息；部分消息不能冒称独立 Markdown。
位置与水位只定位内容，不授予读取权限。

`configurations.agent` 是从原生 agent 类型生成的配置 schema，供包声明使用。
与 `contracts.agent` 配合，不在产品构建器中维护第二份原生配置或服务协议。

`Component<State, Props, View>` 可以为入口指定平台场景语言的类型。内核将
`View` 视为不透明 JSON，由选定平台的纯场景编译器解释，不提供统一布局语言。
`ComponentSlot` 仍是公共的子组件声明：绑定从原父组件的 import 解析，绘制状态
不能提供权限。因此同一插件的 Desktop/TUI 入口可以分别设计呈现方式。平台必须
在激活 JavaScript component 前以名义语言身份安装编译器。组件 entry 声明
`scene: { name, version }`；全部 enabled entry 必须在任何工厂或 guest 执行前精确匹配。
Desktop 为 `{ name: "maka.desktop.scene", version: 1 }`，TUI 为
`{ name: "maka.tui.scene", version: 1 }`。该本地要求不改变公开 props/events/captures；
纯 Value/Service 与原生 entry 省略 scene。原场景 JSON 参与回调、事务及驻留预算。

`raw` 事件可返回 `decision: { preventDefault: true, stopPropagation: true }`，省略的标志为 false。决定仅在整个因果事务提交后可见，不保留在 frame。其他事件来源和生命周期回调不能返回决定。它作用于平台实际监听阶段，不能撤销更早的原生行为。

```sh
npm run schema:generate -w @maka-agent/plugin-sdk
npm run schema:check -w @maka-agent/plugin-sdk
node packages/plugin-sdk/scripts/bundle.mjs packages/plugin-sdk/examples/counter/desktop.ts /tmp/counter.mjs
```

[计数器](examples/counter/) 提供独立的 Desktop/TUI 入口；
[Host 转发器](examples/forwarder.ts) 显式映射当前调用的资源槽。
打包后的模块在实际应用集成测试中执行。使用包导出前先构建 workspace。
Bundle 必须没有未解析 import 和顶层 await；它不包含原生扩展，也不授予权限。

Host 应用服务支持 query、subscription 和携带请求私有资源快照的持久 mutation。
Subscription handler 使用 `await context.invoke({operation: 'emit', value})` 发布；
数据项与最终返回值均使用声明的 output schema。每流允许 16 个队列项、16 个 pending emit，
每项完整 JSON 最多 64 KiB。等待每次发布以遵守读取端背压；原生 Host 客户端负责取消，
真实 cleanup 单独观察。
`context.invoke({ operation: 'mutate', ..., operationId, attachments: { text: 'text' } })` 返回 `OperationReply`，
即使结果未知或无法交付，也保留原 `reference`。只有 `receipt.kind === 'recorded'`
描述 journal 中的事实；应分别检查 `outcome` 与 `cleanup`。
`recover` 使用原引用，不依赖当前 import，并需要独立的恢复授权。
`not_recorded` 表示经过授权的精确读取没有找到原记录；`unavailable` 不断言记录内容，
两者均不会自动重做。保留原引用用于恢复；新的 operation identity 可能再次执行效果。
UI component 返回 `subscriptions: Action[]` 声明当前完整订阅集合，与一次性
Query/Mutation 的 `actions` 分开。key 在本组件内唯一；key 与完整请求不变时保留
原流，即使已经自然结束也不自动重放，修改请求才启动新流。成功提交后移除需求会取消流。
资源仅在首次启动时捕获，不随画面重新取样。每项 feedback 为
`{ status: 'updated', value }`，终值沿用 `succeeded`/`failed`；反馈事务提交后才消费
下一项。替换或退休会丢弃旧反馈，原执行仍须完成真实 cleanup。Desktop/TUI 共用这套
机制；认证跨进程 transport 仍待实现。

持久 UI component 通过普通 `actions` 提交 mutation（可携带声明的原捕获资源）；内核生成 operation
identity，先记录原 caller intent 再派发。feedback 的 `ActionOutcome` 使用
`status: 'operation'` 和原 `OperationReply`；组件应处理 unknown/unavailable 回执，
不能自动重复 action。
声明的资源槽在原生命令之前捕获 immutable snapshot，准备、caller 持久化、派发与
真实 cleanup 全程保留原 owner 和容量。Host 仅将原 bundle 的私有 nonce 写入操作
fingerprint，不记录快照字节或 metadata；同 operation UUID 的新捕获是冲突的新原请求。
恢复只读取原回执，不能重新构造或重放资源。
显式组合的 `maka.operations.history` 原生实例通过 `History` 导出提供本地
`contracts.callerHistory` 服务。`page` Query 接收 `CallerHistoryPage`（`after`、`limit`），
返回实际调用的持久 deployment/instance 的 `CallerIntent[]`，没有 namespace 输入。
使用最后实际返回的 cursor 继续；count 只是上界，完整数组最多 1 MiB。如果完整回调包装
超过交付预算，query 返回 `failed`，健康组件可以减小页大小后重试。原引用跨来源替换和
Root 重开保留；这些观察不授予 Host 权限，也不能代替 confirmed caller 执行 token。
跨进程资源传输和旧业务迁移仍待实现。

持久 Host 组合可选择 `maka.operations.receipts`，通过 `Receipts` 导出提供
`contracts.receiptReader`。普通 `read` Query 接收原 `RecoveryRef`，返回
`OperationReply`；应独立于当前业务 import 绑定，读取 owner 固定持有的原 Root，
且需要单独的恢复授权。错误 Root 或拒绝只产生 `unavailable`，不能冒充 `not_recorded`。
configuration 不能选择其它 Root 或 namespace。UI 从 caller history 取得原引用后，
可以通过普通 `actions` 查询；成功 Query feedback 的 `value` 是回执，仍须检查其中的
receipt，不能把本次查询成功当成原效果已成功。传输或查询失败使用 `failed`，应保留原引用。
可移植回执不能表示的大值或整数降为同原引用的 `unavailable`；raw native 恢复仍保留
原始精确值。本次 Query cleanup 与原操作 cleanup 是独立事实。

可以通过
`maka application commit --root ROOT --deployment UUID --expected-revision 0 --input input.json --package bundle.maka-plugin`
将部署来源存入实际持锁的 State Root；替换时必须提供已观察的修订号。
`maka application read --root ROOT --deployment UUID` 会检查已保存的输入与精确包字节。
返回的 `DeploymentRef` 只标识数据域与来源修订，不授予权限，也不表示运行就绪。
Host 可以从该快照启动，并在已接受的工作及资源实际收尾期间保留存储所有权。
原回执可以跨来源替换和进程重启读取；未知外部副作用的权威恢复、
跨进程资源传输与在线组合收敛仍待实现。

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


UI activate 返回带 component 或 value kind 的 `UiExport`。value 通过普通 Value 契约
发布按 schema 校验的不可变 JSON。所有组件回调及 activate(configuration, values)
的最后参数提供本 activation 捕获的 Value imports。`ComponentValues` 在编写类型中
将 map 与成员数组设为只读；实际运行提供独立副本，不提供修改供应者或重新路由的
权限。optional import 保留空数组，未声明 import 不出现。重复绑定的数据每次均计入
复制前的完整 1 MiB 输入；注册值与 frame 共用 32 MiB 保留额度。不接受旧的无 kind
激活格式。
