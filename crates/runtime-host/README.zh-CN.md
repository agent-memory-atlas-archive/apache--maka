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

# Host 应用服务

`application::Application` 与 UI 使用同一个 plugin-core 组合和生命周期内核，执行服务部署。
输入是不可变应用包与显式 native implementation registry，不默认启用内置业务。

`Application::adopt_committed` 允许可信应用 root 所有者在同一 State Root 中接纳确切的
已提交版本。完整候选先通过静态校验，再进入内核；拒绝其他来源和更早版本，最多同时读取
一个候选快照。返回成功仅表示接纳期望输入，不证明激活或清理完成。`Client::snapshot`
区分 `desired` 与 `effective`（上次完整收敛来源），并报告当前实例阶段、初始化失败和清理
失败。`Application::source` 仍是最初启动来源；`ready` 等待收敛并返回已观察到的失败。

不变实例保留原始来源、native context 和确切 binding，其准备中及后续 mutation 沿用原始
receipt reference。变化实例撤销新准入，已接受工作仍由原 owner 结算。替代必须等待真实
调用资源、module/native close 和最后的 VM shutdown；共享 VM 在仍有模块所有者时保留。
运行中初始化失败保留无关健康实例，后续已提交版本可以恢复。包入库不等于激活；此 root
API 尚未提供远程管理权限入口，也尚未让 server 和呈现执行器接入运行中更新。
现有 server 入口尚未迁移到这个适配器。

owner task 校验整批实际导出后一次发布。JavaScript 服务用 `Captured` 保留实际 receiver/functions，
运行在不含环境 Host router 或 Deno globals 的 `Vm::scoped`。原生激活提供独立 `NativeOwner`，其真实 close completion
覆盖资源清理。异步初始化开始前即登记资源；拒绝或迟到的导出仍沿原 owner 关闭，close 失败保留
实际 native owner 与生命周期 fence。

原生 factory 接收 `(NativeContext, configuration)`。owner 在初始化 fence 建立后构造上下文，
提供实际 activation、选定的确切不可变 package，以及成对的可选 applied source 和 `Arc<EventLog>`。
package 标识执行来源，不决定数据归属。非持久化应用没有 store，
要求 Root 的服务须拒绝初始化。`EventLog::owned_root()` 验证并共享现有 lease，供可信原生存储
（如 `ConfigurationStore`）使用；其真实关闭由返回的 `NativeOwner` 负责。持久数据归属使用
Root domain、deployment UUID 和 instance identity；source revision、activation epoch 仅为
执行事实。初始化依赖不替代逐次请求的 Policy 准入。

绑定捕获准确 consumer 和 registration。最终准入检查实际 Host `Issuer`、Scope 当前取消状态、
方法输入、准确附件槽位/类型与显式 Host `Policy`，这些检查与 core 接受工作之间不等待。转发 child Scope 保留原始
private evidence。包名、JSON 中的来源身份或绑定本身不是业务效果授权。

原生 `Handler::call` 接收 method、input 与请求 `Context`。Scope、实际附件和声明的 import
绑定都是实际能力；owner 在异步激活前捕获 import。原生与 JS 共用 Context 与最终准入路径，
`Context::call` 同步入队，仅返回结果观察 future，始终保留原始 private evidence。

query 接受 `maka_plugins::call::Attachments`，与 UI capture 共用实际不可变 `ReadResource`
payload。metadata 或 JSON 槽名不能构造它们，转发者须显式传递实际 payload；文本资源自行
检查 UTF-8 范围与 chunk 限额。准入前原生 descriptor panic 拒绝当项，保留其它已有工作；
若发生在已接受父调用的转发准备中，还会记录父调用 cleanup failure，捕获业务错误不能释放该工作。
JavaScript query 回调收到 `(input, observation, context)`；冻结的 context 提供请求私有
`invoke`：`metadata` 指定 slot，`read` 指定 slot 与字节 `start`/`end`，返回最多 64 KiB
字节；`call` 指定 `import`、从零开始的 `provider`、`method`、`input`，以及目标槽位到
当前请求源槽位的 `attachments` map，只选择已捕获的 import 与实际 payload。槽名只匹配本次实际附件，撤销后拒绝新读取。保存的 context 在报告回调完成前失效，
不会保活 snapshot。原生 read/metadata panic 记录 Scope cleanup failure，JS catch 不能
改变该事实，同 worker 的无关模块仍可运行。公共应用 SDK 类型来自实际 Rust ABI；初始化 grant 仍须实现。

Policy 接收借用的 `Invocation`，以命名字段提供 consumer、provider、Scope、contract、method、
input 与附件；这些观察值不产生新授权。

child 请求携带父调用的 resource Ticket，沿队列、executor 与实际 cleanup 结算转移，在报告
业务结果前完成 Ticket。普通拒绝或业务失败不会产生 cleanup failure；child 清理未确认时，
即使 JS catch 或未 await 子调用，父子工作与容量仍保留。Context 与 owner 只持弱 dispatch，
观察 future 不持附件 map 或 endpoint。

`Accepted::completion` 分别观察原 owner 的业务结果与真实 executor 清理结果；owner
消失时报告清理未确认。`Accepted::result` 只观察业务，不能确认清理。owner 保留 execution、Scope、实际附件与容量，直到 query 和资源真实
结算。普通业务错误或结果校验失败，在资源清理成功时正常释放工作；无法确认清理或 executor
panic 保留 work、Scope 与容量，并退休相关注册。应用 root Drop 仅请求退休，client handle
不能保活 root 或取消已接受工作。关停成功须 native/module 清理及 VM 真实关闭；共享 VM 失败
撤销该执行域的实际注册与消费者，不影响其它执行域。

Subscription 方法复用原 handler、Binding、Policy 与执行 owner。`Client::subscribe`
创建可独立取消的 reader；原生通过 `Context::emit` 发布，JavaScript 使用
`await context.invoke({operation:'emit', value})`。每个数据项与最终返回值均验证 method.output。
每流最多 16 个队列项、16 个 pending emit，每项完整 JSON 最多 64 KiB；Host 最多持有
16 个 stream buffer，生产已结束但还有未读数据的流仍占额度。生产者等待容量；返回、
错误或 panic 均封闭发布入口，保存旧 Context 或 send future 也不能继续发布或阻止 EOF。
reader 可以排空已接受数据。`Subscription::cancel` 和 Drop 仅请求停止；`completion`
先关闭需求，再等待原 owner 分别报告业务结果和真实清理。取消不回传父 Scope 或兄弟订阅；
清理失败仍保留原 execution、资源与 fence。

调用 JSON 最多 1 MiB，共用 128-call / 32 MiB in-flight 输入与 descriptor 预算。
`Client::start`、`subscribe`、`prepare_mutation` 消费 `CallResources`；原生附件 map
通过 `From<Attachments>` 进入同一路径。附件最多 16 个准确槽位，queued 与 accepted
请求共用 64 MiB；清理未确认保留实际 payload 和容量。最终 owner 在 Policy 准入前
复核实际 descriptor 不得超过原预约。

`Client::capture_resources` 在分配接收快照前预约同一份 call、JSON、payload 容量。
Descriptor 保留 type、长度、metadata 与 Bytes/Utf8 编码，这些事实不授予权限；零字节
接收仍占原 call slot。`write` 只接受准确连续、最多 64 KiB 的分块；`finish` 要求完整长度、
验证声明的 UTF-8，移动原 buffer 而不另复制整包。私有冻结输入把同 permit 与原 Scope
Ticket 转入原 executor，不能跨 Application 或请求 Scope 使用。UTF-8 快照保留字符边界
检查。派发前接收 adapter 必须实际持有并清理 partial/frozen 输入；单独
`Application::shutdown` 不能证明外部接收清空。EOF、timeout 不能确认已接受工作或失败清理。

当前适配器执行 native/JavaScript query、subscription 及持久 mutation；认证网络 ingress、
live 组合收敛和完整产品接线仍须实现。接收接缝不是完整 transport 或兼容 facade。

持久 mutation 要求 `Application::start_from_store`、声明 mutation 方法和显式非 nil 操作 UUID。
原生调用端先 `Client::prepare_mutation`，在自己的 Root 中通过
`EventLog::record_application_intent` 确认原 Ref，再把准备结果和私有 `RecordedIntent`
交给 `Client::mutate`。准备持有原输入和附件预算，但不接受执行、不写业务 journal；最终准入重新检查
捕获的 binding、Scope、执行域与 Policy，不重定位当前 target。`Context::mutate` 在原 Host caller
source 中执行同样的记录与最终准入顺序，并与 query 共用真实 native/JS handler。JS 使用
`context.invoke({operation:'mutate', import, provider, method, input, operationId, attachments})`。
回调实际 `HostObservation.operation` 提供原回执引用；attachments 显式映射当前请求的真实资源槽。

`Client::mutate_from_peer` 接收认证 caller adapter 的有界 `CallerIntent` 观察，不能
重建 `RecordedIntent`、证明另一进程的 SQL commit 或在 Host 补写替代 caller 记录。
实际 owner 在接受执行前核完整原 Ref、普通 `Policy::admit`，以及默认拒绝的
`Policy::confirm_peer_mutation`。可信 Policy 必须从原 Scope 私有证据核实际认证 peer
批准的 caller source、UI consumer 与原 request；UI consumer 与 Host import consumer
是不同执行身份。原 binding、撤销、生命周期和独立历史 recovery 检查仍然适用。
确认 Policy panic 沿现有隔离准入失败路径处理；接受工作保留本地 token 或 peer 观察到
真实清理及粘性失败。远端 caller adapter 必须保留**真实** SQL token/Root lease 到远端
真实 cleanup 确认；peer 观察不能拥有该 Root，断线或业务 receipt 不能替代 cleanup。
这只是执行接缝；认证 application 网络 transport 仍待实现。

caller 历史按稳定 consumer 与 operation 保留首次 source 和原 Ref，跨 source 修订或目标变化
仍读取原记录；不同 Ref 冲突。caller 记录不宣称业务结果，不产生 Host 授权。记录失败或 commit
不确定不能执行 effect；确认 token 在接受工作及真实清理期间持有 caller Root，清理失败继续保留。
纯历史观察不能构造执行 token。

原 Input、准确包 bytes 和契约保留在不可变部署修订中。派发事务确认后才产生私有首次执行 claim；
重复请求不执行 callback、不覆盖原 cleanup。指纹归一对象键序，保守保留实际 JSON 数字表示，
输入变化报冲突；跨 VM 改写数字表示后应读取原 Ref，不能重新提交改写输入。
`Mutation::reference` 只是观察，接受工作本身不证明持久派发已确认。
`Mutation::result` 返回保留原 Ref 的 `OperationReply`，owner 消失也不丢失引用；
只有 recorded receipt 才证明原 journal 事实。
原生呈现适配器通过 `Mutation::completion` 分别观察本次 executor 的实际清理与回执事实。
回执不可用不代表清理失败；历史回执中的 cleanup 不描述本次读取请求的清理。
完成通道丢失时保留原 Ref，明确报告清理未确认。

returned、unknown 与真实资源 cleanup 分别记录。业务错误、非法结果或进程丢失不能证明回滚；
SQL 结算不确定也不冒充物理 cleanup failure。丢弃观察者或应用 root 不取消已接受工作；
实际清理失败保留工作、容量和 State Root lease，不自动重放未知操作。

`Client::recover` 固定原 Root/Ref，不解析当前 provider。读取前后检查实际 consumer lifetime、
Issuer、撤销状态和显式 `Policy::recover`；默认拒绝，包括重复请求释放历史结果。
Ref 不产生授权；替换组合或移除原 provider 后，新 consumer 获得原 Root 的实际恢复 grant 才可读回执。

持久 mutation 的 `Client::prepare_mutation` 接收真实 `Attachments` 并同步排队。
原 Scope ticket 随同一 map/容量在 queued Call、Prepared reply、executor 之间转移；
准备 drop 先释放真实快照再确认 ticket，close 等待真实 custody，包括未收取的 reply。
Host 仅把原 bundle 的私有 nonce 纳入操作 fingerprint，不记录快照字节或 metadata。
同 operation UUID 的新 bundle 是冲突的新原请求；原 recovery reference 不授予资源读取。
JavaScript 的 `mutate` 必须显式提供目标槽到当前源槽的 `attachments` map。
