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

# Computer Use

[English](README.md)

通过独立 JavaScript REPL 提供 Maka 原生应用和浏览器 API。关闭 Code Mode 后，`cua_repl` 与 `cua_reset` 仍可使用。固定的 Cua 驱动版本见[工作区配置](../../Cargo.toml)，对象 API 见 [src/api.md](src/api.md)。

Session 管理 REPL 变量、目标绑定和观察，Host 管理权限检查与原生驱动。每次操作使用当前权限，保留 JavaScript 对象不会保留授权。REPL 不提供环境级 Node、文件系统或任意网络 API。

新 REPL 先选择入口，阅读自动输出的状态后再操作：

```js
const app = await cua.getApp("Calculator");
```

后续调用使用该观察中的索引。`cua.getState()` 列出可用界面，应用与标签页观察会自动输出。内置 [maka-cua skill](skills/maka-cua/SKILL.md)说明目标选择、截图和输入用法。

## 桌面与浏览器配置

`desktop` 默认为 `auto`：WSL 中选择 Windows 主机，其他环境选择本地桌面。`local` 在 WSL 中选择 Linux，`windows` 要求 Windows helper。原生分发通过经过验证的缓存准备匹配版本的 helper。此模式需要 WSL 互操作及交互式 Windows 桌面。

浏览器提供方是显式配置的 CDP 连接，使用已有 profile。没有配置提供方时，可操作原生浏览器窗口。配置与平台方法见 API 说明；缺少提供方不会触发重启浏览器或开启调试。

## 生命周期

重置清除本 Session 的绑定，不关闭用户应用或标签页。导航、窗口变化和新观察可能使原输入目标失效。取消或超时后需要重置，结果不确定的动作不会自动重放。关闭流程等待已接受的原生工作完成清理。

原生依赖在 macOS 上需要 Xcode Command Line Tools，在 Linux 上需要 X11 开发库。输入与捕获行为应在目标桌面验证；REPL 单元测试不能证明原生焦点或无障碍行为。
