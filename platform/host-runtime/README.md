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

# maka-host-runtime

[简体中文](README.zh-CN.md)

Runs checked service compositions and owns call admission, activation and retirement. Products supply native factories and, when needed, a JavaScript backend. See `application` for the owner and client API; `application/native_context.rs` is the starting point for native services.

A committed source is desired configuration. It becomes effective only after its instances converge. Updates preserve unchanged activations and their captured bindings; replacements wait for accepted calls and native resources to settle. Mutation receipts distinguish business results from cleanup, so a lost reply must be resolved through its original receipt.

Persistence comes from the owned EventLog. Stable plugin data belongs to the Root, deployment and instance; an activation epoch identifies execution, not a new data owner.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
