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
不能替换已注册处理器。回调返回 `ComponentOutput { frame, events, actions }`；events/actions 是
本次事务的一次性意图，不属于保留状态。声明 capture 时也在发布前捕获其回调。呈现器只读取已提交
`Frame`。有界批次完整校验后原子替换，每个挂载有独立状态与 revision，旧视图事件
不会调用新处理器。执行域故障撤销该域全部注册与挂载。

`NativeRegistry<V>` 提供随构建编译的工厂；只有组合选中的 entry 才会激活，实际导出
仍由同一个 runtime 校验并发布。原生组件与 JavaScript 共用 props/state/revision 和
整批事务。`Presentation<V>` 可以是有界树或不序列化的类型化不可变原生呈现描述，
内核不依赖 GPUI。原生输入按当前 scene 的 schema 校验；调用在派发前受 1 MiB 限额。
Rust 扩展是受信任同步代码，没有 JavaScript watchdog 隔离；统一提交 deadline 能拒绝
迟到结果，不能中断 Rust。4 MiB 更新及 32 MiB 保留限额计量序列化元数据、树和输入
schema，不计原生内存；原生 owner 负责其实际分配限额。
呈现描述不能持有 entity、输入绑定、订阅或原生 work；它们归呈现器资源 owner 所有，
通过资源凭证等待真实销毁，不依赖退休 frame 的描述被释放。

树协议包含 stack、text、button 和导入组件。子组件引用显式 binding 与 member，
稳定 key 在父组件 props 更新时保留子挂载和局部状态。父子组件共用一次 deadline，
整批提交；任一子组件失败，原有 frame 全部保留。移除后挂载只进入退休，等待真实原生
清理确认；未提交的挂载尚无原生资源。根挂载窗口配置固定，嵌入组件不能创建窗口。
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

生成示例时在输出目录后加 `composed`，再向 desktop 传入额外的
`dynamic.counter.maka-plugin`，即可使用独立包提供的计数器组件。改用 `native` 并传入
`native.label.maka-plugin` 可显式导入编译的 Label；它直接使用 GPUI 样式，布局位置与
props 由产品决定。opaque 原生子组件槽位、同步 consume/default 决策仍未实现。

使用 `editor` 并额外传入 `native.editor.maka-plugin` 可导入 Kit Editor。Rust 契约表达
初始文本、基于原生 revision 的 replace/reset 命令、元数据观察与命令回执；原生 buffer
不放进序列化组件 state。公开 `changed` 事件包含类型化元数据观察与回执；收到
TextRef 不获得全局查找或读取权限；capture 提供请求私有的不可变文本资源。`command: null` 不提出
新命令，不取消已接受的请求；请求保留 pending 直到匹配回执提交，最终原生描述只消费一次。
原生组件 event 同时收到当前挂载 props、state、key 与 input；JavaScript
`event` 的第三个参数是 props。

包驱动的桌面示例命令见 [English README](README.md)。示例与校验使用同一套 Rust
契约类型，无旧插件 SDK 或全局 service catalog。

改用 `events` 生成显式订阅 Editor `changed` 的产品组件，并显示原生草稿 revision；
向 desktop 传入与 editor 示例相同的五个包。该示例验证公开事件送达，不代表资源读取、
业务 action 或最终产品视觉设计已完成。

改用 `resource` 生成产品 Root → 动态 Composer → 私有原生 Editor，以及显式导入的原生
UTF-8 Reader；完整五包命令见 [English README](README.md)。Root 只调用 Composer
公开 capture，不依赖私有 Editor 的挂载路径。

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

最终准入再次核对原 consumer Mount 与 service registration。原生服务 future 在后台运行，
独立于窗口；结果事件为 `{ kind: "action", key, event }`，原始输入无法伪造该来源。
executor 先结算再投递结果，已退出 consumer 不再接收；结果 handler 失败会退休 consumer，
不会静默遗留 pending。业务结果与清理确认是独立事实；清理失败或未知保留 Work/快照，
直到实际成功确认。应用 lifetime owner 在全部窗口和 worker task 退出后仍持有这些记录。

每次输出最多 16 个 action，每请求 16 个资源槽；排队加已接受请求最多 64 个，已捕获资源
累计最多 64 MiB，UTF-8 chunk 最多 64 KiB 且检查字节边界。完整 capture 参数在 clone/
callback 前受 1 MiB 限额，也共用事务的 callback/deadline/累计工作预算。Reader 必须由包组合
显式导入。Host transfer/持久准入、JavaScript binding bridge 与 UI 集成、TUI 接线及产品
恢复界面尚未实现。独立 Host query executor 见[Host 应用服务](../runtime-host/README.zh-CN.md)；
resource 示例证明这条原生异步路径，不代表产品已完成。
