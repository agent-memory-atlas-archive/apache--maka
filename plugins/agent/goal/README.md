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

[简体中文](README.zh-CN.md)

Manages persistent goals, progress reports, control requests and token budgets. `goal` defines the state transitions; `owner` provides the execution-port adapter; the current native Agent connects the same repository and transitions to its original message queue. A goal report and the completion of an execution are distinct observations. The owner decides whether another run may be admitted using the current goal state and available budget. The repository stores the frozen approval with each Goal and can atomically commit a caller’s original operation receipt with the state change.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
