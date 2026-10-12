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

# UI authoring

The Host `Authoring` service persists Agent requests for the connected foreground. Its UI `Worker` invokes the existing deployment manager in the foreground's owned Root and returns the original action outcome. Native grants expose `submit`/`result` to Agents and `watch`/`claim`/`complete` to the explicitly connected UI. Results are scoped to the originating Agent session.

Requests survive activation changes. Claims remove requests from the pending queue atomically; interrupted mutations are recovered from the worker's caller history and original receipt, never automatically replayed. Without a connected worker a request remains queued; a claimed request without a recoverable receipt remains unconfirmed. The queue allows sixteen pending requests. Results wait at most five seconds and preserve management conflicts, rejections and unknown outcomes.

The text authoring methods share the existing package loader and deployment commit. Product wiring lives in `products/maka/bundle`; the authoring workflow and footer example are in `skills/maka-plugin-authoring/references/runtime-ui.md`.
