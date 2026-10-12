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

# Native Rust extensions

Native extensions are compiled into the Host or presenter and selected by an explicit
`implementation` name in a package entry. JavaScript packages cannot supply arbitrary
Rust factories. Register Host factories through `NativeRegistry`; publish exact
contracts and handlers in `NativeActivation` with the actual `NativeOwner`.

`NativeContext` supplies the actual activation, package, persistence and captured
imports. Stable persistence belongs to the Root domain, deployment UUID and instance
identity; source revision and activation epoch do not create new data owners.
A factory that fails after acquiring resources must return an owner that can settle
them; an initializer error does not prove cleanup.

Service calls receive the original `Context.scope()`. Native I/O keeps a resource
ticket until physical completion. Use the scope's cancellation for child work and
finish the real request before reporting cleanup. A task tracker or retained ticket
must outlive a lost observation. `NativeOwner.close` confirms actual settlement;
returning success without joining owned workers breaks retirement and capacity.

`NativeServices` captures original imports and adds private native-origin evidence;
every call still passes Host policy. Public `Identity` is observation, not permission.
`execution_basis` may attest only immutable facts that fully identify equivalent
execution semantics. Leave opaque owners activation-specific when that cannot be
proven; do not override it to force a recovery test to pass.

Relevant examples: `plugins/skills/native/src/lib.rs`, `plugins/workspace/workspace-web/native/src/host.rs`
and `platform/host-runtime/src/application/native_context.rs`. Native UI objects remain
in their presenter owner/main-thread lifecycle; the plugin kernel owns descriptions,
checked bindings and teardown coordination, not framework objects.
