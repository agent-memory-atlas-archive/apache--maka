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

组件的元素 arena、GPUI 保留状态和输入闭包持有原生资源凭证。子组件从显示树移除并
退休后，最后一份凭证真实释放才结算；UI owner 的回执任务无需等下一次输入。
仍显示的退役界面继续保留清理阻塞。按原挂载父链核对归属，一个 surface 不能确认
另一个 surface 的资源。回执任务归 surface 所有，使用应用弱引用，surface 销毁时取消。
后续原生控件的 entity、订阅及未结算原生工作也必须持有相应凭证。

运行方式见 [组件示例](../ui/README.md)。这是初始呈现器；原生编辑器/IME、子 surface、
恢复 UI、运行中动态安装与完整 Host/TUI 接线仍未完成。

GPUI 的窗口移除通知早于平台销毁；当前进程退出路径只 retire 挂载并保留清理阻塞，
不宣称收到原生关闭确认。可复用的进程内 surface 必须先接入真实平台销毁完成通知。

依赖固定到 GPUI Fast `e4aabe75` 与 GPUI Kit `f89c5cc9`。使用上游 gpui-pre
重导出选择 Fast，不包含第二套 GPUI。Windows dwrote 与构建期 cbindgen 采用
MPL-2.0，不 vendoring 源码；分发须带许可证与源码链接。当前尚无此 crate 的发布打包。
