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
  Emission,
  ConnectionTarget,
  RecoveryRef,
  OperationReply,
  CallerIntent,
  CatalogMutationResult,
  SettingsModelToolPreferences,
} from '../../../packages/plugin-sdk/src/index.js';
import { modelDefaultContract, type ModelDefaultProps } from '../../shared/contracts.js';
import { scrollRows, visibleRows, resetScroll } from '../../shared/scroll.js';
import { copy, type Copy } from '../../shared/locale.js';
import { sameValue } from '../../shared/value.js';
import {
  compactLabel,
  fill,
  output,
  pixels,
  type Presentation,
  type Node,
} from '../../shared/view.js';

type Request =
  | { kind: 'mutate' }
  | { kind: 'receipt'; reference: RecoveryRef }
  | { kind: 'history'; after: number }
  | { kind: 'earlier'; reference: RecoveryRef; cursor: number };
interface Intent {
  revision: number;
  target: ConnectionTarget | null;
  identity: string[];
}
interface State {
  toolsIdentity: string;
  tools: SettingsModelToolPreferences;
  revision: number;
  serial: number;
  confirmation: Intent | null;
  current: {
    kind: 'default' | 'tools';
    revision: number;
    target: ConnectionTarget | null;
    reference: RecoveryRef | null;
  } | null;
  pending: Record<string, Request>;
  notice: keyof Copy | null;
  conflictRevision: number | null;
  historyOpen: boolean;
  earlier: { cursor: number; reference: RecoveryRef; result: OperationReply | null }[];
  historyNext: number | null;
  historyFailed: boolean;
}
function request(
  state: State,
  actions: Action[],
  r: Request,
  binding: string,
  method: string,
  input: unknown,
) {
  const key = 'model-default-' + ++state.serial;
  state.pending[key] = r;
  actions.push({ key, binding, member: 0, method, input, resources: {} });
}
function syncTools(state: State, props: ModelDefaultProps) {
  const identity = JSON.stringify(props.selection?.selection ?? null);
  if (state.toolsIdentity === identity) return;
  state.toolsIdentity = identity;
  state.tools = {
    codeMode: props.selection?.tools?.codeMode ?? null,
    applyPatch: props.selection?.tools?.applyPatch ?? null,
  };
}
function toolsChanged(state: State, props: ModelDefaultProps) {
  return (
    state.tools.codeMode !== (props.selection?.tools?.codeMode ?? null) ||
    state.tools.applyPatch !== (props.selection?.tools?.applyPatch ?? null)
  );
}
function canStart(state: State, props: ModelDefaultProps) {
  return (
    !props.busy &&
    props.catalogRevision !== null &&
    props.defaultModel !== null &&
    !state.current &&
    !state.confirmation &&
    !state.historyOpen &&
    props.catalogRevision !== state.conflictRevision
  );
}
function canConfirm(state: State, props: ModelDefaultProps) {
  return (
    !props.busy &&
    !state.current &&
    !!state.confirmation &&
    props.catalogRevision === state.confirmation.revision
  );
}
function settle(state: State, reply: OperationReply, events: Emission[]) {
  const current = state.current;
  if (!current || (current.reference && !sameValue(current.reference, reply.reference))) return;
  current.reference = reply.reference;
  if (reply.receipt.kind !== 'recorded' || reply.receipt.outcome.kind !== 'returned') {
    state.notice = 'defaultUnknown';
    return;
  }
  const value = reply.receipt.outcome.value as CatalogMutationResult;
  if (
    value.kind === 'committed' &&
    (current.kind === 'tools'
      ? value.connection?.connectionId === current.target?.connectionId
      : value.connection === undefined)
  ) {
    state.notice =
      current.kind === 'tools'
        ? 'modelToolsSaved'
        : current.target
          ? 'defaultSaved'
          : 'defaultCleared';
    state.current = null;
    events.push({ name: 'changed', value: {} });
  } else if (value.kind === 'committed') state.notice = 'defaultUnknown';
  else {
    state.current = null;
    const conflict = value.kind === 'revision_conflict' || value.kind === 'connection_stale';
    state.conflictRevision = conflict ? current.revision : null;
    state.notice = conflict
      ? 'defaultConflict'
      : current.kind === 'tools'
        ? 'modelToolsRejected'
        : 'defaultRejected';
  }
}
function history(state: State, actions: Action[], after: number) {
  if (Object.values(state.pending).some((r) => r.kind === 'history' || r.kind === 'earlier'))
    return;
  if (!after) resetScroll(state, 'default-history-reading');
  state.historyFailed = false;
  request(state, actions, { kind: 'history', after }, 'history', 'page', { after, limit: 2 });
}
function frame<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  props: ModelDefaultProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const target = presentation.target;
  const ui = presentation.paint(props.theme),
    text = copy(props.locale);
  const saved = props.defaultModel;
  const summary =
    saved === null
      ? text.loadingModels
      : saved.kind === 'ready'
        ? compactLabel(saved.choice.label, 36)
        : saved?.kind === 'unavailable'
          ? text.defaultUnavailable
          : text.noDefault;
  const main = ui.box(
    'default-main',
    [
      ...(props.selection
        ? [
            ui.text('model-tools-name', props.selection.label, 16, props.theme.text, 550),
            ui.text(
              'model-tools-connection',
              props.selection.connectionName,
              12,
              props.theme.muted,
            ),
            ...(['codeMode', 'applyPatch'] as const).map((field) =>
              ui.box(
                'tools-' + field,
                [
                  ui.text(
                    'tools-label-' + field,
                    field === 'codeMode' ? 'Code Mode' : 'Apply Patch',
                    14,
                    props.theme.text,
                    550,
                  ),
                  ui.box(
                    'tools-options-' + field,
                    (['auto', 'on', 'off'] as const).map((option) => {
                      const value = option === 'auto' ? null : option === 'on';
                      return ui.button(
                        `tools-${field}-${option}`,
                        text[
                          option === 'auto'
                            ? 'modelToolsAuto'
                            : option === 'on'
                              ? 'modelToolsOn'
                              : 'modelToolsOff'
                        ],
                        !canStart(state, props),
                        state.tools[field] === value,
                      );
                    }),
                    { row: true, gap: target === 'tui' ? 1 : 8 },
                  ),
                  ui.text(
                    'tools-help-' + field,
                    text[field === 'codeMode' ? 'modelCodeModeHelp' : 'modelApplyPatchHelp'],
                    12,
                    props.theme.muted,
                  ),
                ],
                { gap: target === 'tui' ? 0 : 4 },
              ),
            ),
            ui.text('model-tools-scope', text.modelToolsScope, 12, props.theme.muted),
            ui.box(
              'tools-actions',
              [
                ui.button(
                  'tools-save',
                  text.modelToolsSave,
                  !canStart(state, props) || !toolsChanged(state, props),
                  true,
                ),
                ui.button(
                  'tools-reset',
                  text.cancel,
                  !canStart(state, props) || !toolsChanged(state, props),
                ),
              ],
              { row: true, gap: target === 'tui' ? 1 : 8 },
            ),
          ]
        : []),
      ui.text('default-heading', text.defaultModel, 16, props.theme.text, 550),
      ui.text('default-summary', summary, 13, props.theme.muted),
      ui.box(
        'default-actions',
        [
          ui.button('default-set', text.makeDefault, !canStart(state, props) || !props.selection),
          ...(saved && saved.kind !== 'none'
            ? [ui.button('default-clear', text.clearDefault, !canStart(state, props))]
            : []),
          ...(state.current?.reference
            ? [
                ui.button(
                  'default-check',
                  text.check,
                  Object.values(state.pending).some(
                    (r) => r.kind === 'receipt' || r.kind === 'mutate',
                  ),
                ),
              ]
            : []),
          ui.button('default-history', text.earlierDefaultOperations),
        ],
        {
          row: true,
          gap: target === 'desktop' ? 8 : 1,
          terminal: { wrap: true },
          style: { wrap: true },
        },
      ),
      ...(target === 'desktop'
        ? [ui.text('default-help', text.defaultHelp, 13, props.theme.muted)]
        : []),
      ...(state.notice
        ? [ui.text('default-notice', text[state.notice], 13, props.theme.muted)]
        : []),
    ],
    {
      visible: !state.confirmation && !state.historyOpen,
      gap: target === 'desktop' ? 8 : 1,
      scroll: target === 'tui' ? 'manual' : 'none',
      terminal: { grow: 1 },
    },
  );
  const confirmation = ui.box(
    'default-confirmation',
    [
      ...(state.confirmation
        ? [
            ui.box(
              'default-identity',
              [
                ui.text(
                  'default-question',
                  state.confirmation?.target ? text.confirmSetDefault : text.confirmClearDefault,
                  13,
                ),
                ...(!canConfirm(state, props)
                  ? [ui.text('default-stale', text.defaultConflict, 13, props.theme.muted)]
                  : []),
                ...(state.confirmation?.identity ?? []).map((value, at) =>
                  ui.text('default-line-' + at, value, 13),
                ),
              ],
              {
                scroll: 'manual',
                autoFocus: true,
                gap: target === 'desktop' ? 8 : 1,
                terminal: { grow: 1, basis: 0 },
                style: { width: fill, height: pixels(220), minHeight: 0 },
              },
            ),
          ]
        : []),
      ui.box(
        'default-confirm-actions',
        [
          ui.button('default-cancel', text.cancel),
          ui.button('default-confirm', text.confirmDefault, !canConfirm(state, props), true),
        ],
        {
          row: true,
          gap: target === 'desktop' ? 8 : 1,
          terminal: { wrap: true },
          style: { wrap: true },
        },
      ),
    ],
    {
      visible: !!state.confirmation,
      gap: target === 'desktop' ? 8 : 1,
      terminal: { grow: 1 },
    },
  );
  const earlier = state.historyOpen
    ? presentation.reading(
        'default-history-reading',
        {
          content: ui.box(
            'default-earlier',
            [
              ui.box(
                'default-history-actions',
                [
                  ui.button(
                    'default-history-back',
                    text.back,
                    false,
                    false,
                    target === 'tui' ? { background: 0, align: 'start' } : {},
                  ),
                  ui.button(
                    'default-history-refresh',
                    target === 'tui'
                      ? props.theme.ascii
                        ? 'r'
                        : '↻'
                      : text.refreshDefaultOperations,
                    false,
                    false,
                    target === 'tui' ? { background: 0 } : {},
                    undefined,
                    text.refreshDefaultOperations,
                  ),
                ],
                {
                  row: true,
                  gap: target === 'desktop' ? 8 : 1,
                  terminal: { wrap: true },
                  style: { wrap: true },
                },
              ),
              ...(target === 'desktop'
                ? [ui.text('default-history-help', text.earlierDefaultHelp, 13, props.theme.muted)]
                : []),
              ...(state.historyFailed
                ? [
                    ui.text(
                      'default-history-failed',
                      text.historyUnavailable,
                      13,
                      props.theme.muted,
                    ),
                  ]
                : []),
              ...visibleRows(state, 'default-history-reading', state.earlier).map(
                ({ entry, at }) => {
                  const receipt = entry.result?.receipt;
                  const value =
                    receipt?.kind === 'recorded' && receipt.outcome.kind === 'returned'
                      ? (receipt.outcome.value as CatalogMutationResult)
                      : null;
                  return ui.box(
                    'default-entry-' + at,
                    [
                      ui.button(
                        'default-earlier-' + at,
                        text.check,
                        Object.values(state.pending).some(
                          (r) => r.kind === 'earlier' && r.cursor === entry.cursor,
                        ),
                      ),
                      ui.text(
                        'default-result-' + at,
                        value?.kind === 'committed'
                          ? text.defaultHistorySaved
                          : value
                            ? text.historyRejected
                            : text.historyUnknown,
                        13,
                      ),
                    ],
                    { gap: 0 },
                  );
                },
              ),
            ],
            {
              visible: state.historyOpen,
              gap: target === 'desktop' ? 8 : 1,
              scroll: 'anchor',
              terminal: { grow: 1 },
              style: { width: fill, maxHeight: 280 },
            },
          ),
        },
        { input: 'history-input', observed: 'history-observed' },
      )
    : ui.box('history-hidden', [], { visible: false });

  const out = output(
    state,
    ui.box(
      'model-default',
      target === 'tui'
        ? [
            ui.dialog(
              'default-dialog',
              state.historyOpen ? text.earlierDefaultOperations : text.defaultModel,
              main,
              ui.box('default-dialog-content', [confirmation, earlier], {
                gap: 0,
                terminal: { grow: 1 },
              }),
              !!state.confirmation || state.historyOpen,
              undefined,
              { width: 64, height: state.historyOpen ? 18 : 14 },
            ),
          ]
        : [main, confirmation, earlier],
      { gap: 0, terminal: { grow: 1 } },
    ),
    events,
  );
  out.actions = actions;
  return out;
}
export function modelDefault<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ModelDefaultProps, N> {
  const target = presentation.target;
  return {
    mount(props) {
      const state: State = {
        toolsIdentity: '',
        tools: { codeMode: null, applyPatch: null },
        revision: 0,
        serial: 0,
        confirmation: null,
        current: null,
        pending: {},
        notice: null,
        conflictRevision: null,
        historyOpen: false,
        earlier: [],
        historyNext: null,
        historyFailed: false,
      };
      syncTools(state, props);
      return frame(presentation, state, props);
    },
    update(state, props) {
      state.revision++;
      syncTools(state, props);
      return frame(presentation, state, props);
    },
    event(state, input, props) {
      if (
        input.kind === 'component' &&
        input.child === 'default-history-reading' &&
        input.name === 'input'
      )
        input = input.event as ComponentInput;
      if (
        state.historyOpen &&
        scrollRows(state, input, 'default-history-reading', state.earlier.length)
      )
        input = { kind: 'raw', key: 'default-history-more', event: {} };
      if (input.kind === 'raw' && input.key === 'default-dialog')
        input = { ...input, key: state.historyOpen ? 'default-history-back' : 'default-cancel' };
      state.revision++;
      const actions: Action[] = [],
        events: Emission[] = [];
      if (input.kind === 'action') {
        const r = state.pending[input.key];
        if (!r) return frame(presentation, state, props);
        delete state.pending[input.key];
        const outcome = input.event as ActionOutcome;
        if (r.kind === 'mutate') {
          if (outcome.status === 'operation') settle(state, outcome.reply, events);
          else if (outcome.status === 'not_accepted') {
            state.notice =
              state.current?.kind === 'tools' ? 'modelToolsRejected' : 'defaultRejected';
            state.current = null;
          } else state.notice = 'defaultUnknown';
        } else if (r.kind === 'receipt') {
          if (
            outcome.status === 'succeeded' &&
            sameValue((outcome.value as OperationReply).reference, r.reference)
          )
            settle(state, outcome.value as OperationReply, events);
          else state.notice = 'defaultUnknown';
        } else if (r.kind === 'history') {
          if (outcome.status !== 'succeeded') state.historyFailed = true;
          else {
            const page = outcome.value as CallerIntent[];
            state.historyNext = page.at(-1)?.cursor ?? null;
            state.earlier = [
              ...(r.after ? state.earlier : []),
              ...page
                .filter(
                  (entry) =>
                    entry.reference.contract.name === 'maka.settings.Settings' &&
                    ['setDefault', 'updateModelTools'].includes(entry.reference.method),
                )
                .map((entry) => ({
                  cursor: entry.cursor,
                  reference: entry.reference,
                  result: null,
                })),
            ];
          }
        } else if (
          outcome.status === 'succeeded' &&
          sameValue((outcome.value as OperationReply).reference, r.reference)
        ) {
          const entry = state.earlier.find(
            (entry) => entry.cursor === r.cursor && sameValue(entry.reference, r.reference),
          );
          if (entry) entry.result = outcome.value as OperationReply;
        }
      } else if (input.kind === 'raw') {
        const key = input.key;
        if (key === 'default-cancel') state.confirmation = null;
        else if (key === 'default-confirm' && canConfirm(state, props)) {
          const intent = state.confirmation!;
          state.confirmation = null;
          state.current = {
            kind: 'default',
            revision: intent.revision,
            target: intent.target,
            reference: null,
          };
          state.notice = null;
          request(state, actions, { kind: 'mutate' }, 'settings', 'setDefault', {
            expectedCatalogRevision: intent.revision,
            target: intent.target,
          });
        } else if (
          key === 'default-check' &&
          state.current?.reference &&
          !Object.values(state.pending).some((r) => r.kind === 'mutate' || r.kind === 'receipt')
        )
          request(
            state,
            actions,
            { kind: 'receipt', reference: state.current.reference },
            'receipts',
            'read',
            state.current.reference,
          );
        else if (key === 'default-history') {
          state.historyOpen = true;
          history(state, actions, 0);
        } else if (key === 'default-history-back') state.historyOpen = false;
        else if (key === 'default-history-refresh' && state.historyOpen) history(state, actions, 0);
        else if (key === 'default-history-more' && state.historyOpen && state.historyNext !== null)
          history(state, actions, state.historyNext);
        else if (state.historyOpen && /^default-earlier-(0|[1-9][0-9]*)$/.test(key)) {
          const entry = state.earlier[Number(key.slice(16))];
          if (
            entry &&
            !Object.values(state.pending).some(
              (r) => r.kind === 'earlier' && r.cursor === entry.cursor,
            )
          )
            request(
              state,
              actions,
              { kind: 'earlier', reference: entry.reference, cursor: entry.cursor },
              'receipts',
              'read',
              entry.reference,
            );
        } else if (canStart(state, props)) {
          const selected = props.selection,
            saved = props.defaultModel!;
          if (key.startsWith('tools-') && selected) {
            if (key === 'tools-reset') {
              state.toolsIdentity = '';
              syncTools(state, props);
            } else if (key === 'tools-save' && toolsChanged(state, props)) {
              state.current = {
                kind: 'tools',
                revision: props.catalogRevision!,
                target: {
                  connectionId: selected.selection.connectionId,
                  modelId: selected.selection.modelId,
                },
                reference: null,
              };
              state.notice = null;
              request(state, actions, { kind: 'mutate' }, 'settings', 'updateModelTools', {
                selection: selected.selection,
                tools: { ...state.tools },
              });
            } else {
              const match = /^tools-(codeMode|applyPatch)-(auto|on|off)$/.exec(key);
              if (match)
                state.tools[match[1] as keyof SettingsModelToolPreferences] =
                  match[2] === 'auto' ? null : match[2] === 'on';
            }
          } else if (key === 'default-set' && selected) {
            state.confirmation = {
              revision: props.catalogRevision!,
              target: {
                connectionId: selected.selection.connectionId,
                modelId: selected.selection.modelId,
              },
              identity: [
                ...new Set([selected.connectionName, selected.label, selected.selection.modelId]),
              ],
            };
          } else if (key === 'default-clear' && saved.kind !== 'none') {
            state.confirmation = {
              revision: props.catalogRevision!,
              target: null,
              identity:
                saved.kind === 'ready'
                  ? [
                      ...new Set([
                        saved.choice.connectionName,
                        saved.choice.label,
                        saved.choice.selection.modelId,
                      ]),
                    ]
                  : [copy(props.locale).defaultUnavailable, saved.target.modelId],
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
    ModelDefault: { kind: 'component', contract: modelDefaultContract, component: ['component'] },
  };
}
