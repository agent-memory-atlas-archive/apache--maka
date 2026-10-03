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

`Widget::create` 返回呈现器拥有的资源状态；`capture` 捕获有界请求私有快照，
`commit` 在绘制前消费已提交命令。`render` 读取 owner，并获得准确的子组件 `Slots`。
recipe 不能持有 Entity、Input、Slots、订阅或资源凭证。保留的 Element 和虚拟列表
回调可以 clone Slots，只保有原子树与窗口的真实清理职责。移除或重建 owner 后，
旧 handle 明确拒绝。布局测量可以重复，一次成功显示每个 mount 只可放置一次。
原生不变量错误属于受信代码故障，不提供沙箱隔离或假的清理回执。

参见[桌面所有权](../desktop/README.zh-CN.md)与[共用内核](../ui/README.zh-CN.md)。
