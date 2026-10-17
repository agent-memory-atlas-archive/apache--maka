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

# maka-workspace-api

[English](README.md)

定义工作区文件、进程、终端、协议进程和 Jev 的共用契约。原生提供方与开发工具使用相同声明。请求路径和工作区标识用于描述选择；提供方仍需检查实际绑定目录和调用权限。

Terminal 契约还提供 `readOutput` 和 `watchOutput`，按顺序读取原始 PTY 字节（Base64）及尺寸变化事件。每次读取返回一个事件；缓冲最多保留八个事件，每个数据事件最多包含 16 KiB 字节。`after` 游标必须与原始目标一起保留。`Gap` 会结束订阅并报告最早保留的事件；如需继续，应显式从 `firstAvailable - 1` 恢复。保留的尾段不能还原完整终端状态。原始输出是临时数据：退出或重启后新接入会报告 `Unavailable`，屏幕及历史读取仍可使用。`Closed` 表示输出结束，不代表进程清理已确认。输出观察需要显式方法权限，不需要控制器。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
