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

# maka-ui-js

[English](README.md)

为 `maka-ui-runtime` 执行 JavaScript 组件回调。`execution` 为产品嵌入方配置执行适配器。适配器在 guest 限制内执行激活与回调代码，UI 所有者校验并提交结果批次。原生输入资源、操作回执和挂载清理仍归该所有者管理。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
