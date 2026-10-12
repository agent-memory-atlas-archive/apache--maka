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

# maka-render-terminal

[简体中文](README.zh-CN.md)

Presents checked UI compositions in a terminal using Ratatui and Crossterm. `application` drives input, committed frames, native editors and teardown; `markdown` renders text content. The product supplies its composition and services. This renderer does not select Agent pages or embed JavaScript execution.

[Source](src/lib.rs) · [Cargo manifest](Cargo.toml)
