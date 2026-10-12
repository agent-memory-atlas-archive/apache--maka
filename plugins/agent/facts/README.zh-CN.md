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

# maka-agent-facts

[English](README.md)

定义持久化与执行共用的 Agent、Session 和 Run 事实，包括事件、消息、模型选择、工具调用、计量和续接记录。这里只包含数据类型，不依赖执行引擎或 SQLite。执行策略应留在所属服务中，而非加入持久化事实。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
