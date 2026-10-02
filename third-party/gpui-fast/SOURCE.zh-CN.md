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

# GPUI Fast 平台清理补丁

从 [GPUI Fast](https://github.com/longbridge/gpui-fast/tree/e4aabe752665baabe330464892cfe6882ed3283b) 固定 revision `e4aabe752665baabe330464892cfe6882ed3283b` 保留 `gpui`、`gpui_macos`、`gpui_windows`、`gpui_linux` 四个 crate。其余 helper 仍依赖同一官方 Git revision；根 Cargo source patch 保证 Kit 和所有后端使用同一份核心类型。

`upstream.json` 列出改动前的有限原始文件清单与 SHA-256。保留上游许可和署名；Maka 新文件不进入该清单。隔离 workspace 仅规范化 helper 依赖和显式许可文件路径。

补丁增加一次性平台清理回执：只有原生销毁被确认，且后端持有的 renderer、输入与回调资源都已释放，才报告成功。macOS 的末字段 guard 跨越 NSWindow/NSView 的 superclass dealloc；Windows 在 WM_DESTROY 撤销 OLE 拖放所有权，等待 WM_NCDESTROY/DestroyWindow 完成；X11 使用 checked DestroyWindow，并释放窗口映射与回调 owner；Wayland 在 destroy requests 后使用独立 display-sync callback，不依赖已移除的 surface map；测试后端等待真实 state 析构。其它不支持的后端显式失败。未改动帧调度或编辑器刷新。

回执不承诺 GPU driver 已回收物理内存，也不保活窗口。应用 closing owner 等待该回执及真实资源 lease 共同结算。保持补丁范围有限，改变固定版本前重新评估上游。

省略无关的上游 example 程序和资源，仅保留 core 测试使用的 JPEG，并关闭 example target 自动发现。

macOS 还必须从 State 取出 AccessKit adapter，在 foreground teardown task 中、native close 之前释放。adapter 强持 content view，其 GPUI 子 view 又持有 State Arc；等到 State 最终析构才释放 adapter 会形成引用环，无法收到真实完成回执。
