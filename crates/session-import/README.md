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

# Session import

[简体中文](README.zh-CN.md)

Converts Codex, Claude Code and OpenCode conversations into historical records through public plugin read capabilities. Callers supply the selected source identity and a pinned file or authorized database read view. This crate cannot open ambient paths, dispatch tools or write canonical events.

- Codex uses conversation events and tool response items, applies recorded rollbacks, and excludes provider message mirrors.
- Claude selects rewritten prompts without dropping parallel results or compaction roots. Three digest-checked passes resolve lineage, assemble response fragments and emit history.
- OpenCode reads the selected root Session, messages and parts in one SQLite snapshot, applying whole-message and partial-message reverts. Invalid cells, duplicate identities, orphan parts and missing completed-tool outcomes fail the import; pending calls remain pending.
- Tool calls and results retain their source order and correlation. Missing outcomes stay missing.
- Filesystem catalogs read bounded summaries (Codex: 512 KiB; Claude: 256 KiB head and tail), apply workspace/archive filters, and paginate by modification time and relative path. Query-bound cursors resume after the last delivered entry, including when the 48 KiB wire budget cuts a page short.
- OpenCode catalogs read root Sessions in bounded database batches, ordered by source time and identity. Workspace and text matching use the same normalization as filesystem catalogs; cursors bind the query and database. Pages observe a live source, not a retained snapshot.
- Codex catalogs select the highest `state_N.sqlite` generation and read the selected index. Rust normalizes timestamps and retains only the next page's candidates; rollout paths still require the public file capability. An unavailable initial database falls back to files, never an older database. Continuations retain their selected generation or filesystem source.
- Source workspace/model values are observations, not execution configuration. No credentials or provider options enter the result.
- Corrupt interior JSON, inconsistent source identity and changing multi-pass input fail the import. Only an unfinished final JSON write is omitted and identified in the fingerprint.

JSONL is read in chunks, without application limits on total bytes, line length, record count, or JSON structural count. The native Agent import reader reads the complete selected OpenCode snapshot without the remote database response's row/byte/work limits. Source parsing never truncates a conversation to make an import fit. Catalogs and previews remain paginated.

The plugin application product uses the Agent's `imports`, `importDirectories`, and `import` capabilities. Sources and execution workspaces have separate published roots. A preview captures source evidence; confirmation rereads that source and atomically publishes the complete normalized conversation into the original event log. The original operation journal owns recovery. No imported model, source working directory, or tool observation authorizes execution.

Stored history and model request capacity are separate. Execution reads the complete effective history and fits summary requests to the selected model. Large histories trigger foreground compaction before the first main request; a summary context-overflow rejection retries with smaller input. Original history remains available, and accepted checkpoints survive restart. See [the application design](../../.agents/plan/runtime-host-rust/plugin-application-runtime/conversation-import.md).

The following describes the older plugin route, pending retirement with the legacy Host/TUI. Its append batches are transport units, not a total history quota.

Source configuration uses revision-checked plugin storage. A prepared import atomically saves its explicit destination and normalized payload; removing or changing a source does not change that intent. Delivery reauthorizes the saved destination, queries the Host receipt, appends only the missing suffix, then publishes. Terminal receipts and payload reclamation commit together. Retrying the same operation never creates another copy.

The built-in plugin publishes a native terminal view and a `maka.session-import/manage` Remote endpoint. Both use the same typed handler for sources, catalogs, model choices, preparation, delivery, abandonment and paged copy history. All capabilities are public plugin APIs. Canonical publication and receipts remain Host-owned.

The Client requires an explicit destination Host workspace and execution configuration. Failed replies and refreshes retain the operation identity; only a confirmed receipt or explicitly setting the attempt aside clears it. Saved imports remain available after reopening the page or restarting the Host.

Run Client recovery acceptance with `node --test crates/session-import/tests/client.test.mjs` from the repository root.
