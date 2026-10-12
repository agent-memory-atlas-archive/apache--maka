/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

import type {
  Action,
  ActionOutcome,
  AgentDirectory,
  AgentDirectoryPage,
  AgentImportChange,
  AgentImportEntry,
  AgentImportLocation,
  AgentImportPreview,
  AgentImportResult,
  AgentImportSource,
  AgentImportSourceRef,
  AgentImportsPage,
  AgentSession,
  CallerIntent,
  Component,
  ComponentInput,
  EditorCommand,
  EditorObservation,
  Emission,
  InputProps,
  OperationReply,
  ReadingCommand,
  RecoveryRef,
  TextChunk,
} from '../../../packages/plugin-sdk/src/index.js';
import { importsContract, type ImportsProps } from '../../shared/contracts.js';
import { pageHeader, withPageShell, type PageState } from '../../shared/page-shell.js';
import { copy, type Copy } from '../../shared/locale.js';
import { sameValue } from '../../shared/value.js';
import { compactLabel, fill, output, type Node, type Presentation } from '../../shared/view.js';

type FieldName = 'name' | 'database' | 'search' | 'title';
type Format = AgentImportLocation['kind'];
type Route = 'sources' | 'form' | 'catalog' | 'preview' | 'copies';
type Request =
  | { kind: 'sources' | 'catalog' | 'preview' | 'copies' | 'mutate' | 'receipt' }
  | { kind: 'directory'; purpose: 'source' | 'destination' }
  | { kind: 'field'; field: FieldName; observation: EditorObservation }
  | { kind: 'history'; after: number };
interface Field {
  observation: EditorObservation | null;
  command: EditorCommand | null;
  reset: string | null;
  ready: boolean;
  sequence: number;
}
interface State extends PageState {
  revision: number;
  serial: number;
  open: boolean;
  route: Route;
  focus: ReadingCommand | null;
  sources: AgentImportSource[];
  sourcesRevision: number;
  source: AgentImportSourceRef | null;
  entries: AgentImportEntry[];
  catalogNext: string | null;
  query: string;
  archived: boolean;
  preview: AgentImportPreview | null;
  destination: AgentDirectory | null;
  copies: AgentSession[];
  copiesNext: string | null;
  latest: AgentSession | null;
  selected: string | null;
  form: { id: string | null; format: Format; directory: AgentDirectory | null } | null;
  picker: 'source' | 'destination' | null;
  fields: Record<FieldName, Field>;
  capturedName: string | null;
  pending: { key: string; request: Request } | null;
  restoring: boolean;
  historyFailed: boolean;
  original: RecoveryRef | null;
  current: { input: AgentImportChange; reference: RecoveryRef | null } | null;
  confirm: AgentImportChange | null;
  notice: keyof Copy | null;
}
const field = (): Field => ({
  observation: null,
  command: null,
  reset: null,
  ready: true,
  sequence: 0,
});
const formatName = (format: Format) =>
  format === 'codex' ? 'Codex' : format === 'claude_code' ? 'Claude Code' : 'OpenCode';
function initialize(field: Field) {
  if (field.reset !== null && field.observation && !field.command)
    field.command = {
      owner: field.observation.snapshot.text.owner,
      basedOn: field.observation.revision,
      sequence: ++field.sequence,
      edit: { kind: 'reset', text: field.reset },
    };
}
function reset(state: State, name: FieldName, value: string) {
  const field = state.fields[name];
  field.reset = value;
  field.ready = false;
  initialize(field);
}
function request(
  state: State,
  actions: Action[],
  r: Request,
  binding: string,
  method: string,
  input: unknown,
  resources: Action['resources'] = {},
) {
  if (state.pending) return;
  const key = `imports-${++state.serial}`;
  state.pending = { key, request: r };
  actions.push({ key, binding, member: 0, method, input, resources });
}
function read(
  state: State,
  actions: Action[],
  r: Extract<
    Request,
    { kind: 'sources' | 'catalog' | 'preview' | 'copies' | 'mutate' | 'receipt' }
  >,
  input: unknown,
) {
  request(state, actions, r, 'agent', 'imports', input);
}
function sources(state: State, actions: Action[]) {
  read(state, actions, { kind: 'sources' }, { kind: 'sources' });
}
function catalog(state: State, actions: Action[], cursor: string | null = null) {
  if (state.source)
    read(
      state,
      actions,
      { kind: 'catalog' },
      {
        kind: 'catalog',
        source: state.source,
        query: { text: state.query, includeArchived: state.archived, cursor },
      },
    );
}
function copies(state: State, actions: Action[], after: string | null = null) {
  read(state, actions, { kind: 'copies' }, { kind: 'copies', after });
}
function history(state: State, actions: Action[], after = 0) {
  state.restoring = true;
  state.historyFailed = false;
  if (after === 0) state.original = null;
  request(state, actions, { kind: 'history', after }, 'history', 'page', { after, limit: 16 });
}
function locked(state: State, props: ImportsProps) {
  return !props.open || props.busy || !!state.pending || !!state.current || state.restoring;
}
function capture(state: State, actions: Action[], name: FieldName) {
  const field = state.fields[name],
    observation = field.observation;
  if (!observation || observation.snapshot.marked || field.command || !field.ready) return;
  if (observation.snapshot.text.bytes > 4096) {
    state.notice = 'importsInvalid';
    return;
  }
  request(
    state,
    actions,
    { kind: 'field', field: name, observation },
    'reader',
    'read',
    { start: 0, end: observation.snapshot.text.bytes },
    {
      text: {
        child: `import-${name}`,
        method: 'snapshot',
        input: {
          kind: 'at_revision',
          owner: observation.snapshot.text.owner,
          revision: observation.snapshot.text.revision,
        },
      },
    },
  );
}
function mutate(state: State, actions: Action[], input: AgentImportChange) {
  state.current = { input, reference: null };
  state.notice = null;
  request(state, actions, { kind: 'mutate' }, 'agent', 'import', input);
  actions.at(-1)!.recovery = { kind: 'conversation-import/v1', input };
}
function notice(kind: string): keyof Copy {
  return kind === 'changed'
    ? 'importsChanged'
    : kind === 'invalid'
      ? 'importsInvalid'
      : kind === 'limit'
        ? 'importsLimit'
        : kind === 'busy'
          ? 'importsBusy'
          : 'importsUnavailable';
}
function settle(state: State, reply: OperationReply, actions: Action[], events: Emission[]) {
  if (
    !state.current ||
    (state.current.reference && !sameValue(reply.reference, state.current.reference))
  )
    return;
  state.current.reference = reply.reference;
  if (reply.receipt.kind !== 'recorded' || reply.receipt.outcome.kind !== 'returned') {
    state.notice = 'importsUnknown';
    return;
  }
  const result = reply.receipt.outcome.value as AgentImportResult;
  const input = state.current.input;
  state.current = null;
  state.confirm = null;
  if (result.kind === 'sources') {
    state.sources = result.sources;
    state.sourcesRevision = result.revision;
    state.form = null;
    state.route = 'sources';
    state.notice = input.kind === 'remove_source' ? 'importsRemoved' : 'importsSaved';
    state.selected = state.sources[0] ? 'source-' + state.sources[0].id : null;
    sources(state, actions);
  } else if (result.kind === 'imported') {
    state.latest = result.session;
    state.notice = 'importsDone';
    state.route = 'copies';
    events.push({ name: 'changed', value: {} });
    copies(state, actions);
  } else state.notice = notice(result.kind);
}
function startForm(state: State, source: AgentImportSource | null) {
  state.form = {
    id: source?.id ?? null,
    format: source?.location.kind ?? 'codex',
    directory: source
      ? { name: source.name, path: source.path, selection: source.location.directory }
      : null,
  };
  state.route = 'form';
  state.notice = null;
  reset(state, 'name', source?.name ?? '');
  reset(
    state,
    'database',
    source?.location.kind === 'open_code' ? source.location.database : 'opencode.db',
  );
}
function saveSource(state: State, actions: Action[], database: string | null) {
  if (!state.form?.directory || state.capturedName === null) return;
  const { format, directory, id } = state.form;
  const name = state.capturedName.trim() || formatName(format);
  if ([...name].length > 128 || /[\u0000-\u001f\u007f-\u009f]/u.test(name)) {
    state.notice = 'importsInvalid';
    return;
  }
  const location: AgentImportLocation =
    format === 'open_code'
      ? { kind: format, directory: directory.selection, database: database?.trim() ?? '' }
      : { kind: format, directory: directory.selection };
  mutate(state, actions, {
    kind: 'save_source',
    revision: state.sourcesRevision,
    id,
    name,
    location,
  });
}
export function imports<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ImportsProps, N> {
  function focus(state: State, id: string | null) {
    state.selected = id;
    if (presentation.target === 'tui' && id)
      state.focus = {
        sequence: (state.focus?.sequence ?? 0) + 1,
        target: { kind: 'focus_item', key: id },
      };
  }
  function frame(
    state: State,
    props: ImportsProps,
    actions: Action[] = [],
    events: Emission[] = [],
  ) {
    const ui = presentation.paint(props.theme),
      text = copy(props.locale),
      disabled = locked(state, props);
    const editing = disabled || !!state.confirm || !!state.picker;
    const fieldView = (name: FieldName, label: string, visible: boolean) =>
      ui.box(
        `field-${name}`,
        [
          ui.text(`label-${name}`, label, 13, props.theme.muted),
          presentation.slot(
            `import-${name}`,
            'input',
            {
              appearance: props.theme.input,
              accessibilityLabel: label,
              placeholder: label,
              autoFocus: visible,
              tabIndex: name === 'database' ? 2 : 1,
              readOnly: false,
              disabled: editing || !!state.fields[name].command || !state.fields[name].ready,
              command: state.fields[name].command,
            } satisfies InputProps,
            { changed: `field-${name}`, submitted: `submit-${name}` },
          ),
        ],
        { visible, gap: 0 },
      );
    const row = (id: string, title: string, detail: string, menu: N[] = []) => {
      const body = ui.box(
        'body-' + id,
        [
          ui.text('title-' + id, title, 15, props.theme.text, 500, { truncate: true }),
          ui.text('detail-' + id, detail, 13, props.theme.muted, 400, { truncate: true }),
        ],
        { gap: 0 },
      );
      if (ui.item) {
        const item = ui.item(id, body, state.selected === id, disabled, {
          followFocus: true,
          submit: true,
        });
        return ui.contextMenu && menu.length
          ? ui.contextMenu('menu-' + id, text.projectsActions, item, menu)
          : item;
      }
      return ui.box('row-' + id, [body, ui.button(id, text.importsOpen, disabled), ...menu], {
        surface: true,
        padding: 8,
      });
    };
    const sourceActions = (source: AgentImportSource, prefix = '') => [
      ui.button(prefix + 'browse-' + source.id, text.importsOpen, disabled),
      ui.button(prefix + 'edit-' + source.id, text.importsEdit, disabled),
      ui.button(prefix + 'remove-' + source.id, text.importsRemove, disabled),
    ];
    const rows =
      state.route === 'sources'
        ? state.sources.map((source) =>
            row(
              'source-' + source.id,
              source.name,
              formatName(source.location.kind) + ' · ' + source.path,
              sourceActions(source),
            ),
          )
        : state.route === 'catalog'
          ? state.entries.map((entry, index) =>
              row('entry-' + index, entry.title, entry.cwd ?? entry.path),
            )
          : state.route === 'copies'
            ? state.copies.map((session) =>
                row('copy-' + session.id, session.title, session.workspace.path),
              )
            : state.route === 'preview'
              ? (state.preview?.items ?? []).map((item) =>
                  ui.box(
                    'preview-' + item.index,
                    [
                      ui.text(
                        'role-' + item.index,
                        text[
                          (
                            {
                              user: 'importsUser',
                              assistant: 'importsAssistant',
                              tool_call: 'importsToolCall',
                              tool_result: 'importsToolResult',
                              note: 'importsNote',
                            } as const
                          )[item.kind]
                        ],
                        13,
                        props.theme.muted,
                        500,
                      ),
                      ui.text('text-' + item.index, item.text, 14, props.theme.text),
                      ...(item.truncated
                        ? [
                            ui.text(
                              'truncated-' + item.index,
                              text.importsTruncated,
                              12,
                              props.theme.muted,
                            ),
                          ]
                        : []),
                    ],
                    { gap: 1 },
                  ),
                )
              : [];
    const chosenSource = state.sources.find((source) => state.selected === 'source-' + source.id);
    const title =
      state.route === 'form'
        ? state.form?.id
          ? text.importsEdit
          : text.importsAdd
        : state.route === 'preview'
          ? text.importsPreview
          : state.route === 'copies'
            ? text.importsHistory
            : text.importsTitle;
    const form = ui.box(
      'source-form',
      [
        ui.text('source-help', text.importsSourceHelp, 14, props.theme.muted),
        ui.box(
          'source-formats',
          (['codex', 'claude_code', 'open_code'] as const).map((format) =>
            ui.button(
              'format-' + format,
              formatName(format),
              editing,
              state.form?.format === format,
            ),
          ),
          { row: true, terminal: { wrap: true }, style: { wrap: true } },
        ),
        fieldView('name', text.importsName, state.route === 'form'),
        fieldView(
          'database',
          text.importsDatabase,
          state.route === 'form' && state.form?.format === 'open_code',
        ),
        ...(state.form?.directory
          ? [ui.text('source-path', state.form.directory.path, 13, props.theme.muted)]
          : []),
        ui.button('source-directory', text.importsChooseSource, editing),
      ],
      {
        visible: state.route === 'form',
        gap: 1,
        scroll: 'manual',
        terminal: { grow: 1, basis: 0 },
        style: { width: fill, height: fill, minHeight: 0 },
      },
    );
    const contents = ui.box(
      'imports-content',
      [
        ...(state.route === 'sources' && !rows.length && !state.pending
          ? [ui.text('empty-sources', text.importsEmpty, 14)]
          : []),
        ...(state.route === 'catalog' && !rows.length && !state.pending
          ? [ui.text('empty-catalog', text.importsCatalogEmpty, 14)]
          : []),
        ...(state.route === 'copies' && !rows.length && !state.pending
          ? [ui.text('empty-copies', text.importsHistoryEmpty, 14)]
          : []),
        ...rows,
      ],
      {
        scroll: 'manual',
        focusGroup:
          state.route === 'sources' || state.route === 'catalog' || state.route === 'copies',
        gap: 1,
        terminal: { grow: 1, basis: 0 },
        style: { width: fill, height: fill, minHeight: 0 },
      },
    );
    const body = ui.box(
      'imports-page',
      [
        ...pageHeader(
          presentation,
          state,
          props,
          events,
          title,
          'back',
          !!state.pending && !state.current,
        ),
        ...(state.restoring || state.pending
          ? [
              ui.text(
                'imports-loading',
                state.restoring ? text.projectsChecking : text.connecting,
                13,
                props.theme.muted,
              ),
            ]
          : []),
        ...(state.notice
          ? [ui.text('imports-notice', text[state.notice], 13, props.theme.muted)]
          : []),
        ...(state.historyFailed
          ? [ui.button('history-retry', text.sourceRetry, !!state.pending)]
          : []),
        ...(state.current
          ? [
              ui.button(
                'receipt',
                text.executionReceipt,
                !!state.pending || !state.current.reference,
              ),
            ]
          : []),
        ...(state.latest && state.notice === 'importsDone'
          ? [ui.button('open-latest', text.importsOpen, disabled)]
          : []),
        ui.box(
          'catalog-search',
          [
            fieldView('search', text.importsSearch, state.route === 'catalog'),
            ui.box(
              'catalog-search-actions',
              [
                ui.button('search', text.importsSearch, editing),
                ui.button('archived', text.importsArchived, editing, state.archived),
              ],
              { row: true, terminal: { wrap: true }, style: { wrap: true } },
            ),
          ],
          { visible: state.route === 'catalog', gap: 0 },
        ),
        ui.box(
          'preview-summary',
          [
            fieldView('title', text.importsConversationName, state.route === 'preview'),
            ...(state.preview
              ? [
                  ui.text(
                    'preview-count',
                    `${text.importsRecords}: ${state.preview.records} · ${state.preview.bytes} B`,
                    13,
                    props.theme.muted,
                  ),
                ]
              : []),
            ...(state.preview?.incompleteTail
              ? [ui.text('preview-tail', text.importsIncomplete, 13, props.theme.warning)]
              : []),
            ...(state.destination
              ? [ui.text('destination-path', state.destination.path, 13, props.theme.muted)]
              : []),
          ],
          { visible: state.route === 'preview', gap: 0 },
        ),
        form,
        ui.box(
          'import-list-container',
          [
            presentation.reading(
              'import-list',
              { command: state.focus, content: contents },
              { input: 'list-input' },
            ),
          ],
          {
            visible: state.route !== 'form',
            terminal: { grow: 1, basis: 0 },
            style: { width: fill, height: fill, minHeight: 0 },
          },
        ),
        ...(state.route === 'sources' && chosenSource
          ? [
              ui.box('selected-source-actions', sourceActions(chosenSource, 'footer-'), {
                row: true,
                terminal: { wrap: true },
                style: { wrap: true },
              }),
            ]
          : []),
        ui.box(
          'imports-actions',
          state.route === 'sources'
            ? [
                ui.button('add', text.importsAdd, disabled),
                ui.button('refresh', text.directoryRefresh, disabled),
                ui.button('copies', text.importsHistory, disabled),
              ]
            : state.route === 'form'
              ? [
                  ui.button(
                    'save-source',
                    text.settingsSave,
                    editing ||
                      !state.form?.directory ||
                      !state.fields.name.ready ||
                      !!state.fields.name.command ||
                      (state.form?.format === 'open_code' &&
                        (!state.fields.database.ready || !!state.fields.database.command)),
                  ),
                ]
              : state.route === 'catalog'
                ? [
                    ui.button('refresh-catalog', text.directoryRefresh, disabled),
                    ...(state.catalogNext
                      ? [ui.button('catalog-more', text.projectsMore, disabled)]
                      : []),
                  ]
                : state.route === 'preview'
                  ? [
                      ui.button('destination-directory', text.importsChooseDestination, editing),
                      ui.button(
                        'review-copy',
                        text.importsReview,
                        editing ||
                          !state.destination ||
                          !state.preview ||
                          !state.fields.title.ready ||
                          !!state.fields.title.command,
                      ),
                      ...(state.preview?.next !== null && state.preview?.next !== undefined
                        ? [ui.button('preview-more', text.projectsMore, disabled)]
                        : []),
                      ui.button('preview-first', text.directoryRefresh, disabled),
                    ]
                  : [
                      ui.button('refresh-copies', text.directoryRefresh, disabled),
                      ...(state.copiesNext
                        ? [ui.button('copies-more', text.projectsMore, disabled)]
                        : []),
                    ],
          { row: true, terminal: { wrap: true }, style: { wrap: true }, gap: 1 },
        ),
      ],
      {
        visible: props.open,
        autoFocus: true,
        shortcuts: [{ key: 'escape', modifiers: [], input: 'back', enabled: true }],
        gap: 1,
        terminal: { grow: 1 },
        style: { width: fill, height: fill },
      },
    );
    let view = body;
    for (const purpose of ['source', 'destination'] as const) {
      const picker = presentation.slot(
        `import-${purpose}-picker`,
        'directories',
        {
          theme: props.theme,
          locale: props.locale,
          purpose: purpose === 'source' ? 'import' : 'workspace',
          chooseLabel: text.projectsUseDirectory,
          open: props.open && state.picker === purpose,
          busy: disabled,
        },
        { selected: purpose + '-selected', closed: purpose + '-closed' },
      );
      view = ui.dialog(
        'import-' + purpose + '-dialog',
        purpose === 'source' ? text.importsChooseSource : text.importsDestination,
        view,
        picker,
        state.picker === purpose,
        {},
        { width: 76, height: 24 },
      );
    }
    const confirmation = ui.box(
      'import-confirmation',
      [
        ...(state.confirm?.kind === 'copy'
          ? [
              ui.text('confirm-title', state.confirm.title, 15, props.theme.text, 600),
              ui.text('confirm-directory', compactLabel(state.destination?.path ?? '', 240), 14),
              ui.text(
                'confirm-records',
                `${text.importsRecords}: ${state.preview?.records ?? ''}`,
                13,
              ),
              ui.text('confirm-help', text.importsHelp, 13, props.theme.muted),
            ]
          : [ui.text('confirm-remove', text.importsRemoveHelp, 14)]),
        ...(state.notice
          ? [ui.text('confirm-notice', text[state.notice], 13, props.theme.muted)]
          : []),
        ...(state.current
          ? [
              ui.button(
                'confirm-receipt',
                text.executionReceipt,
                !!state.pending || !state.current.reference,
              ),
            ]
          : []),
        ui.box(
          'confirm-actions',
          [
            ui.button('confirm-cancel', text.cancel, !!state.current || !!state.pending),
            ui.button(
              'confirm',
              state.confirm?.kind === 'copy' ? text.importsConfirm : text.importsRemove,
              disabled,
              true,
            ),
          ],
          { row: true },
        ),
      ],
      {
        gap: 1,
        shortcuts: [
          {
            key: 'escape',
            modifiers: [],
            input: 'confirm-cancel',
            enabled: !state.current && !state.pending,
          },
        ],
      },
    );
    view = ui.dialog(
      'import-confirm-dialog',
      state.confirm?.kind === 'copy' ? text.importsReview : text.confirmDefault,
      view,
      confirmation,
      !!state.confirm,
      {},
      { width: 68 },
    );
    const result = output(
      state,
      ui.box('imports-root', [view], {
        visible: props.open,
        terminal: { grow: 1 },
        style: { width: fill, height: fill },
      }),
      events,
    );
    result.actions = actions;
    return result;
  }
  return withPageShell<State, ImportsProps, N>(
    {
      mount(props) {
        const state: State = {
          revision: 0,
          serial: 0,
          open: props.open,
          route: 'sources',
          focus: null,
          sources: [],
          sourcesRevision: 0,
          source: null,
          entries: [],
          catalogNext: null,
          query: '',
          archived: false,
          preview: null,
          destination: null,
          copies: [],
          copiesNext: null,
          latest: null,
          selected: null,
          form: null,
          picker: null,
          fields: { name: field(), database: field(), search: field(), title: field() },
          capturedName: null,
          pending: null,
          restoring: true,
          historyFailed: false,
          original: null,
          current: null,
          confirm: null,
          notice: null,
        };
        const actions: Action[] = [];
        history(state, actions);
        return frame(state, props, actions);
      },
      update(state, props) {
        state.revision++;
        const actions: Action[] = [];
        if (
          !state.open &&
          props.open &&
          state.route === 'sources' &&
          !state.pending &&
          !state.current &&
          !state.restoring
        )
          sources(state, actions);
        state.open = props.open;
        return frame(state, props, actions);
      },
      event(state, input, props) {
        state.revision++;
        const actions: Action[] = [],
          events: Emission[] = [];
        if (input.kind === 'component' && input.child === 'import-list' && input.name === 'input')
          input = input.event as ComponentInput;
        if (input.kind === 'component') {
          const name = (['name', 'database', 'search', 'title'] as const).find(
            (name) => input.kind === 'component' && input.child === 'import-' + name,
          );
          if (name && input.name === 'changed') {
            const field = state.fields[name];
            field.observation = input.event as EditorObservation;
            const receipt = field.observation.receipt;
            if (
              field.command &&
              receipt?.owner === field.command.owner &&
              receipt.sequence === field.command.sequence
            ) {
              field.ready = receipt.outcome.status === 'applied';
              field.command = null;
              field.reset = null;
              if (!field.ready) state.notice = 'importsUnavailable';
            } else initialize(field);
          } else if (name && input.name === 'submitted')
            input = {
              kind: 'raw',
              key: name === 'search' ? 'search' : name === 'title' ? 'review-copy' : 'save-source',
              event: {},
            } as ComponentInput;
        }
        if (
          input.kind === 'component' &&
          state.picker &&
          input.child === `import-${state.picker}-picker` &&
          props.open
        ) {
          if (input.name === 'closed') state.picker = null;
          else if (input.name === 'selected' && !locked(state, props)) {
            const purpose = state.picker;
            state.picker = null;
            request(
              state,
              actions,
              { kind: 'directory', purpose },
              'agent',
              purpose === 'source' ? 'importDirectories' : 'directories',
              { kind: 'children', selection: input.event, after: null },
            );
          }
        } else if (input.kind === 'action' && input.key === state.pending?.key) {
          const r = state.pending.request,
            result = input.event as ActionOutcome;
          state.pending = null;
          if (r.kind === 'history') {
            if (result.status !== 'succeeded') {
              state.historyFailed = true;
              state.notice = 'historyUnavailable';
            } else {
              const items = result.value as CallerIntent[];
              for (const item of items) {
                const recovery = item.recovery as {
                  kind?: string;
                  input?: AgentImportChange;
                } | null;
                if (
                  item.reference.method === 'import' &&
                  item.reference.contract.name === 'maka.agent.Agent' &&
                  recovery?.kind === 'conversation-import/v1' &&
                  recovery.input
                ) {
                  state.original = item.reference;
                  state.current = { input: recovery.input, reference: item.reference };
                }
              }
              const last = items.at(-1);
              if (last && last.cursor > r.after) history(state, actions, last.cursor);
              else if (last) {
                state.historyFailed = true;
                state.notice = 'historyUnavailable';
              } else {
                state.restoring = false;
                if (state.original)
                  request(state, actions, { kind: 'receipt' }, 'receipts', 'read', state.original);
                else sources(state, actions);
              }
            }
          } else if (r.kind === 'mutate') {
            if (result.status === 'operation') settle(state, result.reply, actions, events);
            else if (result.status === 'not_accepted') {
              state.current = null;
              state.notice = 'importsUnavailable';
            } else state.notice = 'importsUnknown';
          } else if (r.kind === 'receipt') {
            if (result.status === 'succeeded')
              settle(state, result.value as OperationReply, actions, events);
            else state.notice = 'importsUnknown';
          } else if (r.kind === 'field') {
            const chunk = result.status === 'succeeded' ? (result.value as TextChunk) : null;
            if (
              !chunk ||
              !sameValue(chunk.metadata, r.observation.snapshot.text) ||
              new TextEncoder().encode(chunk.text).length !== r.observation.snapshot.text.bytes
            )
              state.notice = 'importsUnavailable';
            else if (r.field === 'name' && state.form) {
              state.capturedName = chunk.text;
              if (state.form.format === 'open_code') capture(state, actions, 'database');
              else saveSource(state, actions, null);
            } else if (r.field === 'database' && state.form) saveSource(state, actions, chunk.text);
            else if (r.field === 'search' && state.route === 'catalog') {
              if ([...chunk.text].length > 1024 || /[\u0000-\u001f\u007f-\u009f]/u.test(chunk.text))
                state.notice = 'importsInvalid';
              else {
                state.query = chunk.text;
                catalog(state, actions);
              }
            } else if (r.field === 'title' && state.preview && state.destination) {
              const title = chunk.text.trim();
              if (!title || [...title].length > 128 || /[\u0000-\u001f\u007f-\u009f]/u.test(title))
                state.notice = 'importsInvalid';
              else
                state.confirm = {
                  kind: 'copy',
                  title,
                  selection: state.preview.selection,
                  directory: state.destination.selection,
                };
            }
          } else if (result.status !== 'succeeded') state.notice = 'importsUnavailable';
          else if (r.kind === 'directory') {
            const page = result.value as AgentDirectoryPage;
            if (page.kind !== 'children') state.notice = 'importsUnavailable';
            else if (r.purpose === 'source' && state.form) state.form.directory = page.directory;
            else if (r.purpose === 'destination') state.destination = page.directory;
          } else {
            const value = result.value as AgentImportsPage;
            if (value.kind === 'sources' && r.kind === 'sources') {
              state.sources = value.sources;
              state.sourcesRevision = value.revision;
              focus(state, value.sources[0] ? 'source-' + value.sources[0].id : null);
            } else if (value.kind === 'catalog' && r.kind === 'catalog') {
              state.entries = value.entries;
              state.catalogNext = value.next;
              state.route = 'catalog';
              focus(state, value.entries.length ? 'entry-0' : null);
            } else if (value.kind === 'preview' && r.kind === 'preview') {
              if (!state.preview || !sameValue(state.preview.selection, value.preview.selection)) {
                reset(
                  state,
                  'title',
                  [...value.preview.title].slice(0, 128).join('') ||
                    formatName(
                      state.sources.find(
                        (source) => source.id === value.preview.selection.source.id,
                      )?.location.kind ?? 'codex',
                    ),
                );
                state.destination = null;
              }
              state.preview = value.preview;
              state.route = 'preview';
            } else if (value.kind === 'copies' && r.kind === 'copies') {
              state.copies = value.sessions;
              state.copiesNext = value.next;
              state.route = 'copies';
              focus(state, value.sessions[0] ? 'copy-' + value.sessions[0].id : null);
            } else state.notice = notice(value.kind);
          }
        } else if (input.kind === 'raw' && props.open) {
          const key = input.key.startsWith('footer-') ? input.key.slice(7) : input.key;
          if (key === 'back' && !state.confirm && !state.picker) {
            if (state.current || state.restoring || state.route === 'sources')
              events.push({ name: 'closed', value: {} });
            else if (!state.pending) {
              if (state.route === 'preview') {
                state.route = 'catalog';
                state.preview = null;
              } else {
                state.route = 'sources';
                state.form = null;
                sources(state, actions);
              }
            }
          } else if (key === 'history-retry' && state.historyFailed && !state.pending)
            history(state, actions);
          else if (
            ['receipt', 'confirm-receipt'].includes(key) &&
            state.current?.reference &&
            !state.pending
          )
            request(
              state,
              actions,
              { kind: 'receipt' },
              'receipts',
              'read',
              state.current.reference,
            );
          else if (key === 'confirm-cancel' && !state.pending && !state.current)
            state.confirm = null;
          else if (
            !locked(state, props) &&
            !state.picker &&
            (!state.confirm || key === 'confirm')
          ) {
            if (key === 'confirm' && state.confirm) mutate(state, actions, state.confirm);
            else if (key === 'open-latest' && state.latest)
              events.push({ name: 'selected', value: { session: state.latest } });
            else if (state.route === 'sources') {
              if (key === 'add') startForm(state, null);
              else if (key === 'refresh') {
                state.notice = null;
                sources(state, actions);
              } else if (key === 'copies') {
                state.notice = null;
                copies(state, actions);
              } else
                for (const source of state.sources) {
                  if (key === 'source-' + source.id) {
                    state.selected = key;
                    if (
                      presentation.target === 'tui' &&
                      (input.event as { kind?: string }).kind !== 'submit'
                    )
                      continue;
                  }
                  if (key === 'browse-' + source.id || key === 'source-' + source.id) {
                    state.source = { id: source.id, revision: state.sourcesRevision };
                    state.query = '';
                    state.archived = false;
                    state.notice = null;
                    state.preview = null;
                    reset(state, 'search', '');
                    catalog(state, actions);
                  } else if (key === 'edit-' + source.id) startForm(state, source);
                  else if (key === 'remove-' + source.id) {
                    state.notice = null;
                    state.confirm = {
                      kind: 'remove_source',
                      revision: state.sourcesRevision,
                      id: source.id,
                    };
                  }
                }
            } else if (state.route === 'form') {
              if (
                key.startsWith('format-') &&
                state.form &&
                ['codex', 'claude_code', 'open_code'].includes(key.slice(7))
              )
                state.form.format = key.slice(7) as Format;
              else if (key === 'source-directory') {
                state.notice = null;
                state.picker = 'source';
              } else if (key === 'save-source' && state.form?.directory) {
                state.capturedName = null;
                capture(state, actions, 'name');
              }
            } else if (state.route === 'catalog') {
              if (key === 'search') capture(state, actions, 'search');
              else if (key === 'archived') {
                state.archived = !state.archived;
                capture(state, actions, 'search');
              } else if (key === 'refresh-catalog') {
                state.notice = null;
                catalog(state, actions);
              } else if (key === 'catalog-more' && state.catalogNext)
                catalog(state, actions, state.catalogNext);
              else if (key.startsWith('entry-')) {
                const entry = state.entries[Number(key.slice(6))];
                if (entry) {
                  state.selected = key;
                  if (
                    presentation.target !== 'tui' ||
                    (input.event as { kind?: string }).kind === 'submit'
                  ) {
                    state.preview = null;
                    state.notice = null;
                    read(
                      state,
                      actions,
                      { kind: 'preview' },
                      {
                        kind: 'preview',
                        source: state.source,
                        entry: { sessionId: entry.id, path: entry.path },
                        digest: null,
                        after: 0,
                      },
                    );
                  }
                }
              }
            } else if (state.route === 'preview') {
              if (key === 'destination-directory') {
                state.notice = null;
                state.picker = 'destination';
              } else if (key === 'review-copy' && state.destination)
                capture(state, actions, 'title');
              else if (state.preview && ['preview-more', 'preview-first'].includes(key)) {
                const selection = state.preview.selection;
                const after = key === 'preview-more' ? state.preview.next : 0;
                if (after !== null)
                  read(
                    state,
                    actions,
                    { kind: 'preview' },
                    {
                      kind: 'preview',
                      ...selection,
                      digest: key === 'preview-first' ? null : selection.digest,
                      after,
                    },
                  );
              }
            } else if (state.route === 'copies') {
              if (key === 'refresh-copies') copies(state, actions);
              else if (key === 'copies-more' && state.copiesNext)
                copies(state, actions, state.copiesNext);
              else {
                const session = state.copies.find((session) => key === 'copy-' + session.id);
                if (session) {
                  state.selected = key;
                  if (
                    presentation.target !== 'tui' ||
                    (input.event as { kind?: string }).kind === 'submit'
                  )
                    events.push({ name: 'selected', value: { session } });
                }
              }
            }
          }
        }
        return frame(state, props, actions, events);
      },
    },
    () => 'back',
  );
}
export function activate() {
  return { Imports: { kind: 'component', contract: importsContract, component: ['component'] } };
}
