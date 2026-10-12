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

# maka-acp

[English](README.md)

通过普通 Host 客户端向编辑器提供 ACP v2 服务。`serve` 将协议请求转换为获准的 Agent 操作和观察，不持有执行或规范存储权限。Maka 调用外部 ACP Agent 的实现位于 `plugins/agent/acp-executor`。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
