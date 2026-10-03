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

配置只选择连接 ID、slug、模型和编译提供的 API provider，并声明 1–16 范围内的运行容量和最大步骤数。密钥、请求头、endpoint 与网络设置由实际 Root 的配置存储持有。每个逻辑步骤读取指定连接；不可用时不回退到其他账号或模型。凭据来自配置存储真实的认证回执。当前服务执行空工具目录的文本对话。

`create`、`send` 是持久化 mutation。`send` 授权指定账号/模型上的完整有界运行，返回已接纳的 invocation ID。返回前，原始 NativeOwner 已持有真实运行句柄及非等待式容量 permit；请求结束或界面关闭不取消已接受的工作。`stop` 指定准确 invocation，迟到的停止请求不会取消后继运行。`list`、`watch` 从 canonical session 元数据和有界投影读取内容，不发布 provider 请求体或凭据。

会话归实际 Root、deployment 与插件 instance 所有，包版本和 activation 变化不创建新的数据所有者。会话只冻结模型/provider 身份，不冻结启动器容量参数。业务 ID 由原始 operation reference 派生；Host journal 内的 UUID 冲突仍然拒绝执行。卸载会请求停止并等待原始任务和配置 SQL 所有者清理；清理未确认时继续持有容量，NativeOwner close 返回失败，应用继续保留 Root fence。

完整产品界面、用户消息及历史分页、连接设置工作流和工具组合仍需继续实现。本 crate 是真实执行服务，尚不是完整 agent 产品。
