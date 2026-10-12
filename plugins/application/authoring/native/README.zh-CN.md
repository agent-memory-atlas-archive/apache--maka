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

# UI 编写

Host 的 `Authoring` 服务持久化 Agent 发给当前前台的请求。UI `Worker` 在前台拥有的 Root 中调用现有部署管理器，返回原始 action 结果。Native grants 分别向 Agent 开放 `submit`/`result`，向明确连接的 UI 开放 `watch`/`claim`/`complete`；结果只能由原 Agent 会话读取。

请求在实例重建后保留。领取请求时原子移出待处理队列；更新中断 UI 后，通过 worker 的 caller history 和原回执恢复，不自动重放修改。没有连接的 worker 时请求保持排队；已领取但无法恢复结果的请求保持未确认。队列最多容纳 16 个待领取请求，结果查询最多等待 5 秒，保留管理冲突、拒绝及未知结果。

文本编写接口复用现有包加载器和部署提交。产品接线位于 `products/maka/bundle`；运行时操作和 footer 示例见 `skills/maka-plugin-authoring/references/runtime-ui.md`。
