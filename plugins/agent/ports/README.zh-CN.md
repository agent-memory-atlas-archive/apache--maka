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

# maka-agent-ports

[English](README.md)

定义 Agent 执行使用的业务接口，包括模型访问、提供商、会话、定价和用量。这些端口允许替换实现，同时将业务策略留在插件领域。作用域资源准入属于 `maka-capabilities`。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
