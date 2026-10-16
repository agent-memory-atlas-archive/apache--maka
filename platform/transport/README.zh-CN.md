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

通过本地 IPC 将前台 UI 连接到独立持有的 Host 部署。启动器提供可信连接配置以及方法和回执恢复权限。公共连接发现绑定 Root 的 Host；不存在时，通过可信 CLI 启动器自动启动。同进程 Host 借用 UI Store。`connections` 管理连接，`Updates` 协调获准的源配置更新。

远端注册信息只描述可用服务，不能授予权限、替换调用方或改写已接受操作的目标。关闭 UI 时，先等待原始物理调用及本地持有的同进程 Host 清理。默认退出要求公共 Host 在没有其他会话时停止；此判断与会话接入原子执行。显式选择仅退出 UI 时，断开会话并保留后台工作。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
