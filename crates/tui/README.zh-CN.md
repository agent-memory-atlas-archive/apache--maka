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

Ctrl+Q 或未被处理的 Ctrl+C 退休界面。当前组件的快捷键优先处理这两个组合键；
捕获输入的交互终端仍接收它们。确认支持增强键盘事件的终端会等到对应松键再退出，
避免把剩余手势报告交给 shell；新的按键或鼠标按下取消待退出手势，松键丢失时两秒后
回退关闭。传统终端仍按下即退出。原 Guard 会检查每项终端恢复操作，即使此前已有操作失败。
原生工作、资源快照与已接受的服务调用保留原所有者，直到真实清理结束。清理失败
会保持应用封闭。`--connections` 使用显式[Host 方法及恢复授权](../application-bridge/README.zh-CN.md)，
退出等待原服务、调用者 SQL 和 Host 关闭回执。

旧业务 TUI、locale/profile 参数、主题存储与整 Host 退出界面已移除。本地化与
主题由选中的产品负责。应用 Schema 和原生组件契约由 Rust 唯一定义并生成到
插件 SDK；呈现器测试不代替完整产品功能与平台体验验收。

场景 Stack 可声明有界的 `shortcuts`，包含 key、modifiers、input 与 enabled。原生编辑优先；最近的已绘制声明独占未消费的按键，禁用声明也阻止继续向外。输入核验原 mount/revision 和当前已提交可见链；无焦点的根使用最后成功呈现的声明。终端中 `primary` 为 Control，无修饰键的可打印字符保留给原生输入。

原生编辑器可通过 `autoFocus` 在已提交结构路径显露且启用时提供一次聚焦机会。成功 flush 只消费原绘制帧版本和 owner 仍有效的机会，覆盖父节点改变而 child 不变的情况。未 flush 或迟到旧帧不能消费新显露机会；视口裁剪与普通重绘不会重新触发。

原生 FilePicker 支持本机绝对路径和目录浏览。使用浏览／上级按钮，随后按方向键与
Enter 或点击选中；Home/End、PageUp/PageDown、滚轮和滚动条支持有界长目录，
缩放保留当前聚焦选项可见。目录读取复用文件捕获的原生 commit／worker 生命周期，
插件只接收所选文件的元数据与不可变字节。

Composer 支持整段粘贴本机绝对文件路径，包括带引号的文件名和 `file://localhost`
地址。每批最多八个不同路径，依次读取文件；重试、跳过文件和取消只作用于原批次。
普通文字仍插入编辑器，批次未完成时 Enter 不会发送。Ctrl+V 只读取一次本机系统
剪贴板，依次优先文件、图片、文字。图片上限 33,554,432 像素，直接编码为不可变 PNG
分块，不产生临时图片文件。文字保留原插入位置，只能插入未修改的原会话草稿。
剪贴板失败须重新物理粘贴；重试只重读原选中文件。普通产品从原本地原生检查点恢复未发送的附件引用。

会话切换保留最多 32 个原生草稿编辑器及其选区、撤销记录。迟到捕获和发送确认仍定位
原 owner/revision。容量用满时只回收已观察为空的空闲草稿，否则需先发送或清空已有
草稿。原生检查点在 UI Root 中保存正文、有效选区和未发送附件引用。普通产品先核对原发送回执再恢复；未知结果不会触发重发。撤销历史只保留在当前挂载内。
