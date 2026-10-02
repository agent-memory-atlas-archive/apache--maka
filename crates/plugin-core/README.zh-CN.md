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

Host、TUI 与 Desktop 共用的声明和组合解析机制，不依赖 Host 业务、JavaScript
引擎或呈现框架。

`composition::Input` 是可编辑的部署输入。`plan::Plan::prepare` 展开不可变包模板、
应用属于具体实例的覆盖并预检显式能力绑定，调用方通过检查后才能提交输入。
运行计划只是投影，不是第二份配置。稳定来源 key 与 alias 分离，包 revision 固定
确切字节；数据 schema 兼容与副作用授权仍需各自的运行检查。

契约声明通过 `schemars` 派生 JSON schema。激活导出批次发布前必须通过
`EntryDeclaration::check_exports`。目前采用保守兼容规则：名义名称、版本和完整
公开结构一致。远端声明只能证明兼容，不能证明就绪或授予权限。

应用包使用 Manifest schema version 2 的 `application` 声明。CLI 可以在不激活
部署的情况下检查它们：

```sh
maka application check --input deployment.json --package /absolute/package/path
```

检查器不执行 JavaScript，也不创建原生 UI 资源。
