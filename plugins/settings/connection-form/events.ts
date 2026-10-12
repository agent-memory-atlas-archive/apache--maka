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
  ComponentInput,
  EditorObservation,
  Emission,
  OperationReply,
  SettingsConnectionDetail,
  SettingsConnectionsPage,
  SettingsProvidersPage,
  TextChunk,
} from '../../../packages/plugin-sdk/src/index.js';
import { type ConnectionFormProps } from '../../shared/contracts.js';
import { sameValue } from '../../shared/value.js';
import { type Node, type Presentation } from '../../shared/view.js';
import {
  busy,
  canRemove,
  fresh,
  history,
  mutate,
  prefill,
  remove,
  request,
  sameConnection,
  save,
  settle,
  utf8Bytes,
} from './operations.js';
import { formPanes, type FieldName, type SettingsValidation, type State } from './state.js';
import { frame } from './view.js';

export function handleEvent<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  input: ComponentInput,
  props: ConnectionFormProps,
  validation: SettingsValidation,
) {
  const target = presentation.target;
  if (input.kind === 'component' && input.child === 'provider-list' && input.name === 'input')
    input = input.event as ComponentInput;
  state.revision++;
  const actions: Action[] = [],
    events: Emission[] = [];
  if (input.kind === 'component' && input.child.startsWith('field-')) {
    const name = input.child.slice(6) as FieldName,
      field = state.fields[name];
    if (!field) return frame(presentation, state, props);
    if (input.name === 'changed') {
      field.observation = input.event as EditorObservation;
      const r = field.observation.receipt;
      if (
        field.command &&
        r?.owner === field.command.owner &&
        r.sequence === field.command.sequence
      ) {
        field.failed = r.outcome.status !== 'applied';
        field.initialized = !field.failed;
        field.command = null;
      } else if (field.enabled && !field.initialized && !field.failed && !field.command)
        prefill(state, name);
    } else if (input.name === 'submitted') save(state, props, actions, validation);
  } else if (input.kind === 'action') {
    const r = state.pending[input.key];
    if (!r) return frame(presentation, state, props);
    delete state.pending[input.key];
    const outcome = input.event as ActionOutcome;
    if (r.kind === 'providers') {
      if (outcome.status !== 'succeeded') state.providersFailed = true;
      else {
        const page = outcome.value as SettingsProvidersPage;
        state.providers = page.providers;
        state.providerNext = page.next;
        state.providerActive = 0;
        if (target === 'tui' && state.providers.length)
          state.providerFocus = {
            sequence: (state.providerFocus?.sequence ?? 0) + 1,
            target: { kind: 'focus_item', key: 'provider-0' },
          };
      }
    } else if (r.kind === 'read') {
      const current = state.current;
      if (current?.attempt === r.attempt && current.remaining.includes(r.field)) {
        const chunk = outcome.status === 'succeeded' ? (outcome.value as TextChunk) : null;
        if (
          !chunk ||
          !sameValue(chunk.metadata, r.captured.snapshot.text) ||
          utf8Bytes(chunk.text) !== r.captured.snapshot.text.bytes
        ) {
          state.current = null;
          state.notice = 'connectionDraftKept';
        } else {
          current.values[r.field] = chunk.text;
          current.remaining = current.remaining.filter((name) => name !== r.field);
          if (!current.remaining.length) mutate(state, actions, validation);
        }
      }
    } else if (r.kind === 'mutate') {
      if (outcome.status === 'operation') settle(state, outcome.reply, events);
      else if (outcome.status === 'not_accepted') {
        state.current = null;
        state.notice = 'connectionRejected';
      } else state.notice = 'connectionUnknown';
    } else if (r.kind === 'receipt') {
      if (
        outcome.status === 'succeeded' &&
        sameValue((outcome.value as OperationReply).reference, r.reference)
      )
        settle(state, outcome.value as OperationReply, events);
      else state.notice = 'connectionUnknown';
    } else if (r.kind === 'review') {
      const detail =
        outcome.status === 'succeeded' ? (outcome.value as SettingsConnectionDetail) : null;
      if (
        detail?.kind === 'stale' &&
        detail.actual &&
        !r.retried &&
        detail.actual.connectionId === state.baseline?.target.connectionId
      )
        request(
          state,
          actions,
          { kind: 'review', basis: detail.actual, retried: true },
          'settings',
          'connection',
          detail.actual,
        );
      else if (
        detail?.kind === 'ready' &&
        detail.target.connectionId === r.basis.connectionId &&
        detail.target.revision === r.basis.revision
      ) {
        state.reviewed = detail;
        state.reviewPage = 0;
        state.confirmation = 'rebase';
      } else state.notice = 'connectionConflict';
    } else if (r.kind === 'reviewCatalog') {
      const page =
        outcome.status === 'succeeded' ? (outcome.value as SettingsConnectionsPage) : null;
      if (page?.kind === 'ready') {
        state.reviewedCatalog = page.revision;
        state.reviewPage = 0;
        state.confirmation = 'rebase';
      } else state.notice = 'connectionConflict';
    } else if (r.kind === 'history') {
      if (outcome.status !== 'succeeded') state.historyFailed = true;
      else {
        const page = outcome.value as CallerIntent[];
        state.historyNext = page.length === 4 ? page[page.length - 1].cursor : null;
        state.earlier = page
          .filter(
            (entry) =>
              entry.reference.contract.name === 'maka.settings.Settings' &&
              ['create', 'update', 'remove'].includes(entry.reference.method),
          )
          .map((entry) => ({
            cursor: entry.cursor,
            reference: entry.reference,
            result: null,
          }));
      }
    } else if (
      r.kind === 'earlier' &&
      outcome.status === 'succeeded' &&
      sameValue((outcome.value as OperationReply).reference, r.reference)
    ) {
      const entry = state.earlier.find(
        (entry) => entry.cursor === r.cursor && sameValue(entry.reference, r.reference),
      );
      if (entry) entry.result = outcome.value as OperationReply;
    }
  } else if (input.kind === 'raw') {
    const key =
      input.key === 'form-auxiliary'
        ? state.historyOpen
          ? 'form-history-back'
          : 'form-cancel'
        : input.key;
    if (key === 'form-save') save(state, props, actions, validation);
    else if (
      key === 'form-remove' &&
      canRemove(state, props) &&
      !state.confirmation &&
      !state.historyOpen
    ) {
      state.confirmation = 'remove';
      state.reviewPage = 0;
    } else if (key === 'form-confirm-remove') remove(state, props, actions);
    else if (key === 'form-back') {
      if (!state.provider && !state.baseline && !state.current)
        events.push({ name: 'closed', value: {} });
      else state.confirmation = 'leave';
    } else if (key === 'form-cancel') {
      state.confirmation = null;
      state.reviewed = null;
      state.reviewedCatalog = null;
    } else if (key === 'form-discard' && state.confirmation === 'leave')
      events.push({ name: 'closed', value: {} });
    else if (
      key === 'form-check' &&
      state.current?.reference &&
      !Object.values(state.pending).some((r) => ['receipt', 'mutate'].includes(r.kind))
    )
      request(
        state,
        actions,
        { kind: 'receipt', reference: state.current.reference },
        'receipts',
        'read',
        state.current.reference,
      );
    else if (key === 'form-review' && !busy(state) && sameConnection(state, props)) {
      if (state.baseline) {
        const basis = {
          connectionId: state.baseline.target.connectionId,
          revision: state.baseline.target.revision,
        };
        request(
          state,
          actions,
          { kind: 'review', basis, retried: false },
          'settings',
          'connection',
          basis,
        );
      } else
        request(state, actions, { kind: 'reviewCatalog' }, 'settings', 'connections', {
          kind: 'start',
        });
    } else if (key === 'review-next' && ['rebase', 'remove'].includes(state.confirmation ?? ''))
      state.reviewPage++;
    else if (key === 'review-prev' && ['rebase', 'remove'].includes(state.confirmation ?? ''))
      state.reviewPage = Math.max(0, state.reviewPage - 1);
    else if (
      key === 'form-rebase' &&
      state.confirmation === 'rebase' &&
      (state.reviewed || state.reviewedCatalog !== null) &&
      !busy(state) &&
      sameConnection(state, props)
    ) {
      // Explicitly accept complete replacement of every opted-in field, including remote config keys.
      if (state.reviewed) {
        state.baseline = state.reviewed;
        if (!state.enabledEdited) state.enabled = state.reviewed.enabled;
      } else state.catalogRevision = state.reviewedCatalog!;
      state.opened = props.connection;
      state.reviewed = null;
      state.reviewedCatalog = null;
      state.confirmation = null;
      state.conflict = false;
      state.notice = null;
    } else if (key === 'form-history') {
      state.historyOpen = true;
      history(state, actions, 0);
    } else if (key === 'form-history-back') {
      state.historyOpen = false;
      if (state.providerFocus)
        state.providerFocus = {
          sequence: state.providerFocus.sequence + 1,
          target: { kind: 'focus_item', key: 'provider-' + state.providerActive },
        };
    } else if (key === 'form-history-refresh') history(state, actions, 0);
    else if (key === 'form-history-more' && state.historyNext !== null)
      history(state, actions, state.historyNext);
    else if (/^form-earlier-(0|[1-9][0-9]*)$/.test(key) && state.historyOpen) {
      const entry = state.earlier[Number(key.slice(13))];
      if (
        entry &&
        !Object.values(state.pending).some((r) => r.kind === 'earlier' && r.cursor === entry.cursor)
      )
        request(
          state,
          actions,
          { kind: 'earlier', reference: entry.reference, cursor: entry.cursor },
          'receipts',
          'read',
          entry.reference,
        );
    } else if (!busy(state) && !state.confirmation && !state.historyOpen && fresh(state, props)) {
      if (key === 'connection-enabled') {
        state.enabled = !state.enabled;
        state.enabledEdited = true;
      } else if (key === 'form-advanced') {
        state.advanced = !state.advanced;
        state.pane = 0;
      } else if (key === 'field-next')
        state.pane = Math.min(state.pane + 1, formPanes(state).length - 1);
      else if (key === 'field-previous') state.pane = Math.max(0, state.pane - 1);
      else if (key === 'provider-refresh' && !state.provider && !state.baseline)
        request(state, actions, { kind: 'providers' }, 'settings', 'providers', {
          offset: 0,
        });
      else if (key === 'provider-more' && !state.provider && !state.baseline) {
        if (state.providerNext !== null)
          request(state, actions, { kind: 'providers' }, 'settings', 'providers', {
            offset: state.providerNext,
          });
      } else if (/^provider-(0|[1-9][0-9]*)$/.test(key) && !state.provider && !state.baseline) {
        const provider = state.providers[Number(key.slice(9))];
        if (provider) {
          state.providerActive = Number(key.slice(9));
          if (target === 'tui' && (input.event as { kind?: string }).kind !== 'submit')
            return frame(presentation, state, props, actions, events);
          state.provider = provider;
          for (const name of ['name', 'slug', 'endpoint', 'models'] as FieldName[]) {
            state.fields[name].enabled = true;
            prefill(state, name);
          }
        }
      } else if (key.startsWith('edit-')) {
        const name = key.slice(5) as FieldName,
          f = state.fields[name];
        if (
          f &&
          (name !== 'slug' || !state.baseline) &&
          !(name === 'configuration' && state.fields.endpoint.enabled) &&
          !(name === 'endpoint' && state.fields.configuration.enabled)
        ) {
          f.enabled = !f.enabled;
          if (f.enabled && !f.initialized) prefill(state, name);
        }
      } else if (key.startsWith('empty-')) {
        const name = key.slice(6) as FieldName;
        if (state.fields[name]?.enabled && state.fields[name].failed) prefill(state, name, true);
      }
    }
  }
  return frame(presentation, state, props, actions, events);
}
