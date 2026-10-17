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

# Maka plugin SDK

[English](README.md)

用于 Maka Host 服务和原生 UI 组件的 TypeScript 契约与辅助函数。从 `@maka-agent/plugin-sdk` 导入；guest 代码需要先打包，再交给运行时。

| 源码 | 用途 |
| --- | --- |
| [index.ts](src/index.ts) | `HostService`、`RequestContext`、`Component` 和公开导出 |
| [ui.ts](src/ui.ts) | 终端与桌面场景构造函数 |
| [generated/runtime.ts](src/generated/runtime.ts) | Rust 协议类型与原生组件 props |
| [contracts.ts](src/contracts.ts) | 服务和组件契约 |
| [configurations.ts](src/configurations.ts) | 原生配置 schema |
| [native.ts](src/native.ts) | 包声明使用的原生实现定义 |

生成文件以 Rust 声明为源。修改声明后，在仓库根目录运行：

```sh
npm --workspace @maka-agent/plugin-sdk run schema:generate
npm --workspace @maka-agent/plugin-sdk run schema:check
npm --workspace @maka-agent/plugin-sdk run typecheck
```

生成过程使用 [maka-schema](../../tools/schema/README.zh-CN.md)，无需构建产品或 V8。运行时也可通过 `maka application schema` 导出目录。

组件的同步回调提出帧和动作，终端与桌面使用各自场景类型。原生控件管理编辑器缓冲区、选择和 IME；应通过声明的契约捕获资源。Host 服务接收请求作用域内的导入与附件，不提供环境级 Node API。

客体运行时提供 `TextEncoder`、`TextDecoder`、`atob`、`URL` 和 `URLSearchParams` 进行纯文本与字节转换。可用 `Uint8Array.from(atob(value), c => c.charCodeAt(0))` 将 Base64 解码成字节；这些 API 不授予文件或网络访问权限。

TypeScript 帮助发现编写错误，运行时另外校验值、契约兼容性、资源槽位和权限。参阅[插件指南](../../plugins/README.zh-CN.md)和[可运行示例](../../skills/maka-plugin-authoring/references/quickstart.md)。
