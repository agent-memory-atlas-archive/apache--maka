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

会话归实际 Root、deployment 与插件 instance 所有，包版本和 activation 变化不创建新的数据所有者。会话保留对话元数据，每次 canonical invocation 记录实际选中的 binding；同一会话后续发送可以换模型。单次运行固定 provider 身份、原配置与真正执行参数，改名、测试、目录描述与其它模型不会中断它；endpoint、协议、预算或所选模型成员变化则拒绝后续步骤。凭据、请求头与网络策略继续由原配置 owner 提供。业务 ID 由原始 operation reference 派生；Host journal 内的 UUID 冲突仍然拒绝执行。卸载会请求停止并等待原始任务和配置 SQL 所有者清理；清理未确认时继续持有容量，NativeOwner close 返回失败，应用继续保留 Root fence。

`history` 首次选择已提交的 `through` 位置，后续准备、轮询和 UTF-8 字节游标继续使用该位置。每页最多两块各 4096 字节的文本，为 JSON 转义保留空间；不发布附件或 provider 请求体。`start` 从头扫描；`event` 为 inclusive 原始事件编号，由 Host 转换成 sparse transcript row；返回的 `cursor` 必须带原固定 `through` 继续。活动 part 提供原 start sequence，用于完整重放前缀，thinking 不当作回答。规范历史变化有显式 delivery invalidation。每次读取重新检查私有会话范围。公开 Rust 协议类型直接生成插件 SDK。

初版产品组合见 [agent-product](../../packages/agent-product/README.zh-CN.md)。完整工具与连接设置流程仍需继续实现；本 crate 是执行服务，尚不是完整 agent 产品。
