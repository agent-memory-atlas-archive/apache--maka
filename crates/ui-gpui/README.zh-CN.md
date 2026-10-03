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
纯 `scene::compile`；动态组件 entry 必须声明精确匹配的场景语言。动态 JSON 场景提供原生 flex 布局、
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
