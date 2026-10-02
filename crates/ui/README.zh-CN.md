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

# 组件执行

`maka-ui` 从不可变包激活已检查的 UI entry，向共用核心发布实际组件。
前台 JavaScript 只在激活、挂载初始化和输入处理时执行；呈现器只读取已提交
`Frame`。有界批次完整校验后原子替换，每个挂载有独立状态与 revision，旧视图事件
不会调用新处理器。执行域故障撤销该域全部注册与挂载。

首个呈现协议包含 stack、text、button；挂载期间窗口配置固定。
原生编辑器/IME、子组件呈现、action import 与恢复界面尚未实现。
`retire` 只停止新输入；`close` 必须收到实际原生清理确认；`shutdown` 不丢弃
尚未确认的 mount、work 或 resource。

包驱动的桌面示例命令见 [English README](README.md)。示例与校验使用同一套 Rust
契约类型，无旧插件 SDK 或全局 service catalog。
