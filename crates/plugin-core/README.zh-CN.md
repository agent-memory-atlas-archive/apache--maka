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

# 插件核心

Host、TUI 与 Desktop 共用的声明、组合解析与生命周期权威，不依赖 Host 业务、JavaScript
引擎或呈现框架。

`composition::Input` 是可编辑的部署输入。`plan::Plan::prepare` 展开不可变包模板、
应用属于具体实例的覆盖并预检显式能力绑定，调用方通过检查后才能提交输入。
运行计划只是投影，不是第二份配置。稳定来源 key 与 alias 分离，包 revision 固定
确切字节；数据 schema 兼容与副作用授权仍需各自的运行检查。

契约声明通过 `schemars` 派生 JSON schema。激活导出批次发布前必须通过
`EntryDeclaration::check_exports`。目前采用保守兼容规则：名义名称、版本和完整
公开结构一致。远端声明只能证明兼容，不能证明就绪或授予权限。

`runtime::Runtime<T>` 管理激活和注册 epoch、原子检查实际导出批次、撤销消费者，
并跟踪已接受执行及资源清理。执行适配器负责运行、取消和关闭确认；核心没有执行器，
不要求原生值实现 `Send`/`Sync`。初始化中的消费者可以调用就绪依赖，然后才发布自身导出。
撤销关闭新准入，已接受工作仍由提供者持有，直到明确结算；丢弃执行 token 不等于结算。
失败或未知清理阻止替换，失败结算返回新撤销的 owner，供适配器发送停止信号。
纯模板的 imports 只路由依赖，不成为整个产品的执行依赖。

授权和持久业务操作准入仍由执行环境负责。此生命周期核心目前解析本地实际注册；
配置的远端绑定在 transport 注册适配器实现前保持未就绪。

应用包使用 Manifest schema version 2 的 `application` 声明。CLI 可以在不激活
部署的情况下检查它们：

```sh
maka application check --input deployment.json --package /absolute/package/path
```

检查器不执行 JavaScript，也不创建原生 UI 资源。

组件挂载是独立所有者；撤销一个挂载会停止其子挂载，不撤销可复用提供者。
提供者撤销会停止依赖挂载。原生 close 确认与 accepted call 结算是两个独立清理条件，
不能互相替代或提前释放清理阻塞。

组件契约声明命名 capture、输入 schema 与名义只读资源类型；service method 声明类型化
资源输入槽。资源名称/版本描述形状，不授予权限；实际 payload 与授权归执行适配器的
请求私有附件，不能由 JSON metadata 伪造。声明必须显式提供 `captures` / `resources`
map，不为旧格式保留解码回退。
