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

配置仅声明 1–16 范围内的运行容量和最大步骤数。每次 `send` 显式携带 `model`：连接 ID、所观察的 revision 与模型 ID；首次 SQL snapshot 在准备运行前核 revision 和启用的模型成员关系。密钥、请求头、endpoint 与网络设置由实际 Root 的配置存储持有。每个逻辑步骤读取指定连接；不可用时不回退到其他账号或模型。凭据来自配置存储真实的认证回执。当前服务执行空工具目录的文本对话。

`create`、`send` 是持久化 mutation。`send` 授权指定账号/模型上的完整有界运行，返回已接纳的 invocation ID。返回前，原始 NativeOwner 已持有真实运行句柄及非等待式容量 permit；请求结束或界面关闭不取消已接受的工作。`stop` 指定准确 invocation，迟到的停止请求不会取消后继运行。`list`、`observe`、`history`、`watch` 从 canonical session 元数据、同快照的 run/活动 part 身份、文本历史分页和有界投影读取内容，不发布 provider 请求体或凭据。

`uploadAttachment` 在 `file` slot 接收原生不可变二进制资源，参数包含 `sessionId`、文件名、字节数及 SHA-256。owner 验证最多 50 MiB 的实际内容，原子写入原 SessionArtifact，返回带所属会话的规范描述。无效文件、容量占满、关闭和归档均有明确拒绝结果；原请求 Scope 保持 worker 至 SQL 完成，每个 Agent owner 只允许一个物化副本。丢回复后查询原 operation reference；重新捕获文件不能冒充原上传重放。

`send.attachments` 最多接受当前私有会话内八个不重复的上传 ID，由后端重建规范元数据，支持仅附件消息；队列正文编辑保留附件。Files 的 `readText`／`readImage` 在原 live invocation 授权后解析附件 URI、短前缀和续读地址，不把附件地址当作工作区路径。文本读取最多 10 MiB、只接受无 NUL 的 UTF-8，有界页预留服务外层空间以保留模型可用的 `next`；图片沿原 5 MiB 订阅读取，PDF 解码仍不支持。本阶段只提供附件输入的服务链路，普通产品的选择器、剪贴板、草稿持久化和附件管理仍待接入。

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

`agent.messageExecution({sessionId, messageId})` 从消息的规范所有者及延续谱系
读取 pending/cancelled/delivered，标记共享 Run，不跟随无关的会话最新活动或授予
控制权限。直接启动的任务读取准确 invocation；worker 重新激活后仍可恢复原接纳
回执，但新 binding 不读取旧任务进度。取消沿原 native 清理时限处理，未确认清理
继续持有 Host 工作及 Root fence。跨会话附件转移、新 worker 创建与控制、结果回流
及 WorkHub 产品界面仍待完成。
