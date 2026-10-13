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

# maka-goal

[English](README.md)

管理持久目标、进度报告、控制请求和 token 预算。`goal` 定义状态转换，`owner` 提供执行端口适配；当前原生 Agent 将同一存储与状态转换接入原有消息队列。目标报告与一次执行的完成是不同的观察结果；所有者根据当前目标状态和剩余预算决定能否接受下一次运行。存储将冻结的授权保存在目标内，并可将调用方的原始操作回执与状态变化原子提交。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
