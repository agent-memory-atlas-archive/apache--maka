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
运行在不含 Host bridge 的 `Vm::pure`。原生激活提供独立 `NativeOwner`，其真实 close completion
覆盖资源清理。异步初始化开始前即登记资源；拒绝或迟到的导出仍沿原 owner 关闭，close 失败保留
实际 native owner 与生命周期 fence。

绑定捕获准确 consumer 和 registration。最终准入检查实际 Host `Issuer`、Scope 当前取消状态、
方法输入与显式 Host `Policy`，这些检查与 core 接受工作之间不等待。转发 child Scope 保留原始
private evidence。包名、JSON 中的来源身份或绑定本身不是业务效果授权。

`Accepted` 结果只是观察者。owner 保留 execution、Scope 与输入容量，直到 query 和资源真实
结算。普通业务错误或结果校验失败，在资源清理成功时正常释放工作；无法确认清理或 executor
panic 保留 work、Scope 与容量，并退休相关注册。应用 root Drop 仅请求退休，client handle
不能保活 root 或取消已接受工作。关停成功须 native/module 清理及 VM 真实关闭；共享 VM 失败
撤销该执行域的实际注册与消费者，不影响其它执行域。

调用 JSON 最多 1 MiB，共用 128-call / 32 MiB in-flight 输入预算。当前适配器执行 query；
mutation、subscription 与 request-local resource 附件在 durable、stream 和附件 executor
实现前于激活前拒绝。Host transport、组合持久提交/收敛、操作恢复、JavaScript binding bridge
与 SDK，以及 server/TUI/UI 接线仍须实现。这是执行基础，不是完整 Host 迁移或旧 catalog 兼容 facade。
