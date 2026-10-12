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

# maka-network

[English](README.md)

为 HTTP 和 WebSocket 连接应用不可变的出站路由策略。`Policy` 描述路由，`proxy` 处理代理配置，`probe` 检查连接。调用方仍需取得网络能力；选择路由不会授予目标访问权限。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
