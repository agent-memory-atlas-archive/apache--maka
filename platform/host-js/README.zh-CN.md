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

# maka-host-js

[English](README.md)

通过 `maka-host-runtime` 的后端接口执行 JavaScript Host 服务。`factory` 接收显式运行限制并创建适配器。Guest 调用沿用原始 Host 上下文、导入绑定和附件槽位；适配器负责执行，Host 负责准入和清理。只使用原生实现的嵌入程序可以不依赖本 crate。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
