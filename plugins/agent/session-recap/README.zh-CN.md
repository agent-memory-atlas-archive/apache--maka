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

# maka-session-recap

[English](README.md)

通过已授权的历史和模型端口生成、保存手动会话回顾。回顾是派生数据，不修改规范历史。生成流程在调用模型前记录操作；重复的操作 ID 用于查询原请求。结果未知时不能自动发起新的付费请求。实现与回执处理见 `recap`。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
