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

# maka-scheduler

[English](README.md)

管理定时任务定义、调度决策和投递状态。`controller` 驱动流程，`repository` 持久化任务，`authorization` 检查任务权限。任务到期仍需正常执行准入；投递与恢复记录用于避免将结果不确定误当作再次派发的许可。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
