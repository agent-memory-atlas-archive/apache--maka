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

# maka-agent-authority

[English](README.md)

根据原始调用事实检查 Agent 权限并处理交互等待。`permissions` 负责准入，`interaction_wait` 跟踪待处理的用户交互。公开会话 ID 和 JSON 请求字段不能替代已捕获的调用权限。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
