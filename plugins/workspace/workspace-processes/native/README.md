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

# maka-processes-native

[简体中文](README.zh-CN.md)

Provides workspace command, terminal and protocol-process services. `protocol` handles structured subprocess communication; `terminal` manages interactive terminal resources. Configuration supplies the installation ceiling, and each call still needs its own authority. Closing an observer does not terminate ownership of a running process.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
