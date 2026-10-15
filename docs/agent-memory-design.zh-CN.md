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

# Agent 内存设计

状态：设计与测量工具，尚未实施生产内存优化。基线为 `ae6c7304f`（已推送到 `origin/rust-preview`）。目标同时覆盖 1000 个轻量 Agent，以及长上下文、长输出；不重新引入 Agent 数量上限。

## 判断

优先减少每个活跃任务的实际持有量和数据副本，再补齐大载荷的字节背压。最后才评估分配器/V8 回收策略。当前证据不能把约 580 MiB 的进程峰值全部归为 Agent 对象，更不能承诺通过一个 semaphore 把 RSS 限在某个数值。

内存应拆成：共享基础设施 + 存活会话元数据 + 活跃任务状态 + 上下文/输出载荷 + 通道/网络缓冲 + 分配器与运行时保留页。会话数和真实工作量都有不可消除的成本；目标是去掉重复持有，并让等待中的大请求尽量只持有可重建的描述。

## 已验证的证据

macOS arm64 release；真实 Host、native owner、共享 provider SDK、本地 HTTP fixture 同进程；不包含 TUI 和外部模型。生命周期暂停用于归因，其时间指标无效。以下是各规模一次探索性测量，不是五次 A/B 优化结果。

1000 个轻量 Agent，每个输入 `Hello`，输出 32 × 6 B：全部正文、1000 条取消连接关闭、取消状态和新会话恢复均验证通过。

| 阶段 | RSS MiB | physical footprint（工具原值） | malloc 存活 MiB |
| --- | ---: | ---: | ---: |
| Host 就绪 | 185.9 | 97.9M | 87.5 |
| 1000 个会话创建完成 | 231.7 | 142.1M | 89.4 |
| 全部 HTTP 请求活跃、尚未输出 | 447.7 | 346.4M | 240.4 |
| 输出完成、任务退出 | 551.7 | 358.6M | 110.0 |
| 取消波次完成 | 552.8 | 357.8M | 101.7 |
| Host shutdown | 553.2 | 180.5M | 26.0 |
| fixture drop | 544.9 | 171.9M | 11.7 |

`malloc` 不覆盖全部 V8/VM 分配；RSS、physical footprint 和 malloc 存活量不是可以相加的三部分。最终 malloc 区域 dirty 100.8M、allocated 11.7M、fragmentation 89.1M，说明释放后的保留页确实存在。这既不能证明有泄漏，也不能凭一波负载排除长期增长。10/100 Agent 的 drop 后 malloc 存活量分别约 10.3/10.7 MiB。

独立启用 MallocStackLogging 的 1000 Agent 归因运行：

| 活跃期独立分配来源 | 工具报告的存活量 | 设计意义 |
| --- | ---: | --- |
| Engine 主任务 future，1000 个 | 46.9M | 约 49 KiB/run，优先定位跨 await 的大状态 |
| native submit → drive future，1000 个 | 9.77M | 第二层任务状态同样随活跃数增长 |
| ModelEvent / ProviderEvent 通道块 | 6.84M / 2.93M | 小容量通道仍可能因大 enum 槽付出固定成本 |
| Hyper 写缓冲、读缓冲、dispatch 通道 | 各约 9.77M | 网络连接并非零成本；不把它们误算成业务对象 |
| Contract/Method JSON clone 的两个 RawVec 分支 | 24.0M + 16.9M | 输出结束后仍保留，属于共享基础设施的复制成本 |

上表取分配调用树中的独立分支，没有把父子栈重复相加。追踪工具改变运行条件，因此不能用该运行的 RSS 与未追踪结果比较；原始报告也提示少量早期分配没有栈。

源码另确认了长载荷的风险：

- `engine/src/model_attempt.rs::prepare_request` 为摘要构造完整 `Value`，再构造完整序列化 `Vec<u8>`；重试路径还保留原 prompt 并为尝试克隆。
- `model/src/sdk.rs::stream` 在取得 SDK 输入预算前已经把 `ModelRequest` 转成完整 JSON。等待者仍持有这些数据。
- `js-runtime/src/trusted.rs::model` 的 32 MiB 窗口按序列化体积计量，超大请求占满窗口独占执行；它不是总内存上限。
- `op_model_emit` 的 V8 → Rust JSON 转换发生在输出预算取得前；`ProviderEvent::into_value` 又在 normalizer、model 通道和 StepBuilder 使用结束前释放预算。
- `StepBuilder::finish` 本身移动数据；复制发生在 engine 写 `Fact::ModelCompleted { output: output.clone() }` 和后续事件序列化。应改真正的复制点。

长载荷最终运行：每个 Agent 输入 64 KiB、输出 64 KiB（32 × 2048 B），10/100 Agent 各一次；全部正文按同一历史快照逐页核验，分别关闭 10/100 条取消连接，并验证新会话恢复的完成状态和正文，错误为零。

| Agent 数 / 阶段 | RSS MiB | physical footprint（工具原值） | malloc 存活 MiB |
| --- | ---: | ---: | ---: |
| 10 / 全部请求活跃 | 251.1 | 151.0M | 105.1 |
| 10 / 输出完成 | 268.1 | 163.9M | 101.0 |
| 100 / 全部请求活跃 | 363.6 | 262.7M | 185.1 |
| 100 / 输出完成 | 416.7 | 316.2M | 117.9 |
| 100 / 取消请求活跃 | 549.7 | 456.6M | 243.3 |
| 100 / fixture drop | 427.0 | 171.7M | 10.7 |

取消波次要加载已有长历史，并可能进入摘要模型，和第一波的工作并不相同。其内存明显更高，说明优化验收必须覆盖后续轮次，不能只看首轮生成。尚未对该增量做长载荷分配栈归因，不能把差值全部归给某个 clone。以上是基线归因，不是优化收益。

## 设计与实施顺序

### 1. 缩小轻量 Agent 的活跃状态

先测 `runner`、`steps`、`model_attempt`、native `drive` 各阶段 future 的大小和等待时持有的字段。将只在准备/完成阶段使用的大状态移入各自作用域；必要时只对大型、互斥的阶段 future 单独装箱，使阶段退出即释放。只给整个 runner 加 `Box::pin` 不会减少总分配量，不算优化。

通道保持现有顺序、取消和关闭语义。若大 enum 的少数变体造成全部槽位膨胀，仅对那些大变体间接存储；不笼统地把每个小 delta 都改成独立分配。用通道块分配栈与实际消息分布验证收益。

另一个确定目标是 Host `capture_bindings/initialize` 上游的不可变 Contract/Method/schema 复制。`Binding` 已使用 Arc，但注册、publication 和 plan 的其他拥有者仍可能深复制。沿实际 clone 栈让内部注册图共享冻结契约，边界 DTO 继续按需序列化；不增加按字符串重新查找的全局缓存。以 Host 就绪的 live heap 和 clone 分配栈验收，不将这项固定收益乘以 Agent 数。

共享内容只有在确实不可变且生命周期一致时才共享。运行身份、授权判断、取消状态仍各自拥有；不得为了减内存让旧 activation 引用新的契约或能力。

验收：1000 请求仍能同时进入模型；输出、取消后连接关闭、恢复、前台查询均保持正确；活跃期对应 malloc 分支减少，而不只是换到另一处分配。

### 2. 减少上下文与完成输出的整份副本

先做不改变协议的改动：摘要通过借用序列化视图写入哈希 writer，删除中间 JSON 树和字节向量。必须以现有序列化字节序列为准；字段顺序、可选字段和 projection 规则变化会影响 input digest，不能仅以语义 JSON 相等验收。

将每次尝试真正不变的 prompt/tools 与尝试可变状态分开。优先借用或移动；只有跨异步所有权边界且确实需要共存时才使用不可变共享数据。provider 的 fallback/projection 是转换，不应让多个尝试可变共享同一份 prompt。

完成事件直接从借用的输出序列化，或由单一所有者移动完成数据并返回必要摘要，删除 `output.clone()`。必须保留事件事实、重放结果、工具执行输入和取消结算语义。

验收：用长输入/输出、工具参数、媒体、重试及 provider projection 验证内容与摘要；比较相同输出字节下的峰值、分配量和 CPU，不以“clone 少了”代替结果。

### 3. 大载荷按所有权实施背压

预算必须先于受控的大分配，并覆盖其实际存活期。等待队列持有 invocation、冻结的来源/版本、取消句柄和可重建内容的位置；大历史在获准后加载与投影。新消息先按既有事件契约提交，避免每个等待任务长期保存一份重复的大正文。重建必须基于冻结的事件边界，不能读取等待期间变化后的“最新上下文”。

分清三个边界：准备/输入、在途事件、保留输出。它们有不同释放条件；不能用一个覆盖整次运行的 permit 含混表示。

- 输入：预估或读取持久化字节元数据，取得准备预算后才物化。估算不足时不能让所有任务持有部分预算并等待追加；应使用可原子满足的申请或退回描述后重新申请。超窗请求若继续允许独占，必须明确其单次载荷仍可能超过窗口。
- 在途事件：预算随载荷穿过 SDK bridge、normalizer、model 通道、批处理，直到消费/持久化后释放。保留取消可打断等待的能力。V8 中已经产生的对象仍是窗口之外的成本，不能把 Rust permit 宣称为 V8 硬上限。
- 保留输出：只延长 delta permit 到 StepBuilder 结束会造成累积到窗口后无人能完成的死锁。第一阶段先减少副本并单独计量。只有长输出实测表明有必要时，再让已提交的完整文本段由事件存储持有，builder 保留语义状态与尾段；工具调用参数等尚未闭合数据要有明确的单项尺寸边界。

事件存储已是权威来源，不新增一套平行的 spool 权威。若改为分段保留，必须一并调整完成事件、history、重放和恢复的消费方式，避免结尾重新加载整份输出制造同样峰值。这个协议改动独立实施，不夹在低风险去复制改动中。

不会通过减小 Agent 数量掩盖内存问题。1000 个 Agent 可以处于执行/等待状态，但在有限字节预算下，不保证 1000 个任意大的 HTTP 请求同时物化。

### 4. 最后评估保留页

同一进程重复波次，分别记录完成、空闲、会话退休、Host 关闭后的 malloc 与 V8/VM 存活量。确认是否达到平台后，再评估分配器策略或空闲时回收；同时测下一波启动延迟和 CPU。默认不加每次完成强制 GC、allocator purge 或频繁 runtime 重建。

## 可复现与验收

`scripts/agent-memory.py` 驱动 `owner::tests::scale::real_host_agent_scale` 的生命周期 rendezvous，保留每阶段 ps、vmmap、heap、可选 malloc_history、失败状态和二进制 SHA256。只启动并检查自己创建的测试进程。checkpoint 只暂停负载驱动，其他任务、GC 和 idle watchdog 仍可运行；它不是原子全进程快照。脚本将工具总采样时间限制为每阶段约 40 秒并记录实际暂停时间，超时保留诊断，不视为有效负载结果。

```sh
cargo test --release -p maka-agent-native -p maka-agent-engine -p maka-event-log -p maka-host-js -p maka-cli -p maka-tools --lib --no-run
python3 scripts/agent-memory.py --binary target/release/deps/maka_agent_native-<hash> --agents 10 100 1000 --output /tmp/maka-memory-light
python3 scripts/agent-memory.py --binary target/release/deps/maka_agent_native-<hash> --agents 10 100 --prompt-bytes 65536 --delta-bytes 2048 --deltas 32 --output /tmp/maka-memory-long
python3 scripts/agent-memory.py --binary target/release/deps/maka_agent_native-<hash> --agents 1000 --stacks --output /tmp/maka-memory-stacks
```

当前 fixture 的成功与取消波次都会等待所有 HTTP 请求进入后才推进。长载荷测试因此使用 10/100 Agent；不要把 1000 × 64 KiB 塞进该 barrier 并调大预算来让它通过。验证 1000 个大任务时，应增加独立的无全员 barrier 场景：所有 Agent 同时提交，服务器逐个开始响应，记录实际最大 HTTP 重叠数，并在等待预算、网络流中、持久化阶段分别取消。此场景尚待实现。

优化验收矩阵：10/100/1000 轻载；10/100 长输入长输出；1000 混合尺寸排队；同进程至少五波负载；慢消费者和取消。每个 A/B 性能点至少五次未追踪 release 运行，另做栈归因；报告输出字节、最大活跃数、错误、CPU、吞吐、前台 p99、峰值和静置后内存。不得把生命周期暂停的时间当作性能数据。

最初轻载归因与 MallocStackLogging 使用扩展长载荷参数之前的测试二进制（SHA256 `e7a1b23b51e623db6c6ffc16c99f3a308ff06af496019356a25caa6b2bb89f57`）；新 fixture 在验证请求后立即释放服务端解析树，减少测量夹具的重复持有；恢复探针在新会话中验证共享执行能力，检查完成状态和完整正文，避免长历史自动摘要超出单响应 fixture 的协议。取消波次沿用原会话，因而可能取消摘要请求。上方最终 1000 轻载与长载荷表格使用同版二进制，SHA256 为 `af127813314f8442fdf5bd614b265b9c6e505cfb2af75bbd4566ebbb430414ff`。因此后续 A/B 必须使用同版夹具。

本轮原始证据在 `/tmp/maka-memory-probe`、`/tmp/maka-memory-stages`、`/tmp/maka-memory-stacks`、`/tmp/maka-memory-long-v3`、`/tmp/maka-memory-light-final`；这些是本机临时产物。上方表格保留决策相关结果，脚本支持重新生成完整证据。

排除的试运行：`/tmp/maka-memory-long` 只读 history 首页而误报；`/tmp/maka-memory-long-v2` 的成功输出已逐页核验，但恢复探针把摘要失败后的 idle 误判为恢复成功。这两组不作为完整负载通过证据。
