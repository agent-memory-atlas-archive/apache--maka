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

# Root 设置能力

`maka.settings` 是编译提供的原生扩展，导出 `Settings`（`maka.settings.Settings` v1）。启动器只登记 availability，普通组合必须显式激活与绑定。实际 `NativeContext` 提供持久化及所属 Root；原生 owner 持有配置 SQL owner，关闭时等待真实 shutdown，清理失败保留 Host owner 与 Root fence。

`models` 每页最多返回十六个公开选项，包括准确的连接观察 revision。cursor 绑定原目录 revision，目录变化时返回 `changed`，不混合两个 snapshot。投影只有标签、模型 ID 和明确的默认标记，不含凭据、endpoint 或原生执行权限；选择一个选项不等于执行授权。

`setDefault` 是沿原目录 CAS 的持久 mutation，required nullable `target` 明确设置或清空默认项。SQL 不确定保留原 operation 的 Unknown，recovery 不重复写入。契约由实际 Rust 序列化形态生成，领域校验仍归配置 owner。

默认 Agent 产品通过普通模型选择组件绑定此能力。完整连接编辑、原生密码 enrollment 与 provider discovery 工作流仍待接入。
