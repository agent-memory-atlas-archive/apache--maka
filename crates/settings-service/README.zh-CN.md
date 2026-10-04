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

`models` 每页最多返回十六个公开选项，包括准确的连接观察 revision。cursor 绑定原目录 revision，目录变化时返回 `changed`，不混合两个 snapshot。每个 ready 页包含同一 snapshot 下的单个 `defaultModel` 投影：`none`、带已保存目标的 `unavailable`、或带准确选项的 `ready`。它独立于分页，与选项使用同一资格和标签逻辑；ready 只表示目录可选，不保证认证或网络可达。选项只含标签和模型 ID，不含凭据、endpoint 或原生执行权限；选择一个选项不等于执行授权。

`setDefault` 是沿原目录 CAS 的持久 mutation，required nullable `target` 明确设置或清空默认项。SQL 不确定保留原 operation 的 Unknown，recovery 不重复写入。契约由实际 Rust 序列化形态生成，领域校验仍归配置 owner。

`connections` 按目录 revision 每页提供最多十六项公开摘要。`connection` 比较准确的连接 basis，返回公开配置和基本编辑字段，或 `stale`；详情不复制隐藏的模型目录与 overrides。`providers` 分页提供编译可用的 API provider descriptor，每页最多八项、编码结果最多 128 KiB。descriptor 表示可用事实，不代表激活或授权。

`create`、`update`、`remove` 复用原配置 owner 与持久 mutation journal。创建只物化一次 provider 默认值；修改配置校验提交的对象，不重新补默认。部分更新在原连接 CAS 后合并提交字段：省略保留原值，false 与空数组仍是有效更新；null 只清空原 `modelOverrides` 与 `requestBodyOverlay` patch。基本编辑无需把隐藏的大模型集合放进请求。

明确的校验失败返回 `rejected` 和封闭的原因码，表单可以保留草稿并继续修改；单事务中的配置校验必须等确认 rollback 后才返回拒绝。SQL、rollback、commit 和 owner 不确定仍为 Unknown。凭据输入在 enrollment 前校验并拒绝；prepare 或 claim 后的错误保持 Unknown，除非原 enrollment ticket 已确认终态。结果不公开原始错误或输入。

`credentialStatus` 提供公开 vault 元数据，`deleteCredential` 在同一事务核原连接与凭据 generation。`saveCredential` 必须携带原观察 connection target、明确可空的 credential basis 和 `maka.text.utf8` 类型的不可变 `secret` 附件。只有已准入 mutation callback 才读取该 request-local 附件，经 compiled noninteractive API-key provider 认证后完成原 enrollment 事务，不额外取得网络权限。secret 与资源 metadata 不进入公开 operation、caller intent 或 enrollment receipt。attempt identity 来自完整原操作 reference；恢复读取原操作，不重新交换或写入。

默认 Agent 产品通过普通模型选择、默认设置、连接表单与安全密钥组件绑定此能力，各组件按显式契约独立替换。
