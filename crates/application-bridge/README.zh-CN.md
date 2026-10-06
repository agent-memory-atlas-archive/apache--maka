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

# 应用 Host 连接

Desktop 与 TUI 启动器支持 `--connections FILE`。这份可信外部配置为每个选定部署
显式选择 process 或 cohost，不修改双方组合。process 拥有独立 Application 与 Root；
cohost 在 UI 原 owned EventLog 中执行不同 UUID 的 Host 部署。两者复用同一个
Server、PeerPolicy 和 native 调用 adapter。网络认证仍需继续。
完整 JSON 示例见[英文说明](README.md)。

`hosts[].name` 对应 UI remote observation 的 deployment alias；`location` 显式选择
`{kind:"process",root:"..."}` 或 `{kind:"cohost"}`，`deployment` 选择实际 Host 来源；`consumer` 是已有 Host 消费者的 InstanceId 数组。
`imports` 选择该消费者的显式绑定，每项提供明确的 `methods` 白名单和独立
`recovery` 布尔。方法必须存在于实际捕获的服务契约，endpoint 必须匹配选定的 UI
来源观察。启动捕获精确 `DeploymentRef`，后续 desired source 更新不能继承原批准。
普通 query/mutation 权限不授予原操作回执恢复权限。

Host 组合显式选择 `maka.application.gateway` 实现的实例及其 imports，也可选择
其他适当的 compiled consumer；availability 不自动激活实例。其他 Rust factory 由可信
可执行文件的 compiled Host role 提供，动态 JavaScript 从已提交的不可变包加载。配置中的批准默认绑定直接
endpoint，转发到其他 consumer 会拒绝。Rust 编译扩展在选定 Host 内执行；所有 UI
remote service 使用同一物理 transport adapter。

Mutation 的原 immutable snapshots 随准备、caller 持久化、最终 Host 准入转移。
Session 将请求资源一次转移进原 queued/prepared custody；prepare 与 close 并发时，
Scope cleanup 仍须等待真实释放。快照字节和 metadata 不进入操作日志；
认证网络 transport 尚未接入。

配置采用闭合 JSON，上限 1 MiB、16 个 Host、每 Host 128 个 import、每 import 64
个方法。省略配置表示没有本地连接。相对 Root 路径按启动器工作目录解析。

真实 surface 与已接受 Work 完成清理后，启动器继续等待 caller SQL owner 的真实关闭
确认及 Host 清理；失败保留 store、Root 和 runtime。原生退出也受同一 gate 约束。
`drain_store` 和 `Connections::drain` 保留所有权，不把 drop 当作 cleanup 回执。
UI endpoint 保留实际 catalogue selection、原 epoch 与原 physical-call custody；Session 固定原 caller 批准，
不会转向替代 Host，也不拥有 Host 部署生命周期。

`Connections::prepare` 在启动 Host 前核选定 presentation 与精确 immutable source，
UI consumer 集合来自实际 Plan 中启用的 executable Remote bindings。启动器先持有
Connections，再观察 `start(&mut self)`；取消等待只释放借用，原 Child/Control 仍在同一
owner 内。close 在轮询等待前同步撤销全部连接，并清理所有 child；每项须同时观察
same-child Closed 与实际 Child.wait。aggregate cleanup 不结算旧 Lost Call 或 caller SQL token。

SQL 启动错误须等待真实连接关闭；typed `StartupCleanupFailed`、worker crash 或 reader
关闭不确定时，原 SQL worker 保留原 Root authority。启动器保持 runtime，包括 Ready 前
失败。有限的 [SQLx owned-startup 接缝](../../third-party/sqlx-sqlite/SOURCE.md)仅拆开原
establish/configure，不增加另一 SQL owner。

Subscription 经私有 IPC 复用实际 Host `Client.subscribe`、原 Binding/Peer Policy。UI取消
会取消实际 Host scope，仍等待原准入 observer、原执行 completion 和实际 scope cleanup；
不能丢弃准入观察者冒充清理。私有跨进程调用流见下文；认证网络仍待实现。

## 私有跨进程调用流

`transport::Server` 在原 Application 上管理独立调用流。可信 embedding 认证实际通道，
交付持有精确 caller source 和 consumer 批准集合的 `Peers`。`PeerPolicy` 创建私有请求证据，
最终准入核原 Binding 和方法 grant；mutation confirmation 另核原 caller 和 operation。
传入 JSON、endpoint alias、资源 metadata 都只是观察。

`maka application serve` 在进程整个生命周期持有同一个 Host。实际选中的 child 从继承
stdin 接收 `process::Bootstrap`：Root/deployment/namespaces、精确 UI source/消费者集合、
Host consumer/import grants、私有 IPC 地址与非空 incarnation nonce。闭合 JSON 上限
512 KiB。Stdin/stdout 传 Bootstrap/Ready、Capture/Captured、Approve/Approved、Close/Closed；每个 import 明确选择一个
实际 provider。Gateway/Agent/Settings 编译 availability 由该 executable 提供，持久组合
仍决定实际执行的实例。

`runtime_host::ipc` 是两条 Host 路径共用的物理 listener/connector。Unix socket 要求
私有 current-account 父目录并用 0600；Windows pipe 拒绝 remote client、保留
current-account/SYSTEM ACL。`transport::ipc::Connector` 用原 child nonce 配对新 stream；
Hello 在任何 Scope/Open 之前受原 Server 的 16 wire leases、TaskTracker、close fence
约束。配对不授予业务权限；独立调用共享同一原 Host Application 与 JavaScript 状态。

`process::Control` 捕获实际 child Catalogue，控制 read/write 被中断后保留原 stdout。
不确定写入后不再追加第二命令。Child 先撤销 listener、drain 原 Server/Application，
再真实关闭 SQL/Root，最后发布匹配 nonce 的 Closed。打开 store 后启动失败也经过
同一收尾；start 即使未见 Ready 也保留真实 Closed，同时返回启动错误。启动器持实际
Child 并单独观察它的终止，正常关闭要求成功退出；启动失败可能已确认 cleanup 却
退出非零。聚合 Closed 或进程退出都不结算旧 Lost Call。

Input 与完整 resource descriptors 分别使用有界 JSON frame，raw chunk 最大 64 KiB。
接收之前预约原 Host slot/JSON/payload 配额，空 bundle 也一样。16 个 connection leases
只约束 wire buffers，不产生第二资源预算或执行 owner。Subscription 先确认准入，
然后只响应原调用方下一次 Pull，不在后台预取消息。

取消/EOF 撤销原 Scope，连接 task 继续保留同一准入与 completion observers。
终态分别报告业务结果和实际 Scope cleanup。启动器先 drain 接收/连接，再关闭原
Application 和 SQL；清理失败或 panic 保原 fence。控制帧写入被中断后禁止追加新帧，
但仍可半关闭写端并读取同一原通道完整的终态确认；取消交付失败不抹掉已发出的回执。

`transport::Call` 保持 caller immutable inputs 与实际 `RecordedIntent`；I/O 失败不释放。
`disconnect()` 只关闭物理两端，保留未确认的资源/token；EOF 请求取消，不是远端清理确认。
新的原回执查询不结算丢失 dispatch，不自动重发 mutation。默认 Desktop/TUI
`--connections` 使用共享 child supervisor 与此物理 transport；完整原生平台和产品验收仍需继续。

CLI 在接受调用前，从原捕获 Bindings 发布有界 `Catalogue`。`Selection` 固定 endpoint
alias 与 registration epoch；每个 Open 携带原 epoch，另一 Host incarnation 即使发布
同名 alias 也不能接管旧选择。Provider/contract 描述只读，不授予权限、不构造 Binding。
新的恢复查询选择新 Host registration，只读取原 operation Ref。

`transport::native::{Connection, Service}` 把通道接入原 UI kernel 的 query、subscription
和 prepared mutation 接口。可信 `Connector` 只连接发布 catalogue 的同一认证 Host
incarnation。取消等待原 opening 明确完成；构造、poll、future 析构 panic 均保原建立
屏障。适配器核 UI kernel 私有来源与已批准 caller，在创建 dispatch observer 时同步
接收实际 SQL token。Stop 在 session mutex 之前发出，cleanup 等同一调用及实际本地
资源析构完成才释放容量。失败 custody 保留在原 Connection；两个启动器均使用此
适配器，没有另一套进程内 UI service/session 实现。

## 精确来源更新

可信 Root manager 可以对精确已提交 UI source 调用 `Connections::approve_source`，
同时明确保留仍有原消费者存活的 revision。此操作委托原配置的方法及恢复权限；
观察 SQL head 本身不授予权限，也不能改变 Host Root、deployment、可执行文件、
gateway consumer 或 native grants。

发布前，原 child 通过继承控制通道捕获实际 Bindings；parent 完成整批 target、contract、
唯一 registration、批准集合与 wire 上限校验。Child 发布同一批原 Bindings，不在校验
和批准之间重新按 alias 绑定。不再使用的 Host 只更新保留集合，无需绑定 endpoint。
已知校验错误保留旧批准；跨 child 物理 I/O 失败不是事务或回滚回执，原 Child/Control
owner 继续承担实际状态与清理。

真实 Host registration epoch 未变时复用原 service。Session 与已签发 Scope 固定原批准
ticket；撤销拒绝后续准入，包括已准备但尚未接受的 mutation，不丢弃已接受工作的原
执行、资源及清理。同一 controller 生命周期内，已撤销或被越过的 revision 不可重新获批；
重装原内容必须提交新 revision，仍存活的来源保留原 ticket。未退休 actor 仍使用原 source，
manager 须保留对应 revision，直到最后一个原 executable consumer 完成真实退休清理。


## 原 Root 同进程 Host

`location:{kind:"cohost"}` 借用 UI 启动器的原 Store，不接受 Root 路径、不重新打开 Root。
Host deployment UUID 必须不同于 UI 和其他 cohost。编译可用实现仍由 embedding 提供，
组合、绑定、方法和 recovery grant 才决定激活及调用；Duplex 复用原 Server/native adapter，
不增加 socket 或另一调用协议。

cohost 提供 `maka.application.ui-management` 的 compiled availability，固定原 UI deployment
及 presentation；产品须显式组合并绑定。操作仍属该 Host 的原 namespace/journal，业务结果
中的 source 属 UI desired 部署。提交或观察 source 不产生 endpoint 批准，也不表示前台收敛。

capture 保存实际 Binding，whole-batch approval 复用；同 epoch 保持同 service implementation。
取消 start 观察仍保原 initializer/owner。启动器依次等待 UI 工作、全部原 Connection/Server/Host
Application 清理，再关闭唯一共享 SQL Store。失败保留原 owners，包括 startupErr 和实际 GPUI
退出 owner。


## 前台更新

生产 TUI/Desktop 启动器在原 Store/Connections 上持有唯一 `Updates` 管线：合并 head 观察、
在事件循环外加载确切快照、由原 Application 预检 native 实现和 scene language，再明确委托
原连接配置。`PreparedUpdate` 不可编辑并绑定原前台 owner；最终 adoption 重查来源、版本及
关闭状态。已知预检错误保留当前可用界面。

未变的 actor、registration epoch 和物理界面保持身份；替换仍等待原 terminal/window 的真实
清理回执。保留集合来自当前 desired revision 与真实 executable activation 的原 origins，包含
退休尚未清理的 owner；结构实例不产生消费者权限。Retention 不绑定 endpoint 或委托新消费者。
本地撤销不依赖 child controller 健康；逐原 controller 尝试远端保留更新，失败保原 owner。

同时最多一个 load/approval/retention。退出停止新 adoption，等待原在途 worker，再关闭
Connections 与 SQL；不 abort 交换，也不把 observer 失败当作清理回执。产品插件管理界面和
完整视觉验收仍属后续产品工作。
