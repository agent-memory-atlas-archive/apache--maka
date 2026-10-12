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

import type { AppsConfiguration } from '../../shared/contracts.js';

import type {
  Action,
  ActionOutcome,
  BindingCandidatesPage,
  CallerIntent,
  ComponentInput,
  EditorObservation,
  Emission,
  FileObservation,
  OperationReply,
  PackageSettingsPage,
  TextChunk,
} from '../../../packages/plugin-sdk/src/index.js';
import { type AppsProps } from '../../shared/contracts.js';
import { settingOptions } from '../../shared/instance-settings.js';
import { sameValue } from '../../shared/value.js';
import { type Node, type Presentation } from '../../shared/view.js';
import {
  bindings,
  capture,
  catalog,
  change,
  choose,
  edit,
  entries,
  history,
  initialize,
  locked,
  mutate,
  request,
  reset,
  settings,
  settle,
  value,
} from './operations.js';
import { type Mutation, type State } from './state.js';
import { frame } from './view.js';

export function handleEvent<N extends Node>(
  presentation: Presentation<N>,
  configuration: AppsConfiguration,
  state: State,
  input: ComponentInput,
  props: AppsProps,
) {
  state.revision++;
  const actions: Action[] = [],
    events: Emission[] = [];
  if (input.kind === 'component' && input.child === 'apps-list' && input.name === 'input')
    input = input.event as ComponentInput;
  if (input.kind === 'component') {
    const name =
      input.child === 'apps-name' ? 'name' : input.child === 'apps-setting' ? 'setting' : null;
    if (name && input.name === 'changed') {
      const f = state.fields[name];
      f.observation = input.event as EditorObservation;
      const receipt = f.observation.receipt;
      if (
        f.command &&
        receipt?.owner === f.command.owner &&
        receipt.sequence === f.command.sequence
      ) {
        f.ready = receipt.outcome.status === 'applied';
        f.command = null;
        f.reset = null;
        if (!f.ready) state.notice = 'appsUnavailable';
      } else initialize(f);
    } else if (name === 'name' && input.name === 'submitted')
      input = { kind: 'raw', key: 'review', event: {} } as ComponentInput;
    else if (input.child === 'apps-file' && input.name === 'changed')
      state.file = input.event as FileObservation;
    else if (input.name === 'closed' && input.child.startsWith('app-')) {
      const id = input.child.slice(4);
      state.opened = state.opened.filter((o) => o !== id);
      if (state.selected === id) {
        state.selected = null;
        state.route = 'list';
      }
    }
  }
  if (input.kind === 'action' && input.key === state.pending?.key) {
    const r = state.pending.request,
      result = input.event as ActionOutcome;
    state.pending = null;
    if (r.kind === 'history') {
      if (result.status !== 'succeeded') {
        state.restoring = false;
        state.historyFailed = true;
        state.notice = 'historyUnavailable';
      } else {
        const items = result.value as CallerIntent[];
        for (const item of items) {
          const recovery = item.recovery as { kind?: string; input?: Mutation } | null;
          if (
            item.reference.contract.name === 'maka.application.DeploymentManager' &&
            ['install', 'editComposition'].includes(item.reference.method) &&
            recovery?.kind === 'application-apps/v1' &&
            recovery.input
          ) {
            state.original = item.reference;
            state.current = { input: recovery.input, reference: item.reference };
          }
        }
        const last = items.at(-1);
        if (last && last.cursor > r.after) history(state, actions, last.cursor);
        else if (last) {
          state.restoring = false;
          state.historyFailed = true;
          state.notice = 'historyUnavailable';
        } else {
          state.restoring = false;
          if (state.original)
            request(state, actions, { kind: 'receipt' }, 'receipts', 'read', state.original);
          else catalog(state, actions, configuration);
        }
      }
    } else if (r.kind === 'mutation') {
      if (result.status === 'operation') settle(state, result.reply, actions, configuration);
      else if (result.status === 'not_accepted') {
        state.current = null;
        state.notice = 'appsUnavailable';
      } else state.notice = 'appsUnknown';
    } else if (r.kind === 'receipt') {
      if (result.status === 'succeeded')
        settle(state, result.value as OperationReply, actions, configuration);
      else state.notice = 'appsUnknown';
    } else if (r.kind === 'catalog' || r.kind === 'bindings') {
      if (result.status !== 'succeeded') state.notice = 'appsUnavailable';
      else {
        const page = result.value as BindingCandidatesPage;
        if (r.input.source && !sameValue(page.source, r.input.source)) state.notice = 'appsChanged';
        else if (r.kind === 'catalog') state.catalog = page;
        else if (state.draft) state.draft.choices = page;
      }
    } else if (r.kind === 'settings') {
      if (result.status !== 'succeeded') state.notice = 'appsUnavailable';
      else if (
        state.draft &&
        sameValue(state.draft.path, r.path) &&
        state.draft.after === r.after
      ) {
        const page = result.value as PackageSettingsPage;
        if (
          !sameValue(page.source, state.catalog?.source) ||
          state.draft.candidate.kind !== 'package' ||
          !sameValue(page.package, state.draft.candidate.package)
        )
          state.notice = 'appsChanged';
        else state.draft.page = page;
      }
    } else if (r.kind === 'field') {
      const chunk = result.status === 'succeeded' ? (result.value as TextChunk) : null;
      if (
        !chunk ||
        !sameValue(chunk.metadata, r.observation.snapshot.text) ||
        new TextEncoder().encode(chunk.text).length !== r.observation.snapshot.text.bytes
      )
        state.notice = 'appsUnavailable';
      else if (state.draft) {
        if (r.name === 'name') {
          const label = chunk.text.trim();
          if (!label || [...label].length > 256 || /[\u0000-\u001f\u007f-\u009f]/u.test(label))
            state.notice = 'appsInvalid';
          else {
            state.draft.label = label;
            edit(state, configuration, label);
          }
        } else if (state.draft.setting) {
          const { descriptor, path } = state.draft.setting;
          const value = descriptor.kind === 'number' ? chunk.text.trim() : chunk.text;
          if (
            [...value].length > 4096 ||
            (descriptor.kind === 'number' &&
              (value.length > 128 || !/^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$/.test(value)))
          )
            state.notice = 'appsInvalid';
          else {
            change(state.draft, path, {
              kind: descriptor.kind === 'number' ? 'number' : 'text',
              text: value,
            });
            state.draft.setting = null;
            settings(state, actions);
          }
        }
      }
    }
  } else if (input.kind === 'raw' && props.open) {
    const key = input.key,
      d = state.draft;
    if (key === 'back' && !state.confirm && !d?.setting) {
      if (state.current || state.restoring || state.route === 'list')
        events.push({ name: 'closed', value: {} });
      else if (!state.pending) {
        if (state.route === 'bindings') state.route = 'configure';
        else {
          state.route = 'list';
          state.draft = null;
          state.selected = null;
        }
      }
    } else if (
      ['receipt', 'confirm-receipt'].includes(key) &&
      state.current?.reference &&
      !state.pending
    )
      request(state, actions, { kind: 'receipt' }, 'receipts', 'read', state.current.reference);
    else if (key === 'history-retry' && state.historyFailed && !state.pending)
      history(state, actions);
    else if (key === 'confirm-cancel' && !state.current && !state.pending) state.confirm = null;
    else if (!locked(state, props) && (!state.confirm || key === 'confirm')) {
      if (key === 'confirm') mutate(state, actions);
      else if (key === 'setting-cancel' && d) d.setting = null;
      else if (key === 'setting-apply' && d?.setting) capture(state, actions, 'setting');
      else if (!d?.setting) {
        if (key === 'refresh') {
          state.notice = null;
          state.diagnostic = null;
          catalog(state, actions, configuration);
        } else if (key === 'manage') events.push({ name: 'manage', value: {} });
        else if (key === 'add') {
          state.route = 'choose';
          state.notice = null;
          catalog(state, actions, configuration);
        } else if (key === 'file') {
          state.route = 'file';
          state.notice = null;
          if (!state.catalog) catalog(state, actions, configuration);
        } else if (
          key === 'install' &&
          state.catalog &&
          state.file?.phase === 'ready' &&
          state.file.file
        )
          state.confirm = { kind: 'install', source: state.catalog.source, file: state.file };
        else if (key.startsWith('open-')) {
          const id = key.slice(5);
          if (configuration.entries.some((e) => e.id === id)) {
            state.selected = id;
            state.route = 'app';
            if (!state.opened.includes(id)) state.opened.push(id);
          }
        } else if (key.startsWith('close-')) {
          const id = key === 'close-current' ? state.selected : key.slice(6);
          state.opened = state.opened.filter((o) => o !== id);
          if (state.selected === id) {
            state.selected = null;
            state.route = 'list';
          }
        } else if (
          key.startsWith('remove-') &&
          state.catalog &&
          sameValue(entries(state), configuration.entries)
        ) {
          const index = Number(key.slice(7));
          if (configuration.entries[index])
            state.confirm = {
              kind: 'edit',
              outcome: 'remove',
              input: {
                expectedRevision: state.catalog.source.revision,
                edits: [
                  {
                    kind: 'update',
                    name: configuration.root,
                    configuration: {
                      entries: entries(state).filter((_, at) => at !== index),
                    },
                    bindings: {
                      pages: state.catalog.current.filter((_, at) => at !== index),
                    },
                  },
                ],
              },
            };
        } else if (key.startsWith('candidate-') && state.route === 'choose') {
          const candidate = state.catalog?.candidates[Number(key.slice(10))];
          if (candidate) choose(state, candidate, actions);
        } else if (key === 'catalog-more' && state.catalog?.next)
          catalog(state, actions, configuration, state.catalog.source, state.catalog.next);
        else if (key === 'review' && state.route === 'configure') capture(state, actions, 'name');
        else if (key.startsWith('setting-choice-') && d?.page) {
          const f = d.page.fields[Number(key.slice(15))];
          const option = f && settingOptions(f)[Number((input.event as { key?: string }).key)];
          if (option) {
            change(d, [...d.path, f.key], option);
            settings(state, actions);
          }
        } else if (key.startsWith('setting-') && d?.page) {
          const f = d.page.fields[Number(key.slice(8))];
          if (f?.editable) {
            if (f.kind === 'object' || f.kind === 'array') {
              d.path = [...d.path, f.key];
              d.after = null;
              d.previous = [];
              settings(state, actions);
            } else if (settingOptions(f).length) {
              const options = settingOptions(f),
                index = options.findIndex((v) => sameValue(v, value(d, f)));
              change(d, [...d.path, f.key], options[(index + 1) % options.length]);
              settings(state, actions);
            } else {
              d.setting = { descriptor: f, path: [...d.path, f.key] };
              const v = value(d, f);
              reset(state, 'setting', v?.kind === 'text' || v?.kind === 'number' ? v.text : '');
            }
          }
        } else if (key === 'settings-up' && d?.path.length) {
          d.path = d.path.slice(0, -1);
          d.after = null;
          d.previous = [];
          settings(state, actions);
        } else if (key === 'settings-next' && d?.page?.next) {
          d.previous.push(d.after);
          d.after = d.page.next;
          settings(state, actions);
        } else if (key === 'settings-previous' && d?.previous.length) {
          d.after = d.previous.pop()!;
          settings(state, actions);
        } else if (key.startsWith('choose-binding-') && d?.page) {
          const i = d.page.imports[Number(key.slice(15))];
          if (i) {
            d.choosing = i;
            d.choices = null;
            state.route = 'bindings';
            bindings(state, actions);
          }
        } else if (key.startsWith('binding-') && d?.choosing && d.choices) {
          const candidate = d.choices.candidates[Number(key.slice(8))];
          if (candidate?.kind === 'bound') {
            const i = d.choosing,
              current = d.bindings[i.name] ?? [],
              has = current.some((t) => sameValue(t, candidate.target));
            d.bindings[i.name] = has
              ? current.filter((t) => !sameValue(t, candidate.target))
              : i.cardinality.kind === 'many'
                ? [...current, candidate.target]
                : [candidate.target];
          }
        } else if (key === 'bindings-clear' && d?.choosing) d.bindings[d.choosing.name] = [];
        else if (key === 'bindings-more' && d?.choices?.next)
          bindings(state, actions, d.choices.next);
        else if (key === 'bindings-done') state.route = 'configure';
      }
    }
  }
  return frame(presentation, configuration, state, props, actions, events);
}
