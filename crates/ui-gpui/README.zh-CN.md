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

# GPUI 组件 adapter

Longbridge GPUI Fast/Kit 的公开原生呈现 ABI。`maka-ui` 内核提供注册、组件事务与
挂载身份；本库提供不可变 `Widget` 描述、原生输入绑定、作用域内 `Slots` 以及可选
Label/Editor 编译实现。不持有 application、window、Host、产品导航或第二套挂载注册。

实现 `NativeComponent<dyn Widget>`，向 `NativeRegistry` 注册实际契约，并返回
`NativeView::new(Rc::new(recipe), input_schemas, slots)`。产品包显式声明 imports，
组合选中后才激活编译实现。`registry()` 提供内建 Label/Editor availability，可信
launcher 接收前可添加其他实现。

`registry()` 也以 `scene::language()`（`maka.desktop.scene`、version 1）安装平台自己的
纯 `scene::compile`；动态组件 entry 必须声明精确匹配的场景语言。动态 JSON 场景提供原生 flex 布局（`style.wrap` 明确控制自动换行）、
作用域内字体与颜色、鼠标和键盘按钮、焦点样式及纵向滚动。导入组件仍归原父组件的
slots；局部 key 在布局改变时保留同一机械焦点/滚动 owner，删除或换类型即关闭旧输入
准入。编译器在回调 deadline 内检查形状、样式和数值限额；原 JSON 先验证，再缩窄为
原生浮点数。


`Canvas` 在 `contents` 中提供有界 Rect/Line/Text 绘制命令与显式按钮型区域。
插件定义绘制、鼠标、键盘与放置的同步规则；原生 Owner 提供焦点、辅助功能激活和
可选拖拽轮廓。绘制不执行插件。Down/AX 保留原实际绘制的输入；持续手势与提前的
KeyDown 拦截读取同一 Owner 的最新 dispatcher。未订阅的 move 完全在原生层完成。
区域删除、准入或交互路由改变、失焦或窗口失活都会取消原手势。
几何先以有界 f64 验证，再转原生数值；指针位置使用最后实际绘制的 Canvas 坐标，
拖拽差量始终使用窗口坐标。颜色为 RGBA，线宽与单行文本均有界。
可用 `canvas` 导出模式运行普通动态[工作流示例](../ui/examples/counter/canvas.mjs)。

`Input` 派发返回已提交的 `InputDecision`。`bind` 仅在当前原生监听器内阻止默认行为或传播；批次失败时拒绝默认行为并停止传播。它不能撤销更早的原生 action 或 mouse-down 焦点。Owner 观察直接调用 dispatcher，不把标志应用到已完成输入。Desktop 在内核提交后同步原生副作用；同步失败单独诊断，仍保留已提交决定。

`Widget::create` 返回呈现器拥有的资源状态；`capture` 捕获有界请求私有快照，
`commit` 在绘制前消费已提交命令。`deactivate` 在逻辑退休时释放原生焦点/输入，不等待
旧绘制资源销毁。`render` 读取 owner、获得准确的子组件 `Slots`，返回 Result 将放置失败
交给 presenter。
recipe 不能持有 Entity、Input、Slots、订阅或资源凭证。保留的 Element 和虚拟列表
回调可以 clone Slots，只保有原子树与窗口的真实清理职责。移除或重建 owner 后，
旧 handle 明确拒绝；缓存的 recipe 与 Slots 不能在已替换的 frame revision 上重新渲染。布局测量可以重复，一次成功显示每个 mount 只可放置一次。
原生不变量错误属于受信代码故障，不提供沙箱隔离或假的清理回执。

参见[桌面所有权](../desktop/README.zh-CN.md)与[共用内核](../ui/README.zh-CN.md)。

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

元数据事务被拒绝后，原 native owner 保留待发布观察。后续应用同步即使
component frame 未变也会重新发布，不重执行已完成的原生命令。

保留式呈现是原 frame 的不可变投影。场景 Stack 可设置 `visible: false`；原生父组件
通过 `NativeView::with_hidden_slots` 指定已声明的精确 slot key。全部子组件保持挂载，
隐藏子组件测量尺寸为零，不发布焦点顺序和输入目标；场景 Stack 也排除它们的间距。
原输入 buffer、选区、撤销历史、
capture 能力与 command receipt 继续由同一 owner 持有并结算。显示时在原 owner 上
恢复当前 props。Desktop 在下次 paint 前失效机械输入准入，并拒绝保留的子组件 render；
TUI 仅在成功 flush 后切换几何准入。自定义原生布局也通过公共 Slots 边界执行隐藏规则。
这一投影不退役资源，也不增加第二份可写草稿。

`Popup` 是锚定的非模态场景节点，包含 `trigger`、`content`、`open`、
`label`、`anchor`、`offset` 和 `style`。仅触发器占正常布局；style 只作用于浮层内容，
受实际窗口边框和边距约束。超大 border/padding 按实际尺寸缩减，每轴至少保留一半
内部空间用于可滚动内容。内容保留原滚动与焦点 owner。两个分支永久声明 slot；
关闭只隐藏内容，不退役草稿或工作流。Escape 和外部鼠标输入向 popup key 发出
`{kind: "dismiss", reason: "escape" | "outside"}`，由插件提交打开状态并决定传播；
事务被拒绝时，原生层不会自行关闭。触发器仍执行普通输入。Deferred 子浮层使用
真实 hit-test 的原 dispatch 祖先，包含缓存和 slot，不会误判为父层外部。
每次打开隔离旧监听器；关闭后在完整 presentation pass 结束时恢复可用的 weak
前焦点，已有其他控件取得焦点时不抢回。初建隐藏场景 owner 不取得焦点。
页面隐藏当前焦点控件但浮层仍打开时，沿原生 focus-lost 路径恢复最近存活的 scope，
保持 Escape 可用。

Embedding 通过 `NativeRegistry::with_render_depth_limit` 声明 GPUI 的实际 deferred
深度上限；scene 和原生工厂通过 `NativeView::with_render_depth` 报告本地成本与
各 slot 外围成本，原生编辑器包含其菜单预算。共享内核在提交前校验整棵拟提交树
的可见路径，包含未更新祖先和仅更新的子组件；隐藏 slot 保留 custody，但不消耗
绘制深度。这是 renderer 派生元数据，不是内核布局词汇或插件可写配额。

Scene 文本使用稳定的可访问文本节点，保留完整 value。`Style.truncate: true` 按原生测量显示单行省略号，不截断可访问文本或按钮名称；可用空间由宽度与 flex 约束决定，不由 JavaScript 估算 Desktop 字形宽度。

`maka.ui.markdown` 通过普通 registry 导出 `Markdown`。原 Kit TextViewState 唯一拥有
解析、选区、链接 occurrence 与焦点；不可变 recipe 不持原生资源。source 扩展及主题
变化保留原 owner，主题显式提供六级标题、正文/代码字体、颜色与 logical px 段距。
隐藏、禁用、退休在下一帧前撤销输入并释放内部焦点；原 surface BaseRoot 提供窗口选区层。
链接批次先于原 Kit 默认行为，失败拒绝；用户激活仅允许 HTTP(S) 打开。文档图片提供明确
失败 source，不能隐式读取网络或文件；带资源图片需显式能力实现。
