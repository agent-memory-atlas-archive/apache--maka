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
  CallerIntent,
  RecoveryRef,
  OperationReply,
  CredentialStatus,
  CredentialVersionBasis,
  CredentialVaultQueryResult,
  ConnectionCredentialTarget,
  SettingsCredentialSaveResult,
  SettingsConnectionDetail,
  ConnectionVersionBasis,
  CredentialMutationResult,
  EditorObservation,
  EditorCommand,
} from '../../plugin-sdk/src/index.js';
import { credentialContract, type CredentialProps } from './contracts.js';
import { copy, type Copy } from './locale.js';
import { sameValue } from './value.js';
import { compactLabel, fill, output, paint, pixels, slot, type Target } from './view.js';

type Request =
  | { kind: 'status' }
  | { kind: 'reviewConnection'; basis: ConnectionVersionBasis; retried: boolean }
  | { kind: 'reviewStatus'; target: ConnectionCredentialTarget }
  | { kind: 'mutate' }
  | { kind: 'receipt'; reference: RecoveryRef }
  | { kind: 'history'; after: number }
  | { kind: 'earlier'; reference: RecoveryRef; cursor: number };
interface Current {
  kind: 'save' | 'delete';
  captured: EditorObservation | null;
  reference: RecoveryRef | null;
}
interface Earlier {
  cursor: number;
  reference: RecoveryRef;
  result: OperationReply | null;
}
interface State {
  revision: number;
  serial: number;
  target: ConnectionCredentialTarget;
  openedTarget: ConnectionCredentialTarget;
  reviewed: { target: ConnectionCredentialTarget; status: CredentialStatus } | null;
  status: CredentialStatus | null;
  statusFailed: boolean;
  observation: EditorObservation | null;
  command: EditorCommand | null;
  commandSequence: number;
  reviewPage: number;
  pending: Record<string, Request>;
  current: Current | null;
  finished: boolean;
  notice: keyof Copy | null;
  confirmation: 'leave' | 'remove' | 'rebase' | null;
  historyOpen: boolean;
  historyFailed: boolean;
  earlier: Earlier[];
  historyAfter: number;
  historyNext: number | null;
}
function request(
  state: State,
  actions: Action[],
  request: Request,
  binding: string,
  method: string,
  input: unknown,
  resources: Action['resources'] = {},
) {
  const key = 'credential-' + ++state.serial;
  state.pending[key] = request;
  actions.push({ key, binding, member: 0, method, input, resources });
}
function locator(state: State) {
  return {
    scope: 'connection' as const,
    connectionId: state.target.connectionId,
    kind: 'provider' as const,
  };
}
function basis(status: CredentialStatus | null): CredentialVersionBasis | null {
  return status?.configured && status.credentialId !== null && status.revision !== null
    ? { locator: status.locator, credentialId: status.credentialId, revision: status.revision }
    : null;
}
function busy(state: State) {
  return (
    !!state.current ||
    !!state.command ||
    Object.values(state.pending).some((r) =>
      ['status', 'reviewConnection', 'reviewStatus'].includes(r.kind),
    )
  );
}
function fresh(state: State, props: CredentialProps) {
  return sameValue(state.openedTarget, props.target) && !state.finished;
}
function canSave(state: State, props: CredentialProps) {
  return (
    !busy(state) &&
    !state.historyOpen &&
    !state.confirmation &&
    fresh(state, props) &&
    !!state.status &&
    !!state.observation?.snapshot.text.bytes &&
    !state.observation.snapshot.marked
  );
}
function readStatus(state: State, actions: Action[]) {
  if (busy(state) || state.finished || state.confirmation) return;
  // An observation used by an in-flight mutation is never overwritten by a late query.
  state.status = null;
  state.statusFailed = false;
  request(state, actions, { kind: 'status' }, 'settings', 'credentialStatus', {
    locator: locator(state),
  });
}
function review(state: State, props: CredentialProps, actions: Action[]) {
  if (busy(state) || state.confirmation || props.target.connectionId !== state.target.connectionId)
    return;
  state.reviewed = null;
  const original = { connectionId: state.target.connectionId, revision: state.target.revision };
  request(
    state,
    actions,
    { kind: 'reviewConnection', basis: original, retried: false },
    'settings',
    'connection',
    original,
  );
}

function save(state: State, props: CredentialProps, actions: Action[]) {
  if (!canSave(state, props)) return;
  const captured = state.observation!;
  if (captured.snapshot.text.bytes > 65536) {
    state.notice = 'keyTooLarge';
    return;
  }
  state.current = { kind: 'save', captured, reference: null };
  state.notice = null;
  request(
    state,
    actions,
    { kind: 'mutate' },
    'settings',
    'saveCredential',
    { expected: state.target, expectedCredential: basis(state.status) },
    {
      secret: {
        child: 'secret',
        method: 'snapshot',
        input: {
          kind: 'at_revision',
          owner: captured.snapshot.text.owner,
          revision: captured.revision,
        },
      },
    },
  );
}
function known(state: State, value: unknown, events: Emission[]) {
  const current = state.current!;
  if (current.kind === 'save') {
    const result = value as SettingsCredentialSaveResult;
    if (result.kind === 'saved') {
      if (
        result.connection.connectionId !== state.target.connectionId ||
        result.connection.slug !== state.target.slug ||
        !sameValue(result.connection.provider, state.target.provider)
      )
        return false;
      state.finished = true; // Saving changes connection revision; never guess its next basis.
      state.notice = 'keySaved';
      const captured = current.captured!;
      state.command = {
        owner: captured.snapshot.text.owner,
        sequence: ++state.commandSequence,
        basedOn: captured.revision,
        edit: { kind: 'reset', text: '' },
      };
      events.push({ name: 'changed', value: {} });
    } else if (result.kind === 'rejected') {
      state.notice = ['connection_changed', 'credential_changed', 'attempt_conflict'].includes(
        result.reason,
      )
        ? 'keyConflict'
        : 'keyRejected';
      if (state.notice === 'keyConflict') state.finished = true;
    } else return false;
  } else {
    const result = value as CredentialMutationResult;
    if (result.kind === 'committed') {
      if (!sameValue(result.status.locator, locator(state))) return false;
      state.status = result.status;
      state.notice = 'keyRemoved';
      events.push({ name: 'changed', value: {} });
    } else if (result.kind === 'rejected') state.notice = 'keyRejected';
    else if (
      ['connection_not_found', 'connection_stale', 'credential_stale'].includes(result.kind)
    ) {
      state.finished = true;
      state.notice = 'keyConflict';
    } else return false;
  }
  state.current = null;
  return true;
}
function receipt(state: State, reply: OperationReply, events: Emission[]) {
  const current = state.current;
  if (!current) return;
  if (current.reference && !sameValue(current.reference, reply.reference)) {
    state.notice = 'keyUnknown';
    return;
  }
  current.reference = reply.reference;
  if (
    reply.receipt.kind !== 'recorded' ||
    reply.receipt.outcome.kind !== 'returned' ||
    !known(state, reply.receipt.outcome.value, events)
  )
    state.notice = 'keyUnknown';
}
function history(state: State, actions: Action[], after: number) {
  if (Object.values(state.pending).some((r) => r.kind === 'history' || r.kind === 'earlier'))
    return;
  state.historyFailed = false;
  request(state, actions, { kind: 'history', after }, 'history', 'page', { after, limit: 4 });
}
function earlierLabel(entry: Earlier, text: Copy): string {
  const receipt = entry.result?.receipt;
  if (receipt?.kind !== 'recorded' || receipt.outcome.kind !== 'returned')
    return text.historyUnknown;
  const result = receipt.outcome.value as SettingsCredentialSaveResult | CredentialMutationResult;
  if (entry.reference.method === 'saveCredential' && result.kind === 'saved')
    return text.historySaved + ' ' + result.connection.slug;
  if (
    entry.reference.method === 'deleteCredential' &&
    result.kind === 'committed' &&
    result.status.locator.scope === 'connection'
  )
    return text.historyRemoved + ' ' + result.status.locator.connectionId;
  return text.historyRejected;
}

function frame(
  target: Target,
  state: State,
  props: CredentialProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const ui = paint(target, props.theme),
    text = copy(props.locale);
  const pendingStatus = Object.values(state.pending).some((r) => r.kind === 'status');
  const status = state.statusFailed
    ? text.keyUnavailable
    : pendingStatus || !state.status
      ? text.keyLoading
      : state.status.configured
        ? text.keyConfigured
        : text.keyAbsent;
  const canReview =
    !busy(state) && !state.confirmation && props.target.connectionId === state.target.connectionId;
  const form = ui.box(
    'credential-form',
    [
      ui.button('back', text.back),
      ui.text('heading', text.credentials, 22, props.theme.text, 600),
      ui.text(
        'connection',
        compactLabel(
          state.target.provider.name + ' · ' + state.target.slug,
          target === 'tui' ? 60 : 120,
        ),
        14,
        props.theme.muted,
      ),
      ui.text('status', status, 13, props.theme.muted),
      ...(state.statusFailed ? [ui.button('refresh-status', text.keyRefresh, busy(state))] : []),
      ...(!sameValue(state.openedTarget, props.target)
        ? [ui.text('target-changed', text.connectionChanged, 13, props.theme.muted)]
        : []),
      ui.box(
        'key-input',
        [
          slot(
            'secret',
            'secret',
            {
              appearance: props.theme.input,
              accessibilityLabel: text.credentials,
              readOnly: false,
              disabled: false,
              tabIndex: 0,
              placeholder: text.keyPlaceholder,
              command: state.command,
            },
            { changed: 'changed', submitted: 'submitted' },
          ),
        ],
        { style: { width: fill, height: pixels(44), shrink: 0 } },
      ),
      ...(target === 'desktop' ? [ui.text('help', text.keyHelp, 13, props.theme.muted)] : []),
      ui.box(
        'actions',
        [
          ui.button('save', text.saveKey, !canSave(state, props), true),
          ...(basis(state.status)
            ? [
                ui.button(
                  'remove',
                  text.removeKey,
                  busy(state) || !fresh(state, props) || !!state.confirmation,
                ),
              ]
            : []),
          ...(state.current?.reference
            ? [
                ui.button(
                  'check',
                  text.check,
                  Object.values(state.pending).some((r) => r.kind === 'receipt'),
                ),
              ]
            : []),
          ...(state.finished || !sameValue(state.openedTarget, props.target)
            ? [ui.button('review', text.reviewConnection, !canReview)]
            : []),
        ],
        { row: target === 'desktop', gap: target === 'desktop' ? 8 : 0 },
      ),
      ...(state.notice ? [ui.text('notice', text[state.notice], 13, props.theme.muted)] : []),
      ...(state.confirmation && state.confirmation !== 'rebase'
        ? [
            ui.text(
              'confirmation',
              state.confirmation === 'leave'
                ? state.current
                  ? text.leaveUnknown
                  : text.discardKey
                : state.confirmation === 'remove'
                  ? text.confirmRemoveKey
                  : text.reviewReady,
              14,
              props.theme.text,
            ),
            ui.box(
              'confirmation-actions',
              [
                ui.button(
                  'confirm',
                  state.confirmation === 'leave'
                    ? state.current
                      ? text.leave
                      : text.discard
                    : state.confirmation === 'remove'
                      ? text.confirmRemove
                      : text.useConnection,
                ),
                ui.button(
                  'cancel',
                  state.confirmation === 'leave' ? text.keepEditing : text.cancel,
                ),
              ],
              { row: target === 'desktop', gap: 0 },
            ),
          ]
        : []),
      ui.button('history', text.earlierOperations),
    ],
    {
      visible: !state.historyOpen && state.confirmation !== 'rebase',
      gap: target === 'desktop' ? 14 : 0,
      style: { width: fill, minWidth: 0 },
    },
  );
  // History has its own bounded pane. The hidden native input stays mounted;
  // reading another operation cannot adopt its basis or clear this draft.
  const earlier = ui.box(
    'earlier-operations',
    [
      ui.button('history-back', text.back),
      ui.text('history-heading', text.earlierOperations, 18, props.theme.text, 550),
      ui.text('history-help', text.earlierHelp, 13, props.theme.muted),
      ui.box(
        'history-actions',
        [
          ui.button(
            'history-refresh',
            text.keyRefresh,
            Object.values(state.pending).some((r) => r.kind === 'history' || r.kind === 'earlier'),
          ),
          ...(state.historyNext !== null
            ? [
                ui.button(
                  'history-more',
                  text.moreOperations,
                  Object.values(state.pending).some(
                    (r) => r.kind === 'history' || r.kind === 'earlier',
                  ),
                ),
              ]
            : []),
        ],
        { row: target === 'desktop', gap: 0 },
      ),
      ...(state.historyFailed
        ? [ui.text('history-error', text.historyUnavailable, 13, props.theme.muted)]
        : []),
      ...state.earlier.flatMap((entry, at) => [
        ui.text(
          'earlier-label-' + at,
          compactLabel(earlierLabel(entry, text), target === 'tui' ? 60 : 120),
          13,
          props.theme.muted,
        ),
        ui.button(
          'earlier-' + at,
          text.check,
          Object.values(state.pending).some(
            (r) => r.kind === 'earlier' && r.cursor === entry.cursor,
          ),
        ),
      ]),
    ],
    { visible: state.historyOpen, gap: target === 'desktop' ? 12 : 0 },
  );

  const reviewed = state.reviewed?.target;
  const details = reviewed
    ? [
        ...(typeof reviewed.configuration.baseUrl === 'string'
          ? [reviewed.configuration.baseUrl]
          : []),
        JSON.stringify(reviewed.provider),
        reviewed.slug,
        JSON.stringify(reviewed.configuration),
      ].flatMap((value) => {
        const characters = [...value];
        const rows: string[] = [];
        for (let at = 0; at < characters.length; at += 36)
          rows.push(characters.slice(at, at + 36).join(''));
        return rows;
      })
    : [];
  const pageSize = target === 'tui' ? 8 : 24;
  const pages = Math.max(1, Math.ceil(details.length / pageSize));
  const page = Math.min(state.reviewPage, pages - 1);
  const reviewPane = ui.box(
    'review-target',
    [
      ui.text('review-heading', text.reviewConnection, 20, props.theme.text, 550),
      ui.text('review-question', text.reviewReady, 14, props.theme.text),
      ui.box(
        'review-actions',
        [
          ui.button('review-confirm', text.useConnection, !state.reviewed || busy(state)),
          ui.button('review-cancel', text.cancel),
          ...(page > 0 ? [ui.button('review-previous', text.previousModels)] : []),
          ...(page + 1 < pages ? [ui.button('review-next', text.moreTarget)] : []),
        ],
        { row: target === 'desktop', gap: 0 },
      ),
      ui.text('review-page', page + 1 + ' / ' + pages, 13, props.theme.muted),
      ...details
        .slice(page * pageSize, (page + 1) * pageSize)
        .map((line, at) => ui.text('target-line-' + at, line, 14, props.theme.text)),
    ],
    { visible: state.confirmation === 'rebase', gap: target === 'desktop' ? 12 : 0 },
  );
  const view = ui.box('credential', [form, earlier, reviewPane], {
    gap: 0,
    padding: target === 'desktop' ? 24 : 0,
    scroll: true,
    style: { width: fill, height: fill, maxWidth: 680, minWidth: 0, minHeight: 0 },
  });
  const out = output(state, view, events);
  out.actions = actions;
  return out;
}

export function credential(target: Target): Component<State, CredentialProps> {
  return {
    mount(props) {
      const state: State = {
        revision: 0,
        serial: 0,
        target: props.target,
        openedTarget: props.target,
        reviewed: null,
        status: null,
        statusFailed: false,
        observation: null,
        command: null,
        commandSequence: 0,
        reviewPage: 0,
        pending: {},
        current: null,
        finished: false,
        notice: null,
        confirmation: null,
        historyOpen: false,
        historyFailed: false,
        earlier: [],
        historyAfter: 0,
        historyNext: null,
      };
      const actions: Action[] = [];
      readStatus(state, actions);
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
        const r = state.pending[input.key];
        if (!r) return frame(target, state, props);
        delete state.pending[input.key];
        const result = input.event as ActionOutcome;
        if (r.kind === 'mutate') {
          if (result.status === 'operation') receipt(state, result.reply, events);
          else if (result.status === 'not_accepted') {
            state.current = null;
            state.notice = 'keyRejected';
          } else state.notice = 'keyUnknown';
        } else if (r.kind === 'receipt') {
          if (
            result.status === 'succeeded' &&
            sameValue((result.value as OperationReply).reference, r.reference)
          )
            receipt(state, result.value as OperationReply, events);
          else state.notice = 'keyUnknown';
        } else if (r.kind === 'reviewConnection') {
          const detail =
            result.status === 'succeeded' ? (result.value as SettingsConnectionDetail) : null;
          if (
            detail?.kind === 'stale' &&
            detail.actual &&
            detail.actual.connectionId === state.target.connectionId &&
            !r.retried
          )
            request(
              state,
              actions,
              { kind: 'reviewConnection', basis: detail.actual, retried: true },
              'settings',
              'connection',
              detail.actual,
            );
          else if (
            detail?.kind === 'ready' &&
            detail.target.connectionId === r.basis.connectionId &&
            detail.target.revision === r.basis.revision
          )
            request(
              state,
              actions,
              { kind: 'reviewStatus', target: detail.target },
              'settings',
              'credentialStatus',
              { locator: locator(state) },
            );
          else state.notice = 'keyConflict';
        } else if (r.kind === 'reviewStatus') {
          const value =
            result.status === 'succeeded' ? (result.value as CredentialVaultQueryResult) : null;
          if (value?.kind === 'status' && sameValue(value.status.locator, locator(state))) {
            state.reviewed = { target: r.target, status: value.status };
            state.reviewPage = 0;
            state.confirmation = 'rebase';
          } else state.notice = 'keyUnavailable';
        } else if (r.kind === 'status') {
          const value =
            result.status === 'succeeded' ? (result.value as CredentialVaultQueryResult) : null;
          if (value?.kind === 'status' && sameValue(value.status.locator, locator(state)))
            state.status = value.status;
          else state.statusFailed = true;
        } else if (r.kind === 'history') {
          if (result.status !== 'succeeded') state.historyFailed = true;
          else {
            const page = result.value as CallerIntent[];
            state.historyAfter = r.after;
            state.historyNext = page.length === 4 ? page[page.length - 1].cursor : null;
            state.earlier = page
              .filter(
                (entry) =>
                  entry.reference.contract.name === 'maka.settings.Settings' &&
                  ['saveCredential', 'deleteCredential'].includes(entry.reference.method),
              )
              .map((entry) => ({ cursor: entry.cursor, reference: entry.reference, result: null }));
          }
        } else if (r.kind === 'earlier') {
          const entry = state.earlier.find(
            (entry) => entry.cursor === r.cursor && sameValue(entry.reference, r.reference),
          );
          if (
            entry &&
            result.status === 'succeeded' &&
            sameValue((result.value as OperationReply).reference, r.reference)
          )
            entry.result = result.value as OperationReply;
          // Earlier operation facts never become current intent/target/status or reset commands.
        }
      } else if (input.kind === 'component' && input.child === 'secret') {
        if (input.name === 'changed') {
          state.observation = input.event as EditorObservation;
          const r = state.observation.receipt;
          if (
            state.command &&
            r?.owner === state.command.owner &&
            r.sequence === state.command.sequence
          ) {
            if (r.outcome.status === 'conflict') state.notice = 'draftKept';
            state.command = null;
          }
        } else if (input.name === 'submitted') save(state, props, actions);
      } else if (input.kind === 'raw') {
        if (input.key === 'save') save(state, props, actions);
        else if (input.key === 'refresh-status') readStatus(state, actions);
        else if (input.key === 'review') review(state, props, actions);
        else if (input.key === 'review-next' && state.confirmation === 'rebase') state.reviewPage++;
        else if (input.key === 'review-previous' && state.confirmation === 'rebase')
          state.reviewPage = Math.max(0, state.reviewPage - 1);
        else if (input.key === 'back') {
          if (
            state.observation?.snapshot.text.bytes ||
            state.observation?.snapshot.marked ||
            state.command ||
            state.current
          )
            state.confirmation = 'leave';
          else events.push({ name: 'closed', value: {} });
        } else if (input.key === 'cancel' || input.key === 'review-cancel') {
          state.confirmation = null;
          state.reviewed = null;
        } else if (
          input.key === 'remove' &&
          !busy(state) &&
          fresh(state, props) &&
          basis(state.status)
        )
          state.confirmation = 'remove';
        else if (input.key === 'review-confirm' && state.confirmation === 'rebase') {
          if (
            state.reviewed &&
            !busy(state) &&
            props.target.connectionId === state.target.connectionId
          ) {
            state.target = state.reviewed.target;
            state.openedTarget = props.target;
            state.status = state.reviewed.status;
            state.finished = false;
            state.notice = null;
          }
          state.reviewed = null;
          state.confirmation = null;
        } else if (input.key === 'confirm' && state.confirmation === 'leave')
          events.push({ name: 'closed', value: {} });
        else if (input.key === 'confirm' && state.confirmation === 'remove') {
          state.confirmation = null;
          const expected = basis(state.status);
          if (expected && !busy(state) && fresh(state, props)) {
            state.current = { kind: 'delete', captured: null, reference: null };
            state.notice = null;
            request(state, actions, { kind: 'mutate' }, 'settings', 'deleteCredential', {
              expected,
              expectedConnection: state.target,
            });
          }
        } else if (
          input.key === 'check' &&
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
        else if (input.key === 'history') {
          state.historyOpen = !state.historyOpen;
          if (state.historyOpen) history(state, actions, state.historyAfter);
        } else if (input.key === 'history-back') state.historyOpen = false;
        else if (state.historyOpen && input.key === 'history-refresh')
          history(state, actions, state.historyAfter);
        else if (state.historyOpen && input.key === 'history-more' && state.historyNext !== null)
          history(state, actions, state.historyNext);
        else if (state.historyOpen && /^earlier-(0|[1-9][0-9]*)$/.test(input.key)) {
          const entry = state.earlier[Number(input.key.slice(8))];
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
        }
      }
      return frame(target, state, props, actions, events);
    },
  };
}
export function activate() {
  return {
    Credential: { kind: 'component', contract: credentialContract, component: ['component'] },
  };
}
