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

# maka-model

[English](README.md)

通过提供商适配器构造模型请求并接收流式响应。`ModelExecutor` 管理请求，`Conversation` 构造会话输入，`reasoning` 处理推理配置。凭据通过 `AuthResolver` 提供，调用方传入已授权的网络与计量上下文。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
