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

# Workspace plugins

[简体中文](README.zh-CN.md)

Workspace services can be composed independently of the Agent engine. Each leaf declaration selects a native provider, while UI consumers bind the contracts they need.

- [Files](workspace-files/native/README.md) uses the file [engine](workspace-files/engine/README.md) and pure [patch parser](workspace-files/patch/README.md).
- [Processes](workspace-processes/native/README.md) owns commands, terminals and protocol processes.
- [Web](workspace-web/native/README.md) provides fetch and search; its [API crate](workspace-web/api/README.md) contains the declarations.
- [Jev](workspace-jev/native/README.md) exposes the structured evaluation [engine](workspace-jev/engine/README.md).

The shared [API](api/README.md) defines workspace contracts. [Support](support/README.md) handles configured roots, directory selection and private-path exclusions. A service receives its resource limits and authority explicitly; a path in request JSON cannot widen them.
