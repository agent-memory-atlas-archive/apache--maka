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

# 插件产品组合

[English](README.md)

这是一份普通的文本 agent 插件组合，同一组包分别提供 desktop/TUI 入口。Workspace、Composer、Navigation、Timeline、Appearance 可以独立替换；启动器不包含产品页面。Surface、编辑器、文本捕获、调用历史及 Host agent 服务由随构建提供的原生包承担，各部分通过明确的公开契约连接。

当前切片包含创建/选择对话、规范历史分页、流式回复、精确 invocation 停止与原始 mutation 回执恢复。连接浏览、创建、公开字段编辑与原生 API 密钥保存/移除已接入普通插件；默认模型设置与清除已接入独立普通组件，具备明确确认与回执恢复；工具流程、附件、历史虚拟化和各平台视觉/IME/可访问性验收尚未完成，不把运行时测试当作成品验收。

独立普通包 `maka.studio.workbench` 提供双端“配置工作台”：用顶部导航管理连接、
默认模型和插件组合，隐藏页面保留同一挂载与原生草稿。工作台不包含会话、输入区
或 Agent 绑定；它的独立 Host 组合仅包含 Settings、Receipts 及明确授权的 gateway。
共享设置组件仍可替换。模型选择属于工作台本地状态，保存默认项仍通过原确认操作。

编写来源时选择 `--product studio`，默认是 `agent`。工作台使用下述安装字段与 keys，
不需要 `agent` 配置；两个产品使用相同的通用启动命令与组合替换机制，启动器不判断
产品名称。安装根插件不会自动切换产品，运行中的替换需要单独确认来源变更，并完成
旧界面实际清理；新根不继承旧产品的原生草稿。切换 Host 位置还需明确修改可信连接
配置，来源中的声明本身不授权访问另一 Host。

在仓库根执行 `npm --workspace @maka-agent/product run build`，再运行：

```sh
node packages/agent-product/scripts/compose.mjs \
  --configuration installation.json --output product-source \
  --maka /absolute/path/to/maka
```

安装配置包含 `presentation`（`desktop`/`tui`）、完整 `locale`、`scheme`（`light`/`dark`）、`hostRoot`、`hostDeployment`、`managementDeployment`、SDK 生成的 `AgentConfiguration` 对应 `agent`，以及 `keys`。连接凭据和 endpoint 属于选中的 Host Root，不进入产品组合。

`keys` 为 `src/composition.ts` 导出的所有包名分配稳定非零 UUID；安装时生成一次并保存，构建新 revision 时沿用。部署 UUID 同样属于安装身份，不能每次构建重建业务 namespace。具体字段、包名和配置约束见英文文档。

组合命令调用真实 Rust CLI 校验包并计算精确引用，输出并检查 `host.json`、`management-host.json`、`ui.json`、`remote-exports.json`、`connections.json`、`packages.json`。它不初始化 Root、不提交部署、不启动执行。`connections.json` 属于可信启动器输入，其中的逐方法和恢复 grant 需要明确选定。

可信 connections 文件可以预声明后续组合所需的其他 Host；未选中的声明不占用 Root，校验后的已提交来源选择它时才按原显式配置和权限启动。

分别初始化 Host/UI Root，用 `maka application commit` 提交相应输入、部署 UUID、当前 expected revision 和包路径；UI 同时提供 remote exports。初次提交使用 expected revision 0。同一 UI Root 还须以稳定且不同于 UI/业务 Host 的 `managementDeployment` 提交 `management-host.json`；该明确选择的管理 cohost 借用原 Store。随后通过 `maka application run` 或 `maka-desktop run` 指定 UI Root、部署及 connections 文件。发送需要从选定 Host Root 中明确选择已认证并启用连接的模型；[本地 Host 连接](../../crates/application-bridge/README.zh-CN.md)拥有启动与清理，不自动使用其他模型。

原生 editor 独占草稿、选区和 IME；产品只保留元数据与工作流。发送按原始 owner/revision 捕获不可变资源，只有持久准入回执才可清除原草稿。结果未知时沿原 RecoveryRef 查询，不自动重发。重新挂载从原消费者调用历史恢复，不能清除新 editor 的草稿。

历史固定事件 fence，使用 Host 返回的 transcript cursor 分页。活动文本从原 PartStarted 重放，thinking 不作为回答；规范历史变化不重启原订阅。显示节选有界并明确标注，后续分页仍是独立读取。

类型检查和测试使用 `npm --workspace @maka-agent/product run typecheck` / `test`。设置 `MAKA_PRODUCT_CLI_CHECK=1` 额外验证实际包引用与两端组合。原生 opt-in 测试使用实际打包 guest、TUI 编辑器、持久调用意图、已授权原生服务和本地 HTTP provider，命令见英文文档；独立 CLI PTY 测试通过原生文件输入安装 Studio、由原 manager 保存组合，再由生产管线自动替换 surface 并启动预声明的设置 Host；验证持久来源与包、最终 Root 释放和真实终端恢复，命令见英文文档。最终桌面视觉和三平台交互需要另行实际验收。

普通 `models` 组件从显式绑定的 Root 设置能力读取有界模型选项，默认投影独立于选项分页，首屏十六项之外的默认模型也能准确建议；仅在尚未选择模型时建议已保存默认项。每次发送固定该次选择的连接 revision；Desktop 使用滚动列表，TUI 每屏三个选项并保留导航。宿主不隐式替换模型，未知结果不自动重发；目录刷新保留原 connection/model 身份并核验实际 revision，不改选无关默认项。`agent` 配置包含 `maxActiveRuns`、`maxSteps` 与显式 `tools`；纯文本产品填写 `tools: []`。工具须由 Agent 原生条目声明服务 import，并由 Host 独立授权具体调用边。

`connection-form` 从实际提供商描述创建连接，编辑时只提交显式选中的公开字段。未读取的模型集合、覆盖配置与请求体附加配置保持原值，只有用户选择完整替换才提交。编辑端点保留其他配置键，完整配置是独立选项。原生编辑器跨 TUI 字段分页和确认保留全部草稿；保存按每个原 owner/revision 捕获，并使用原 Rust 契约在构建期生成的校验函数检查最终输入。字段类型错误和 CAS 拒绝不会退休表单。冲突需要复核并确认当前连接或目录版本；未知结果查询原操作，不自动重试。此组件拥有读取公开字段的 Reader，不导入 SecretInput。 编辑字段限制为 64 KiB UTF-8；JSON 非有限数值及超过 64 层容器嵌套在提交前拒绝。 移除已保存连接复用同一表单，以准确观察版本和已保存身份明确确认；关联密钥及该连接的默认模型会清除，会话保留。取消保留原生草稿，冲突须复核并再次确认；结果未知仅查询原回执，不重新删除。历史删除回执独立于当前草稿。

普通 `connections` 组件提供有界目录分页，密钥操作交给独立 `credential` 组件。Credential 不导入 Reader，不向父组件公开 capture；它只将原生 SecretInput 的指定版本资源连同所观察的连接/凭据版本交给 Settings。确认保存才请求清空被捕获的编辑版本；未知结果查询原 Ref，冲突保留原生草稿。保留的草稿可以通过显式复核连接与密钥版本、用户确认后继续保存。移除已保存的密钥也需要确认和双版本校验，不清除草稿。

历史操作独立分页展示，因为同一 consumer instance 的多个 mount 共用调用历史，而 Ref 不含原连接输入；查询历史结果不改变当前目标或清新草稿。进入设置隐藏对话呈现，同时保留原生 composer owner。

`model-default` 是模型选择器显式绑定的独立组件，区分未设置与已保存但不可用的目标。设置或清除冻结观察到的目录 revision 和目标，确认后才写入；冲突需要刷新并再次确认，结果未知仅查询原回执。历史回执独立有界分页，不采用当前目标。保存刷新目录，不改变当前会话选择的模型；隐藏选择器保留同一子组件 slot 和进行中的工作流。可选默认项只表示目录资格，不保证认证、执行或网络可用。

Desktop 模型选择器使用公开锚定 Popup scene。打开模型、默认设置或操作历史时，
输入区位置不变；Escape 或外部点击由插件处理关闭。默认设置子组件在关闭和
重开之间持续挂载。Terminal entry 保留正常流内的分页呈现。

Desktop 侧栏可收起，导航组件与输入区持续挂载。上下文与模型操作分行排列，长标题和模型按钮由原生测量显示省略号，保留完整可访问值；Terminal 导航名称按自身呈现边界显示。

普通 Deployment 组件提供 Desktop/TUI 的固定版本八实例分页、确认启用/停用、冲突刷新与
原操作查询。Host 对原组合输入做 CAS，保留模板参数和其它覆盖；必需连接失效则明确拒绝。
界面显示 desired 配置，不将保存当运行就绪；未知结果不自动重放。独立高级页通过有界
UTF-8 分块和原编辑器回执加载整份来源，原生 buffer 独占草稿。可编辑配置、显式绑定、
已安装包选择和启动根，保留模板来源结构；保存确认冻结 owner/revision，并将不可变文本
资源交给 Host 校验完整计划、原执行环境、已安装版本和 CAS。规划诊断保留草稿；冲突
必须明确确认丢弃/重载，未知整份替换也从原回执/调用历史恢复。关闭页面保留原编辑器。
文档预算 1 MiB，提交 input 与 remote observations 仍共用 512 KiB。完整
产品/运行时与平台体验验收仍待完成。

插件页面通过保留的原生 FilePicker 安装 `.maka-plugin` 文件。确认时捕获选中文件
owner/revision 对应的不可变字节，以观察到的来源版本交给 Host 校验。安装只保存
插件包，不改变组件启用状态或权限。未知结果仅查询原操作；版本冲突需刷新后重新
确认。高级组合草稿不会被安装覆盖，显式重新加载后可看到新保存的插件引用。
用 `maka application export-package --package DIRECTORY --output FILE.maka-plugin`
导出编写好的目录，已有目标文件不会被覆盖。
