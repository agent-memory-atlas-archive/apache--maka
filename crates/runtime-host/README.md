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

# Host application services

`application::Application` runs a service deployment using the same plugin-core
composition and lifecycle kernel as the UI. It accepts immutable application
packages and an explicit native implementation registry; it does not activate
built-in business plugins by default. The existing server entry point has not
yet migrated to this adapter.

The owner task publishes an entire validated export batch. JavaScript services
retain actual receiver/functions through `Captured`, in bridge-disabled
`Vm::pure` realms. Native activation supplies an independent `NativeOwner` whose
real close completion covers its resources. Initialization is fenced before any
asynchronous work starts. Rejected or late exports still close under that original
owner; failed close retains the actual native owner and lifecycle fence.

Bindings capture the exact consumer and registration. Final admission checks the
Host `Issuer`, current Scope cancellation, method input and explicit Host `Policy`,
then accepts work through the core without waiting between those checks. Forwarded
child Scopes retain the original private evidence. A package name, source identity
in JSON, or binding is not effect authorization.

An `Accepted` result is only an observer. The owner retains execution, Scope and
input capacity until the actual query and its resources settle. Ordinary business
errors and result validation errors settle cleanly when resource cleanup succeeds.
Unconfirmed cleanup or an executor panic preserves work, Scope and capacity, and
retires affected registrations. Dropping the application root requests retirement;
client handles do not keep the root alive or cancel accepted work. Shutdown success
requires native/module cleanup and actual VM shutdown. Shared VM failure retires
its actual registrations and consumers, leaving unrelated domains independent.

Calls have a 1 MiB JSON limit, with 128-call and 32 MiB in-flight input budgets.
This adapter currently executes queries. Mutation, subscription and request-local
resource attachments are rejected before activation until their durable, stream
and attachment executors are implemented. Host transport, persistent composition
commit/reconciliation, operation recovery, the JavaScript binding bridge and SDK,
and server/TUI/UI integration remain required work. This foundation is not a
completed Host migration or a compatibility facade for the previous catalog.
