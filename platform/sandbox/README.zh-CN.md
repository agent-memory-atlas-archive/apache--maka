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

# maka-sandbox

[English](README.md)

为已授权命令准备操作系统隔离。Host 决定权限并管理执行记录，本 crate 应用文件系统与网络限制。进程 I/O 和生命周期由 [maka-process](../process/README.zh-CN.md)负责。

macOS 使用 Seatbelt，Linux 使用 bubblewrap 和网络命名空间，Windows 使用预配置的低权限账户、WFP 规则及 Job。Linux 需要 `/usr/bin/bwrap` 提供 bubblewrap 0.11 或更新版本。不支持的策略在执行前失败。Windows 通过 `maka sandbox setup --root PATH` 显式设置，Host 保持非提权运行。

限制目标的网络流量经过执行所有者持有的网关。路由和凭据留在 Host，重定向目标需要独立权限。Linux 与 Windows 在启动前展开拒绝 glob；之后新增的匹配路径由下次启动覆盖，不属于已有快照。

通过实际后端测试策略：

```sh
maka sandbox run --policy policy.json --cwd /absolute/workspace --command 'your command'
```

Windows 还需传入已配置的 `--root`。只读且无网络的策略示例：

```json
{"kind":"managed","filesystem":{"default":"read","rules":[],"denyGlobs":[]},"network":"denied"}
```

设置、状态查询和移除命令见 `maka sandbox --help`。这些机制约束模型驱动的副作用；可信原生插件代码运行在 Host 进程内。

Seatbelt 策略的上游版本、版权和许可证记录见[英文版来源说明](README.md#source-provenance)。
