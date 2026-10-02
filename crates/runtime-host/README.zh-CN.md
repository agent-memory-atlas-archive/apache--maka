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
现有 server 入口尚未迁移到这个适配器。

owner task 校验整批实际导出后一次发布。JavaScript 服务用 `Captured` 保留实际 receiver/functions，
运行在不含环境 Host router 或 Deno globals 的 `Vm::scoped`。原生激活提供独立 `NativeOwner`，其真实 close completion
覆盖资源清理。异步初始化开始前即登记资源；拒绝或迟到的导出仍沿原 owner 关闭，close 失败保留
实际 native owner 与生命周期 fence。

绑定捕获准确 consumer 和 registration。最终准入检查实际 Host `Issuer`、Scope 当前取消状态、
方法输入、准确附件槽位/类型与显式 Host `Policy`，这些检查与 core 接受工作之间不等待。转发 child Scope 保留原始
private evidence。包名、JSON 中的来源身份或绑定本身不是业务效果授权。

原生 `Query::call` 接收 method、input 与请求 `Context`。Scope、实际附件和声明的 import
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
改变该事实，同 worker 的无关模块仍可运行。公共 SDK 与初始化 grant 尚未实现。

Policy 接收借用的 `Invocation`，以命名字段提供 consumer、provider、Scope、contract、method、
input 与附件；这些观察值不产生新授权。

child 请求携带父调用的 resource Ticket，沿队列、executor 与实际 cleanup 结算转移，在报告
业务结果前完成 Ticket。普通拒绝或业务失败不会产生 cleanup failure；child 清理未确认时，
即使 JS catch 或未 await 子调用，父子工作与容量仍保留。Context 与 owner 只持弱 dispatch，
观察 future 不持附件 map 或 endpoint。

`Accepted` 结果只是观察者。owner 保留 execution、Scope、实际附件与容量，直到 query 和资源真实
结算。普通业务错误或结果校验失败，在资源清理成功时正常释放工作；无法确认清理或 executor
panic 保留 work、Scope 与容量，并退休相关注册。应用 root Drop 仅请求退休，client handle
不能保活 root 或取消已接受工作。关停成功须 native/module 清理及 VM 真实关闭；共享 VM 失败
撤销该执行域的实际注册与消费者，不影响其它执行域。

调用 JSON 最多 1 MiB，共用 128-call / 32 MiB in-flight 输入预算；附件最多 16 个准确槽位，
queued 与 accepted 请求共用 64 MiB，清理未确认时保留实际 payload 和容量。当前适配器执行
原生及 JavaScript query，支持附件；mutation、subscription 在 durable、stream executor
实现前于激活前拒绝。Host transport、组合持久提交/收敛、操作恢复、公共 SDK，以及 server/TUI/UI 接线仍须实现。这是执行基础，不是完整 Host 迁移或旧 catalog 兼容 facade。
