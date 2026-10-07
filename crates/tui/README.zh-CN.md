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

# Maka TUI

[English](README.md)

Runtime Host 的终端客户端，通过 `maka tui` 启动。

侧栏按工作区组织会话，**待处理**直接筛选目录，保留当前对话。窄窗口或专注模式下，`Ctrl+B` 通过浮层打开导航，`Esc` 返回原草稿。Tab 跨过列表和字段，方向键在列表内移动；`F1` 在当前页面上打开帮助，Host 连接详情位于设置中。

`Enter` 发送消息，模型工作时排入下一轮。支持增强键盘协议的终端用 `Shift+Enter` 换行；传统终端（包括通过 WSL 使用的 Windows Terminal）可用 `Ctrl+J`。粘贴多行文字不会自动发送。`Ctrl+K` 打开命令，`Ctrl+F` 查找对话，`Ctrl+N` 新建会话；弹层和搜索优先处理各自的按键。

原生和插件控件统一用底色表示当前值或已打开项，用下划线表示键盘焦点，悬停不会
让另一行看起来被选中。需要审批或回答的会话会在侧栏显示「待处理」；composer
上方同时提供请求入口，点击或按 `Ctrl+G` 进入审阅，不会自动批准。向上阅读历史
时，「回到底部」也显示在 composer 上方。

Read 工具行显示紧凑资源地址，展开参数保留原始输入。`Read.path` 除完整地址外，
支持 `task:<ID前缀>`、`attachment:<ID前缀>` 和 `archive:<事件ID前缀>`。
建议保留 `attachment-` 等固定 ID 前缀，再取后面 12 个字符；精确 ID 优先，
否则必须在当前会话可读资源中唯一匹配，有歧义时补长。后续分页完整传回 `next`。

Composer 支持 `Ctrl+V` 或「+ → 从剪贴板粘贴」，读取 macOS、Windows、Linux
本机剪贴板中的图片、复制的文件或文字。终端传入的完整绝对路径或本地 `file://`
URL 会转为附件，支持引号、转义空格和多文件；其他文字保留在草稿中。
文件沿用附件校验与上传队列，无效路径、目录、超限文件会显示具体附件错误。
剪贴板图片随本地草稿保存，移除并成功保存草稿后回收。`Cmd+V` / `Ctrl+Shift+V`
由终端处理；终端不转发图片数据时使用 `Ctrl+V`。SSH 下读取的是 TUI 所在主机的剪贴板。

跨屏选择正文时，单击起点，滚动后用 `Alt+点击` 选择终点（macOS 使用 Option）。
普通点击清除选区，`Shift+方向键`、`Shift+PageUp/PageDown` 也可扩选。
终端转发 Shift 鼠标事件时，`Shift+点击` 使用同一套选区。Maka 在 Unix 下运行时
通过 `XTSHIFTESCAPE` 请求转发，退出时释放；部分终端仍会保留 Shift 原生选区，
这种选区可能包含侧栏，Maka 无法清除。`Alt+点击` 可避开该 Shift 策略。
Otty 1.5.4 可额外设置 `mouse-shift-to-select = false` 来转发 Shift；这是可选的
终端全局偏好，使用 `Alt+点击` 不需要它。

侧栏的「搜索」打开命令面板，按名称、工作区、ID 和目录展示的标签扫描 Host 的分页会话目录；宽泛搜索可选择「更多匹配会话」继续。模型选择器可跨目录分页搜索已启用的聊天模型。

Host 尚无模型连接或默认模型时，工作区首页会显示相应的设置入口；此时 `Ctrl+N` 进入缺失的设置步骤。现有会话仍可打开。

Chat 与插件 Transcript 共用 Markdown、选区、搜索和流式呈现。新增正文通过短暂明暗渐变显现，不延迟已收到的内容，也不改变排版；减少动态效果和终端默认配色下直接显示。

设置 → 插件可管理本地包、实例、配置与服务绑定。修改提交前需要审查，激活状态自动更新；配置草稿仅保存在内存中，结果不确定时不自动重试。

会话的**技能库**支持导入本地 Markdown、安装来源、审查受管更新及确认目录删除。WorkHub 可配置新任务、修复失效模型，并在协调会话中直接发送消息；其它受管会话须由所属行为明确开放原生输入。Rust 与 JavaScript 插件通过同一套公开组件提供页面、设置、面板与嵌套视图。

- `maka-client` 负责传输和协议校验；TUI 负责呈现和输入。
- 提供商设置使用公开的描述、配置与认证契约。
- 本地状态保存草稿和恢复身份，不保存认证输入。写入结果不确定时查询事实，不自动重放。
- Fluent 文案覆盖英文、简体中文和繁体中文。

新的应用呈现器由 `maka application run --root PATH --deployment UUID` 启动，
直接消费共用 `maka-ui` 内核；[同包双 entry 示例](../ui/README.zh-CN.md) 分别选择 Desktop/TUI。
Tab/Shift+Tab 切换焦点，Enter/Space 或左键激活按钮，Ctrl+C 退休界面。
实际服务 worker 在终端恢复后仍由 owner 持有直到真实清理完成；失败保留应用 fence，
不靠丢弃 owner 返回。现经平台自己的纯场景编译器和共用 `NativeView` 路径支持
Stack/Text/Button/导入组件及编译的 Label recipe；场景语言拥有终端几何，不继承桌面布局。
原生编辑、滚动与选区由保留式 presenter owner 实现；交互恢复尚待实现。上文 `maka tui` 业务客户端仍待迁移删除，
不是新架构的兼容承诺。

应用启动器另支持 `--connections FILE`，选择[明确授权的本地 Host 连接](../application-bridge/README.zh-CN.md)。
终端恢复先于服务清理，进程退出还要等待 caller SQL 和 Host 的真实关闭确认。
前台执行丢失不能被认定为终端或 module 清理成功。


编译的终端原生布局也可使用作用域内的 `Slots`：`measure` 测量已提交子组件，
`paint` 经同一 presenter 绘制并保留真实输入区域与挂载身份。测量可重复；重复实际
放置会在 flush/输入区域发布前拒绝待显示场景。原生布局自行选择终端几何，
不复制桌面像素，也不另建挂载运行时。

`maka.ui.input` 提供固定的 `TextInput` 与 `SecretInput` exports，共用 `Editor*`
命令、观察与捕获协议。单行 props 提供完整 `InputAppearance`、标签、placeholder、
可编辑性和 tab 顺序。控件初始为空，首次元数据观察允许按 revision 发起 prefill 命令。
控制字符在剪贴板或 IME 规范化前按整次输入拒绝；Reset 清空原生撤销历史。

SecretInput 在原 owner 生命周期中保持掩码。AccessKit password 节点不发布明文 value
或 TextRun；平台 surrounding-text 查询按原 UTF-16 单位返回掩码。捕获仍为原不可变
UTF-8 bytes；掩码不会撤销显式授予的 Reader 能力。普通设置组合应将 secure capture
绑定给可信 setter，并不绑定 Reader。原生 Enter 在 keybinding 前 consume，随后发布
`submitted` 业务事件，不返回同步原生 decision。TUI 将它与可重试的观察/回执分开发送。
OS 输入法与屏幕阅读器仍需逐平台验收。

保留式呈现是原 frame 的不可变投影。场景 Stack 可设置 `visible: false`；原生父组件
通过 `NativeView::with_hidden_slots` 指定已声明的精确 slot key。全部子组件保持挂载，
隐藏子组件测量尺寸为零，不发布焦点顺序和输入目标；场景 Stack 也排除它们的间距。
原输入 buffer、选区、撤销历史、
capture 能力与 command receipt 继续由同一 owner 持有并结算。显示时在原 owner 上
恢复当前 props。Desktop 在下次 paint 前失效机械输入准入，并拒绝保留的子组件 render；
TUI 仅在成功 flush 后切换几何准入。自定义原生布局也通过公共 Slots 边界执行隐藏规则。
这一投影不退役资源，也不增加第二份可写草稿。


可信 Presenter owner 可在不借用前台状态的情况下读取确切 committed update，
用显式 remote registrations 接纳，并观察 `deployment_status`。pump 在 paint 外
推进计划，即使收敛失败也先撤销退休控件的原生输入。未变控件保留原草稿、选区、
焦点和撤销 owner；独立插件初始化失败仅记录有界诊断，健康终端继续运行。
根退休仍需要原终端恢复回执，由 `requires_surface_replacement` 报告；启动器
先恢复旧终端，再为新 launch 进入终端。生产更新管理入口仍待接入。

根交接保留同一 Application 和已接受任务的原观察器。adoption 仅在退休原 live root
时记录替换，无关 adoption 不会把原 realm fault 变成交接。`surface_closed` 接收
确切原 Mount 及实际恢复结果；重复或迟到旧回执不能关闭新根，清理失败保留 fence。
`surface_recovered` 是同一失败物理界面的显式成功恢复重试。确认旧清理后，
`try_replace_surface` 等待相关 provider 结算，再用全新原生状态挂载 desired launch；
未替换 provider 的原已接受任务可以与新根并存。不把旧草稿、焦点和选区默认为
新实现的数据。

可用的 `maka.ui.file` / `FilePicker` 将用户输入的绝对路径保存在原生编辑器内。
原生打开/Enter 手势及其已提交的私有意图才启动有界后台读取，props 不能指定本地文件。
Widget 创建获得 `NativeContext`，其中包含呈现器共享预算与原资源组。退休时关闭
原工作组的新 admission，仍在工作的 ticket 会保留原 owner；Pending 不提交失败
kernel 回执，已知失败则保持 sticky。原生 recipe 类型切换也必须等待原任务，包括
A→B→A 变化，之后才创建新的原生 owner。
