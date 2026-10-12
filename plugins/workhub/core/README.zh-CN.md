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

# maka-workhub

[English](README.md)

协调 WorkHub 任务、worker 选择、队列和结果。`Manager` 管理流程，`repository` 存储任务事实，`recovery` 恢复中断的协调过程。worker 执行和权限仍由 Agent 服务负责。worker 结果不确定时应查询原操作，而非重新派发。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
