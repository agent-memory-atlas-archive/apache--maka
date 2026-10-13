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

布局与绘制读取已提交的数据。JavaScript 组件回调在后台 worker 上执行，编辑器缓冲区、焦点和选区由原生所有者管理。原窗口所有者在唤醒时发布就绪候选，无需等待绘制。必要且可失败的原生准备先于原子发布，原生命令只在发布之后执行。

每个原生挂载独立持有 `LiveInput` 与发布队列。原始回执保留精确值，直到原逻辑提交确认。挂载固定一种 recipe 实现类型，切换实现需要新子键或重新挂载。准备失败保留已提交的所有者与几何。挂载退役后，需要释放实体、订阅和保留帧资源才能完成关闭。窗口启动由产品的 desktop crate 负责。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
