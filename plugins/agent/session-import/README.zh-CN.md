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

# maka-session-import

[English](README.md)

发现并转换 Claude、Codex 和 OpenCode 的外部会话格式。`Transcript` 是统一的导入表示，`source` 与 `catalog` 负责发现。导入通过所属服务产生会话数据，不执行导入文本中的指令，也不替代 Host 的规范事件写入方。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
