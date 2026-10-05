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
连接同进程内独立的 UI、Host 部署，不修改双方组合。私有跨进程调用流见下文；
网络认证和默认 UI connector 仍需继续。
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

Mutation 的原 immutable snapshots 随准备、caller 持久化、最终 Host 准入转移。
Session 将请求资源一次转移进原 queued/prepared custody；prepare 与 close 并发时，
Scope cleanup 仍须等待真实释放。快照字节和 metadata 不进入操作日志；
认证网络 transport 尚未接入。

配置采用闭合 JSON，上限 1 MiB、16 个 Host、每 Host 128 个 import、每 import 64
个方法。省略配置表示没有本地连接。相对 Root 路径按启动器工作目录解析。

真实 surface 与已接受 Work 完成清理后，启动器继续等待 caller SQL owner 的真实关闭
确认及 Host 清理；失败保留 store、Root 和 runtime。原生退出也受同一 gate 约束。
`drain_store` 和 `Connections::drain` 保留所有权，不把 drop 当作 cleanup 回执。
UI endpoint 只持有实际 Client/Binding/Scope 准入，不拥有 Host 部署生命周期。

本地 Subscription 复用实际 Host `Client.subscribe`、原 Binding/Scopes/Policy。UI取消
会取消实际 Host scope，仍等待原准入 observer、原执行 completion 和实际 scope cleanup；
不能丢弃准入观察者冒充清理。私有跨进程调用流见下文；认证网络及默认 UI connector 仍待实现。

## 私有跨进程调用流

`transport::Server` 在原 Application 上管理独立调用流。可信 embedding 认证实际通道，
交付批准精确 caller source 和 consumer 集合的 `Peer`。`PeerPolicy` 创建私有请求证据，
最终准入核原 Binding 和方法 grant；mutation confirmation 另核原 caller 和 operation。
传入 JSON、endpoint alias、资源 metadata 都只是观察。

`maka application serve --root ROOT --deployment UUID --peer FILE` 通过启动器提供的
私有 stdin/stdout pipes 服务一次调用。闭合批准文件上限 512 KiB，包含 `source`
（DeploymentRef）、`consumers`（UI 实例 IDs）、`consumer`（Host 实例 ID）和 `imports`
（前述 methods/recovery grants）；每个 import 必须明确选中一个现有 provider。
启动器选择实际进程、持私有 pipe handles 并交付批准文件。CLI 只登记编译提供的
Gateway/Agent/Settings availability，持久 Host 组合仍决定实际激活哪些实例。

Input 与完整 resource descriptors 分别使用有界 JSON frame，raw chunk 最大 64 KiB。
接收之前预约原 Host slot/JSON/payload 配额，空 bundle 也一样。16 个 connection leases
只约束 wire buffers，不产生第二资源预算或执行 owner。Subscription 先确认准入，
然后只响应原调用方下一次 Pull，不在后台预取消息。

取消/EOF 撤销原 Scope，连接 task 继续保留同一准入与 completion observers。
终态分别报告业务结果和实际 Scope cleanup。启动器先 drain 接收/连接，再关闭原
Application 和 SQL；清理失败或 panic 保原 fence。半帧写入被中断后，禁止追加新帧。

`transport::Call` 保持 caller immutable inputs 与实际 `RecordedIntent`；I/O 失败不释放。
`disconnect()` 只关闭物理两端，保留未确认的资源/token；EOF 请求取消，不是远端清理确认。
新的原回执查询不结算丢失 dispatch，不自动重发 mutation。默认 Desktop/TUI
`--connections` 仍用 `LocalService`；共享认证 listener、NativeService connector 和
原生平台完整验收仍需继续。
