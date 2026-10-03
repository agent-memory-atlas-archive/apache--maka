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

# 本地应用连接

Desktop 与 TUI 启动器支持 `--connections FILE`。这份可信外部配置在同一进程中
连接独立的 UI、Host 部署，不修改双方组合。跨进程认证和 transport 仍需另行实现。
完整 JSON 示例见[英文说明](README.md)。

`hosts[].name` 对应 UI remote observation 的 deployment alias；`root` 和
`deployment` 选择实际 Host 来源；`consumer` 是已有 Host 消费者的 InstanceId 数组。
`imports` 选择该消费者的显式绑定，每项提供明确的 `methods` 白名单和独立
`recovery` 布尔。方法必须存在于实际捕获的服务契约，endpoint 必须匹配选定的 UI
来源观察。启动捕获精确 `DeploymentRef`，后续 desired source 更新不能继承原批准。
普通 query/mutation 权限不授予原操作回执恢复权限。

Host 组合显式选择 `maka.application.gateway` 实现的实例及其 imports，也可选择
其他适当的 compiled consumer；availability 不自动激活实例。其他 Rust factory 由可信
启动器提供，动态 JavaScript 从已提交的不可变包加载。配置中的批准默认绑定直接
endpoint，转发到其他 consumer 会拒绝；其他可信 embedding 可以通过 `LocalService`
提供其独立 Host Policy 与只创建无资源 Scope 的 `Scopes` factory。

Mutation 的 resource snapshots 需要其 executor 协议，目前在准备前拒绝；streams
和认证网络 transport 尚未接入。

配置采用闭合 JSON，上限 1 MiB、16 个 Host、每 Host 128 个 import、每 import 64
个方法。省略配置表示没有本地连接。相对 Root 路径按启动器工作目录解析。

真实 surface 与已接受 Work 完成清理后，启动器继续等待 caller SQL owner 的真实关闭
确认及 Host 清理；失败保留 store、Root 和 runtime。原生退出也受同一 gate 约束。
`drain_store` 和 `Connections::drain` 保留所有权，不把 drop 当作 cleanup 回执。
UI endpoint 只持有实际 Client/Binding/Scope 准入，不拥有 Host 部署生命周期。
