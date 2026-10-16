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

# Native build tooling

[English](README.md)

这些脚本从指定源码构建原生包及 npm 启动器。对外构建与发布顺序见 [packages/cli](../../packages/cli/README.zh-CN.md)。

macOS 原生打包需要 Xcode 26 或更新版本。Linux 需要 GLib、X11/XCB、xkbcommon、Wayland、Fontconfig 和 FreeType 开发文件，以及 cargo-zigbuild 与 Zig。[原生 CI 工作流](../../.github/workflows/npm-publication.yml)记录构建环境。

发布包使用关闭可选 LGPL glibc 数学代码的固定 V8 构建。完成的库、绑定和 notices 缓存在 `target/v8` 或 `$CARGO_TARGET_DIR/v8`。缓存未命中时，构建器下载 [v8-prebuilts.json](v8-prebuilts.json)指定的包并校验摘要。缓存损坏或固定来源失效会直接报错；没有对应固定来源时才从源码构建。源码构建还需要 Python、Git、Clang/libclang 19 或更新版本、C++ 工具链，以及较多磁盘空间和时间。

缓存键标识 V8 源码、目标、特性和生成代码。新增键时，使用已完成原生工作流输出的精确归档，将其发布为带版本的依赖预发布资产，再把 URL、哈希和验证依据写入 manifest。保留已有资产及键的含义。每个包仍执行原生链接和 smoke 检查。

依赖变更后执行：

```sh
node scripts/rust/dependencies.mjs generate
node scripts/rust/notices.mjs generate
node scripts/rust/dependencies.mjs check
node scripts/rust/notices.mjs check
```

检查生成的清单和许可证文本。[notices-sources.json](notices-sources.json)记录已审查的上游例外。发布组装还包含实际 V8 构建与 Rust 标准库的 notices。TSV 清单不能替代完整许可证文本。

在 macOS/Linux 上比较直接启动 TUI 的耗时：先构建并保留前后两个可执行文件，再运行：

```sh
python3 scripts/rust/benchmark-startup.py --binary target/debug/maka \
  --baseline /tmp/maka-before --runs 5 --output /tmp/maka-startup
```

基准使用独立数据目录，首次安装单独计时，重开按前后版本交替执行。它等待首页出现并确认命令面板可用后退出。输出目录保留原始终端输出、失败次数及耗时中位数和范围。两版应使用相同构建配置，测量期间避免同时构建。

长任务对比使用两个版本保留的 release 测试可执行文件，基准夹具必须相同。
通过 `cargo test --release --no-run` 构建 `maka-event-log --test archive`
和 `maka-agent-engine --test integration`，然后运行：

```sh
python3 scripts/rust/benchmark-long-task.py --kind engine \
  --baseline /tmp/engine-before --candidate /tmp/engine-after \
  --sizes 250,1000 --runs 5 --output /tmp/maka-long-task
```

归档测试可执行文件使用 `--kind prune`；真实上下文压缩后的 Engine 测量使用
`--checkpoint`。脚本交替运行两个版本，检查实际完成的工作相同，保留原始日志、
错误、中位数和范围。造数据不计时；Engine 耗时涵盖请求送达与执行完成，不包含
Host IPC。[Engine 夹具](../../plugins/agent/engine/tests/integration/long_task_perf.rs)
和 [prune 夹具](../../platform/event-log/tests/archive/performance.rs) 提供工具稀疏、
工具密集及投影大小等参数。测量时不要同时构建；CPU 剖析与耗时对比分开运行。
