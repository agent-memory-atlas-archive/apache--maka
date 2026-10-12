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

# maka-processes-native

[English](README.md)

提供工作区命令、终端和协议进程服务。`protocol` 处理结构化子进程通信，`terminal` 管理交互终端资源。配置给出安装权限上限，每次调用仍需自己的授权。关闭观察者不会结束对运行中进程的所有权。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
