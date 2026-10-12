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

# maka-ui-runtime

[English](README.md)

管理组件激活、状态、输入分发和帧的原子更新。`Application` 执行经过校验的 UI 组合，`NativeRegistry` 提供编译进程序的工厂和场景编译器。需要 JavaScript 时，由产品通过 `backend` 注入。

回调提交状态、帧和一次性动作的候选结果，运行时校验整个批次后再提交。渲染器消费已提交帧并管理平台资源，不在布局或绘制时执行 guest 代码。终端和桌面使用各自的场景语言。

原生编辑器保留缓冲区并提供有界捕获。异步变更沿用原始回执和调用方身份。替换组件必须等待已接受工作和原生资源完成清理，丢弃帧或结果观察者并不足够。相关实现见 `actions`、`operations`、`editor` 和 `checkpoint`。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
