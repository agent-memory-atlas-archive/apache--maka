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

# maka-render-gpui

[English](README.md)

使用 GPUI 渲染已提交的 UI 帧。`Widget` 和 `Slots` 支持原生组件提供元素和子挂载，`scene` 编译桌面场景语言。适配器还管理原生编辑器和输入绑定。

布局与绘制读取已提交的数据，组件回调在绘制之外执行，编辑器缓冲区由原生所有者管理。挂载退役后，需要释放实体、订阅和保留帧资源才能完成关闭。窗口启动由产品的 desktop crate 负责。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
