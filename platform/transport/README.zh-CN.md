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

# maka-transport

[English](README.md)

通过本地 IPC 将前台 UI 连接到独立持有的 Host 部署。启动器提供可信可执行文件路径、连接配置以及方法和回执恢复权限。`connections` 管理连接，`Updates` 协调获准的源配置更新。

远端注册信息只描述可用服务，不能授予权限、替换调用方或改写已接受操作的目标。关闭流程需要等待原始 UI 工作、Host 清理和子进程退出。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
