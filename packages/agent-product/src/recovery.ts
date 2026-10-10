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
  CallerIntent,
  Component,
  ComponentInput,
  ComponentValues,
  OperationReply,
  RecoveryRef,
} from '../../plugin-sdk/src/index.js';
import { recoveryContract, type Theme } from './contracts.js';
import { copy, resolveLocale } from './locale.js';
import { sameValue } from './value.js';
import { compactLabel, fill, output, pixels, type Node, type Presentation } from './view.js';

type Request =
  | { kind: 'history'; after: number }
  | { kind: 'receipt'; reference: RecoveryRef; manager: boolean };
interface State {
  revision: number;
  serial: number;
  route: 'operations' | 'deployment';
  pending: { key: string; request: Request } | null;
  entries: CallerIntent[];
  next: number | null;
  selected: RecoveryRef | null;
  reply: OperationReply | null;
  page: number;
  failed: boolean;
}
function query(state: State, actions: Action[], request: Request) {
  if (state.pending) return;
  const key = 'recovery-' + ++state.serial;
  state.pending = { key, request };
  state.failed = false;
  actions.push({
    key,
    binding:
      request.kind === 'history' ? 'history' : request.manager ? 'managementReceipts' : 'receipts',
    member: 0,
    method: request.kind === 'history' ? 'page' : 'read',
    input: request.kind === 'history' ? { after: request.after, limit: 4 } : request.reference,
    resources: {},
  });
}
function boundary(text: string, page: number) {
  const at = Math.min(text.length, page * 16384);
  const unit = text.charCodeAt(at);
  return unit >= 0xdc00 && unit <= 0xdfff ? at - 1 : at;
}
export function recovery<N extends Node>(
  presentation: Presentation<N>,
  configuredLocale: string,
): Component<State, Record<string, never>, N> {
  function frame(state: State, values: ComponentValues, actions: Action[] = []) {
    const locale = resolveLocale(configuredLocale, values.locale?.[0]);
    const theme = values.appearance![0] as unknown as Theme;
    const text = copy(locale),
      ui = presentation.paint(theme);
    const desktop = presentation.target === 'desktop',
      busy = !!state.pending;
    const recorded = state.reply?.receipt.kind === 'recorded' ? state.reply.receipt : null;
    const source = recorded ? JSON.stringify(recorded, null, 2) : '';
    const start = boundary(source, state.page),
      end = boundary(source, state.page + 1);
    const out = output(
      state,
      ui.box(
        'recovery-root',
        [
          ui.text('recovery-title', text.recoveryTitle, 24, theme.text, 600),
          ui.text('recovery-help', text.recoveryHelp, 14, theme.muted),
          ui.box(
            'recovery-navigation',
            [
              ui.button('operations', text.recoveryOperations, false, state.route === 'operations'),
              ui.button('deployment', text.deploymentTitle, false, state.route === 'deployment'),
            ],
            {
              row: true,
              gap: desktop ? 12 : 1,
              terminal: { wrap: true },
              style: { wrap: true, shrink: 0 },
            },
          ),
          ui.box(
            'recovery-operations',
            [
              ...(state.failed
                ? [ui.text('recovery-refresh-failed', text.recoveryRefreshFailed, 14, theme.muted)]
                : []),
              presentation.reading(
                'recovery-reading',
                {
                  content: ui.box(
                    'recovery-records',
                    state.selected
                      ? [
                          ui.text(
                            'recovery-original',
                            compactLabel(
                              state.selected.contract.name + ' / ' + state.selected.method,
                              120,
                            ),
                            15,
                          ),
                          ui.text(
                            'recovery-observation',
                            busy
                              ? text.loading
                              : recorded
                                ? text.deploymentRecorded
                                : text.recoveryUnknown,
                            14,
                          ),
                          ...(recorded
                            ? [
                                presentation.slot(
                                  'recovery-source-' + state.page,
                                  'source',
                                  {
                                    source: source.slice(start, end),
                                    appearance: theme.markdown.code,
                                    selection: theme.markdown.selection,
                                    terminal: theme.terminal,
                                    disabled: false,
                                  },
                                  {},
                                ),
                              ]
                            : []),
                        ]
                      : [
                          ...(state.entries.length
                            ? state.entries.map((entry, index) =>
                                ui.box(
                                  'recovery-entry-' + index,
                                  [
                                    ui.text(
                                      'recovery-name-' + index,
                                      compactLabel(
                                        '#' +
                                          entry.cursor +
                                          ' · ' +
                                          entry.reference.contract.name +
                                          ' / ' +
                                          entry.reference.method,
                                        120,
                                      ),
                                      14,
                                    ),
                                    ui.button('check-' + index, text.check, busy),
                                  ],
                                  { gap: 0 },
                                ),
                              )
                            : [
                                ui.text(
                                  'recovery-empty',
                                  busy
                                    ? text.loading
                                    : state.failed
                                      ? text.historyUnavailable
                                      : text.recoveryEmpty,
                                  14,
                                  theme.muted,
                                ),
                              ]),
                        ],
                    {
                      gap: desktop ? 12 : 1,
                      scroll: 'manual',
                      terminal: { grow: 1, basis: 0 },
                      style: { width: fill, height: pixels(360), minHeight: 0 },
                    },
                  ),
                },
                { input: 'reading-input' },
              ),
              ui.box(
                'recovery-history-controls',
                [
                  ui.button('refresh', text.refreshDefaultOperations, busy),
                  ui.button('next', text.moreOperations, busy || state.next === null),
                ],
                {
                  visible: !state.selected,
                  row: true,
                  gap: desktop ? 12 : 1,
                  terminal: { wrap: true },
                  style: { wrap: true, shrink: 0 },
                },
              ),
              ui.box(
                'recovery-result-controls',
                [
                  ui.button('back', text.back, busy),
                  ui.button('check-again', text.check, busy),
                  ui.button('previous-source', text.sourcePrevious, busy || state.page === 0),
                  ui.button('next-source', text.sourceNext, busy || end >= source.length),
                ],
                {
                  visible: !!state.selected,
                  row: true,
                  gap: desktop ? 12 : 1,
                  terminal: { wrap: true },
                  style: { wrap: true, shrink: 0 },
                },
              ),
            ],
            {
              visible: state.route === 'operations',
              terminal: { grow: 1 },
              style: { width: fill, grow: 1, minHeight: 0 },
            },
          ),
          ui.box(
            'recovery-deployment',
            [
              presentation.slot(
                'recovery-deployment-content',
                'deployment',
                { theme, locale },
                { closed: 'closed' },
              ),
            ],
            {
              visible: state.route === 'deployment',
              terminal: { grow: 1 },
              style: { width: fill, grow: 1, minHeight: 0 },
            },
          ),
        ],
        {
          padding: desktop ? 24 : 1,
          gap: desktop ? 12 : 1,
          terminal: { grow: 1 },
          style: { width: fill, height: fill, minWidth: 0 },
        },
      ),
    );
    out.actions = actions;
    out.frame.surface = desktop
      ? { kind: 'window', binding: 'surface', title: text.recoveryTitle, width: 900, height: 720 }
      : { kind: 'terminal', binding: 'surface' };
    return out;
  }
  return {
    mount(_props, values) {
      const state: State = {
        revision: 0,
        serial: 0,
        route: 'operations',
        pending: null,
        entries: [],
        next: null,
        selected: null,
        reply: null,
        page: 0,
        failed: false,
      };
      const actions: Action[] = [];
      query(state, actions, { kind: 'history', after: 0 });
      return frame(state, values, actions);
    },
    update(state, _props, values) {
      return frame({ ...state, revision: state.revision + 1 }, values);
    },
    event(previous, event, _props, values) {
      const state = { ...previous };
      state.revision++;
      const actions: Action[] = [];
      if (
        event.kind === 'component' &&
        event.child === 'recovery-reading' &&
        event.name === 'input'
      ) {
        const forwarded = event.event as ComponentInput;
        if (forwarded.kind !== 'raw') return frame(state, values);
        event = forwarded;
      }
      if (event.kind === 'action' && event.key === state.pending?.key) {
        const request = state.pending.request,
          result = event.event as ActionOutcome;
        state.pending = null;
        if (request.kind === 'history') {
          state.failed = result.status !== 'succeeded';
          if (result.status === 'succeeded') {
            const entries = result.value as CallerIntent[];
            state.entries = entries;
            // The byte cap can end a page before its requested count.
            state.next = entries.length ? entries[entries.length - 1].cursor : null;
            state.selected = null;
            state.reply = null;
            state.page = 0;
          }
        } else if (state.selected && sameValue(state.selected, request.reference)) {
          const reply = result.status === 'succeeded' ? (result.value as OperationReply) : null;
          if (
            reply &&
            sameValue(reply.reference, request.reference) &&
            reply.receipt.kind === 'recorded'
          )
            state.reply = reply;
          else if (!request.manager) query(state, actions, { ...request, manager: true });
          else state.failed = true;
        }
      } else if (event.kind === 'component' && event.key === 'closed') state.route = 'operations';
      else if (event.kind === 'raw') {
        const key = event.key;
        if (key === 'operations' || key === 'deployment') state.route = key;
        else if (state.route === 'operations' && !state.pending) {
          if (key === 'refresh') query(state, actions, { kind: 'history', after: 0 });
          else if (key === 'next' && state.next !== null)
            query(state, actions, { kind: 'history', after: state.next });
          else if (key === 'back') {
            state.selected = null;
            state.reply = null;
            state.page = 0;
          } else if (key.startsWith('check-') && key !== 'check-again') {
            const entry = state.entries[Number(key.slice(6))];
            if (entry) {
              state.selected = entry.reference;
              state.reply = null;
              state.page = 0;
              query(state, actions, {
                kind: 'receipt',
                reference: entry.reference,
                manager: false,
              });
            }
          } else if (key === 'check-again' && state.selected)
            query(state, actions, { kind: 'receipt', reference: state.selected, manager: false });
          else if (key === 'previous-source') state.page = Math.max(0, state.page - 1);
          else if (key === 'next-source' && state.reply?.receipt.kind === 'recorded') {
            const source = JSON.stringify(state.reply.receipt, null, 2);
            if (boundary(source, state.page + 1) < source.length) state.page++;
          }
        }
      }
      return frame(state, values, actions);
    },
  };
}
export function activate() {
  return { Recovery: { kind: 'component', contract: recoveryContract, component: ['component'] } };
}
