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

# maka-workspace-api

[简体中文](README.zh-CN.md)

Defines the shared contracts for workspace files, processes, terminals, protocol processes and Jev. Native providers and authoring tools use the same declarations. Request paths and workspace identifiers describe a selection; the provider still checks the actual bound directory and call authority.

The Terminal contract also exposes `readOutput` and `watchOutput` for ordered raw PTY bytes (Base64) and resize events. Each read returns one event from a ring of at most eight events, with up to 16 KiB of bytes per data event. Keep each `after` cursor with its original target. `Gap` ends the watch and reports the first retained event; resume explicitly from `firstAvailable - 1`. A retained suffix cannot restore complete terminal state. Raw output is ephemeral: a new attachment after exit or restart reports `Unavailable`, while screen/history reads remain available. `Closed` means output ended, not that process cleanup was confirmed. Output observation requires explicit method grants and does not require a controller.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
