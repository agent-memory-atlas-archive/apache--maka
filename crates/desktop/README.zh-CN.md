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

# 原生桌面

`maka-desktop` 用长桥 GPUI Fast 与 GPUI Kit 呈现包中的组件，只接受 Desktop
部署输入。共用执行适配器由 UI owner 线程持有；layout/paint 只读 Rust 已提交帧，
按钮回调携带 revision 派发输入后再通知重绘。二进制没有内置业务壳或默认产品。
随构建提供的组件由 `NativeRegistry<dyn Widget>` 和普通包绑定选择，Widget 直接呈现
原生 GPUI 元素；共用输入绑定持有准确 mount/revision 和资源凭证，paint 不派发输入。
呈现描述只包含不可变数据；entity/订阅归独立 renderer owner 所有，GPUI 保留状态与
arena 也可持有资源凭证，不能缓存到清理期间保活的描述中。owner 在整批提交之后、
layout/paint 之前准备资源、捕获请求私有快照，再消费已提交命令。Kit editor 的资源凭证保留到实际 entity
释放处理；仅 drop handle 不等于实体已销毁。


公开的 [GPUI adapter](../ui-gpui/README.zh-CN.md) 向其他 Rust crate 提供
`Widget`、`Slots`、输入绑定与 editor availability。原生布局可在 GPUI Element 或
虚拟列表静态回调里呈现已提交 slots。handle 捕获原子树和具体 native owner 身份，
不能改指重建 owner、呈现 detached 子组件或跨原平台窗口。测量可以重复；prepaint
每个 mount 只允许一次实际放置，缓存重放与 Err 重试回滚同样检查。handle 只持自身
子树的资源凭证，不阻塞无关兄弟组件清理。

组件的元素 arena、GPUI 保留状态和输入闭包持有原生资源凭证。子组件从显示树移除并
退休后，最后一份凭证真实释放才结算；UI owner 的回执任务无需等下一次输入。
仍显示的退役界面继续保留清理阻塞。按原挂载父链核对归属，一个 surface 不能确认
另一个 surface 的资源。独立 window owner 保有应用，直到后端销毁、资源释放和挂载清理真实确认，
surface 销毁不会取消该职责。
后续原生控件的 entity、订阅及未结算原生工作也必须持有相应凭证。

显式导入的 Editor 由 Kit 唯一持有 buffer/selection/undo 与平台输入引擎，父 props 更新
不会重建它。replace/reset 带原生 revision 与 sequence，修订冲突或活跃 composition 会
拒绝命令；前一命令回执发布前不准入下一命令。实际执行后给出 applied/conflict 回执。
非 composition 时 reset 清除 undo，不宣称取消操作系统 IME。观察只携带资源元数据，
读取校验已绑定 owner 的身份、修订与 UTF-8 边界，最多 64 KiB；平台 UTF-16 查询共用
这条读取路径。

运行方式见 [组件示例](../ui/README.md)。counter/editor 示例用于验证运行时，
不代表默认产品的最终视觉设计。生产运行中的动态安装与共用 Host/TUI 接线已可用；
完整 Agent UI、真实 OS IME 验收、子 surface 与恢复 UI 仍未完成。

GPUI 的窗口移除通知早于平台销毁；独立 window owner 等待补丁提供的后端销毁回执、
实际 entity/保留帧释放与子挂载清理，才确认 root 完成。最后窗口移除不能提前退出应用，
清理失败继续保留 fence。已接受的异步服务独立持有不可变快照，不保留原生 UI 资源；
application lifetime owner 在窗口和 executor task 都退出后仍保留未结算 Work，只有真实
成功清理才允许 shutdown。显式 Composer/Reader 路径与限额见[资源示例](../ui/README.zh-CN.md)。

依赖固定到 GPUI Fast v0.1.2（`8e1e1987`）与 GPUI Kit `29c04574`，通过 Kit
官方 `gpui-fast` feature 选择原生后端。本地补丁保留 Maka 的原生输入、选择与清理契约。
Windows dwrote 与构建期 cbindgen 采用
MPL-2.0，不 vendoring 源码；分发须带许可证与源码链接。当前尚无此 crate 的发布打包。

生产启动使用 `--root PATH --deployment UUID`，读取原存储来源和精确包字节，
激活前检查 Desktop 目标。desired source 更新不改写运行来源。应用保有 Root 至真实
成功清理；UI shutdown 释放自身 custody，与 SQL 连接真实关闭分别确认。

两个启动器都支持 `--connections FILE`，使用[可信本地 Host 连接](../application-bridge/README.zh-CN.md)。
唯一前台 lifetime owner 在平台循环丢失后仍持有真实 worker observer。原生退出必须等
物理 UI、已接受 Work、caller SQL 关闭确认和 Host 清理全部完成；失败继续保留
store/Root/runtime。macOS 已用真实窗口及阻塞的 File/SQL owner 验证原生退出；
Windows/Linux 原生验收和完整产品 UI 仍未完成。

生产启动使用 `maka-desktop run --root PATH --deployment UUID`。默认启动器选同一
executable 的内部 `application-host` role，共享 Host 经私有 IPC 提供业务服务；编译
availability 在 child 内登记，持久组合决定实例。父子 runtime 均启用 Tokio IO；真实 UI
清理后继续撤销原连接、等 same-child Closed 与实际 child exit。其他 embedding 显式
提供自己 compiled role 的可信 HostExecutable，不跨进程传递 native factory 闭包。


前台原生 pump 在 paint 外推进已接纳的 committed plan，并在继续资源准备和动作处理前
撤销退休控件的原生输入。未改变的 Mount 保留原 editor entity 和阅读作用域。
独立新插件初始化失败是部署诊断，不关闭仍健康的原窗口；实际原生同步与清理错误
仍保留原错误和 fence 语义。生产管理写入使用原 caller operation 与持久来源；
同一前台 Updates 管线经原连接和 application owner 批准并接纳已提交来源。
生产 launch owner 对初始和替代根共用窗口创建路径；替换须等
原窗口 owner 确认平台销毁、实际 entity 释放和 mount 清理，再以同一 application 和
已接受任务 owner 打开全新根。无关已接受任务可与新根并存；替换其 provider 才等待
原结算。清理失败不能放行替换。显式原生退出先禁止新准入与替换，再移除窗口。
GPUI 测试覆盖真实 Kit entity 持有、原快照、provider fence 及交接期间退出。

macOS 实窗已通过系统文件面板安装普通包、保存完整来源，并在同一进程内自动
从 Agent 替换为 Configuration Studio，启动原先闲置的独立 Host；新产品保存的
连接只进入该 Host 的设置存储。正常关闭窗口后，父进程、两个子进程和三个 Root
均已释放。这不证明交接时旧 Host 即时释放。输入法、屏幕阅读器、Windows/Linux
及完整产品视觉验收仍待完成。

foreground 执行 owner 在创建平台清理观察者之前保有每个原 `NativeResources` 对象，
只有原 surface 真实成功结算才释放。观察失败或消失时，窗口/Surface 销毁也不会丢弃
原生 worker custody 与分配配额。`LiveInput` 提供原 mount 的工作组与呈现器共享预算。
健康清理必须同时满足真实 lease 销毁和 sealed 工作组成功结算；ticket 实际释放
tracker token 后唤醒同一个关闭 owner。连接与 caller 存储必须在创建执行 owner 时
交入，后续冲突的 backend custody 会被拒绝，不能静默忽略。
