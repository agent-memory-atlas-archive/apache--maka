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

# Agent 主界面

`AgentScreen` 负责会话导航、对话、composer 和产品页面。会话的 `workspace` 仍指 Host 工作目录。组件既能拥有前台 surface，也能嵌入 WorkHub。

| 模块 | 职责 |
| --- | --- |
| `index.ts` | 组件构造、初始状态和生命周期接线 |
| `state.ts` | 唯一的组件状态、请求类型和导航书签 |
| `actions.ts` | 可用命令与明确的服务调用意图 |
| `results.ts` | 查询结果、修改回执及实时事件 |
| `messages.ts` | 有界实时文本和时间线展示投影 |
| `events.ts`、`component-events.ts`、`raw-events.ts` | 同一候选帧事务内的统一路由、子组件回调和原始命令 |
| `frame.ts` | 根组合、对话框及订阅 |
| `conversation-view.ts` | 对话、输入区和 footer |
| `task-overview.ts`、`outputs-view.ts` | 真实任务状态与有界产物／验证视图 |
| `navigation-view.ts`、`header-view.ts`、`pages-view.ts` | 导航、标题栏和保留状态的产品页面 |
| `tui.ts`、`desktop.ts`、`package.ts` | 展示目标适配和插件声明 |

各模块共用原有 `State`，没有独立 store 或额外事件循环。视图函数返回原生场景；候选帧和调用意图仍在组件边界一次发布。自定义 footer 是绑定的组件，详见[运行时 UI 指南](../../../skills/maka-plugin-authoring/references/runtime-ui.md)。

「持续目标」命令打开当前会话的目标面板，包含目标内容、轮数上限和可选 token 阈值。原生编辑器提供不可变快照供创建前查看；确认内容保留所选模型、工作区和权限边界。控制操作使用展示时的目标版本，收到新版本后会清除过期确认。操作恢复读取已保存的原始回执，不会再次提交目标。
