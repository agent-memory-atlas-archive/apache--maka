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

# Product bundle

[English](README.md)

本目录选择 Maka 分发的包，并将其绑定成 Agent、WorkHub 或 Studio 组合。插件声明与实现位于 [plugins](../../../plugins/README.zh-CN.md)。

- [package-inputs.json](package-inputs.json)选择参与构建的包。
- [src/composition.ts](src/composition.ts)构造实例、绑定和安装身份。
- [scripts/build.mjs](scripts/build.mjs)将包内容与组合生成代码输出到 `dist`。

## 运行与开发

在仓库根目录运行：

```sh
just run
just run --root /tmp/maka-workhub --product workhub
```

首次启动确认工作区并安装所选终端产品，之后重新打开保存的配置。默认 UI Root 为 `~/.maka/application`，Host Root 位于其 `host` 子目录。Settings 和 Plugins 修改已安装应用；启动选项不会覆盖已有安装。

`just product` 重建 TypeScript bundle。开发二进制读取 `dist` 或 `MAKA_PRODUCT_BUNDLE` 指定的目录，仅修改 TypeScript 无需重新编译 Rust。发布二进制嵌入 bundle。需要与开发构建同时生成产物时，使用独立输出目录：

```sh
node products/maka/bundle/scripts/build.mjs --output /tmp/maka-product-bundle
```

## 终端交互

未配置模型也能创建任务和输入草稿。发送时若缺少模型，会进入连接配置；完成后返回原草稿并打开模型选择。提供商按服务分组，套餐、地区等变体及高级配置按需展开。

任务侧栏与工作区并排显示。拖动分隔线可调整宽度，也可用 Tab 聚焦分隔线，再用左右方向键调整、Home 恢复默认宽度。详情和运行中的终端显示在辅助栏；工作区较窄时显示当前辅助面板，关闭后恢复对话和草稿。Esc 优先关闭最近的临时界面。

模型、连接、项目和任务列表随滚动加载或显示后续条目。渲染窗口保持有界，移动窗口时用重叠条目保持视口位置。滚动不抢键盘焦点；方向键和 Enter 提供对应的列表操作。审批和问题随当前任务展示，操作历史仍可用于恢复未确认的结果。

## 生成安装配置

CLI 可以生成经过校验的配置和包，运行时无需 Node：

```sh
maka application compose --configuration installation.json --output product-source
```

输出目录必须不存在。命令生成 Host、管理服务和 UI 配置、连接授权、完整安装身份及 `.maka-plugin` 包，不安装或启动它们。

`installation.json` 选择 `presentation`（`tui` 或 `desktop`）、`locale`、`scheme`、`hostRoot` 和服务配置。Agent 使用生成的 `AgentConfiguration`；WorkHub 另外配置独立 `coordinator`。可选服务和 ACP 执行器需要显式选择。支持字段见[配置解析器](src/composition.ts)和 [SDK 类型](../../../packages/plugin-sdk/src/generated/runtime.ts)。

重新生成已有安装时，保留输出的 `keys`、`hostDeployment` 和 `managementDeployment`，它们标识已有所有者及数据。包修订标识内容，可以独立变化。

终端安装可通过 `maka application launch --root UI_ROOT --configuration installation.json` 初始化并启动。重新打开时只传 `--root UI_ROOT`；初始化中断后会沿原准备记录恢复。

显式安装时，先用 `maka host init` 初始化 Host 与 UI Root，再用 `maka application commit` 提交 `host.json`、`management-host.json` 和 `ui.json`。管理服务属于 UI Root。提交时提供部署 ID、预期修订、导出的包，UI 还需 `remote-exports.json`。通过 `maka application run` 或 `maka desktop` 启动 UI，传入 Root、部署 ID 和生成的 `connections.json`。各命令的 `--help` 列出必要参数。

## 验证修改

```sh
npm --workspace @maka-agent/product run typecheck
npm --workspace @maka-agent/product test
MAKA_PRODUCT_CLI_CHECK=1 npm --workspace @maka-agent/product test
```

测试只构建一次，再执行保留的组件行为检查。设置 `MAKA_PRODUCT_CLI_CHECK` 后，还会使用实际 CLI 检查包和组合。原生渲染、焦点和输入变更需要在目标呈现端验证。
