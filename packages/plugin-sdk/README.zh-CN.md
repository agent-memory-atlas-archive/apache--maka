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
Agent 服务合计最多 8192 字节、两块节选，完整源窗口预算为 64 KiB，实际输出另做交付检查。
节选的 offset 与 `totalBytes` 表明是否为完整消息；部分消息不能冒称独立 Markdown。
位置与水位只定位内容，不授予读取权限。

Agent history 每块节选显式携带 `tool: HistoryTool | null`。工具片段保留 canonical
调用身份和名称；结果另含 `isError` 及此前调用的名称，没有此前调用时名称为 `null`。
空文本工具仍占一块节选并推进游标，数字 JSON 和图片结果不会仅因文本投影为空而消失。
这些是呈现事实：`isError` 不区分执行失败与结果未知；工具身份不授予执行或重试权限。
完整工具参数和结果 payload 不属于这个文本窗口 API。

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

`ui.desktop` 和 `ui.terminal` 提供完整、强类型的原生场景声明。
两者都有 `text`、`button`、`stack`、`dialog`、`component`，Desktop 另有 `popup`、
`canvas`。`input`、`textarea`、`label`、`markdown`、`filePicker`、`image`、
`sourceText`、`selectionMenu`、`reading` 会检查内置组件 props；绑定和事件路由仍需显式提供。
`component` 接受外部插件契约的 props，不会根据绑定名称推测契约。

入口使用 `Component<State, Props, DesktopNode>` 或 `TerminalNode`。
构造接口阻止跨目标子节点混用，包括变量与 reading 的嵌套内容。
非序列化 symbol 只标记编写时的目标，不改变场景 JSON，也不替代原生准入。
生成的 `DesktopScene`、`TerminalScene` 是两个渲染器直接使用的 wire 类型，统一定义在
`maka-ui`。SDK 提供界面声明；主题、默认样式、产品交互封装由产品插件负责。
计数器示例展示独立的强类型场景和共享业务逻辑，无需另一套可移植中间树。

```ts
import { ui } from '@maka-agent/plugin-sdk';

const view = ui.terminal.stack({
  key: 'root', direction: 'column', gap: 1, padding: 1, background: 0,
  children: [ui.terminal.button({ key: 'save', label: '保存' })],
});
```

Terminal 按钮用 `align`（start/center/end，默认 center）指定标签对齐，预留四列点击间距，超宽时按字素省略。
可选 RGB 字段 `background` 填充整个区域，`hover_background` 提供悬停填充。
`bold` 表达强调样式。键盘焦点使用下划线；悬停不选择、不夺取焦点。禁用按钮变暗且不接收输入，仍保留显式指定的背景。Stack 背景可省略，使纯布局容器继承周围表面。

Terminal Dialog 支持 `ascii` 边框及独立的 `title_color`；Input、Textarea 的
appearance 同样支持 `ascii`。Markdown、原文、Diff 和 Rows 接受产品提供的终端
调色板（包括 ANSI 终端颜色），呈现器不选择产品主题。
`LabelProps.terminalForeground` 仅在终端替代 RGBA 前景色。

终端原生 `maka.ui.checkpoint` 工厂导出 `maka.ui.ValueCheckpoint`，沿原组件实例的
checkpoint 作用域保存小型 UI 偏好。存活期间 `key`、`format` 固定，写入携带观察到
的 revision，JSON 上限 16 KiB，已接纳写入在退休前排空。`changed` 报告
loading/saving/ready/failed，不构成 Host Settings 的第二份权威。

`maka.ui.morph-mark` 导出无交互的 `maka.ui.MorphMark`：产品通过配置提供两组有界
路径、颜色与周期，props 指定背景、前景、动画和 ASCII 偏好。仅在可见且启用动画
时使用终端共享时钟；Logo 和具体几何留在产品包。

`TerminalLayout` 从实际终端协议生成。Stack 的 `layout.grow`
在固定兄弟节点占位后分配剩余格子；`basis` 指定其在父主轴上的基础长度。Row 的
`wrap` 将控件分到后续行，每行分别分配弹性宽度，并按实际宽度测量换行高度。
Column Stack 的 `scroll` 选择 `none`、`manual`、`anchor` 或 `follow_end`。
阅读视口通过 `layout.grow` 或 `basis` 分配高度，内部内容保留自然高度。位置、滚轮/
Page/Home/End 与稳定子节点锚点由原生 owner 持有。绘制只读，只有成功终端输出后
才接受匹配的视口几何；隐藏路由保留阅读基线。隐藏子树同样要求节点 key 唯一。


`raw` 事件及同一次输入的同步组件观察者可返回 `decision: { preventDefault: true, stopPropagation: true }`，省略标志为 false。私有准入仅沿原经过检查的 `on` 路径传递，公开组件事件仍为 `component`。旗标按 OR 合并，完整事务提交后才生效，不保留在 frame。生命周期 emission 与 action/subscription feedback 不能通过 payload 中的 raw DTO 获得准入。决定作用于实际平台监听阶段，不能撤销更早的原生行为。

Desktop 与 TUI 通过显式 `maka.ui.locale` 原生实例提供 `contracts.locale`（`Locale` 值导出）。激活时依次读取 `LC_ALL`、`LC_MESSAGES`、`LANG` 与系统语言，规范化 POSIX 名称并保留语言、文字和地区。`C`/`POSIX`、缺失或无效值回退到 `en`。支持哪些语言以及显式偏好的优先级由产品决定；该值不授予环境变量读取能力。

TUI 的 `maka.ui.text-file` 原生实例导出 `contracts.textFile`（`TextFile`）。安装配置只授予一个绝对 `path`，null 表示禁用；带 owner 的读写命令不能选择路径。它仅接受 16 KiB 以内的普通 UTF-8 文件，拒绝文件入口的符号链接，保存时检查预期 SHA-256 摘要后原子替换文件。这可检测通常的外部修改，但不是跨进程原子 CAS。原生 worker 仅在 commit 后启动，遵守资源预算并在退役时排空；观察结果不会自动持久化产品状态。文件格式及导入/导出策略由产品插件维护。

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

一次性 `Action.key` 是原 component Mount 内的反馈相关性。排队、准备、执行或等待反馈
期间不能重用；需要并发时使用不同 key。反馈事务可以原子地以自己的 key 发起下一次请求。
未完成的 cleanup 仍由原执行独立持有；释放 key 不恢复已撤销的准入。一次性 action key
不能同时作为订阅需求的 key。

Component capture 声明 `output: { kind: 'resource', resource: { name, version } }`
或 `output: { kind: 'continuation' }`。Resource source 填充服务精确声明的附件槽；可选
`Action.continuations: CaptureSource[]` 仅用于 Query。两者复用已检查的 child/forwarding
路径，并终止于原生捕获，JavaScript 不能提供原生见证。Continuation 快照留在原前台请求中，
执行结算后也保留到反馈前。Desktop/TUI 在调用反馈前检查原路径、原生 owner 和当前提交的
recipe。失效返回 `{ status: 'stale' }`，不发布业务值、不撤销执行，也不确认 cleanup。
原生 widget 实现实际观察检查；缺少捕获会返回 `not_accepted`。这些机械边界本身不代表
产品历史导航已完成。

Desktop 和 TUI 提供 `contracts.readingArea`（`maka.ui.reading`，导出 `ReadingArea`）。
`content` 使用所选呈现器经过检查的 scene：带滚动策略的 column Stack，子组件通过本实例
声明的绑定组合。原生滚动与文本选区 owner 保留几何和选区；插件只接收 `observed` 事实
和 `input` 事件，不保存像素 offset。递增编号的 `start`、`end`、`reveal_child` 命令结算为
`applied`、`cancelled` 或 `rejected`；省略命令不会重放已接受的命令，只保留当前命令与最近回执。
TUI 还接受 `focus_item`，key 指向本 ReadingArea 场景中的可用 Item。它最小滚动显露条目，
等待实际呈现成功后聚焦，按条目声明发出 follow-focus activation，不提交条目。
新的物理输入、结构隐藏或退休取消待处理请求；不存在或禁用的条目被拒绝。Desktop 目前拒绝此 TUI 命令。
聚焦阅读区 Item 会清除该阅读区原生文本选区；modal 内的 Item 不清除背景阅读区选区。
TUI 阅读区标题上的 Shift 导航从所属消息首个可见正文行开始，继续延伸原生选区。
Item 的 `submit` 将 Enter 配置为 `submit` 事件，`submit_space` 独立将物理 Space 配置为同一事件；
两者默认 false，焦点跟随与鼠标点击仍发出 `activate`。
TUI 另报告 `selectedChildKey`，表示原生选区焦点（含折叠光标）所在的直接内容子节点。
归属只在实际输出成功后提交；modal 暂停保留上下文，原文退休/转移则撤销。其它呈现器
目前返回 null。它是展示观察，不授予原文读取能力，也不增加第二个选区 owner。
原生 `tail` continuation 仅在原实际绘制的末尾、阅读意图和本阅读区选区仍符合条件时准入隐式替换；
滚轮或活动选区使反馈成为 `stale`，不撤销独立服务读取。

Desktop 还提供 `contracts.sourceText`（`maka.ui.source`，导出 `SourceText`），用于只读原文。
原生选择与 Copy 保留空白，并可在同一 ReadingArea 的 SourceText 与 Markdown 间复制。
前缀追加及换行保留原字节选区；替换原文撤销该选区。该组件不解析标记，也不授予图片或网络访问。

普通 agent 产品保留固定 fence 下的一页精确 canonical 历史。Earlier/Next 替换该窗口；
Latest 显式捕获新 fence。Watch 进度不自动替换正在阅读的窗口。实时节选只保留显示 fence
之后的有界前缀；仅完整 canonical 消息使用 Markdown。产品还提供固定 fence 的单消息原文
阅读器，以有界开头/上一段/下一段/末尾导航访问完整原文，失败保留当前段并可精确重试，
返回时保留原对话窗口。两种呈现器均组合原生 ReadingArea 和 SourceText；完整 Desktop assistant 消息额外使用 Markdown。

TUI 提供同契约、同实现名的原生 `SourceText`。选区保留原始 UTF-8 字节，
包括 CRLF、tab 与控制字符；控制字符安全显示，Ctrl+Shift+C 通过有界 OSC52 请求复制，
不表示操作系统已确认写入。追加文本保留原端点，替换/禁用撤销旧选区。TUI ReadingArea
将逻辑 SourceText 纳入同一个选区，包含被裁剪原文的全选。选区和正在进行的鼠标选择
阻止自动 tail 替换；超过 64 KiB 的 Copy 在分配或输出前拒绝并保留原选区。
Shift 或 Alt/Option 点击延伸原选区；Shift+方向键及 Shift+Home/End 沿原生视觉行移动，
保留软折行行尾归属及纵向列位置。Shift+PageUp/PageDown 延伸可见视口减一行，
可跨 SourceText、Markdown 和 Diff；成功 flush 后才显露移动端点。
新的焦点或指针操作取消待显露请求，保留原选区。
完整阅读器的真实 OS 验收仍待完成。

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

Agent 图片工具使用生成的 `AgentToolImageFrame` 契约及 `result: "image"`，声明为不带输入资源的订阅。逐块 `await emit`：`mimeType`／`totalBytes` 恒定，`offset` 连续，`data` 为带 padding 的 Base64；每块解码后固定 32768 字节，最后一块为正数余量。支持 PNG、JPEG、GIF、WebP，总量 1–5242880 字节、每边 1–2000 像素。全部发送后返回 `{ kind: "complete" }`；将 complete 作为事件发送、仅 EOF、失败返回、取消或未确认清理均不能发布图片。Agent 校验实际字节并写入原不可变工具产物，路径／产物 ID／类似图片的 JSON 不提供权限。JSON 工具须显式选择 `result: "json"`，结果仍按 JSON 处理。

终端 `ui.terminal.contextMenu` 在 `background` 对象旁提供可见的「⋯」入口，支持右击与 Shift+F10。菜单 `content` 使用普通按钮，方向键选择，Esc 或外部点击关闭，接受操作后返回原焦点。可选 `copy_selection_label` 只在打开手势捕获到有界原生选区时提供复制项，实际复制仍要求物理操作。原生视图的命名输入上限为 256，保留 64 个子 slot、1 MiB 回调和编译时限。

### Agent Executor 服务

`contracts.executor` 是只有 `execute` mutation 的普通 Host 服务。`ExecutorExecute`
包含原 invocation、conversation key、正文、工作区路径、指令及后端自己的模型／思考设置。
这些字段只描述工作，不授予文件、进程或其它服务权限；相应 import 和 grant 必须独立声明。
实际工作与资源结算后才能返回 `ExecutorOutcome`。声明的 `progress` schema 接收
`ExecutorOutput` 正文、思考增量及工具活动观察。

方法资源槽直接声明 `{ type: { name, version }, optional }`。Host 与 UI 都拒绝缺少必填槽、
未声明的槽及类型不匹配。Executor 声明可选的 `attachment0` 至 `attachment7` 二进制槽；
`attachmentCount` 只描述本次实际提供的连续槽数，不授予权限。
通过 `context.invoke({ operation: 'metadata', slot: 'attachment0' })` 读取
`{ type, bytes, metadata: { name, mimeType, bytes, digest } }`，再用 `read` 读取最多 64 KiB 的字节区间。
原生后端使用 `Context::attachments()`。捕获先保留原 Host 容量，实际清理完成后才释放；
文件名和摘要不能用于打开文件路径。

包 manifest 声明该 export（JavaScript 入口必须包含 realm），普通 `activate` 返回
`{ executor: { contract: contracts.executor, service: ['backend'] } }`，导出的 `backend.execute`
接收 `ExecutorExecute`，返回例如 `{ status: 'completed', text: input.message }`。
回调第三参数为 `RequestContext`，通过
`await context.invoke({ operation: 'emit', value: { type: 'output_delta', text: '正文' } })`
逐项发出进度。有界临时流最多排队 16 项，每项 64 KiB；emit 接受不代表 Agent 已持久化事件。
Agent 通过原 executor recorder 记录每项进度后才发布完成；工具帧只观察后端活动，不派发 Agent 工具。
原生消费方通过 `Context::begin_mutation` 和 `Mutation::recv` 读取，再等待原 completion；
仅等待回执的调用方自动排空进度。丢弃读取端关闭进度需求，原 Scope 仍负责实际 effect 的清理。
恢复只读取原回执，不重放进度或执行。

消费方原生 Agent 显式声明匹配契约的 import，并配置
`executors: [{ name: "external", import: "backend" }]`；组合将 `backend` 绑定到实际 provider，
另行授予该 Agent origin/consumer 对该 provider 的 `execute` 与 recovery。
`agent.executors({})` 仅列出配置名称，实际运行使用
`agent.execute({ sessionId, message, executor: { executor: "external", settings: { model: "provider-model" } } })`。
它复用原 Session、队列、stop、work 和历史，不读取 Host 模型凭据；后端自己解释模型设置。
替换执行 binding 不能消费原排队输入，不确定结果不重发，也不能当成原生模型 invocation 恢复。
WorkHub 可发现后端，并在 `route` 中以 executor 选择替代 model/thinkingLevel。

普通 Composer 已支持配置的执行后端和 Host 模型。`agent.execute` 与 `agent.send` 的
`attachments` 都接收当前 Session 的不可变上传 ID；有附件时正文可以为空。
WorkHub 先将原上传转交到 worker Session，再提交原 execute。未知执行只读取原回执，
不重新捕获附件或重发执行。

Mutation 方法可声明 `progress` schema。普通 UI Action 在原操作持有期间收到
`ActionOutcome {status: "updated", value}`；有界交付在组件提交反馈后继续。
组件返回 `cancel: [actionKey]` 可请求取消自己的原 mutation。最终 `operation` 反馈
仍携带原回执；进度和取消均不证明业务完成或资源清理。
