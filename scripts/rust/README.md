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

[简体中文](README.zh-CN.md)

These scripts build source-bound native packages and their npm launcher. The public command sequence is in [packages/cli](../../packages/cli/README.md).

Native macOS packaging requires Xcode 26 or newer. Linux needs GLib, X11/XCB, xkbcommon, Wayland, Fontconfig and FreeType development files, plus cargo-zigbuild and Zig. The [native CI workflow](../../.github/workflows/npm-publication.yml) records the build environment.

Release packages use pinned V8 builds with optional LGPL glibc math disabled. Completed libraries, bindings and notices are cached in `target/v8`, or `$CARGO_TARGET_DIR/v8`. On a miss, the builder downloads the bundle named by [v8-prebuilts.json](v8-prebuilts.json) and checks its digest. A corrupt cache or broken pin fails explicitly. An absent pin triggers a source build, which also needs Python, Git, Clang/libclang 19 or newer, a C++ toolchain, substantial disk space and time.

A cache key identifies the V8 source, target, features and producer code. For a new key, use the exact archives emitted by a completed native workflow. Publish them as versioned dependency prerelease assets, then record their URLs, hashes and validation evidence in the manifest. Preserve existing assets and key meanings. Native link and smoke checks still run for each package.

After dependency changes:

```sh
node scripts/rust/dependencies.mjs generate
node scripts/rust/notices.mjs generate
node scripts/rust/dependencies.mjs check
node scripts/rust/notices.mjs check
```

Review the generated inventory and license texts. [notices-sources.json](notices-sources.json) records reviewed upstream exceptions. Release assembly also includes notices from the actual V8 build and Rust standard library. The TSV inventory does not replace full license texts.

To compare direct TUI startup on macOS/Linux, build and retain each executable,
then run:

```sh
python3 scripts/rust/benchmark-startup.py --binary target/debug/maka \
  --baseline /tmp/maka-before --runs 5 --output /tmp/maka-startup
```

The benchmark uses isolated roots, measures installation separately, and alternates
reopens. It waits for the home screen and verifies the command palette before
quitting. The output directory retains raw terminal output, failures, and timing
medians/ranges. Keep build profiles equal and run without competing builds.

Long-task comparisons use retained release test executables from both revisions,
with the same benchmark fixtures. Build `maka-event-log --test archive` and
`maka-agent-engine --test integration` using `cargo test --release --no-run`, then run:

```sh
python3 scripts/rust/benchmark-long-task.py --kind engine \
  --baseline /tmp/engine-before --candidate /tmp/engine-after \
  --sizes 250,1000 --runs 5 --output /tmp/maka-long-task
```

Use `--kind prune` for the archive test executables and `--checkpoint` for an
Engine run after real context compaction. The script alternates revisions and
checks equal completed work, preserving raw logs, errors, medians and ranges.
Fixture setup is excluded; Engine timings cover request delivery and completion,
not Host IPC. The [Engine fixture](../../plugins/agent/engine/tests/integration/long_task_perf.rs)
and [prune fixture](../../platform/event-log/tests/archive/performance.rs) expose
workload settings for sparse tools, dense tools and projection sizes. Run without
competing builds; collect CPU profiles separately from timing comparisons.
