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

# 原生 agent 应用服务

本 crate 提供普通原生应用实现 `maka.agent.api`，导出 `agent`（`maka.agent.Agent` v1）。Desktop 和 TUI 启动器登记可用实现，产品组合决定是否激活。初始化不会打开旧业务 Host 或自动安装旧业务插件。

配置仅声明 1–16 范围内的运行容量和最大步骤数。每次 `send` 显式携带 `model`：连接 ID、所观察的 revision 与模型 ID；首次 SQL snapshot 在准备运行前核 revision 和启用的模型成员关系。密钥、请求头、endpoint 与网络设置由实际 Root 的配置存储持有。每个逻辑步骤读取指定连接；不可用时不回退到其他账号或模型。凭据来自配置存储真实的认证回执。服务工具由 `tools` 显式选择；`recall: true` 另行启用原生历史工具，默认关闭。

`create`、`send` 是持久化 mutation。`send` 授权指定账号/模型上的完整有界运行，返回已接纳的 invocation ID。返回前，原始 NativeOwner 已持有真实运行句柄及非等待式容量 permit；请求结束或界面关闭不取消已接受的工作。`stop` 指定准确 invocation，迟到的停止请求不会取消后继运行。`list`、`observe`、`history`、`watch` 从 canonical session 元数据、同快照的 run/活动 part 身份、文本历史分页和有界投影读取内容，不发布 provider 请求体或凭据。

`uploadAttachment` 在 `file` slot 接收原生不可变二进制资源，参数包含 `sessionId`、文件名、字节数及 SHA-256。owner 验证最多 50 MiB 的实际内容，原子写入原 SessionArtifact，返回带所属会话的规范描述。无效文件、容量占满、关闭和归档均有明确拒绝结果；原请求 Scope 保持 worker 至 SQL 完成，每个 Agent owner 只允许一个物化副本。丢回复后查询原 operation reference；重新捕获文件不能冒充原上传重放。

`send.attachments` 最多接受当前私有会话内八个不重复的上传 ID，由后端重建规范元数据，支持仅附件消息；队列正文编辑保留附件。Files 的 `readText`／`readImage` 在原 live invocation 授权后解析附件 URI、短前缀和续读地址，不把附件地址当作工作区路径。文本读取最多 10 MiB、只接受无 NUL 的 UTF-8，有界页预留服务外层空间以保留模型可用的 `next`；图片沿原 5 MiB 订阅读取，PDF 解码仍不支持。普通产品的选择器、剪贴板、持久草稿和附件管理已接入此服务链路。

会话归实际 Root、deployment 与插件 instance 所有，包版本和 activation 变化不创建新的数据所有者。会话保留对话元数据，每次 canonical invocation 记录实际选中的 binding；同一会话后续发送可以换模型。单次运行固定 provider 身份、原配置与真正执行参数，改名、测试、目录描述与其它模型不会中断它；endpoint、协议、预算或所选模型成员变化则拒绝后续步骤。凭据、请求头与网络策略继续由原配置 owner 提供。业务 ID 由原始 operation reference 派生；Host journal 内的 UUID 冲突仍然拒绝执行。卸载会请求停止并等待原始任务和配置 SQL 所有者清理；清理未确认时继续持有容量，NativeOwner close 返回失败，应用继续保留 Root fence。

`execution` 按 `sessionId` / `invocationId` 读取精确物理执行的状态与执行内事件计数 `highWater`，不会转到会话最新一轮。`resume` 绑定该原执行及计数，并显式选择本次模型，可覆盖思考策略；只允许已封口的失败或取消执行。原 Engine 验证有界谱系、工作区身份、回放与当前工具，未结算的副作用继续阻止恢复。已结算的服务 Unknown 结果保留原 operation 引用及未知状态。原已准备输入和已完成工具结果只作回放，不重新派发旧调用。后续执行复用原 worker、容量、实时服务权限和 canonical opening 回执；繁忙、过期、归档或不安全请求返回 `parked`。读取回执不授予执行权限，丢回复仍查询原 operation reference。本次服务接口不自动安装产品恢复按钮或 WorkHub 编排。

`history` 首次固定已提交的原始事件 `through` 水位。准备工作有界；重试保留同一 position 和返回的水位。`start` 从头向前扫描，`tail` 读取末尾；`before` 在真实 sparse transcript row 的 UTF-8 字节游标处排他结束，`after` 从该游标处包含开始，两者必须携带原固定水位。Ready 始终返回 `before`、`after`，只有 `null` 表示该方向结束。反向窗口也按源顺序返回，每个窗口合计最多 8192 UTF-8 字节、两块节选；实际序列化结果还必须通过 64 KiB 检查。不发布附件或 provider 请求体。`event` 用 inclusive 原始事件编号定位执行书签，不伪造 transcript cursor。活动 part 提供原 start sequence，用于完整重放前缀，thinking 不当作回答。规范历史变化有显式 delivery invalidation。每次读取重新检查私有会话范围。公开 Rust 协议类型直接生成插件 SDK。

初版产品组合见 [agent-product](../../packages/agent-product/README.zh-CN.md)。完整工具与连接设置流程仍需继续实现；本 crate 是执行服务，尚不是完整 agent 产品。

`maka.workhub.api` 导出同一 `agent` 服务及 `workhub`
（`maka.workhub.Coordinator` v1），内部复用原 Agent 执行器和生命周期，
使用固定协调提示与独立稳定 Session namespace。配置为 `{agent, workers}`，
agent 使用原 Agent schema，workers 最多选择八个显式 Agent import alias；
工具导入及 native grants 独立声明。
`workhub.coordinator({})` 只读指定协调会话；
`workhub.resolve({workspace: directorySelection})` 先冻结首个创建意图，
并发或重新激活后沿原 operation 身份解析同一会话。后续 workspace 参数不会
移动已有或待创建的协调器，原目录变化会拒绝创建。普通 Agent export 提供
模型选择、消息、历史、队列、停止与恢复；其它 Session 不改变指定协调器。
协调行为随准入消息冻结，跨 activation 的排队消息保留原行为来源。
选择 workers 后，私有 `workhub_tasks` 协调工具可列出 worker 与会话，向已有
worker 会话路由原用户正文和有界说明，并查询或推进该原任务。派发前持久冻结
来源、worker binding、子 operation 和完整输入；原 caller intent journal 保存
完整 operation reference。Unknown 不重发，替换 binding 不接管旧任务。
只读 `workhub.tasks` 返回有界页，`workhub.task` 读取原接纳回执及可用的规范进度；
已接纳不代表已完成。排队的协调消息也冻结 worker binding，变化后准备失败。

`agent.work({sessionId, target, cursor})` 从原 invocation 或排队消息沿规范延续谱系
读取准确执行、待处理交互及有界终态答案，三者来自同一快照。答案游标绑定 invocation
和 UTF-8 字节偏移；延续变化返回 `changed`。无关会话活动不替换结果；共享 Run 明确
标记且不授予整轮控制。`execution` 仍只表示精确物理执行。

私有 `wait` 观察已接纳任务，直到完成、需要输入、暂停或进度不可用，然后把原任务
交回本轮模型。取消只关闭观察，并等待实际清理，worker 独立执行继续。会话发现每轮
从上个 ID 刷新，属于实时浏览，不承诺跨轮次目录快照。

`control` 派发前冻结停止、恢复或队列撤回；`controls`、`inspect_control` 和
`advance_control` 沿原命令恢复。公共 `workhub.controls` / `workhub.control` 只读。
队列竞态不会将撤回升级为停止后来的 Run；请求停止不代表物理结束，Unknown 不重发。

worker 重新激活后仍可恢复原接纳/控制回执，但新 binding 不读取旧任务进度。
取消沿原 native 清理时限处理，未确认清理继续持有 Host 工作及 Root fence。
跨会话附件转移、新 worker 创建与普通 WorkHub 产品界面已接入；协调会话结束后的结果通知仍待完成。

`maka.workspace.protocol-processes` 是独立显式原生实现，导出 `protocol`
（`maka.workspace.ProtocolProcesses` v1），复用 workspace Processes 的安装配置、
沙箱和权限准备。`run` 直接捕获绝对 executable、argv、env 与已授权会话的目录句柄；
原 mutation 一直等待实际进程退出，progress 先返回 `started {id}`，随后传递每块最多
8 KiB 的原始 stdout/stderr 字节。`input {id, bytes}` 等待实际写入；`bytes: null`
关闭管道后才确认。每次输入都要求同一实际入站 activation 与 Session、活动的 Agent
operation、当前权限及独立 method grant，句柄本身不授权。原生调用方可持有独立
NativeRequest 跨轮次复用，但卸载时必须取消并等待 finish。权限变化或会话退休也会
停止空闲进程；实际终止/reap 后才清理原 Session 资源记录。未知结果查询原 operation。
下述 ACP 后端通过此传输调用独立授权的文件／表单服务。

原生实现 `maka.agent.acp` 通过普通 Executor 契约导出 `executor`，配置包含 ACP
`agent` 启动描述和可选请求权限 `permissions`。`processes`、`files`、`forms`
三个 import 分别绑定一个 ProtocolProcesses、Files、Forms 服务，并独立授权。
启动权限仍须在安装上限内经过 Session 同意；后续 invocation 输入可再次请求所需权限。
外部代理的权限表单本身不授予 Host 文件或网络权限。

适配器核对真实入站 invocation 与工作区，以实际调用方、Session 和原生部署命名空间
确定会话归属，复用共享 ACP v1/v2 journal 和投影。原生 owner 跨 prompt 持有并清理
独立进程请求；SDK 收到回复后暂停传输轮询也不阻止已接纳 stdin 写入结算。
普通产品后端选择器与不可变附件传输已接入。


Executor 配置可用零起始 `member` 选择命名 import 的确切成员（默认 `0`），
允许显式有界多绑定，不增加全局发现目录。准入捕获该成员的实际 binding 并独立授权。
member 随原排队执行依据冻结，后续重排不能把已接纳消息改派到其它后端。


Executor 配置可另给 `setup: {import, member}`，绑定确切的 `maka.agent.ExecutorSetup` v1。
Agent 以 `executors.setup` 返回已配置名称，`executorSetup {executor, action}` 代理原 mutation、
有界进度、回执和清理。ACP 同时导出 `executor` 与 `setup`；设置动作是 `check` 和
`authenticate {methodId}`，共享 ACP v1/v2 实现。检查返回方法，登录返回确认或未确认结果，
`authorization_url` 进度仅携带经验证的原始 HTTPS 链接；需要尚未支持终端的登录方法只展示，不执行。

ProtocolProcesses 的 `launch` 须独立授权并显式配置 `setupDirectory`。它使用已持有目录、
安装文件／网络上限和当前 Host 网络路由，始终排除 Host 私有 Root。stdin 只接纳原调用方 activation
及应用请求；不构造 Agent invocation 或 Session 进程记录。原 mutation 持有进程至实际退出，
取消或进度不证明登录或清理完成，恢复查询不会重新认证。

`recall` 仅搜索当前 Agent 命名空间内的会话（包括归档），复用原有 Unicode 字面检索、最近 200 个会话上限和规范历史快照。普通 Recall 页面支持标题／正文、UTC 日期、会话筛选，并定位原消息。显式 `recall: true` 为模型提供 `Recall`、`RecallMore`、`RecallMaterial`，冻结的定义参与正常运行恢复。工具使用 Engine 的真实调用上下文及原生激活所持请求，每页复核活动 invocation，并排除当前轮。附件先在 SQLite 中复制到当前会话，再复用有界文本／图片读取；原请求资源和 worker 容量持有到复制完成。
