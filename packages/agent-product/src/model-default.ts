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
  Emission,
  ConnectionTarget,
  RecoveryRef,
  OperationReply,
  CallerIntent,
  CatalogMutationResult,
} from '../../plugin-sdk/src/index.js';
import { modelDefaultContract, type ModelDefaultProps } from './contracts.js';
import { copy, type Copy } from './locale.js';
import { sameValue } from './value.js';
import { compactLabel, output, type Presentation, type Node } from './view.js';

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
  revision: number;
  serial: number;
  confirmation: Intent | null;
  page: number;
  current: {
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
  if (value.kind === 'committed' && value.connection === undefined) {
    state.notice = current.target ? 'defaultSaved' : 'defaultCleared';
    state.current = null;
    events.push({ name: 'changed', value: {} });
  } else if (value.kind === 'committed') state.notice = 'defaultUnknown';
  else {
    state.current = null;
    state.conflictRevision = value.kind === 'revision_conflict' ? current.revision : null;
    state.notice = value.kind === 'revision_conflict' ? 'defaultConflict' : 'defaultRejected';
  }
}
function history(state: State, actions: Action[], after: number) {
  if (Object.values(state.pending).some((r) => r.kind === 'history' || r.kind === 'earlier'))
    return;
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
  const lines = (state.confirmation?.identity ?? [])
    .flatMap((value) => value.split(/\r?\n/u))
    .flatMap((line) => {
      if (target === 'desktop') return [line];
      const characters = [...line],
        result: string[] = [];
      for (let at = 0; at < characters.length; at += 36)
        result.push(characters.slice(at, at + 36).join(''));
      return result.length ? result : [''];
    });
  const size = target === 'tui' ? 3 : 18,
    pages = Math.max(1, Math.ceil(lines.length / size));
  const page = Math.min(state.page, pages - 1);
  const main = ui.box(
    'default-main',
    [
      ui.text('default-heading', text.defaultModel, 16, props.theme.text, 550),
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
        { row: target === 'desktop', gap: 0, style: { wrap: true } },
      ),
      ui.text('default-summary', summary, 13, props.theme.muted),
      ...(target === 'desktop'
        ? [ui.text('default-help', text.defaultHelp, 13, props.theme.muted)]
        : []),
      ...(state.notice
        ? [ui.text('default-notice', text[state.notice], 13, props.theme.muted)]
        : []),
    ],
    { visible: !state.confirmation && !state.historyOpen, gap: target === 'desktop' ? 8 : 0 },
  );
  const confirmation = ui.box(
    'default-confirmation',
    [
      ui.box(
        'default-confirm-actions',
        [
          ui.button('default-confirm', text.confirmDefault, !canConfirm(state, props)),
          ui.button('default-cancel', text.cancel),
          ...(page > 0 ? [ui.button('default-previous', text.previousModels)] : []),
          ...(page + 1 < pages ? [ui.button('default-next', text.moreTarget)] : []),
        ],
        { row: target === 'desktop', gap: 0, style: { wrap: true } },
      ),
      ui.text(
        'default-question',
        state.confirmation?.target ? text.confirmSetDefault : text.confirmClearDefault,
        13,
      ),
      ...(!canConfirm(state, props)
        ? [ui.text('default-stale', text.defaultConflict, 13, props.theme.muted)]
        : []),
      ...(pages > 1
        ? [ui.text('default-page', page + 1 + ' / ' + pages, 12, props.theme.muted)]
        : []),
      ...lines
        .slice(page * size, (page + 1) * size)
        .map((line, at) => ui.text('default-line-' + at, line, 13)),
    ],
    { visible: !!state.confirmation, gap: target === 'desktop' ? 8 : 0 },
  );
  const earlier = ui.box(
    'default-earlier',
    [
      ui.box(
        'default-history-actions',
        [
          ui.button('default-history-back', text.back),
          ui.button('default-history-refresh', text.refreshDefaultOperations),
          ...(state.historyNext !== null
            ? [ui.button('default-history-more', text.moreOperations)]
            : []),
        ],
        { row: target === 'desktop', gap: 0, style: { wrap: true } },
      ),
      ...(target === 'desktop'
        ? [ui.text('default-history-help', text.earlierDefaultHelp, 13, props.theme.muted)]
        : []),
      ...(state.historyFailed
        ? [ui.text('default-history-failed', text.historyUnavailable, 13, props.theme.muted)]
        : []),
      ...state.earlier.map((entry, at) => {
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
      }),
    ],
    { visible: state.historyOpen, gap: target === 'desktop' ? 8 : 0 },
  );
  const out = output(
    state,
    ui.box('model-default', [main, confirmation, earlier], { gap: 0 }),
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
        revision: 0,
        serial: 0,
        confirmation: null,
        page: 0,
        current: null,
        pending: {},
        notice: null,
        conflictRevision: null,
        historyOpen: false,
        earlier: [],
        historyNext: null,
        historyFailed: false,
      };
      return frame(presentation, state, props);
    },
    update(state, props) {
      state.revision++;
      return frame(presentation, state, props);
    },
    event(state, input, props) {
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
            state.current = null;
            state.notice = 'defaultRejected';
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
            state.historyNext = page.length === 2 ? page[page.length - 1].cursor : null;
            state.earlier = page
              .filter(
                (entry) =>
                  entry.reference.contract.name === 'maka.settings.Settings' &&
                  entry.reference.method === 'setDefault',
              )
              .map((entry) => ({ cursor: entry.cursor, reference: entry.reference, result: null }));
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
        else if (key === 'default-next' && state.confirmation) state.page++;
        else if (key === 'default-previous' && state.confirmation)
          state.page = Math.max(0, state.page - 1);
        else if (key === 'default-confirm' && canConfirm(state, props)) {
          const intent = state.confirmation!;
          state.confirmation = null;
          state.current = { revision: intent.revision, target: intent.target, reference: null };
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
          if (key === 'default-set' && selected) {
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
            state.page = 0;
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
            state.page = 0;
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
