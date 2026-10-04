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

# Agent 产品组合

[English](README.md)

这是一份普通的文本 agent 插件组合，同一组包分别提供 desktop/TUI 入口。Workspace、Composer、Navigation、Timeline、Appearance 可以独立替换；启动器不包含产品页面。Surface、编辑器、文本捕获、调用历史及 Host agent 服务由随构建提供的原生包承担，各部分通过明确的公开契约连接。

当前切片包含创建/选择对话、规范历史分页、流式回复、精确 invocation 停止与原始 mutation 回执恢复。连接浏览与原生 API 密钥保存/移除已接入普通插件；完整连接创建/编辑、工具流程、附件、历史虚拟化和各平台视觉/IME/可访问性验收尚未完成，不把运行时测试当作成品验收。

在仓库根执行 `npm --workspace @maka-agent/product run build`，再运行：

```sh
node packages/agent-product/scripts/compose.mjs \
  --configuration installation.json --output product-source \
  --maka /absolute/path/to/maka
```

安装配置包含 `presentation`（`desktop`/`tui`）、完整 `locale`、`scheme`（`light`/`dark`）、`hostRoot`、`hostDeployment`、SDK 生成的 `AgentConfiguration` 对应 `agent`，以及 `keys`。连接凭据和 endpoint 属于选中的 Host Root，不进入产品组合。

`keys` 为 `src/composition.ts` 导出的十七个包名分配稳定非零 UUID；安装时生成一次并保存，构建新 revision 时沿用。部署 UUID 同样属于安装身份，不能每次构建重建业务 namespace。具体字段、包名和配置约束见英文文档。

组合命令调用真实 Rust CLI 校验包并计算精确引用，输出并检查 `host.json`、`ui.json`、`remote-exports.json`、`connections.json`、`packages.json`。它不初始化 Root、不提交部署、不启动执行。`connections.json` 属于可信启动器输入，其中的逐方法和恢复 grant 需要明确选定。

分别初始化 Host/UI Root，用 `maka application commit` 提交相应输入、部署 UUID、当前 expected revision 和包路径；UI 同时提供 remote exports。初次提交使用 expected revision 0。随后通过 `maka application run` 或 `maka-desktop` 指定 UI Root、部署及 connections 文件。发送需要从选定 Host Root 中明确选择已认证并启用连接的模型；[本地 Host 连接](../../crates/application-bridge/README.zh-CN.md)拥有启动与清理，不自动使用其他模型。

原生 editor 独占草稿、选区和 IME；产品只保留元数据与工作流。发送按原始 owner/revision 捕获不可变资源，只有持久准入回执才可清除原草稿。结果未知时沿原 RecoveryRef 查询，不自动重发。重新挂载从原消费者调用历史恢复，不能清除新 editor 的草稿。

历史固定事件 fence，使用 Host 返回的 transcript cursor 分页。活动文本从原 PartStarted 重放，thinking 不作为回答；规范历史变化不重启原订阅。显示节选有界并明确标注，后续分页仍是独立读取。

类型检查和测试使用 `npm --workspace @maka-agent/product run typecheck` / `test`。设置 `MAKA_PRODUCT_CLI_CHECK=1` 额外验证实际包引用与两端组合。原生 opt-in 测试使用实际打包 guest、TUI 编辑器、持久调用意图、已授权原生服务和本地 HTTP provider，命令见英文文档；最终桌面视觉和三平台交互需要另行实际验收。

普通 `models` 组件从显式绑定的 Root 设置能力读取有界模型选项，仅在尚未选择模型时建议已保存默认项。每次发送固定该次选择的连接 revision；Desktop 使用滚动列表，TUI 每屏三个选项并保留导航。宿主不隐式替换模型，未知结果不自动重发；目录刷新保留原 connection/model 身份并核验实际 revision，不改选无关默认项；添加连接与公开字段编辑仍待产品接入。`agent` 配置仅包含 `maxActiveRuns` 与 `maxSteps`。


普通 `connections` 组件提供有界目录分页，密钥操作交给独立 `credential` 组件。Credential 不导入 Reader，不向父组件公开 capture；它只将原生 SecretInput 的指定版本资源连同所观察的连接/凭据版本交给 Settings。确认保存才请求清空被捕获的编辑版本；未知结果查询原 Ref，冲突保留原生草稿。保留的草稿可以通过显式复核连接与密钥版本、用户确认后继续保存。移除已保存的密钥也需要确认和双版本校验，不清除草稿。

历史操作独立分页展示，因为同一 consumer instance 的多个 mount 共用调用历史，而 Ref 不含原连接输入；查询历史结果不改变当前目标或清新草稿。进入设置隐藏对话呈现，同时保留原生 composer owner。
