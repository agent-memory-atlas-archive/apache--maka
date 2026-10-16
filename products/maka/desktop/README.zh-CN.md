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

# maka-desktop

[English](README.md)

构建 `maka-desktop` 可执行文件并管理原生窗口。它加载已提交的 Desktop 组合，注入 UI JavaScript 执行适配器，并通过 `maka-render-gpui` 呈现帧。页面与工作流来自插件包。

使用 `just build` 构建，再以 `target/debug/maka desktop --root PATH --deployment UUID --connections FILE` 启动已安装的 Desktop 部署。CLI 会启动同目录前端。启动时连接配置的本机 Host；不存在时，通过同目录 `maka` 自动启动。关闭最后一个窗口或退出应用时，若没有其他客户端连接，Host 也会退出。可通过应用菜单的“Close UI and Keep Host”仅关闭界面，保留后台任务。平台特有的 IME 与无障碍行为需要在真实窗口中验证。

[源码](src/main.rs) · [Cargo 配置](Cargo.toml)
