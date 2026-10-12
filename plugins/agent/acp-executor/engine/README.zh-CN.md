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

# maka-external-agent

[English](README.md)

实现出站 ACP Agent 会话、安装和配置，将 Maka 执行端口与外部 Agent 进程相连接。原生包装层提供显式绑定的进程、文件和交互服务。这里负责调用外部执行器；面向编辑器的 ACP 服务端位于 `products/maka/acp`。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
