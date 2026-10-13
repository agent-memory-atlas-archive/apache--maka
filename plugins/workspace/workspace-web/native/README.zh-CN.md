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

# maka-web

[English](README.md)

通过作用域网络能力实现 Web 抓取与搜索。`fetch` 和 `search` 管理内容策略，`settings` 管理提供商配置，`provide` 注册 Host 工厂。该服务可以在不包含 Agent 引擎的 Host 组合中运行。

`probe` 支持进程健康检查：先发 HEAD，仅在 405/501 时回退一次 GET，不跟随重定向、不读取响应正文，并等待响应资源清理。探测需要独立网络授权，在无痕模式下不可用；检索开关和搜索凭据不控制端点探测。耗时包含等待授权的时间。HTTP 2xx 表示健康，3xx 表示未知，4xx/5xx 表示不健康；拒绝或失败的请求返回端点状态未知。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
