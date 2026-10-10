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

TUI 外观页提供 Maka、Dusk、Paper、Terminal 与自定义配色，支持英文、简中、繁中、ASCII 符号与减少动画。自动语言依次跟随原生进程的 `LC_ALL`、`LC_MESSAGES`、`LANG` 与系统语言；已保存的显式语言优先，选择“自动”可恢复跟随环境，即使组合配置了固定语言。自定义名称、基底和语义/语法色即时预览；保存沿原实例 checkpoint 持久化 UI 偏好，关闭未保存的编辑会恢复已保存外观，并保留会话草稿。安装配置可用绝对路径 `themeFile` 开启主题文件的重新读取与保存（16 KiB 以内的 UTF-8 自定义主题 JSON）。重新读取只改变预览，保存到文件只导出自定义主题；当前偏好仍需“保存并应用”。编辑器显示文件路径，外部修改后必须重新读取才能导出。完整原 TUI 体验对齐仍未完成。

安装配置包含 `presentation`（`desktop`/`tui`）、完整 `locale`（或 `auto`）、`scheme`（`light`/`dark`）、`hostRoot`、`hostDeployment`、`managementDeployment`、SDK 生成的 `AgentConfiguration` 对应 `agent`，以及 `keys`。连接凭据和 endpoint 属于选中的 Host Root，不进入产品组合。

`keys` 为 `src/composition.ts` 导出的所有包名分配稳定非零 UUID；安装时生成一次并保存，构建新 revision 时沿用。部署 UUID 同样属于安装身份，不能每次构建重建业务 namespace。具体字段、包名和配置约束见英文文档。

组合命令调用真实 Rust CLI 校验包并计算精确引用，输出并检查 `host.json`、`management-host.json`、`ui.json`、`remote-exports.json`、`connections.json`、`packages.json`。它不初始化 Root、不提交部署、不启动执行。`connections.json` 属于可信启动器输入，其中的逐方法和恢复 grant 需要明确选定。

可信 connections 文件可以预声明后续组合所需的其他 Host；未选中的声明不占用 Root，校验后的已提交来源选择它时才按原显式配置和权限启动。

分别初始化 Host/UI Root，用 `maka application commit` 提交相应输入、部署 UUID、当前 expected revision 和包路径；UI 同时提供 remote exports。初次提交使用 expected revision 0。同一 UI Root 还须以稳定且不同于 UI/业务 Host 的 `managementDeployment` 提交 `management-host.json`；该明确选择的管理 cohost 借用原 Store。随后通过 `maka application run` 或 `maka desktop` 指定 UI Root、部署及 connections 文件。发送需要从选定 Host Root 中明确选择已认证并启用连接的模型；[本地 Host 连接](../../crates/application-bridge/README.zh-CN.md)拥有启动与清理，不自动使用其他模型。

原生 editor 独占草稿、选区和 IME；产品只保留元数据与工作流。发送按原始 owner/revision 捕获不可变资源，只有持久准入回执才可清除原草稿。结果未知时沿原 RecoveryRef 查询，不自动重发。重新挂载从原消费者调用历史恢复，不能清除新 editor 的草稿。

历史固定事件 fence，使用 Host 返回的 transcript cursor 分页。活动文本从原 PartStarted 重放，thinking 不作为回答；规范历史变化不重启原订阅。显示节选有界并明确标注，后续分页仍是独立读取。

类型检查和测试使用 `npm --workspace @maka-agent/product run typecheck` / `test`。设置 `MAKA_PRODUCT_CLI_CHECK=1` 额外验证实际包引用与两端组合。原生 opt-in 测试使用实际打包 guest、TUI 编辑器、持久调用意图、已授权原生服务和本地 HTTP provider，命令见英文文档；独立 CLI PTY 测试通过原生文件输入安装 Studio、由原 manager 保存组合，再由生产管线自动替换 surface 并启动预声明的设置 Host；验证持久来源与包、最终 Root 释放和真实终端恢复，命令见英文文档。最终桌面视觉和三平台交互需要另行实际验收。

普通 `models` 组件从显式绑定的 Root 设置能力读取有界模型选项，默认投影独立于选项分页，首屏十六项之外的默认模型也能准确建议；仅在尚未选择模型时建议已保存默认项。每次发送固定该次选择的连接 revision；两端均在有界目录页内使用原生滚动。宿主不隐式替换模型，未知结果不自动重发；目录刷新保留原 connection/model 身份并核验实际 revision，不改选无关默认项。`agent` 配置包含显式 `workspaceRoots`（`[{ label, path }]`，绝对目录）、`maxActiveRuns`、`maxSteps` 与显式 `tools`；纯文本产品填写 `tools: []`。工具须由 Agent 原生条目声明服务 import，并由 Host 独立授权具体调用边。

`connection-form` 从实际提供商描述创建连接，编辑时只提交显式选中的公开字段。未读取的模型集合、覆盖配置与请求体附加配置保持原值，只有用户选择完整替换才提交。编辑端点保留其他配置键，完整配置是独立选项。原生编辑器跨 TUI 字段分页和确认保留全部草稿；保存按每个原 owner/revision 捕获，并使用原 Rust 契约在构建期生成的校验函数检查最终输入。字段类型错误和 CAS 拒绝不会退休表单。冲突需要复核并确认当前连接或目录版本；未知结果查询原操作，不自动重试。此组件拥有读取公开字段的 Reader，不导入 SecretInput。 编辑字段限制为 64 KiB UTF-8；JSON 非有限数值及超过 64 层容器嵌套在提交前拒绝。 移除已保存连接复用同一表单，以准确观察版本和已保存身份明确确认；关联密钥及该连接的默认模型会清除，会话保留。取消保留原生草稿，冲突须复核并再次确认；结果未知仅查询原回执，不重新删除。历史删除回执独立于当前草稿。

普通 `connections` 组件提供有界目录分页，密钥操作交给独立 `credential` 组件。TUI 连接和提供商列表使用原生滚动与方向键选择，Enter 打开选中的可编辑连接或选择提供商；连接条目显示默认模型或启用模型数，上下文菜单提供凭据和编辑操作。设置内容内按 Esc 返回目录或退出尚未选择的提供商列表；编辑及未结算操作保留原离开确认。TUI 连接页和编辑页在进入时聚焦可用控件，启用的编辑字段优先；确认与历史操作窗复用原生弹窗的焦点及返回。帮助页的快捷键行适配窄终端，打开后即可用键盘翻页。Credential 不导入 Reader，不向父组件公开 capture；它只将原生 SecretInput 的指定版本资源连同所观察的连接/凭据版本交给 Settings。确认保存才请求清空被捕获的编辑版本；未知结果查询原 Ref，冲突保留原生草稿。保留的草稿可以通过显式复核连接与密钥版本、用户确认后继续保存。移除已保存的密钥也需要确认和双版本校验，不清除草稿。

历史操作独立分页展示，因为同一 consumer instance 的多个 mount 共用调用历史，而 Ref 不含原连接输入；查询历史结果不改变当前目标或清新草稿。进入设置隐藏对话呈现，同时保留原生 composer owner。

`model-default` 是模型选择器显式绑定的独立组件，区分未设置与已保存但不可用的目标。设置或清除冻结观察到的目录 revision 和目标，确认后才写入；冲突需要刷新并再次确认，结果未知仅查询原回执。历史回执独立有界分页，不采用当前目标。保存刷新目录，不改变当前会话选择的模型；隐藏选择器保留同一子组件 slot 和进行中的工作流。可选默认项只表示目录资格，不保证认证、执行或网络可用。

Desktop 模型选择器使用公开锚定 Popup scene。

两端模型选择器保留同一个原生搜索框，通过有界分页搜索完整启用目录。TUI 的 Up/Down 进入原生滚动列表，方向键暂选模型，套用或 Enter 核对原生查询 revision 后确认模型与思考等级。光标移动只确认文本，不重新搜索。关闭、默认设置页或 busy 作用域停止自动续查；默认设置刷新独立读取元数据，不受无效搜索草稿阻断。TUI 默认设置的确认与回执历史使用嵌套原生弹窗，Esc 逐层返回；确认中的完整目标文字可滚动，取消与确认按钮固定。

打开模型、默认设置或操作历史时，
输入区位置不变；Escape 或外部点击由插件处理关闭。默认设置子组件在关闭和
重开之间持续挂载。Terminal entry 保留正常流内的分页呈现。

Desktop 侧栏可收起，导航组件与输入区持续挂载。上下文与模型操作分行排列，长标题和模型按钮由原生测量显示省略号，保留完整可访问值；Terminal 导航名称按自身呈现边界显示。

`command-palette` 包提供可显式组合的普通 `CommandPalette` 内容组件：父组件控制 `open` 与当前命令启用条件，接收 `invoked`/`selected`/`closed` 事件。它导入 TextInput、TextReader、ReadingArea 和 Agent，按命令 label/id/hint 或会话 id/title 匹配全部空白分隔的查询词，扫描原 revision 约束的目录，保留最多 32 个匹配会话并提供显式 More 窗口。选择前捕获原生查询文本 revision；父组件处理命令 id，通过 Agent 重新观察所选会话，所在 scene 负责呈现与模态输入隔离。

普通 Deployment 组件提供 Desktop/TUI 的固定版本八实例分页、确认启用/停用、冲突刷新与
原操作查询。Host 对原组合输入做 CAS，保留模板参数和其它覆盖；必需连接失效则明确拒绝。
界面显示 desired 配置，不将保存当运行就绪；未知结果不自动重放。独立高级页通过有界
UTF-8 分块和原编辑器回执加载整份来源，原生 buffer 独占草稿。可编辑配置、显式绑定、
已安装包选择和启动根，保留模板来源结构；保存确认冻结 owner/revision，并将不可变文本
资源交给 Host 校验完整计划、原执行环境、已安装版本和 CAS。规划诊断保留草稿；冲突
必须明确确认丢弃/重载，未知整份替换也从原回执/调用历史恢复。关闭页面保留原编辑器。
文档预算 1 MiB，提交 input 与 remote observations 仍共用 512 KiB。完整
产品/运行时与平台体验验收仍待完成。

TUI 插件列表复用原生 ReadingArea/Item 展示两行条目，方向键选择，Enter 或条目菜单进入启用/停用确认。确认和操作历史使用原生 Dialog，正文滚动、底部按钮固定；Esc 返回一层。导入与高级编辑页进入时聚焦，Esc 先取消确认再返回列表。保存/重新载入及导入确认也使用原生 Dialog，保留编辑草稿和文件选择。独立 Otty 在 80×24、40×16 下验证了屏外导航、键鼠取消及焦点返回，产品与真实 CLI 校验通过。宽窗对话框在背景中文字符处的边框缺口仍待排查。

插件页面通过保留的原生 FilePicker 安装 `.maka-plugin` 文件。确认时捕获选中文件
owner/revision 对应的不可变字节，以观察到的来源版本交给 Host 校验。安装只保存
插件包，不改变组件启用状态或权限。未知结果仅查询原操作；版本冲突需刷新后重新
确认。高级组合草稿不会被安装覆盖，显式重新加载后可看到新保存的插件引用。
用 `maka application export-package --package DIRECTORY --output FILE.maka-plugin`
导出编写好的目录，已有目标文件不会被覆盖。

原生 Agent 只读捕获发布的工作区目录。新会话复用已选会话的不可变工作区；首次创建先查询发布目录选择，再提交创建操作。会话投影包含真实路径、目录／工作区身份与标签。`metadata` 使用原 Session revision CAS 修改标题／标签；确定的业务拒绝返回普通结果，持久化不确定性仍保留原操作日志。命令面板同时搜索标题、ID、路径和标签。“在其他目录新建对话”在两种呈现中打开可替换的普通 DirectoryPicker 插件。它浏览已发布根与一页有界子目录，使用原生 Input／Reader 筛选本页，将原始观察到的选择交给 Root 的记录式创建路径。筛选框中上下方向键与 Enter 浏览目录，鼠标或 Tab 可前往“在此新建”。同一受控 Dialog 保留命令面板和目录选择器的挂载，切换或取消时保留各自原生输入。可替换的普通 SessionDetails 插件使用原生 Input／Textarea／Reader 编辑标题和标签，以原 Session revision 保存。冲突保留草稿，显式复核后才能采用新的保存依据；关闭弹窗保留草稿与未决操作引用，成功保存不会替换对话 epoch、run、watch 或 Composer。

普通 `help` 插件提供两端的键盘帮助。F1 在当前页面上打开帮助，Esc 或关闭按钮返回原页面和原生草稿；命令面板也提供帮助入口。它只导入 ReadingArea，主题和翻译由产品维护。正文独立滚动，页脚在窄终端中仍可关闭。

可选 Questions 服务沿用原有 canonical 交互记录。要启用模型提问，在 `agent.tools` 中加入 `{ "result": "json", "confirmation": "preapproved", "name": "AskUserQuestion", "description": "询问继续任务所需的 1–3 个有界问题。", "import": "questions", "method": "ask" }`。组合生成器显式选择原生提供者及其调用授权；`tools: []` 仍为纯文字组合。待回答问题显示在会话中，普通 QuestionEditor 支持选项、原生文字草稿和逐题跳过，提交复用 Workspace 的原操作回执与历史恢复。关闭弹窗保留草稿。权限审批已接入；客户端能力授权仍待接入。

类型化表单使用可选 `forms` 导入、`ask` 方法及 `result: "json"`（工具可命名为 `AskUserForm`）。只有显式选择的工具才安装原生 Forms 提供者并获得调用授权。Host 提供请求方身份并校验字段、类型、格式和字节上限。两端支持文字、数字、整数、布尔、单选、多选、可选字段及默认值；关闭保留原生草稿，提交、拒绝填写与取消分别记录原规范回答。未知结果须检查原响应回执后才能再次响应。旧外部表单生产端及客户端能力授权仍需接入。


每个已选工具须显式声明 `confirmation: "required"` 或 `"preapproved"`。需要确认时，界面展示原绑定服务，并复用分页源码查看器读取完整原参数。默认选择拒绝；允许仅放行本次调用，仍须通过原服务授权与存活绑定检查。关闭弹窗保留待处理请求，停止会关闭请求且不调用服务；未知响应沿原操作回执恢复。这不替代文件／进程的沙箱权限策略。

可选原生进程工具使用 `import: "processes"`，提供 `execute`、`read`、`writeStdin`、`stop`。启用时须在安装配置顶层显式提供 `processes`，包含 `filesystem`、`network` 及可选 `helper`；例如 `{ "filesystem": { "default": "read", "rules": [], "denyGlobs": [] }, "network": "denied", "helper": null }` 只允许读取。写入路径须由安装上限显式允许，会话策略与原权限审批不能突破该上限或私有 Root/元数据保护。Windows 要求可信 Maka CLI helper，Linux 的受限网络模式也需要它。后台 pipe/PTY 由原生提供者持有；撤权后拒绝旧终端输入和尺寸修改，仍可读取与停止。提供者关闭须等待真实进程清理，无法确认时保留失败。完整交互终端 UI 仍待后续接入。

每个已选工具还须声明结果类型：查询／变更使用 `result: "json"`，图片订阅使用 `result: "image"`。工作区图片可配置为 `{ "result": "image", "confirmation": "preapproved", "name": "ReadImage", "description": "读取所选工作区中的图片。", "import": "files", "method": "readImage" }`。支持 PNG、JPEG、GIF、WebP，最多 5 MiB、每边最多 2000 像素。成功读取的字节进入不可变 canonical 图片记录；视觉模型接收实际图片，Desktop 复用图片查看器，TUI 保留查看入口并提示在桌面预览。读取沿用原工作区权限：元数据只读，私有 Host Root 与所选目录外路径不可读取。


普通组合为选中的 `files.readText/write/edit/applyPatch` 提供显式 Files v1 展示标识；已有标识保持不变，`presentation: null` 保留普通原文。标识随原调用冻结，不依赖模型可见别名。展开后通过原会话权限和历史边界读取文件路径、当时记录的内容、请求替换内容及补丁；长字段分别提供有界双向翻页。TUI 将完整的小型前后内容合并为原生逐行差异，合计限制 8 KiB／256 行；大内容或未加载完整的字段继续独立分页。完整小型创建／更新补丁经过整体解析后显示变更边栏，非法补丁保留字面原文。这些视图复用原选择／查找 owner，Desktop 当前使用字面原文块。Write 只显示当时已记录的原内容，业务失败与原参数／结果／回执入口仍保留。折叠文件卡片显示请求路径的原生复制按钮及业务失败状态，只读取路径、结果状态和成功写入／替换的记录路径，内容与诊断在展开时读取。同一 ToolDetails 保留字段页和待继续的翻页请求，当前内容查找命中嵌套原文会展开所属卡片。展开详情保留完整路径及“复制文件路径”；复制使用 Host 返回的有界路径并沿用原生真实输入门禁。TUI 命令面板的“复制文件路径”沿原生选区、浏览标题或首个可见消息定位；完整工具卡片体验仍待后续对齐。

会话导航显示原持久化未读状态。持久化文字消息旁的“标记已读至此”仅在该消息仍为存储中的最新可见文字时清除未读；打开或滚动不会自动确认。该操作沿用原 Agent mutation 回执恢复。

附件使用原生本机 FilePicker 与 Host 二进制资源捕获。打开“附件”、选择文件（TUI 输入本机绝对路径），再加入当前会话；每条消息最多八个附件，每个最多 50 MiB，共享原生分配预算可能拒绝并发的大文件选择。支持仅附件发送；本次界面挂载期间，附件草稿跨会话导航保留，只在原发送回执确认接纳后移除。上传或发送结果不确定时查询原回执。移除草稿引用不会删除已存材料。附件草稿尚不支持重启恢复，目录浏览、多路径粘贴与剪贴板图片导入也尚未完成。队列编辑保留规范附件，允许含附件的空正文。

TUI 命令面板提供“复制消息”和“复制消息原文”，沿原生选区、浏览标题或首个
可见消息定位，在原 Session／历史边界下读取完整 canonical 字段，不把窗口预览
当作完整消息。Assistant／thinking 复用原生 Markdown 解析器；用户正文保留字面文本，
工具卡片明确标注复制的是原结果或参数。每份文本上限为 64 KiB UTF-8、JSON 转义
后内容 128 KiB。读取中、流式、空消息、失败或超限时禁用对应命令；格式化失败不
丢弃已完成的原文。关闭面板撤销请求，旧响应不能提供新命令的文本。只有原生物理
输入写入剪贴板，后台读取完成不会自动复制。

已保存的 ChatGPT 订阅连接可参与原生模型选择和执行。到期凭证复用原有持久刷新 owner，保留并发交换合并、已收到替换凭证的结算与退出排空。当前连接编辑器只显示其能够编辑的 API Key 提供方操作；交互式订阅登录尚未接入该编辑器。

TUI 的常用操作保留在页面内，命令面板作为备用入口。新建会话、会话行、消息与工具卡片提供右击菜单及可见的「⋯」按钮；新建菜单可选择其他工作目录，会话详情就地打开，完整文本消息可直接复制原文，有原生选区时提供复制选区。未完整加载的消息保留原文查看入口。菜单复用现有操作条件和原生弹层焦点管理。
