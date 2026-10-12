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

# maka-acp-native

[简体中文](README.zh-CN.md)

Registers the native ACP Executor service and connects the engine to imported process, file and form services. `provide` adds its factories to a Host registry. The product composition chooses executable instances and bindings; the service does not obtain process access merely by being linked.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
