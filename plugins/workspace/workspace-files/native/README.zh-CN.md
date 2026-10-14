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

# maka-files-native

[English](README.md)

为显式选择的工作区提供文件操作。`provide` 注册服务，`attachments` 处理捕获的文件资源。包装层向文件引擎传递原始调用作用域与目录权限。文件工具与命令共用会话沙箱策略：允许读取工作区外文件；workspace-write 默认允许写入工作区及系统临时目录，其他写入需要权限审批。所选目录仍通过捕获的句柄访问，审批不能绕过工作区元数据与 Host 私有路径保护。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
