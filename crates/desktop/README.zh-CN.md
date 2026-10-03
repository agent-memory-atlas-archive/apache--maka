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

运行方式见 [组件示例](../ui/README.md)。这是初始呈现器；真实 OS IME 验收、子 surface、恢复 UI、运行中动态安装与完整 Host/TUI 接线仍
未完成。counter/editor 示例用于验证运行时，不代表默认产品的最终视觉设计。

GPUI 的窗口移除通知早于平台销毁；独立 window owner 等待补丁提供的后端销毁回执、
实际 entity/保留帧释放与子挂载清理，才确认 root 完成。最后窗口移除不能提前退出应用，
清理失败继续保留 fence。已接受的异步服务独立持有不可变快照，不保留原生 UI 资源；
application lifetime owner 在窗口和 executor task 都退出后仍保留未结算 Work，只有真实
成功清理才允许 shutdown。显式 Composer/Reader 路径与限额见[资源示例](../ui/README.zh-CN.md)。

依赖固定到 GPUI Fast `e4aabe75` 与 GPUI Kit `f89c5cc9`。使用上游 gpui-pre
重导出选择 Fast，不包含第二套 GPUI。Windows dwrote 与构建期 cbindgen 采用
MPL-2.0，不 vendoring 源码；分发须带许可证与源码链接。当前尚无此 crate 的发布打包。

生产启动使用 `--root PATH --deployment UUID`，读取原存储来源和精确包字节，
激活前检查 Desktop 目标。desired source 更新不改写运行来源。应用保有 Root 至真实
成功清理；UI shutdown 释放自身 custody，与 SQL 连接真实关闭分别确认。

两个启动器都支持 `--connections FILE`，使用[可信本地 Host 连接](../application-bridge/README.zh-CN.md)。
唯一前台 lifetime owner 在平台循环丢失后仍持有真实 worker observer。原生退出必须等
物理 UI、已接受 Work、caller SQL 关闭确认和 Host 清理全部完成；失败继续保留
store/Root/runtime。macOS 已用真实窗口及阻塞的 File/SQL owner 验证原生退出；
Windows/Linux 原生验收和完整产品 UI 仍未完成。
