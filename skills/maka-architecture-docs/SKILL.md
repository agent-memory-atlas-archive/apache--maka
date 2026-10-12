---
name: maka-architecture-docs
description: Write or revise Maka architecture guides and crate READMEs from the current code, with natural English and Chinese counterparts.
---
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

# Maka documentation

Write for the person trying to use or change the code. Read the relevant implementation and commands before explaining them. Existing prose is a lead to investigate, not evidence that a feature still exists.

Use the owning directory's README for component guidance. Keep the root architecture guide about the relationship between platform, plugins and products. Explain a mechanism once and link to it elsewhere. Preserve licensing, source provenance and release requirements when removing old material.

A crate README should say what the crate does, where to start, and which boundary matters to its callers. Add a command or example when it makes the crate usable. Small declaration crates may need only a paragraph and source links. Larger runtimes may need an explanation of lifecycle and failure behavior; the number of sections should follow the subject.

Use the current vocabulary precisely:

- A package declares entries and contracts; a composition selects instances and bindings.
- A Cargo crate is a compilation unit. It does not imply plugin activation.
- Committing desired configuration, activating it and completing cleanup are separate events.
- A binding locates a compatible capability; Host admission checks permission.
- Business completion and physical cleanup have separate outcomes. Lost replies retain their original operation identity.

Write direct, connected prose. Replace old paths and APIs instead of appending migration caveats. Keep proposals clearly separate from implemented behavior. Link to generated schemas for field-level detail rather than copying their contents.

Maintain English and Chinese counterparts for repository guides. Translate the meaning naturally while keeping identifiers, examples, limitations and commands aligned. Check local links and runnable examples; report only verification that was actually performed.
