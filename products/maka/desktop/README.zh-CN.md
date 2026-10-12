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

使用 `just build` 构建，再以 `target/debug/maka desktop --root PATH --deployment UUID --connections FILE` 启动已安装的 Desktop 部署。CLI 会启动同目录前端。窗口关闭需要等待原生资源释放、已接受操作和 Host 连接清理。平台特有的 IME 与无障碍行为需要在真实窗口中验证。

[源码](src/main.rs) · [Cargo 配置](Cargo.toml)
