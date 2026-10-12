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

# maka-process

[English](README.md)

通过管道、PTY 和分离进程执行已确定的命令。`Command` 描述启动参数，`ShellExecutor`、`pipe` 和 `pty` 管理执行与 I/O。沙箱准备由 `maka-sandbox` 提供。调用方负责授权及持久化后台记录，并应保留进程所有权，直到确认退出和清理完成。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
