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

# 原生终端呈现

[English](README.md)

`maka-tui` 通过共用的 `maka-ui` 组件内核呈现插件应用组合，提供终端布局、输入、
焦点、滚动、选区、有界原生编辑器和不可变资源捕获。产品导航、页面、主题、
工作流状态及业务调用由普通插件定义。

用 `maka application run --root PATH --deployment UUID [--connections FILE]`
启动已校验并提交的来源。来源选择启动组件与全部能力绑定，终端适配器不自动启动
Agent，也不保存业务页面注册表。[普通 Agent 产品](../../packages/agent-product/README.zh-CN.md)
和[双端示例](../ui/README.zh-CN.md)使用与 Desktop 相同的组件内核。

Tab/Shift+Tab 移动焦点，Enter/Space 或鼠标左键激活按钮。原生 Unicode 编辑器
在兼容的组件更新及隐藏路由中保留文本、选区与撤销历史，样式由产品提供。终端
场景支持弹性视口和原生滚动策略，SourceText/ReadingArea 提供有界原文选择与复制。

Ctrl+C 退休界面。原 Guard 会检查每项终端恢复操作，即使此前已有操作失败。
原生工作、资源快照与已接受的服务调用保留原所有者，直到真实清理结束。清理失败
会保持应用封闭。`--connections` 使用显式[Host 方法及恢复授权](../application-bridge/README.zh-CN.md)，
退出等待原服务、调用者 SQL 和 Host 关闭回执。

旧业务 TUI、locale/profile 参数、主题存储与整 Host 退出界面已移除。本地化与
主题由选中的产品负责。应用 Schema 和原生组件契约由 Rust 唯一定义并生成到
插件 SDK；呈现器测试不代替完整产品功能与平台体验验收。
