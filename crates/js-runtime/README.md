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

# JavaScript execution

Model Code Mode and plugin execution use separate V8 runtimes. Plugin `Vm` runs
prebuilt ESM bundles on a background worker; `Foreground` runs bounded synchronous
component callbacks on the UI owner thread. Neither installs ambient filesystem,
network or process capabilities. Legacy background system work uses the selected module's
explicit `Bridge`; request-scoped work uses the native context below. A module
name or JSON identity does not confer authority.


`Vm::pure` provides a background realm without Host ops, ambient Deno or legacy
SDKs. It rejects any bridge-bearing or SDK load, including request contexts.
An absent Bridge on a normal VM alone is not this isolation boundary.

`Vm::scoped` installs only the private request op. Its trusted factory captures
the op and intrinsics before loading guest code, then removes Deno globals. It
rejects legacy Bridge/SDK loads. `Captured::call_with_context` appends a frozen
context backed by a native cppgc type and a Weak request owner, without an ID
router. Completion and VM termination release accepted/queued contexts before
reporting completion. Saved closures or pending op observers cannot keep that
request owner alive. Request ops have 1 MiB JSON input/output limits and shared
128-op / 32 MiB input budgets. Trusted `RequestBridge` adapters own real work and
cleanup independently of JS observers; native failure cannot be acknowledged by
a guest catching rejection. Host uses this realm for attached JavaScript queries.

Before publishing an actual service registration, call `Module::capture(namespace,
methods)`. Its opaque `Captured` handle retains the actual receiver and functions;
`Captured::call` supports Promise results without looking up the namespace again.
Later function/namespace replacement cannot redirect that handle. Names are limited
to 128 bytes, paths to 32 segments, methods to 64 distinct names, and the VM to
1,024 captured registrations. Last handle drop releases captured globals. Even an
unclaimed successful capture reply carries its own release obligation.

All worker calls retain their module and VM until the actual Promise completes,
including when the waiting future and every external VM/module handle are dropped.
`Module::close` stops new calls and waits for admitted work before reclaiming code.
VM failure destroys pending JavaScript and globals before acknowledging reclamation.
These are code-lifetime facts, not confirmation that an external business effect or
Host resource has settled; the embedding executor must retain and settle those.

Call input is limited to 1 MiB before copying into V8, with the shared 128-call
and 32 MiB in-flight input budgets. Foreground and every worker result use one
JSON codec: stringify and check 1 MiB of UTF-8 before copying to Rust. Native
Bridge input is checked against 32 MiB before that copy, then against the selected
method's input budget. Getters and `toJSON` execute under the VM watchdog; cycles
and BigInt cannot cross the JSON boundary.

JSON numbers remain JavaScript Numbers through module calls and native Bridges.
Native Rust integers that cannot be represented exactly as a Number are rejected,
including inside arrays and objects. Values decoded from V8 retain the original
binary Number when its shortest decimal spelling resembles a different integer.
There is one codec for all calls, with no previous serializer fallback.
`Statistics::captured_callbacks` reports live registrations, not function count
or authorization.

This VM API is the execution foundation for the new application service adapter.
Host deployment activation, explicit service bindings, transport authorization and
durable operation recovery are separate integration work; VM capture alone does
not implement them. Component execution is described in [maka-ui](../ui/README.md).
