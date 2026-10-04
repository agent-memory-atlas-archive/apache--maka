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

# Root 配置

`ConfigurationStore::for_root` 共享实际 `RootOwner`，由拥有的 SQL connection 处理连接目录、凭据 vault 及各自独立 revision。嵌入方在释放原生 owner 前等待 `shutdown`；公开目录和状态快照不授予执行或 Root 访问权限。

连接修改在原写事务中比较 caller 的确切目录或连接 basis。删除凭据要求显式 `expectedConnection`：连接凭据使用原完整连接 target，网络凭据为 null。同一 vault 事务先核此 target、再核凭据 generation；endpoint 已变化时，旧删除请求不能仅凭 saved key 未变继续成功。

`oauth::LoginStart` 要求 `expectedCredential`，caller 观察未配置时也须显式 null。已有连接登录包含原 `ConnectionCredentialTarget`。prepare 在同一 SQL snapshot 核两者，complete 再核准备时的连接和 vault generation；refresh 或 logout 不会被基于旧观察的请求静默覆盖。原认证 fingerprint 包含该观察，recovery 只观察原 attempt，不授权第二次 exchange。

Provider 凭据沿原 authentication/enrollment owner 与受校验 envelope；普通 `set_credential` 拒绝 provider key。可信原生扩展可在真实 Host 准入后使用同一个 `Definition::authenticate` 校验，再通过原 enrollment 发布。认证输入与替换 secret 保持私有，公开 query 仅返回目录事实和凭据状态。commit 不确定、业务成功与实际清理结果分别保留，不能互相代替。
