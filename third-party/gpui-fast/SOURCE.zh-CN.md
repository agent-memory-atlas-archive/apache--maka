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

App 提供原当前事件传播标记的只读查询。原生分发完成观察者能据此排除已消费的输入及此前已停止的观察，不复制事件状态、不新增键盘注册表。Kit 的既有 Root 默认行为可覆盖有焦点和无焦点的窗口，原生绑定及 raw 插件决定仍先执行。

原生焦点显露派生自原 Window focus generation 与已提交的 prepaint 几何。焦点子树只给原 autoscroll request 标注 generation，精确 child caret 优先于整个控件的 fallback，无关兄弟请求不能覆盖焦点目标。滚动容器计划最小且被原 native limits 钳制的 offset，仅按自身 viewport 裁剪目标向祖先传递；正常后续 frame 一起重建 paint、hitbox 与 accessibility。一次 generation 完成后不抢回用户手动滚动；pointer 移焦保持 down/up 与拖选几何。List 完整 prepaint 可见 pass 后才选 retry 请求；失败事务撤回显露几何事实，保留已经写入的 offset 计划以安排重绘。pending 时绕过 retained/layer replay。

原 TestWindow 记录 platform decoration facts、resize requests 和 client insets；TestAppContext 可改变原平台事实并请求原生重画，再观察实际平台调用。该有限 harness 接缝验证 Base Root 客户端 frame 行为，不增加第二窗口或 frame owner，不宣称 Linux OS 验收。
