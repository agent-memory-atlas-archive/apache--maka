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

# JavaScript 执行

模型 Code Mode 与插件使用独立 V8 runtime。插件 `Vm` 在后台 worker 运行预构建 ESM
bundle；`Foreground` 在 UI owner 线程运行有界同步组件回调。两者均不安装环境文件、
网络或进程能力；后台系统操作通过该模块显式选择的 `Bridge`，模块名或 JSON 身份不授予权限。


`Vm::pure` 提供不含 Host op、环境 Deno 或旧 SDK 的后台执行域，拒绝任何带 bridge/SDK 的
load。Host 应用服务适配器用此执行域运行纯 JavaScript query，不共享旧 bridged VM。
普通 VM 中单个模块没有 Bridge，并不等于这种隔离边界。

发布实际服务注册前调用 `Module::capture(namespace, methods)`。不透明 `Captured` 保留
实际 receiver/functions，`Captured::call` 支持 Promise，不重新查找 namespace；之后替换
函数或 namespace 不能重定向旧能力。名称最多 128 bytes，路径 32 层，64 个不重复方法，
每 VM 最多 1,024 个注册。最后 handle Drop 释放 globals；成功但未被等待者取走的 capture
reply 本身也承担释放义务。

所有后台调用保留 module/VM，直到实际 Promise 结束；等待者和全部外部 VM/module handle
退出也不提前终止。`Module::close` 停止新调用，已接受调用真实结束后才回收代码。VM failure
先销毁 pending JavaScript/globals 再确认代码回收。这些事实不能替代外部业务 effect 或 Host
resource 的真实结算，嵌入 executor 仍须持有并确认相应工作。

调用参数在复制进 V8 前受 1 MiB 限额，共用 128-call / 32 MiB in-flight 输入预算。
Foreground 和全部 worker result 使用同一 JSON codec：先 stringify 与 UTF-8 1 MiB
检查，再复制到 Rust。Native Bridge 输入在这次复制前检查 32 MiB 上限，之后还须符合所选
方法的输入预算。getter/toJSON 处于 watchdog 覆盖内；循环引用与 BigInt 不能跨 JSON 边界。

JSON 数字经过模块调用和 native Bridge 始终保持 JavaScript Number。Rust 原生整数若不能
由 Number 精确表示则拒绝，数组与对象内部也如此。来自 V8 的数值若最短十进制写法像另一个
整数，解码仍保留原始二进制 Number。所有调用共用这一 codec，不保留旧 serializer 回退。
`Statistics::captured_callbacks` 是实际注册数量，不是 function 数量或授权证明。

这是新应用服务执行适配器的基础；Host 部署激活、显式服务绑定、transport 授权与持久操作
恢复仍属于独立接线工作，不能以 VM capture 宣称完成。组件执行见 [maka-ui](../ui/README.zh-CN.md)。
