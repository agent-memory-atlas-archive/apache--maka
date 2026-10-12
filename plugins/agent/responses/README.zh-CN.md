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

# maka-responses

[English](README.md)

实现 Responses 协议，包括请求编码、流解码、SSE 处理和连接通道。连接属于可丢弃的传输状态；规范会话和执行事实仍归 Agent 所有者管理。入口是 `Adapter`、`request` 和 `stream`。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
