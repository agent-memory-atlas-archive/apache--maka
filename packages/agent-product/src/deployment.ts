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
  DeploymentRef,
  DeploymentInstances,
  ManagedInstance,
  ManagedInstancesPage,
  OperationReply,
  RecoveryRef,
  CallerIntent,
  ManagementResultForDeploymentRef,
} from '../../plugin-sdk/src/index.js';
import { deploymentContract, type ConnectionsProps } from './contracts.js';
import { copy, type Copy } from './locale.js';
import { sameValue } from './value.js';
import { compactLabel, fill, output, paint, pixels, type Target } from './view.js';

type Request =
  | { kind: 'page'; input: DeploymentInstances; movement: 'reset' | 'next' | 'previous' }
  | { kind: 'mutate' }
  | { kind: 'receipt'; reference: RecoveryRef }
  | { kind: 'history'; after: number };
interface Intent {
  source: DeploymentRef;
  instance: ManagedInstance;
  enabled: boolean;
}
interface State {
  revision: number;
  serial: number;
  pending: { key: string; request: Request } | null;
  page: ManagedInstancesPage | null;
  after: ManagedInstancesPage['next'];
  previous: ManagedInstancesPage['next'][];
  confirmation: Intent | null;
  current: { intent: Intent; reference: RecoveryRef | null } | null;
  notice: keyof Copy | null;
  conflict: boolean;
  earlier: { cursor: number; reference: RecoveryRef; reply: OperationReply | null }[];
  historyOpen: boolean;
  historyNext: number | null;
}
function request(
  state: State,
  actions: Action[],
  r: Request,
  binding: string,
  method: string,
  input: unknown,
) {
  if (state.pending) return;
  const key = 'deployment-' + ++state.serial;
  state.pending = { key, request: r };
  actions.push({ key, binding, member: 0, method, input, resources: {} });
}
function page(
  state: State,
  actions: Action[],
  input: DeploymentInstances,
  movement: 'reset' | 'next' | 'previous' = 'reset',
) {
  request(state, actions, { kind: 'page', input, movement }, 'management', 'instances', input);
}
function settle(state: State, reply: OperationReply, actions: Action[]) {
  const current = state.current;
  if (!current || (current.reference && !sameValue(current.reference, reply.reference))) return;
  current.reference = reply.reference;
  if (reply.receipt.kind !== 'recorded' || reply.receipt.outcome.kind !== 'returned') {
    state.notice = 'deploymentUnknown';
    return;
  }
  const value = reply.receipt.outcome.value as ManagementResultForDeploymentRef;
  if (value.status === 'committed') {
    state.current = null;
    state.confirmation = null;
    state.notice = 'deploymentSaved';
    state.conflict = false;
    page(state, actions, { kind: 'start' });
  } else {
    state.current = null;
    state.conflict = value.status === 'conflict';
    state.notice =
      value.status === 'conflict'
        ? 'deploymentConflict'
        : value.reason === 'parent_disabled'
          ? 'deploymentParentDisabled'
          : 'deploymentRejected';
  }
}
function frame(
  target: Target,
  state: State,
  props: ConnectionsProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const ui = paint(target, props.theme),
    text = copy(props.locale);
  const busy = !!state.pending || !!state.current;
  const entries = state.page?.instances ?? [];
  const current = state.confirmation;
  const history = state.historyOpen;
  const body = current
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
    : history
      ? [
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
        ]
      : entries.map((entry, at) =>
          ui.box(
            'deployment-instance-' + at,
            [
              ui.text(
                'deployment-path-' + at,
                compactLabel(entry.path.join(' / '), target === 'desktop' ? 100 : 36),
                15,
                props.theme.text,
                550,
              ),
              ui.text(
                'deployment-package-' + at,
                compactLabel(entry.package.id, target === 'desktop' ? 100 : 36),
                13,
                props.theme.muted,
              ),
              ui.box(
                'deployment-row-' + at,
                [
                  ui.text(
                    'deployment-state-' + at,
                    entry.parentDisabled
                      ? text.deploymentParentDisabled
                      : entry.disabled
                        ? text.connectionDisabled
                        : text.connectionEnabled,
                    13,
                    props.theme.muted,
                  ),
                  ui.button(
                    'deployment-toggle-' + at,
                    entry.disabled ? text.deploymentEnable : text.deploymentDisable,
                    busy || state.conflict || (entry.disabled && entry.parentDisabled),
                  ),
                ],
                { row: target === 'desktop', gap: 0, style: { wrap: true } },
              ),
            ],
            { gap: target === 'desktop' ? 4 : 0 },
          ),
        );
  const header = ui.box(
    'deployment-header',
    [
      ui.box(
        'deployment-title-row',
        [
          ui.button('deployment-back', text.back),
          ui.text('deployment-title', text.deploymentTitle, 22, props.theme.text, 600),
        ],
        { row: target === 'desktop', gap: target === 'desktop' ? 8 : 0 },
      ),
      ...(!current && !history
        ? [ui.text('deployment-help', text.deploymentHelp, 13, props.theme.muted)]
        : []),
      ...(state.notice
        ? [ui.text('deployment-notice', text[state.notice], 13, props.theme.muted)]
        : []),
      ...(state.pending
        ? [ui.text('deployment-loading', text.connecting, 13, props.theme.muted)]
        : []),
    ],
    { gap: target === 'desktop' ? 8 : 0, style: { shrink: 0 } },
  );
  const footer = ui.box(
    'deployment-controls',
    current
      ? [
          ui.button('deployment-confirm', text.confirmDefault, busy),
          ui.button('deployment-cancel', text.cancel, busy),
        ]
      : history
        ? [
            ui.button('deployment-history-back', text.back),
            ui.button('deployment-history-refresh', text.refreshDefaultOperations, !!state.pending),
            ...(state.historyNext !== null
              ? [ui.button('deployment-history-more', text.moreOperations, !!state.pending)]
              : []),
          ]
        : [
            ui.button('deployment-refresh', text.deploymentRefresh, busy),
            ...(state.previous.length
              ? [ui.button('deployment-previous', text.deploymentPrevious, busy)]
              : []),
            ...(state.page?.next ? [ui.button('deployment-next', text.deploymentMore, busy)] : []),
            ...(state.current?.reference
              ? [ui.button('deployment-check', text.check, !!state.pending)]
              : []),
            ui.button('deployment-history', text.deploymentHistory, !!state.pending),
          ],
    { row: target === 'desktop', gap: 0, style: { wrap: true, shrink: 0 } },
  );
  const out = output(
    state,
    ui.box(
      'deployment-panel',
      [
        header,
        ui.box(
          'deployment-body',
          body.length
            ? body
            : [ui.text('deployment-empty', text.deploymentEmpty, 14, props.theme.muted)],
          {
            scroll: 'manual',
            terminal: { grow: 1 },
            style: { width: fill, height: pixels(0), grow: 1, shrink: 1, minHeight: 0 },
            gap: target === 'desktop' ? 16 : 1,
          },
        ),
        footer,
      ],
      {
        style: { width: fill, height: fill },
        terminal: { grow: 1 },
        gap: target === 'desktop' ? 16 : 1,
      },
    ),
    events,
  );
  out.actions = actions;
  return out;
}
export function deployment(target: Target): Component<State, ConnectionsProps> {
  return {
    mount(props) {
      const state: State = {
        revision: 0,
        serial: 0,
        pending: null,
        page: null,
        after: null,
        previous: [],
        confirmation: null,
        current: null,
        notice: null,
        conflict: false,
        earlier: [],
        historyOpen: false,
        historyNext: null,
      };
      const actions: Action[] = [];
      page(state, actions, { kind: 'start' });
      return frame(target, state, props, actions);
    },
    update(state, props) {
      state.revision++;
      return frame(target, state, props);
    },
    event(state, input, props) {
      state.revision++;
      const actions: Action[] = [],
        events: Emission[] = [];
      if (input.kind === 'action') {
        const pending = state.pending;
        if (!pending || pending.key !== input.key) return frame(target, state, props);
        state.pending = null;
        const outcome = input.event as ActionOutcome,
          r = pending.request;
        if (r.kind === 'page') {
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
                entry.reference.method === 'setEnabled',
            )
            .map((entry) => ({ cursor: entry.cursor, reference: entry.reference, reply: null }));
        } else state.notice = 'historyUnavailable';
      } else if (input.kind === 'raw') {
        const key = input.key;
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
      return frame(target, state, props, actions, events);
    },
  };
}
export function activate() {
  return {
    Deployment: { kind: 'component', contract: deploymentContract, component: ['component'] },
  };
}
