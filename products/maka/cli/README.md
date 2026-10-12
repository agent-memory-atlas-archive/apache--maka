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

# maka-cli

[简体中文](README.zh-CN.md)

Builds the `maka` executable. It starts the terminal product, serves Hosts, exposes ACP and provides application inspection, composition and installation commands. Product startup wires native factories and JavaScript backends into the platform owners.

From the repository root, run `just run` or `just run --help`. Development builds load `../bundle/dist`; `just run` rebuilds that bundle before Cargo starts. Release builds embed it. Desktop runs in a separate sibling executable.

[Source](src/main.rs) · [Cargo manifest](Cargo.toml)
