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

# maka-cli

[English](README.md)

构建 `maka` 可执行文件，负责终端产品启动、Host 服务、ACP，以及应用检查、组合和安装命令。产品启动将原生工厂和 JavaScript 执行适配器接入平台所有者。

在仓库根目录执行 `just run` 或 `just run --help`。开发构建读取 `../bundle/dist`，`just run` 会先重建该 bundle 再启动 Cargo。发布构建将其嵌入程序。桌面端使用独立的同目录可执行文件。

[源码](src/main.rs) · [Cargo 配置](Cargo.toml)
