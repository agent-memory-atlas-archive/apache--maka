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

# maka-host-runtime

[English](README.md)

执行经过校验的服务组合，负责调用准入、激活和退役。产品提供原生工厂，并按需注入 JavaScript 执行适配器。所有者与客户端 API 见 `application`；编写原生服务可从 `application/native_context.rs` 开始。

提交的配置表示期望状态，实例完成收敛后才成为有效状态。更新保留未改变的激活及其绑定；替换必须等待已接受调用和原生资源完成清理。操作回执区分业务结果与清理状态，丢失响应后应查询原回执。

持久化由已持有的 EventLog 提供。插件数据归属于 Root、部署和实例；激活 epoch 标识一次执行，不会创建新的数据所有者。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
