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

# State, authority and settlement

A package revision identifies immutable bytes. An instance UUID identifies a state
owner. Configuration revision, activation epoch and individual invocation identify
different changes; none grants permission by itself. A replacement must not borrow
an old activation's capabilities or silently redirect its calls.

Host entries and UI entries run in separate owners. A Component frame contains only
bounded JSON state and a declarative scene. Durable domain state belongs to its Host
service, not to a second UI cache. Native checkpoint components can persist small UI
preferences; use domain contracts for business data.

Use only declared imports and resources supplied by the actual request. Host grants
identify caller/origin, consumer, provider, export and methods. Recovery has its own
grant and exact persisted owner. A successful package check or compatible remote
observation proves shape, not current authorization or availability.

Keep an original mutation's `OperationReply.reference`. Only `receipt.kind ===
"recorded"` describes a journal outcome; inspect both `outcome` and `cleanup`.
An unknown result is not a failure or permission to issue a fresh mutation. Query
`recover` with the exact reference, including after the current import disappears.
`not_recorded` and `unavailable` are different observations and do not trigger replay.

Cancellation requests work to stop. Observer loss, a returned handler or a logically
closed panel does not prove that streams, blocking I/O or processes have settled.
Native work must retain the actual Call resource ticket through completion, and
failed cleanup must remain visible to the original owner. See
[Native Rust](native-rust.md) for the native lifecycle boundary.

JavaScript application entries obtain effects through explicit services. They have
no ambient Node filesystem, network, credentials or process APIs. If a needed durable
capability is absent, add the smallest service that owns it and specify its contract,
permissions, admission and recovery; do not invent a private global accessor.
