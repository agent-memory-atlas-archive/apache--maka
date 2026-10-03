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

# 组件执行

`maka-ui` 从不可变包激活已检查的 UI entry，向共用核心发布实际组件。
前台 JavaScript 只在激活、挂载初始化、props 更新和输入处理时执行。
发布前捕获必需的 `mount`、`event`、`update` 函数及 receiver；命名空间后续变更
不能替换已注册处理器。回调返回 `ComponentOutput { frame, events, actions, decision? }`；events/actions 是
本次事务的一次性意图，不属于保留状态。声明 capture 时也在发布前捕获其回调。呈现器只读取已提交
`Frame`。有界批次完整校验后原子替换，每个挂载有独立状态与 revision，旧视图事件
不会调用新处理器。执行域故障撤销该域全部注册与挂载。

原始输入回调可返回 `decision: { preventDefault, stopPropagation }`，省略的标志为 false。只有整个因果批次提交成功，内核才返回决定。生命周期回调、组件通知与异步 action 结果不能返回决定。决定受事务限额约束，不保留在 `Frame`；平台仅在实际监听阶段采用，不能撤销已经发生的原生行为。

`NativeRegistry<V>` 提供随构建编译的工厂；只有组合选中的 entry 才会激活，实际导出
仍由同一个 runtime 校验并发布。原生组件与 JavaScript 共用 props/state/revision 和
整批事务。已提交 frame 只包含 `NativeView<V>`，即类型化的不可变原生呈现描述；
内核不提供布局词汇，也不依赖 GPUI。原生输入按当前 scene 的 schema 校验；调用在派发前受 1 MiB 限额。
Rust 扩展是受信任同步代码，没有 JavaScript watchdog 隔离；统一提交 deadline 能拒绝
迟到结果，不能中断 Rust。4 MiB 更新及 32 MiB 保留限额计量序列化元数据、场景来源和输入
schema，不计原生内存；原生 owner 负责其实际分配限额。
呈现描述不能持有 entity、输入绑定、订阅或原生 work；它们归呈现器资源 owner 所有，
通过资源凭证等待真实销毁，不依赖退休 frame 的描述被释放。


JavaScript 返回平台自己的 JSON 场景。embedding 必须在激活 JavaScript component
之前通过 `NativeRegistry::with_scene_compiler(language, compiler)` 安装纯编译器。
组件 entry 声明 `scene: { name, version }`；全部 enabled entry 的语言必须精确匹配，
在任何工厂或 guest 执行前拒绝缺失或错配。纯 Value/Service entry 与原生实现不声明
scene。语言和编译器由同一个可用性对象持有，无额外路由或 fallback。编译器只收到 JSON 与当前 deadline，返回 `NativeView`。
内核独立保留原场景 JSON 计费，编译器的元数据不能消除这份来源。编译失败回滚整批，
不退休健康 realm。同一插件的 TUI 与 Desktop 入口可以使用不同场景语言。

`ComponentSlot { key, binding, member, props, on }` 属于
`NativeView::new(recipe, input_schemas, slots)`。slots 有序、最多 64 个，
按父组件自己的 imports 解析，共用子挂载、props 事务、事件路由与清理。
未展示的 slot 保留状态。完整回调元数据（原场景 JSON、slots/state/events/actions）
共用 1 MiB 限额。不能精确投影为 JavaScript 的参数在调用该 consumer 前拒绝，
不退休健康执行域；原生 Rust consumer 仍保留精确整数。

子组件引用显式 binding 与 member，
稳定 key 在父组件 props 更新时保留子挂载和局部状态。父子组件共用一次 deadline，
整批提交；任一子组件失败，原有 frame 全部保留。移除后挂载只进入退休，等待真实原生
清理确认；未提交的挂载尚无原生资源。`Frame.surface` 是封闭声明：Desktop 使用 `Window { binding, title, width, height }`，
TUI 使用 `Terminal { binding }`，variant 与实际 surface 能力都必须匹配 deployment。
根挂载 surface 配置固定，嵌入组件不能创建 surface。
恢复界面尚未实现。
`retire` 只停止新输入；`close` 必须收到实际原生清理确认；`shutdown` 不丢弃
尚未确认的 mount、work 或 resource。

导入引用用 `on` 将公开事件名绑定到父组件的内部 handler key。事件名与每份 payload
按实际导出的公开契约检查；没有订阅的事件也检查。JavaScript `event` 收到
`{ kind: "raw", key, event }` 或 `{ kind: "component", key, event, child, name }`，
第三参数是挂载 props；原生回调收到等价 `EventOrigin`。原始 payload 无法伪造组件来源。

事件捕获准确的父/子 Mount 与 handler，在同一事务内 FIFO 处理；后续回调读取最新 staged
状态。订阅断开或改变时永久剔除其待发事件，恢复同名订阅也不能复活。全部回调共享
deadline、64 次回调与 64 个 emission 限额、4 MiB 累计工作预算；32 MiB 保留限额只计
最终场景。多次因果回调可推进多次 revision，但呈现器只观察最后一次原子发布。异常或
超预算整批回滚。批内创建又移除的私有子树不发布，也不等待呈现器从未拥有的资源回执。
根组件及未订阅事件校验后丢弃，没有全局 event bus。

Desktop 的窗口清理 owner 独立于视觉 Surface。逻辑移除撤销输入并释放 renderer owner；
根挂载还等待平台销毁回执、所有真实资源 lease 和子挂载清理。已知失败保留 failed fence。
有限的 [GPUI 源码补丁](../../third-party/gpui-fast/SOURCE.zh-CN.md) 提供各后端回执，
不把逻辑关闭或排队的 destroy request 当作资源释放，也不承诺 GPU driver 已回收物理内存。

生成示例时在输出目录后加 `composed`，即可使用独立包提供的计数器组件；
生成器将全部选中包保存在 Root。改用 `native` 可显式导入编译的 Label；它直接使用 GPUI 样式，布局位置与
props 由产品决定。平台在实际原生监听阶段应用同步输入决定。

使用 `canvas` 导出动态工作流面板：插件定义吸附、放置与键盘规则，原生层负责绘制、焦点与拖拽追踪。

使用 `editor` 可在存储组合中导入 Kit Textarea。Rust 契约表达
初始文本、基于原生 revision 的 replace/reset 命令、元数据观察与命令回执；原生 buffer
不放进序列化组件 state。公开 `changed` 事件包含类型化元数据观察与回执；收到
TextRef 不获得全局查找或读取权限；capture 提供请求私有的不可变文本资源。`command: null` 不提出
新命令，不取消已接受的请求；请求保留 pending 直到匹配回执提交，最终原生描述只消费一次。
原生组件 event 同时收到当前挂载 props、state、key 与 input；JavaScript
`event` 的第三个参数是 props。

包驱动的桌面示例命令见 [English README](README.md)。示例与校验使用同一套 Rust
契约类型，无旧插件 SDK 或全局 service catalog。

改用 `events` 生成显式订阅 Textarea `changed` 的产品组件，并显示原生草稿 revision；
生成器把所选包与 Desktop 部署保存到 `root`；以 `--root PATH --deployment UUID` 启动。
该示例验证公开事件送达，不代表资源读取、
业务 action 或最终产品视觉设计已完成。

改用 `resource` 生成产品 Root → 动态 Composer → 私有原生 Textarea，以及显式导入的原生
UTF-8 Reader；完整 Root/deployment 命令见 [English README](README.md)。Root 只调用 Composer
公开 capture，不依赖私有 Textarea 的挂载路径。

Action 指明 service binding/member/method 与类型化资源槽，每个槽调用 direct child 的
公开 capture。JavaScript capture 只返回转发至自己 direct child 的纯计划，不能伪造原生
资源。整批事务捕获准确 consumer Mount、source edge 与 service registration；之后移除
或改绑永久取消待接受请求，恢复同名目标不能复活它。

Desktop 单次发布 scene → 准备 native owner → 在 native read barrier 捕获不可变 Rope →
执行 native command → collect 退休 owner。捕获冲突返回 `not_accepted`，不回滚已发布场景。
`current` 显式读取 barrier 当前值，`at_revision` 严格拒绝修订变化。真实资源只在受信任的
请求 sidecar 中；metadata、JSON 槽名及 UUID 都不是读取权限。快照不保留 LiveInput、entity
或 mount resource lease。

`ReadResource` / `Attachments` 位于 `maka_plugins::call`，供 UI 与 Host 执行适配器共用。
UI capture 拥有自己的快照和容量；本地 Host query 接收实际 payload，独立准入与清理。
跨进程资源传输仍须 transport executor。

`Application::new` 显式接收 `RemoteServices`：静态契约观察与 `RemoteService` 实际 endpoint
注册分开。本地及远端服务支持 Query 和无资源附件的 Mutation；资源 mutation 与 Subscription
仍须各自 executor。`NativeService::call` 接收 `ServiceRequest`，包含已检查输入、实际附件及
由内核创建的私有 `ServiceSource`。consumer、mount 与 request 观察不授予 Host 权限。
transport callback 捕获 `remote_activation` 的准确 token，只把该 token 交给 `retire_remote`，
延迟断连不能撤销 replacement kernel。

持久 UI mutation 先构造无 effect 的 `NativeMutation` owner，准备原 native 请求、在 UI 自己
Root 中确认 caller intent，再接受 Work。`Application::finish_task` 回同一前台 owner 重查
原 Mount/registration 后返回 dispatch 或 close 后继。窗口或终端退出后，呈现器仍必须执行
全部后继。准备由 provider 的 Core Resource 保护，准入后 Work 接手；未保存预览拒绝
mutation。native 授权仍独立检查。

mutation feedback 为 `{ status: "operation", reply: OperationReply }`，保留实际原目标 Ref。
回执事实与本次实际 executor cleanup 分别结算；无法交付的结果降为同 Ref `unavailable`，
不改原 journal、不自动重放。清理失败或中断的准备仍受 owner 保有，也会列入
`failed_action_ids`；`complete_action_cleanup` 必须接收 embedding 的实际清理确认，
不能以取消或观察者消失冒充成功。

最终准入再次核对原 consumer Mount 与 service registration。原生服务 future 在后台运行，
独立于窗口；结果事件为 `{ kind: "action", key, event }`，原始输入无法伪造该来源。
executor 先结算再投递结果，已退出 consumer 不再接收；结果 handler 失败会退休 consumer，
不会静默遗留 pending。业务结果与清理确认是独立事实；清理失败或未知保留 Work/快照，
直到实际成功确认。应用 lifetime owner 在全部窗口和 worker task 退出后仍持有这些记录。

每次输出最多 16 个 action，每请求 16 个资源槽；排队、准备、已接受请求及待交付反馈
合计最多 64 个，已捕获资源
累计最多 64 MiB，UTF-8 chunk 最多 64 KiB 且检查字节边界。完整 capture 参数在 clone/
callback 前受 1 MiB 限额，也共用事务的 callback/deadline/累计工作预算。Reader 必须由包组合
显式导入。可信跨进程 transport、subscription 与产品恢复界面仍待实现。
直接 remote 注册调用实际 Host query 已用 Kit 快照跨两个内核验证。独立 Host query executor 见[Host 应用服务](../runtime-host/README.zh-CN.md)；
resource 示例证明这条原生异步路径，不代表产品已完成。

使用 `dual` 生成同一不可变包修订的两端 entry；`input.json` 选择 Desktop，
`input.tui.json` 选择以单元格布局的 TUI。运行命令见 English README。
终端呈现器只读已提交场景，实际 draw/flush 成功后才发布命中几何。
原始鼠标坐标没有 screen epoch，送达时按最后成功输出的场景立即解析，
保留准确 mount/revision。counter 是运行时验收，不是成品 UI/UX。

生产启动直接读取 State Root 的原部署来源和精确包版本，激活前检查选定呈现端。
运行来源不随 desired 更新而改写；失败或待定清理继续保有存储，成功 UI shutdown
仅释放自身 custody，数据库 owner 独立确认真实 SQL 关闭。`Application::new` 用于
未保存的 query-only 检查与预览，没有持久 DeploymentRef。示例生成器同时导出
JSON/包供检查，并保存 Desktop 部署 `00000000-0000-0000-0000-000000000001`；
`dual` 保存 TUI 部署 `00000000-0000-0000-0000-000000000002`。

持久应用在激活前提供原生实现 `maka.operations.history`；产品必须显式选择 provider
实例、`History` 导出及采用 `caller_history_contract()` 的 binding。提供实现不等于
激活插件，未保存预览不提供它。factory configuration 是空对象。普通 `page` Query
接收 `{ after, limit }`，只返回内核实际调用 instance/deployment 的 `CallerIntent[]`；
guest JSON 不能选择另一个 namespace 或 Root。

历史跨来源替换和 Root 重开保留原 caller 来源与 `RecoveryRef`。它们是观察数据，
不是执行 token 或 Host 恢复授权。每页最多 128 条，完整 JSON 数组最多 1 MiB；
以最后实际返回的 cursor 继续，count 只是上界。实际回调还包含当前 state、props 和
action 包装；成功 query 的完整交付超限时返回 `failed`，保留健康 reader，产品可以减小
页大小后重试查询，不会重新执行 mutation。恢复界面和回执读取仍由显式组合的插件提供。
所有 consumer 共用字节预算；JS 整数与编码限制只在实际 JS 回调边界检查。
原生回调精确保留 Rust JSON 整数，即使它返回可移植的树呈现。


不可变值使用普通 `Interface::Value` 导出。JS 激活返回有显式 kind 的 `UiExport`：
component 携带捕获的回调路径，value 携带按 schema 校验的数据；原生工厂返回
`NativeCapability::Value`。工厂接收 `(configuration, values)`；JS 的 activate、mount、
update、event、capture 的最后一个参数都是 `ValueBindings`；原生 call 使用第二参数，
capture 从 `CaptureCall.values` 读取。只包含当前消费者显式 Value imports，保留成员
顺序及 optional 空列表。这些是复制的纯 JSON 观察值，不授予服务权限；guest 修改
副本不能修改提供者。

loading 时捕获实际 capability，后续 inspect 原 registration，不重新按名字绑定。
完整 borrowed 入参在值复制前受 1 MiB 限额；重复 import 的每次出现都计量，JS 精度
检查在 guest 进入前完成，原生消费者保留精确整数。注册值与已提交 frame 共用应用
32 MiB 序列化保留额度，退休不等于回收，真实 registration 清理后才释放。值在本次
activation 内不可变；Label 与 Textarea 已支持局部外观投影，live provider reconcile
与完整产品界面仍需继续实现。

`maka.ui.Textarea` 契约通过 `Textarea` 导出提供 Kit 普通多行文本编辑状态。
props 要求完整 `appearance`，以及 `accessibilityLabel`、`readOnly`、`disabled`、
`softWrap`、`tabIndex`。字体、RGBA（含透明）、间距、边框与实际 scrollbar 几何/动画
投影到原有原生 owner；换肤不重置文本、选区、组合输入或撤销历史。Label 也使用
明确的文字外观。示例品牌是 composition 选择的普通 JS Value 插件，原生层不读全局品牌。

Textarea 向 AccessKit 暴露真实焦点、label、value 与可编辑状态，支持 Focus 和可编辑时
SetValue。headless 测试经过平台回调验证请求与焦点导航；普通编辑右键菜单和 OS
无障碍/IME 验收仍待完成。TUI Label 将颜色/字重投影到 RGB/bold；桌面字体与透明合成
不代表终端有等效呈现。
