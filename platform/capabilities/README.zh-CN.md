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

# maka-capabilities

[English](README.md)

定义原生服务使用的作用域资源，包括调用、文件、HTTP、进程、终端、凭据和存储。取消与资源票据见 `call`，准入见 `authorization`。资源票据必须保留到实际清理完成；丢弃结果观察者不会释放资源。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
