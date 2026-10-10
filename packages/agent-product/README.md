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

# Plugin product compositions

[简体中文](README.zh-CN.md)

This is an ordinary plugin composition for a text agent, with separate desktop and terminal entries. The launchers contain no product pages. Workspace, composer, navigation, timeline and appearance are individually replaceable packages, connected through explicit public contracts. Native packages provide surfaces, the editor, bounded text capture, operation history and the Host agent service.

The initial product supports conversation creation, selection, canonical transcript pages, streaming replies, exact-run stop and recovery of original mutation receipts. It uses typed English, Simplified Chinese and Traditional Chinese catalogs. The desktop design uses scoped light/dark appearance, system typography, quiet navigation and a native multiline composer. Connection browsing, creation, public field editing and native API-key saving/removal are ordinary plugins. Default model setting and clearing are ordinary plugins with explicit confirmation and receipt recovery. Tool workflows, attachments, history virtualization and platform visual/IME/accessibility acceptance remain unfinished.

## Configuration Studio

An independent ordinary `maka.studio.workbench` package provides desktop and terminal
roots for configuring connections, default models and plugin composition. It uses
quiet top navigation and retains hidden page mounts and native drafts. It has no
conversation, composer or Agent binding; its separate Host source contains only
Settings, Receipts and a gateway with their explicit grants. Shared settings
components remain replaceable. Model selection is local to the workbench; saving a
default still uses the original confirmed Settings operation.

Select it at authoring time with `--product studio` (the default is `agent`). The
Studio installation configuration uses the same fields and keys below, except
`agent` is unnecessary. Both products use the same generic launch commands and
checked source replacement; neither requires a launcher product-name branch.
Installing a root package does not switch products. Changing a running product
requires a separately confirmed source update and original surface cleanup. A new
root does not inherit the previous product’s native drafts. Changing Host location
also requires explicit trusted connection configuration; source declarations do
not grant access to a different Host.

## Build and author

From the repository root:

```sh
just setup
npm --workspace @maka-agent/product run build
node packages/agent-product/scripts/compose.mjs \
  --configuration installation.json --output product-source \
  --maka /absolute/path/to/maka
```

The TUI Appearance page provides Maka, Dusk, Paper, Terminal and custom palettes, English/Simplified Chinese/Traditional Chinese, ASCII symbols and reduced motion. Automatic language follows the native process environment (`LC_ALL`, `LC_MESSAGES`, `LANG`, then the platform locale); an explicit saved language overrides it. Selecting Auto restores this behavior even when the composition configures a fixed language. Custom names, base palettes and semantic/syntax colors preview immediately. Save persists UI preferences through the original instance checkpoint; closing an unsaved edit restores the saved appearance without replacing conversation drafts. An optional absolute `themeFile` installation path enables Reload file and Save to file for a bounded 16 KiB UTF-8 custom-theme JSON file. Reload only changes the preview; Save to file exports the custom theme without applying preferences. The file path is shown in the editor. External file changes require a reload before export. Complete legacy TUI parity remains unfinished.

`installation.json` supplies `presentation` (`desktop` or `tui`), the full `locale` tag (or `auto`), `scheme` (`light` or `dark`), `hostRoot`, `hostDeployment`, `managementDeployment`, `agent` and `keys`. `agent` is the SDK's generated `AgentConfiguration`: explicit `workspaceRoots` (`[{ label, path }]`, absolute directories), finite `maxActiveRuns`/`maxSteps`, initial `permissions` (`{ "access": "read_only", "requestPermissions": true }` or `workspace_write`) and explicit `tools` (use `[]` for a text-only composition). Credentials and endpoint settings belong to that Host Root; they do not belong in the product source.

`keys` maps every name in `instanceNames` exported by `src/composition.ts` to a nonnil UUID. Allocate these once for an installation and keep them across rebuilds. `managementDeployment` is a stable Host UUID in the UI Root, distinct from the UI and business Host deployment. New package revisions do not imply new data namespaces.

The authoring command asks the actual Rust CLI to validate every package and compute its pinned reference. It writes and checks `host.json`, `management-host.json`, `ui.json`, `remote-exports.json`, `connections.json` and `packages.json`. It does not initialize a Root, commit a deployment or start execution. `connections.json` is trusted launcher configuration; inspect its explicit method and recovery grants before use.

For installation, initialize separate owned Host/UI Roots with `maka host init --root PATH`, then commit each source with `maka application commit`. Supply the selected deployment UUID, the current `--expected-revision`, and each built directory with `--package PATH`; UI also takes `--remote-exports product-source/remote-exports.json`. A first commit uses expected revision zero. Commit `management-host.json` into the **same UI Root**, using `managementDeployment`; it contains the explicitly selected UI manager, receipt service and gateway. Its connection is a cohost, so it borrows that launcher's original Store. The business Host remains a separate owned Root. Launch the committed UI with:

```sh
maka application run --root UI_ROOT --deployment UI_DEPLOYMENT \
  --connections product-source/connections.json
# For a desktop source:
maka desktop --root UI_ROOT --deployment UI_DEPLOYMENT \
  --connections product-source/connections.json
```

The trusted connections file may declare additional Hosts for later compositions.
Unselected declarations acquire no Root; a checked committed source starts a newly
selected Host using its original explicit configuration and grants.

Sending requires an authenticated, enabled connection and an explicitly selected model from the selected Host Root. The existing [trusted local connection](../../crates/application-bridge/README.md) owns Host startup and cleanup. Product code neither chooses a fallback model nor imports private execution objects.

The optional Questions service uses the existing canonical interaction records. To enable model questions, include `{ "result": "json", "confirmation": "preapproved", "name": "AskUserQuestion", "description": "Ask 1–3 bounded questions required to continue.", "import": "questions", "method": "ask" }` in `agent.tools`. Authoring explicitly selects the native provider and its trusted call grant; `tools: []` keeps a text-only composition. Pending questions appear in the conversation. The ordinary QuestionEditor supports choices, native text drafts and per-question skip; submission uses the original Workspace receipt/history recovery. Closing the sheet retains its draft. The optional Workspace Files provider activates when a selected tool uses `import: "files"`. Its methods are `readText`, `readImage`, `glob`, `grep`, `write`, `edit` and `applyPatch`; for example `{ "result": "json", "confirmation": "preapproved", "name": "Write", "description": "Write a file in the selected workspace.", "import": "files", "method": "write" }`. Authoring grants only the explicitly selected methods. Conversation Details saves the workspace policy and can revoke additional approvals. Read-only writes can request exact additional access when prompts are enabled; the ordinary approval dialog defaults to Deny/Once and requires explicit confirmation. Decisions and recovery use the original interaction records and receipts. Protected metadata and Host-private storage cannot be approved. Process tools use the optional `processes` import with methods `execute`, `read`, `writeStdin` and `stop`. Selecting any process tool requires explicit top-level `processes` installation configuration: `{ "filesystem": { "default": "read", "rules": [], "denyGlobs": [] }, "network": "denied", "helper": null }` is a read-only ceiling; explicitly grant writable paths in that ceiling when needed. Session policy and additional approvals can only narrow or use access within it. Windows requires the trusted Maka CLI `helper` path; Linux restricted-network execution also requires it. Background pipe/PTY tasks retain the original native resource owner and bounded output; input/resize requires the launch permission revision, while read/stop remain available after revocation. Closing the provider drains its processes, and uncertain cleanup remains an explicit failure. The full interactive terminal UI, attachments and client-capability approvals remain unfinished work.

The optional `skills` installation configuration enables Skills in the command palette:
`{ "sources": [{ "id": "local", "label": "Local skills", "path": "/absolute/skills" }] }`.
Set `bundled: true` (with `sources: []` if desired) to include the immutable skills and SDK resources shipped with the build. Built-in instructions are read-only; enabled/pinned preferences remain independently writable. Selected directories and installed copies shadow built-ins with the same skill ID. This flag adds no model tools or preparation grants. Each selected directory contains skill subdirectories with `SKILL.md`; sources are checked in order using the original parser and shadowing rules. The ordinary library supports catalog search, list paging and instruction excerpts, including invalid/shadowed diagnostics. A changed catalog requires refresh. Native inputs and reading areas own text, focus, selection and scrolling. Only the selected directories are readable, and Host-private storage remains excluded. The default UI gateway grants `list`/`read`; reading instructions grants no model or tool execution. Set `manage: true` to add `preview`/`apply` and original receipt recovery. Installation copies a selected skill directory into the current service instance’s private collection. Updates replace instructions and provenance while preserving installed resource files; deletion removes only the installed copy. Each change requires a preview and confirmation, and changed source or installed files invalidate the preview. Closing the library retains an unresolved operation and its original receipt. If the Host result is unknown, the independent `maka.skills.Outcomes.read` service can read the original publication result from the same Root under an explicit recovery grant. The recovery page uses persisted caller history after UI restart, even after the library instance and source directories have been removed. It never replays the change or upgrades the Host’s cleanup status. Recovery uses its independent gateway and reader, so a failed Library initializer does not block original receipt reads. Set `preferences: true` independently of `manage` to enable persistent Enable/Disable and Pin/Unpin controls. The gateway grants `setPreference` only when selected; edits use revision comparisons and keep unresolved original operations for receipt recovery. File browsing continues for disabled skills. Preferences are isolated by the original Root, deployment and instance and survive configuration replacement. Model tools are selected separately in `agent.tools`: use `import: "skills"`, `method: "search"` or `"load"`, `result: "json"`, a name (for example `SkillSearch` / `Skill`), description and confirmation policy. Composition binds the Library, selects `input: "skill"` and grants only those query methods. Search takes `{ query, limit? }` and returns eligible matches plus a revision; load takes `{ name, revision, resource?, instructionOffset? }`. Omit `instructionOffset` initially; `resource` is a relative `ReadInput`. Copy a returned `next` object unchanged to continue instructions or a resource. Each page fits the existing model archive budget, while `truncated` still describes the original skill body limit. A changed catalog, preference, resource or actual tool list returns `changed`, requiring a fresh search. The Agent supplies its actual tool names outside model arguments; declared skill tools grant no authority. Instructions and resources are bounded and expose truncation/continuations. Merely configuring collections adds no model tools. Set `agent.skillLibrary: "skills"` to separately enable input preparation and its native `prepare` grant. Each send or queue edit captures eligible metadata using the actual selected search/load tool names. Without a load tool, explicit `/skill:NAME` still works but no automatic loading prompt is added. Explicit references reuse the original enablement, compatibility and partial-failure rules. Missing/disabled/incompatible-only requests and oversized expanded input are rejected before admission, preserving the draft and attachments. Prepared text, original display text and skill receipts persist with the message; later queue execution uses that captured version. Editing prepares a new version, while a rejected edit preserves the existing queue entry. Skill content and tool declarations cannot grant permissions. Merely selecting collections or model tools does not enable preparation. Arbitrary-path imports and automatic source updates still need migration.

Typed forms use the optional `forms` import with method `ask` and `result: "json"` (for example, a tool named `AskUserForm`). Only an explicitly selected tool installs the native Forms provider and its call grant. The Host supplies requester identity and validates fields, types, formats and byte limits. Both presentations support text, numbers, integers, booleans and single/multiple choices, optional fields and defaults. Closing retains native drafts; Submit, Decline and Cancel record distinct canonical answers through the original response receipt. Unknown outcomes require checking the original receipt before another response. Client-capability authorization and legacy external form producers still need integration.


## State and verification

Each conversation retains its own native editor, selection, undo and IME within the original Composer mount. Up to 32 drafts stay mounted; at capacity only an observed empty, idle editor can be reclaimed. Otherwise the UI asks the user to send or clear an existing draft. JavaScript retains only observations, command receipts and workflow state. Sending captures an immutable resource at the original editor owner/revision; only a durable admission permits clearing that draft. Unknown results keep the original recovery reference and are queried, never resent automatically. Late captures and send receipts route to the original editor owner even after conversation navigation. New mounts read their persisted caller history and recover original receipts without clearing a new editor. Send actions also persist versioned caller-local correlation for the captured editor owner/revision, conversation and attachment IDs before dispatch; this metadata contains no body or secret and is omitted from peer confirmation. Native checkpoints restore text, selection and unsent attachment references after restart, reconciled against original send receipts as described below.

Turn footers use the original transcript start and terminal timestamps at the history fence. Only the latest window with the same observed live Turn advances; handoffs preserve the original start. Completed, failed and aborted durations remain fixed across reopening. The native Label renders wall time (including user waits), not tool or model CPU time. Missing or invalid boundaries are omitted; clock skew can affect live display.

History fixes its log fence and follows native transcript cursors. Active parts replay from their original start events; thinking is excluded from answer text. Canonical transcript invalidations refresh durable messages without restarting the watch. Display excerpts are bounded and labelled; they are not a substitute for complete history pagination.

```sh
npm --workspace @maka-agent/product run typecheck
npm --workspace @maka-agent/product test
# Adds actual immutable-package and Host/UI plan checks:
MAKA_PRODUCT_CLI_CHECK=1 npm --workspace @maka-agent/product test
# Requires the normal Node build dependencies and an actual native test binary:
cargo nextest run -p maka-agent-service --run-ignored only \
  -E 'test(packaged_product_creates)'
# Actual foreground install and automatic Agent -> Studio surface/backend switch:
cargo nextest run -p maka-cli --test integration --run-ignored only --locked \
  -E 'test(agent_installs_studio)'
```

The opt-in Rust integration uses the actual packaged guest, native TUI editor, persisted caller intents, authenticated native service and a local HTTP provider. It exercises creation, input, send, canonical response and reopening the same deployment. The separate CLI PTY test installs Studio through native file input and commits its
composition through the original manager. The production pipeline replaces the
surface and opens a previously idle, explicitly configured settings Host. It checks
the persisted source/package, final Root release and real terminal restoration.
Passing protocol tests does not establish the final desktop visual or three-platform input quality.

The ordinary `models` component reads bounded choices from the explicitly bound Root settings capability. Its default projection is independent of the choice page, so a default beyond the first sixteen choices can still be suggested. It offers the saved default only when no model is selected; choosing another model captures its observed connection revision for that send. Both presentations use native scrolling within each bounded catalog page. No host-selected model fallback or unknown-result resend is performed. Catalog refresh preserves the chosen connection/model identity and rechecks its actual revision; it never switches to an unrelated default.

`connection-form` creates connections from observed provider descriptors and edits only explicitly selected public fields. Unread model collections, overrides and body overlays remain unchanged unless the user chooses complete replacement. Endpoint editing preserves other configuration keys; complete configuration editing is a separate choice. Native editors retain all drafts across TUI field pages and confirmations. Saving captures every selected original owner/revision and validates the complete typed input against build-time compiled Rust schemas before issuing a mutation. Invalid JSON types and CAS rejection leave the form editable. A conflict requires explicit review and confirmation of the current connection/catalog version; an unknown result checks the original operation without retrying it. This component has Reader access for public fields and no SecretInput import. Edited fields are bounded to 64 KiB of UTF-8; JSON rejects nonfinite numbers and more than 64 nested containers before dispatch. Removing a saved connection uses the same form, its exact observed version and explicit confirmation of the saved identity. It removes associated saved keys and clears that connection’s default model while keeping conversations. Cancelling preserves all native drafts; conflicts require review and a new confirmation. Unknown removal checks its original receipt without issuing another deletion. Earlier removal receipts are independent of the current draft.

The ordinary `connections` component presents bounded catalog pages and opens a separate `credential` component. TUI connection and provider lists use native scrolling and arrow-key selection; Enter opens the selected editable connection or chooses a provider. Connection rows show the default model or enabled-model count, with credential and editing actions in the context menu. Within the settings content, Escape returns from the catalog or unselected provider chooser; editing and unresolved operations retain their existing leave confirmation. TUI connection and edit pages focus an available control on entry, with enabled fields taking priority; confirmation/history panes use native modal focus and return. Help adapts its shortcut rows to narrow terminals and accepts scrolling keys immediately on opening. Credential has no Reader import and exposes no capture to its parent. It sends the original native SecretInput resource directly to Settings, together with the observed connection and credential versions. Confirmed saving resets only that captured editor version; unknown results query the original reference, and conflicts keep the native draft. An explicit connection/key review lets a retained draft use newly observed versions only after confirmation. Removing a saved key also requires confirmation and both versions; it leaves the draft untouched.

Earlier operations have a separate bounded pane because caller history is shared by mounts of the same consumer instance and its references do not contain original connection inputs. Checking those facts never adopts a current target or clears a new draft. The settings route hides the conversation presentation while retaining its native composer owner.

`model-default` is a separate component bound explicitly by the model selector. It distinguishes an unset default from an unavailable saved target. Setting or clearing freezes the observed catalog revision and target for confirmation; a conflict requires refresh and a new confirmation. Unknown results query the original receipt without another write. Earlier receipts have a separate bounded pane and never adopt a current target. Saving refreshes the catalog without changing the model selected for the current conversation. Hiding the selector retains the same child slot and pending workflow. A selectable default describes catalog eligibility, not authenticated execution or network availability.

The desktop model selector uses the public anchored Popup scene.

Both model selectors retain one native search input and search the complete enabled catalog through bounded pages. TUI Up/Down enters the native scrolling list; arrows stage a model, with Apply or Enter confirming the model and thinking level against the original query revision. Caret movement acknowledges text without restarting search. Closing, defaults and busy scopes stop automatic scanning; defaults refresh reads its independent metadata even with an invalid search draft. TUI default confirmations and receipt history use nested native dialogs: Escape returns one level, and confirmation identity text scrolls above fixed Cancel/Confirm controls.

Opening its model/default/history workflows keeps the composer in place; Escape or an
outside click asks the plugin to close. The default child stays mounted across
close/reopen. Terminal entries keep paged in-flow presentation.

Desktop navigation can be collapsed without unmounting its component or the composer. Context and model controls use separate rows; long titles and model triggers are measured and truncated by the native renderer, with complete accessible values. Terminal navigation labels remain bounded by its own presentation.

Conversation navigation subscribes to the native Agent catalog. Background activity, metadata and pending interactions update the current directory page without replacing the selected conversation, history window or native drafts. Needs You shows the complete private pending-session count, independent of the 32-row page. Refresh retains the current page's first-session anchor and falls back when its tail disappears; Next/Previous use the latest snapshot revision. Successful explicit creation reveals the new conversation. A failed subscription retains visible content until explicit Refresh. The command palette keeps its separate revision-bound list query.


The `command-palette` package exports ordinary `CommandPalette` content for explicit compositions, with parent-controlled `open`, current command guards and `invoked`/`selected`/`closed` events. It imports TextInput, TextReader, ReadingArea and Agent. Search requires all whitespace-separated terms in a command's label/id/hint or a session's id/title; it scans the original revision-bound directory and keeps at most 32 matching sessions, with an explicit More window. Service and navigation choices capture the original native query revision. Optional command `copyText` uses the physical button default or TextInput's revision-checked `submitCopy`, so keyboard Enter copies once while the native query retains focus; scripted component events only notify the parent. The parent handles command ids and re-observes selected sessions through Agent; presentation and modal input isolation belong to its scene.

## Plugin controls

The ordinary Deployment component provides Desktop/TUI variants for fixed-source,
eight-instance pages and confirmed enable/disable intents. The management Host changes
the original input on CAS, preserving template parameters and unrelated overrides;
required connections must remain valid. Desired input is distinct from runtime readiness.
Conflicts require refresh and a new confirmation. Unknown results expose only original
operation checks and durable earlier intents, never automatic replay. The current screen
covers existing instances. Its separate advanced composition editor loads the complete
committed source into one native buffer through bounded UTF-8 chunks, waiting for each
original editor receipt. It edits configuration, explicit bindings, package selections
and the launch root without flattening template provenance. Saving freezes the native
owner/revision for confirmation and passes an immutable UTF-8 document resource to Host;
Host checks installed revisions, the original execution environment, the full plan and
source budget before CAS. Validation diagnostics retain the draft. Conflicts require
explicit discard/reload; unknown replacement checks its original receipt and is included
in durable caller history. Closing this page retains its native editor. Source documents
are limited to 1 MiB; the committed input and remote observations still share their
512 KiB limit. Full product/runtime acceptance remains required.

The TUI plugin list uses native ReadingArea/Item rows. Arrow keys select; Enter or the row menu opens enable/disable confirmation. Confirmation and operation history use native Dialog with scrolling content and fixed actions; Escape returns one level. Import and advanced editing request focus when revealed, and Escape cancels confirmation before returning to the list. Save/reload and import confirmations also use native Dialog, retaining the editor draft and file selection. Independent Otty checks at 80×24 and 40×16 cover offscreen navigation, keyboard/mouse cancellation and focus return; product and actual CLI checks pass. Native overlays erase crossing background wide glyphs before terminal diffing, keeping their border intact; closing restores the background.

The Plugins page installs `.maka-plugin` files through a retained native FilePicker.
One confirmation captures immutable selected bytes at the observed owner/revision
and submits the saved source revision to Host validation. Installing stores the
package without changing component enablement or permissions. Unknown results
check only their original operation; conflicts require refresh and a new confirmation.
The advanced composition draft remains intact; explicitly reload it to include newly
saved package references. Export an authored directory with
`maka application export-package --package DIRECTORY --output FILE.maka-plugin`;
an existing destination is never overwritten.

The native Agent captures published workspace roots without creating markers. New conversations reuse the selected conversation’s immutable workspace; the first conversation queries a published directory selection before creation. Sessions project canonical workspace path/identities and labels. `metadata` changes only title/labels with the original Session revision CAS; `create` and `metadata` report confirmed refusals as ordinary closed results, while uncertain persistence still uses the original operation journal. The command palette searches title, ID, workspace path and labels. “New conversation in another folder” opens an ordinary, replaceable DirectoryPicker in both presentations. It browses published roots and one bounded children page, filters that page with the original native Input/Reader, and returns an observed selection to the Root’s recorded creation path. Up/Down and Enter browse from the filter; pointer or Tab reaches Create here. The controlled Dialog retains the command palette, picker and ordinary SessionDetails component, preserving their native inputs when switching or cancelling. Details edits title and one label per line through original Input/Textarea/Reader captures and the frozen native Session revision. A conflict keeps the draft and requires explicit review before adopting a new basis; adopting a basis never submits the draft. Reload explicitly confirms discarding the fields. Closing retains drafts and original unknown-operation references. Successful metadata editing refreshes canonical Agent observations without replacing the conversation epoch, run, watch or Composer. Earlier metadata operations use bounded native caller history and never become the current draft or basis.

The ordinary `help` package provides keyboard help in both presentations. F1 opens it over the current page; Escape or Close returns to the original page and native draft. The command palette also offers help. Its only imported capability is ReadingArea; themes and translations remain product policy. A scrollable body and separate footer keep Close reachable in narrow terminals.

Each selected tool declares `confirmation: "required"` or `"preapproved"`. Required confirmation displays the captured service and the original arguments through the existing paged source reader. The default choice is Deny; Allow applies to this call only and still requires its original service grant and live binding. Closing the dialog keeps the request pending. Stop closes it without executing the service; an unknown response is recovered through its original receipt. This does not replace filesystem/process sandbox policy.

Each selected tool also declares `result: "json"` for a query/mutation or `result: "image"` for an image subscription. For workspace images, select `{ "result": "image", "confirmation": "preapproved", "name": "ReadImage", "description": "Read an image in the selected workspace.", "import": "files", "method": "readImage" }`. PNG, JPEG, GIF and WebP are limited to 5 MiB and 2000 pixels per dimension. Successful bytes become an immutable canonical image and model input when the selected model supports vision. Desktop uses the ordinary image viewer; TUI preserves the image action and shows the desktop preview notice. Reads retain the existing workspace policy: metadata is read-only, while Host-private storage and paths outside the selected directory are denied.


The ordinary composition assigns explicit Files v1 presentation keys to selected `files.readText`, `files.write`, `files.edit`, and `files.applyPatch` tools when no presentation is supplied. `presentation: null` retains generic fields; an explicit key is kept. The key is frozen in the original call rather than inferred from its model-facing name. Expanded cards read original file paths, captured content, requested replacements and patch text through fenced JSON selectors. Each long field has its own bounded previous/next excerpts. TUI merges complete small before/after evidence into a native line comparison and validates complete small create/update patches before showing diff gutters. Comparisons are bounded to 8 KiB and 256 combined lines; larger or partial fields retain independent pages. Malformed patches remain literal. These views reuse the original selection/find owner; Desktop currently uses literal source blocks. Write only shows original content when it was recorded; business failures remain visible and raw arguments/results/receipts remain accessible. Collapsed file cards show the requested path with a native Copy action and surface business failures. They read only path/status evidence and the recorded write/edit result path; contents and diagnostics load on expansion. One ToolDetails instance keeps its pages through disclosure, and current-content Find expands the exact owning card for a nested source hit. Expanded details retain the full path and Copy file path control. Clipboard writes require physical native activation and use bounded Host-resolved paths; completed writes/edits prefer their recorded result path. The TUI palette also follows the original native selection or first visible message for Copy file path; complete tool-card parity is still pending.

Tool cards distinguish recorded Host dispatch, returned results, failed execution, unknown outcomes and rejected calls. Returned does not imply business success. Provider/imported results retain their original error indication without claiming a Host outcome. Current waiting labels require the selected latest window and matching active turn; historical calls show recorded evidence. A terminal outcome takes precedence over a delayed pending-interaction snapshot. These labels never execute or retry a tool.

Session navigation displays the original persistent unread state. “Mark read through here” on a durable text message acknowledges only the current stored tail; opening or scrolling never marks read. The operation uses the same receipt recovery as other Agent mutations.

Workspace `glob` and `grep` tools also receive explicit Files v1 presentation keys. Their cards show the recorded search pattern and warn when `complete` is false. Expanding reads the original arguments and paged `files`/`matches` JSON under the same history fence; it never searches the current filesystem. Search strings and result paths do not create file links. Unknown, failed or unavailable evidence retains its original source/retry paths.

Attachments uses the original local FilePicker and Host binary resource capture. Open Attachments, choose a file (an absolute local path or Browse/Parent directory navigation in TUI), and attach it to the selected conversation. Each message accepts up to eight files of at most 50 MiB each; the native shared allocation budget may reject concurrent large selections. Sending supports an empty body with attachments. Draft references survive conversation navigation during this mount and are removed only after the original send receipt confirms admission. Unknown uploads and sends recover through their original receipt. Removing a draft reference does not delete stored material; archived conversations retain a visible attachment entry for this cleanup. TUI also accepts a whole paste of absolute paths, quoted names or local file URLs, up to the remaining eight-file capacity. Files upload serially to the original conversation; failed reads/uploads offer Retry, Skip file and Cancel. An uncertain upload must recover its original receipt before the batch can continue. Pasting prose remains ordinary text. Ctrl+V reads local clipboard files, images (up to 33,554,432 pixels, encoded as PNG), or plain text in that priority order. Text stays bound to its original insertion point and unchanged conversation draft. Clipboard-read failures require a new paste gesture. Images need no temporary file; confirmed uploads use the same durable attachment custody. Unsent attachment references restore from the original local native checkpoint after restart; upload receipts alone never recreate them. Queue edits retain their canonical attachments, including an empty body.

TUI attachments use a keyboard-navigable, two-line file/status list with a separate local-file browsing step. Returning preserves the original file capture; Remove applies to the selected draft reference. The composer summary keeps the first filename, remaining count and Ready status visible, clipping the filename with native terminal layout. File-paste recovery controls appear beside the composer.

Composer persists native text, validated selection and unsent attachment references
under the actual UI Root and stable native provider instance. Agent session rows
carry their original state domain, deployment and namespace. Restoration verifies
that source and reads original caller intents one at a time; only confirmed sends
for the stored editor lineage clear matching text or attachment IDs. Later editor
revisions remain. Unknown outcomes keep the draft held for an explicit result
check and never replay Send. The selected draft alone retains a receipt scan;
hidden drafts keep their full snapshot in their native owner. Returning to one
re-exposes its original metadata. Up to 32 native editors and 64 restored attachment
references remain bounded, and unrelated conversation navigation stays available.

Input is enabled after the new native owner has saved its initial lineage.
Graceful close flushes the latest captured edit and metadata before cleanup;
forced process loss can lose writes not yet acknowledged. Save, source, format,
quota or corruption errors preserve the previous record and expose Retry. There
is no silent overwrite, JavaScript text mirror, restored undo stack or automatic
send. Native clipboard contents are not read during restoration.

The TUI command palette's Copy file path follows the original native reading focus,
falling back to the first visible message when no selection exists. ToolDetails
publishes bounded Host-resolved display metadata; Timeline checks the original
session, fence, tool sequences and presentation before forwarding current context.
History landing and source changes clear stale context. Paths do not grant file
access or open a local path. The remaining TUI presentation and default-launcher
migration are still pending; Desktop product design follows the team branch.

In TUI Browse messages, an arrow at the loaded history boundary requests the
adjacent page at the original history fence. Focus stays on the current message;
the next arrow enters the added records. A page split follows its existing landing
edge. Escape returns to the preserved Composer draft.

The TUI palette supplies Copy message and Copy message source for the original
native selection, browsed header, or first visible message. It reads the complete
canonical field at the retained session/history fence, even when the timeline
shows only an excerpt. Assistant/thinking text uses the original native Markdown
formatter; user text remains literal. Tool cards explicitly name their canonical
result or arguments. Each copied text is bounded to 64 KiB UTF-8 and 128 KiB of
JSON-escaped content. Pending, streaming, empty, failed or oversized copy targets
stay disabled; valid source remains available if formatting fails. Closing the
palette drops the request, and old replies cannot supply a later command. Only
physical native activation writes the clipboard; background completion does not.

Saved ChatGPT subscription connections participate in native model selection and execution. Expiring credentials use the existing durable refresh owner, including single-flight exchange, retained replacements and shutdown settlement. The current connection editor exposes controls only for the API-key providers it can edit; interactive subscription sign-in is not yet part of that editor.

The TUI keeps common actions in the page and uses the command palette as an
alternative. New conversation, conversation rows and message/tool cards have local context menus,
opened by right click or their visible `⋯` control. Conversation details open in
place; the new-conversation menu selects another workspace. Complete text messages offer direct source copying, and a native selection
adds Copy selection. Partial messages retain the original source reader. The
menus share ordinary action guards and native modal focus custody.

Recovery uses its own ordinary ReadingArea instance and Host recovery consumer. Its dependency graph contains no Agent, Skills Library, or Settings service; business initialization failure leaves original receipt and skill-outcome reads available in the same Host.
