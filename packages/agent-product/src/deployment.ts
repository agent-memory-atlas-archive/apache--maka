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
  Component,
  ComponentInput,
  ReadingCommand,
  Emission,
  DeploymentRef,
  DeploymentInstances,
  ManagedInstance,
  ManagedInstancesPage,
  OperationReply,
  RecoveryRef,
  CallerIntent,
  ManagementResultForDeploymentRef,
  DeploymentDocumentChunk,
  EditorObservation,
  FileObservation,
  ManagementResultForPackageInstalled,
} from '../../plugin-sdk/src/index.js';
import { deploymentContract, type ConnectionsProps } from './contracts.js';
import { copy, type Copy } from './locale.js';
import { sameValue } from './value.js';
import { fill, output, type Presentation, type Node } from './view.js';
import {
  acceptDocumentChunk,
  canSaveDocument,
  documentEditor,
  documentView,
  observeDocument,
  type DocumentEditor,
} from './deployment-document.js';

import {
  packageImport,
  importView,
  canInstall,
  type PackageImport,
  type InstallIntent,
} from './deployment-import.js';

type Request =
  | { kind: 'page'; input: DeploymentInstances; movement: 'reset' | 'next' | 'previous' }
  | { kind: 'mutate' }
  | { kind: 'receipt'; reference: RecoveryRef }
  | { kind: 'history'; after: number };
type DocumentRequest =
  | { kind: 'document'; source: DeploymentRef; offset: number }
  | { kind: 'documentReload' };
interface Intent {
  source: DeploymentRef;
  instance: ManagedInstance;
  enabled: boolean;
}
interface State {
  revision: number;
  serial: number;
  pending: { key: string; request: Request | DocumentRequest } | null;
  page: ManagedInstancesPage | null;
  selected: ManagedInstance['instance'] | null;
  focus: ReadingCommand | null;
  after: ManagedInstancesPage['next'];
  previous: ManagedInstancesPage['next'][];
  confirmation: Intent | null;
  current: {
    intent: Intent | { source: DeploymentRef; editor: EditorObservation } | InstallIntent;
    reference: RecoveryRef | null;
  } | null;
  importer: PackageImport;
  document: DocumentEditor | null;
  documentOpen: boolean;
  diagnostic: string | null;
  notice: keyof Copy | null;
  conflict: boolean;
  earlier: { cursor: number; reference: RecoveryRef; reply: OperationReply | null }[];
  historyOpen: boolean;
  historyNext: number | null;
}
function request(
  state: State,
  actions: Action[],
  r: Request | DocumentRequest,
  binding: string,
  method: string,
  input: unknown,
  resources: Action['resources'] = {},
) {
  if (state.pending) return;
  // Initialization owns the one request lane until the final native receipt.
  // A navigation/history query cannot consume it between two document chunks.
  if (state.document?.phase === 'loading' && r.kind !== 'document') return;
  const key = 'deployment-' + ++state.serial;
  state.pending = { key, request: r };
  actions.push({ key, binding, member: 0, method, input, resources });
}
function page(
  state: State,
  actions: Action[],
  input: DeploymentInstances,
  movement: 'reset' | 'next' | 'previous' = 'reset',
) {
  request(state, actions, { kind: 'page', input, movement }, 'management', 'instances', input);
}
function readDocument(state: State, actions: Action[], offset: number) {
  const source = state.document!.source;
  request(state, actions, { kind: 'document', source, offset }, 'management', 'readDocument', {
    source,
    offset,
  });
}
function settle(state: State, reply: OperationReply, actions: Action[]) {
  const current = state.current;
  if (!current || (current.reference && !sameValue(current.reference, reply.reference))) return;
  current.reference = reply.reference;
  if (reply.receipt.kind !== 'recorded' || reply.receipt.outcome.kind !== 'returned') {
    state.notice = 'deploymentUnknown';
    return;
  }
  const isInstall = 'file' in current.intent;
  const value = reply.receipt.outcome.value as
    | ManagementResultForDeploymentRef
    | ManagementResultForPackageInstalled;
  const isDocument = 'editor' in current.intent;
  if (value.status === 'committed') {
    if (isInstall && 'package' in value.value) {
      state.importer.installed = value.value;
      if (state.page) state.page.source = value.value.source;
      state.importer.confirmation = null;
    }
    if (isDocument && state.document && !('package' in value.value)) {
      state.document.source = value.value;
      state.document.confirmation = null;
      state.document.conflict = false;
    }
    state.current = null;
    state.confirmation = null;
    state.notice = isInstall ? 'deploymentInstalled' : 'deploymentSaved';
    state.conflict = false;
    page(state, actions, { kind: 'start' });
  } else {
    if (isInstall) state.importer.confirmation = null;
    if (isDocument && state.document) {
      state.document.confirmation = null;
      state.document.conflict = value.status === 'conflict';
    }
    state.diagnostic = value.status === 'rejected' ? (value.diagnostic ?? null) : null;
    state.current = null;
    state.conflict = value.status === 'conflict';
    state.notice =
      value.status === 'conflict'
        ? 'deploymentConflict'
        : isInstall
          ? 'deploymentInstallRejected'
          : isDocument
            ? value.reason === 'execution_environment'
              ? 'deploymentEnvironment'
              : value.reason === 'package_unavailable'
                ? 'deploymentPackageUnavailable'
                : 'deploymentDocumentRejected'
            : value.reason === 'parent_disabled'
              ? 'deploymentParentDisabled'
              : 'deploymentRejected';
  }
}
function frame<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  props: ConnectionsProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const target = presentation.target;
  const ui = presentation.paint(props.theme),
    text = copy(props.locale);
  const loading = state.document?.phase === 'loading';
  const busy = !!state.pending || !!state.current || loading;
  const entries = state.page?.instances ?? [];
  const current = state.confirmation;
  const history = state.historyOpen;
  const confirmationBody = current
    ? [
        ui.text(
          'deployment-question',
          current.enabled ? text.deploymentConfirmEnable : text.deploymentConfirmDisable,
          16,
          props.theme.text,
          550,
        ),
        ui.text('deployment-confirm-path', current.instance.path.join(' / '), 14),
        ui.text('deployment-confirm-package', current.instance.package.id, 13, props.theme.muted),
        ui.text('deployment-confirm-help', text.deploymentChangeHelp, 13, props.theme.muted),
      ]
    : [];
  const historyBody = [
    ...(target === 'tui' && state.notice
      ? [ui.text('deployment-history-notice', text[state.notice], 13, props.theme.muted)]
      : []),
    ...(target === 'tui' && state.pending
      ? [ui.text('deployment-history-loading', text.connecting, 13, props.theme.muted)]
      : []),
    ui.text('deployment-history-help', text.deploymentHistoryHelp, 13, props.theme.muted),
    ...state.earlier.map((entry, at) =>
      ui.box(
        'deployment-earlier-' + at,
        [
          ui.text(
            'deployment-earlier-status-' + at,
            entry.reply?.receipt.kind === 'recorded' &&
              entry.reply.receipt.outcome.kind === 'returned'
              ? text.deploymentRecorded
              : text.deploymentUnknown,
            13,
          ),
          ui.button('deployment-earlier-check-' + at, text.check, !!state.pending),
        ],
        { gap: 0 },
      ),
    ),
  ];
  const rows = entries.map((entry, at) => {
    const status = entry.parentDisabled
      ? text.deploymentParentDisabled
      : entry.disabled
        ? text.connectionDisabled
        : text.connectionEnabled;
    const toggle = ui.button(
      'deployment-toggle-' + at,
      entry.disabled ? text.deploymentEnable : text.deploymentDisable,
      busy || state.conflict || (entry.disabled && entry.parentDisabled),
    );
    if (ui.item) {
      const selected = sameValue(state.selected, entry.instance);
      return ui.contextMenu!(
        'deployment-instance-' + at,
        text.deploymentTitle,
        ui.item(
          'deployment-select-' + at,
          ui.box(
            'deployment-summary-' + at,
            [
              ui.text(
                'deployment-path-' + at,
                `${selected ? (props.theme.ascii ? '>' : '›') : ' '} ${entry.path.join(' / ')}`,
                14,
                props.theme.text,
                400,
                { truncate: true },
              ),
              ui.text(
                'deployment-detail-' + at,
                `  ${status} · ${entry.package.id}`,
                13,
                props.theme.muted,
                400,
                { truncate: true },
              ),
            ],
            { gap: 0 },
          ),
          selected,
          busy,
          { followFocus: true, submit: true },
        ),
        [toggle],
      );
    }
    return ui.box(
      'deployment-instance-' + at,
      [
        ui.box(
          'deployment-identity-' + at,
          [
            ui.text('deployment-path-' + at, entry.path.join(' / '), 15, props.theme.text, 550),
            ui.text('deployment-package-' + at, entry.package.id, 13, props.theme.muted),
          ],
          {
            gap: target === 'desktop' ? 4 : 0,
            surface: target === 'desktop',
            style: { grow: 1, minWidth: 0 },
          },
        ),
        ui.box(
          'deployment-row-' + at,
          [ui.text('deployment-state-' + at, status, 13, props.theme.muted), toggle],
          {
            row: target === 'desktop',
            gap: 0,
            surface: target === 'desktop',
            style: { wrap: true, align: 'center', shrink: 0 },
          },
        ),
      ],
      {
        row: target === 'desktop',
        gap: target === 'desktop' ? 16 : 1,
        padding: target === 'desktop' ? 12 : 1,
        surface: target === 'desktop',
        style: { width: fill, wrap: true, shrink: 0, radius: 10, align: 'center' },
      },
    );
  });
  const body = target === 'tui' ? rows : current ? confirmationBody : history ? historyBody : rows;
  const header = ui.box(
    'deployment-header',
    [
      ui.box(
        'deployment-title-row',
        [
          ui.button(
            'deployment-back',
            text.back,
            false,
            false,
            target === 'tui' ? { background: 0, align: 'start' } : {},
          ),
          ui.text('deployment-title', text.deploymentTitle, 22, props.theme.text, 600),
        ],
        { row: true, gap: target === 'desktop' ? 8 : 1, terminal: { wrap: true } },
      ),
      ...(target === 'tui' || (!current && !history)
        ? [ui.text('deployment-help', text.deploymentHelp, 13, props.theme.muted)]
        : []),
      ...(state.notice
        ? [ui.text('deployment-notice', text[state.notice], 13, props.theme.muted)]
        : []),
      ...(state.pending
        ? [ui.text('deployment-loading', text.connecting, 13, props.theme.muted)]
        : []),
    ],
    { gap: target === 'desktop' ? 8 : 1, style: { shrink: 0 } },
  );
  const footer = ui.box(
    'deployment-controls',
    target === 'desktop' && current
      ? [
          ui.button('deployment-confirm', text.confirmDefault, busy),
          ui.button('deployment-cancel', text.cancel, busy),
        ]
      : target === 'desktop' && history
        ? [
            ui.button('deployment-history-back', text.back, false, false, {}),
            ui.button(
              'deployment-history-refresh',
              text.refreshDefaultOperations,
              !!state.pending,
              false,
              {},
              undefined,
              text.refreshDefaultOperations,
            ),
            ...(state.historyNext !== null
              ? [ui.button('deployment-history-more', text.moreOperations, !!state.pending)]
              : []),
          ]
        : [
            ui.button(
              'deployment-refresh',
              target === 'tui' ? (props.theme.ascii ? 'r' : '↻') : text.deploymentRefresh,
              busy,
              false,
              target === 'tui' ? { background: 0 } : {},
              undefined,
              text.deploymentRefresh,
            ),
            ...(state.previous.length
              ? [ui.button('deployment-previous', text.deploymentPrevious, busy)]
              : []),
            ...(state.page?.next ? [ui.button('deployment-next', text.deploymentMore, busy)] : []),
            ...(state.current?.reference
              ? [ui.button('deployment-check', text.check, !!state.pending)]
              : []),
            ui.button('deployment-history', text.deploymentHistory, !!state.pending || loading),
            ui.button('deployment-advanced', text.deploymentEdit, busy || !state.page),
            ui.button('deployment-import', text.deploymentInstall, busy || !state.page),
          ],
    {
      row: true,
      gap: target === 'desktop' ? 8 : 1,
      terminal: { wrap: true },
      style: { wrap: true, shrink: 0 },
    },
  );
  const out = output(
    state,
    ui.box(
      'deployment-panel',
      [
        header,
        presentation.reading(
          'deployment-list',
          {
            command: state.focus,
            content: ui.box(
              'deployment-body',
              body.length
                ? body
                : [ui.text('deployment-empty', text.deploymentEmpty, 14, props.theme.muted)],
              {
                scroll: 'manual',
                focusGroup: target === 'tui',
                terminal: { grow: 1, basis: 0 },
                style: { width: fill, height: fill, minHeight: 0 },
                gap: target === 'desktop' ? 16 : 1,
              },
            ),
          },
          { input: 'deployment-input' },
        ),
        footer,
      ],
      {
        style: { width: fill, height: fill },
        visible: !state.documentOpen && !state.importer.open,
        terminal: { grow: 1 },
        autoFocus: true,
        shortcuts:
          target === 'tui'
            ? [{ key: 'escape', modifiers: [], input: 'deployment-back', enabled: true }]
            : [],
        gap: target === 'desktop' ? 16 : 1,
      },
    ),
    events,
  );
  if (target === 'tui') {
    const modalBody = current ? confirmationBody : historyBody;
    out.frame.view = ui.dialog(
      'deployment-dialog',
      current ? text.deploymentTitle : text.deploymentHistory,
      out.frame.view,
      ui.box(
        'deployment-dialog-content',
        [
          ...(current || history
            ? [
                ui.box('deployment-dialog-body', modalBody, {
                  scroll: 'manual',
                  terminal: { grow: 1 },
                  gap: 1,
                }),
              ]
            : []),
          ui.box(
            'deployment-dialog-actions',
            current
              ? [
                  ui.button('deployment-cancel', text.cancel, busy),
                  ui.button('deployment-confirm', text.confirmDefault, busy, true),
                ]
              : [
                  ui.button('deployment-history-back', text.back),
                  ui.button(
                    'deployment-history-refresh',
                    props.theme.ascii ? 'r' : '↻',
                    !!state.pending,
                    false,
                    { background: 0 },
                    undefined,
                    text.refreshDefaultOperations,
                  ),
                  ...(state.historyNext !== null
                    ? [ui.button('deployment-history-more', text.moreOperations, !!state.pending)]
                    : []),
                ],
            { row: true, gap: 1, terminal: { wrap: true } },
          ),
        ],
        {
          terminal: { grow: 1 },
        },
      ),
      !!current || history,
      undefined,
      { width: 64, height: history ? 18 : 14 },
    );
  }
  out.frame.view = ui.box(
    'deployment-pages',
    [
      out.frame.view,
      importView(
        presentation,
        props,
        state.importer,
        busy,
        state.conflict,
        state.notice ? text[state.notice] : null,
        !!state.current?.reference,
        !!state.pending,
      ),
      ...(state.document
        ? [
            documentView(
              presentation,
              props,
              state.document,
              state.documentOpen,
              busy,
              state.notice ? text[state.notice] : null,
              state.diagnostic,
              !!state.current?.reference,
              !!state.pending,
            ),
          ]
        : []),
    ],
    {
      gap: 0,
      padding: target === 'desktop' ? 24 : 1,
      style: { width: fill, height: fill, maxWidth: 808, minWidth: 0 },
      terminal: { grow: 1 },
    },
  );
  if (target === 'desktop')
    out.frame.view = ui.box('deployment-surface', [out.frame.view], {
      gap: 0,
      style: { width: fill, height: fill, align: 'center' },
    });
  out.actions = actions;
  return out;
}
export function deployment<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ConnectionsProps, N> {
  return {
    mount(props) {
      const state: State = {
        revision: 0,
        serial: 0,
        pending: null,
        page: null,
        selected: null,
        focus: null,
        after: null,
        previous: [],
        confirmation: null,
        current: null,
        notice: null,
        conflict: false,
        earlier: [],
        historyOpen: false,
        historyNext: null,
        importer: packageImport(),
        document: null,
        documentOpen: false,
        diagnostic: null,
      };
      const actions: Action[] = [];
      page(state, actions, { kind: 'start' });
      return frame(presentation, state, props, actions);
    },
    update(state, props) {
      state.revision++;
      return frame(presentation, state, props);
    },
    event(state, input, props) {
      state.revision++;
      if (input.kind === 'component' && input.child === 'deployment-list' && input.name === 'input')
        input = input.event as ComponentInput;
      const actions: Action[] = [],
        events: Emission[] = [];
      if (
        input.kind === 'component' &&
        input.child === 'deployment-import-file' &&
        input.name === 'changed'
      ) {
        if (!sameValue(state.importer.observation, input.event)) {
          state.importer.installed = null;
          if (
            !state.current &&
            (state.notice === 'deploymentInstalled' || state.notice === 'deploymentInstallRejected')
          )
            state.notice = null;
        }
        state.importer.observation = input.event as FileObservation;
        if (
          state.importer.confirmation &&
          !sameValue(state.importer.confirmation.file, input.event)
        )
          state.importer.confirmation = null;
      } else if (
        input.kind === 'component' &&
        input.child === 'deployment-document-editor' &&
        input.name === 'changed' &&
        state.document
      ) {
        const next = observeDocument(state.document, input.event as EditorObservation);
        if (next !== null) readDocument(state, actions, next);
      } else if (input.kind === 'action') {
        const pending = state.pending;
        if (!pending || pending.key !== input.key) return frame(presentation, state, props);
        state.pending = null;
        const outcome = input.event as ActionOutcome,
          r = pending.request;
        if (r.kind === 'document') {
          const editor = state.document;
          if (editor && sameValue(editor.source, r.source) && editor.offset === r.offset) {
            if (outcome.status === 'succeeded')
              acceptDocumentChunk(editor, outcome.value as DeploymentDocumentChunk);
            else editor.phase = 'failed';
          }
        } else if (r.kind === 'documentReload') {
          if (outcome.status === 'succeeded' && state.document) {
            const value = outcome.value as ManagedInstancesPage,
              old = state.document;
            state.document = documentEditor(value.source);
            state.document.observation = old.observation;
            state.document.sequence = old.sequence;
            state.page = value;
            state.previous = [];
            state.after = null;
            state.conflict = false;
            state.notice = null;
            state.diagnostic = null;
            readDocument(state, actions, 0);
          } else state.notice = 'deploymentUnavailable';
        } else if (r.kind === 'page') {
          if (outcome.status === 'succeeded') {
            const value = outcome.value as ManagedInstancesPage;
            if (r.input.kind === 'continue' && !sameValue(value.source, r.input.source))
              state.notice = 'deploymentUnavailable';
            else {
              if (r.movement === 'reset') {
                state.previous = [];
                state.after = null;
                state.conflict = false;
              } else if (r.movement === 'next') {
                state.previous.push(state.after);
                state.after = r.input.kind === 'continue' ? r.input.after : null;
              } else {
                state.after = state.previous.pop()!;
              }
              state.page = value;
              if (!value.instances.some((entry) => sameValue(entry.instance, state.selected)))
                state.selected = value.instances[0]?.instance ?? null;
              if (presentation.target === 'tui' && state.selected)
                state.focus = {
                  sequence: (state.focus?.sequence ?? 0) + 1,
                  target: {
                    kind: 'focus_item',
                    key:
                      'deployment-select-' +
                      value.instances.findIndex((entry) =>
                        sameValue(entry.instance, state.selected),
                      ),
                  },
                };
              if (
                state.importer.confirmation &&
                !sameValue(state.importer.confirmation.source, value.source)
              )
                state.importer.confirmation = null;
            }
          } else state.notice = 'deploymentUnavailable';
        } else if (r.kind === 'mutate') {
          if (outcome.status === 'operation') settle(state, outcome.reply, actions);
          else if (outcome.status === 'not_accepted') {
            state.current = null;
            state.notice = 'deploymentRejected';
          } else state.notice = 'deploymentUnknown';
        } else if (r.kind === 'receipt') {
          if (
            outcome.status === 'succeeded' &&
            sameValue((outcome.value as OperationReply).reference, r.reference)
          ) {
            const reply = outcome.value as OperationReply;
            if (state.current?.reference && sameValue(state.current.reference, r.reference))
              settle(state, reply, actions);
            const entry = state.earlier.find((entry) => sameValue(entry.reference, r.reference));
            if (entry) entry.reply = reply;
          } else state.notice = 'deploymentUnknown';
        } else if (outcome.status === 'succeeded') {
          const items = outcome.value as CallerIntent[];
          state.historyNext = items.length === 2 ? items[items.length - 1].cursor : null;
          state.earlier = items
            .filter(
              (entry) =>
                entry.reference.contract.name === 'maka.application.DeploymentManager' &&
                (entry.reference.method === 'setEnabled' ||
                  entry.reference.method === 'replace' ||
                  entry.reference.method === 'install'),
            )
            .map((entry) => ({ cursor: entry.cursor, reference: entry.reference, reply: null }));
        } else state.notice = 'historyUnavailable';
      } else if (input.kind === 'raw') {
        if (
          state.document?.phase === 'loading' &&
          input.key !== 'deployment-document-back' &&
          input.key !== 'deployment-back'
        )
          return frame(presentation, state, props);
        let key = input.key;
        if (key === 'deployment-dialog')
          key = state.confirmation ? 'deployment-cancel' : 'deployment-history-back';
        if (/^deployment-select-(0|[1-9][0-9]*)$/.test(key) && !state.pending && !state.current) {
          const at = Number(key.slice('deployment-select-'.length));
          const entry = state.page?.instances[at];
          if (entry) {
            state.selected = entry.instance;
            if ((input.event as { kind?: string } | null)?.kind === 'submit')
              key = 'deployment-toggle-' + at;
          }
        }
        if (key === 'deployment-import-check') key = 'deployment-check';
        if (key === 'deployment-import-refresh') key = 'deployment-refresh';
        if (key === 'deployment-import-advanced') key = 'deployment-advanced';
        const importer = state.importer;
        if (key === 'deployment-import-history') {
          importer.open = false;
          key = 'deployment-history';
        }
        if (key === 'deployment-import-back') {
          importer.open = false;
          importer.confirmation = null;
        } else if (key === 'deployment-import' && !state.pending && !state.current && state.page) {
          importer.open = true;
          importer.confirmation = null;
          state.documentOpen = false;
          state.historyOpen = false;
        } else if (key === 'deployment-import-cancel' && !state.pending && !state.current) {
          importer.confirmation = null;
        } else if (
          key === 'deployment-import-install' &&
          state.page &&
          canInstall(importer, !!state.pending || !!state.current, state.conflict)
        ) {
          importer.confirmation = JSON.parse(
            JSON.stringify({ source: state.page.source, file: importer.observation }),
          ) as InstallIntent;
        } else if (
          key === 'deployment-import-confirm' &&
          importer.confirmation &&
          state.page &&
          canInstall(importer, !!state.pending || !!state.current, state.conflict) &&
          sameValue(importer.confirmation.file, importer.observation) &&
          sameValue(importer.confirmation.source, state.page.source)
        ) {
          const intent = importer.confirmation;
          importer.confirmation = null;
          state.current = { intent, reference: null };
          state.notice = null;
          request(
            state,
            actions,
            { kind: 'mutate' },
            'management',
            'install',
            { expectedRevision: intent.source.revision },
            {
              package: {
                child: 'deployment-import-file',
                method: 'snapshot',
                input: { owner: intent.file.owner, revision: intent.file.revision },
              },
            },
          );
        }
        if (key === 'deployment-document-check') key = 'deployment-check';
        if (key === 'deployment-document-history') {
          state.documentOpen = false;
          key = 'deployment-history';
        }
        const editor = state.document;
        if (key === 'deployment-document-back') {
          state.documentOpen = false;
          if (editor) {
            editor.confirmation = null;
            editor.reloadConfirmation = false;
          }
        } else if (
          key === 'deployment-advanced' &&
          !state.pending &&
          !state.current &&
          state.page
        ) {
          state.historyOpen = false;
          importer.open = false;
          importer.confirmation = null;
          state.documentOpen = true;
          if (!editor) {
            state.notice = null;
            state.diagnostic = null;
            state.document = documentEditor(state.page.source);
            readDocument(state, actions, 0);
          }
        } else if (
          key === 'deployment-document-cancel' &&
          editor &&
          !state.pending &&
          !state.current
        ) {
          editor.confirmation = null;
          editor.reloadConfirmation = false;
        } else if (
          key === 'deployment-document-save' &&
          editor &&
          canSaveDocument(editor, !!state.pending || !!state.current)
        ) {
          editor.confirmation = JSON.parse(JSON.stringify(editor.observation)) as EditorObservation;
        } else if (
          key === 'deployment-document-confirm' &&
          editor?.confirmation &&
          canSaveDocument(editor, !!state.pending || !!state.current)
        ) {
          const frozen = editor.confirmation;
          editor.confirmation = null;
          state.current = { intent: { source: editor.source, editor: frozen }, reference: null };
          state.notice = null;
          state.diagnostic = null;
          request(
            state,
            actions,
            { kind: 'mutate' },
            'management',
            'replace',
            { expectedRevision: editor.source.revision },
            {
              document: {
                child: 'deployment-document-editor',
                method: 'snapshot',
                input: {
                  kind: 'at_revision',
                  owner: frozen.snapshot.text.owner,
                  revision: frozen.revision,
                },
              },
            },
          );
        } else if (
          key === 'deployment-document-reload' &&
          editor &&
          !state.pending &&
          !state.current &&
          editor.phase !== 'loading'
        ) {
          editor.reloadConfirmation = true;
        } else if (
          key === 'deployment-document-reload-confirm' &&
          editor?.reloadConfirmation &&
          !state.pending &&
          !state.current
        ) {
          editor.reloadConfirmation = false;
          request(state, actions, { kind: 'documentReload' }, 'management', 'instances', {
            kind: 'start',
          });
        }
        if (key === 'deployment-back') events.push({ name: 'closed', value: {} });
        else if (key === 'deployment-cancel' && !state.current) state.confirmation = null;
        else if (key === 'deployment-history-back') state.historyOpen = false;
        else if (key === 'deployment-history' && !state.pending) {
          state.historyOpen = true;
          request(state, actions, { kind: 'history', after: 0 }, 'history', 'page', {
            after: 0,
            limit: 2,
          });
        } else if (key === 'deployment-history-refresh' && !state.pending)
          request(state, actions, { kind: 'history', after: 0 }, 'history', 'page', {
            after: 0,
            limit: 2,
          });
        else if (key === 'deployment-history-more' && !state.pending && state.historyNext !== null)
          request(
            state,
            actions,
            { kind: 'history', after: state.historyNext },
            'history',
            'page',
            { after: state.historyNext, limit: 2 },
          );
        else if (/^deployment-earlier-check-(0|[1-9][0-9]*)$/.test(key) && !state.pending) {
          const entry = state.earlier[Number(key.slice('deployment-earlier-check-'.length))];
          if (entry)
            request(
              state,
              actions,
              { kind: 'receipt', reference: entry.reference },
              'receipts',
              'read',
              entry.reference,
            );
        } else if (key === 'deployment-check' && !state.pending && state.current?.reference)
          request(
            state,
            actions,
            { kind: 'receipt', reference: state.current.reference },
            'receipts',
            'read',
            state.current.reference,
          );
        else if (!state.pending && !state.current) {
          if (key === 'deployment-refresh') {
            state.confirmation = null;
            page(state, actions, { kind: 'start' });
          } else if (key === 'deployment-confirm' && state.confirmation) {
            const intent = state.confirmation;
            state.confirmation = null;
            state.current = { intent, reference: null };
            state.notice = null;
            request(state, actions, { kind: 'mutate' }, 'management', 'setEnabled', {
              expectedRevision: intent.source.revision,
              instance: intent.instance.instance,
              enabled: intent.enabled,
            });
          } else if (key === 'deployment-next' && state.page?.next) {
            page(
              state,
              actions,
              { kind: 'continue', source: state.page.source, after: state.page.next },
              'next',
            );
          } else if (key === 'deployment-previous' && state.page && state.previous.length) {
            page(
              state,
              actions,
              {
                kind: 'continue',
                source: state.page.source,
                after: state.previous[state.previous.length - 1],
              },
              'previous',
            );
          } else if (
            /^deployment-toggle-(0|[1-9][0-9]*)$/.test(key) &&
            state.page &&
            !state.conflict
          ) {
            const instance = state.page.instances[Number(key.slice('deployment-toggle-'.length))];
            if (instance && !(instance.disabled && instance.parentDisabled))
              state.confirmation = {
                source: state.page.source,
                instance,
                enabled: instance.disabled,
              };
          }
        }
      }
      return frame(presentation, state, props, actions, events);
    },
  };
}
export function activate() {
  return {
    Deployment: { kind: 'component', contract: deploymentContract, component: ['component'] },
  };
}
