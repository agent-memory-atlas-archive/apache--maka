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
  AgentDeliveryPage,
  AgentScheduleAlerts,
  AgentSessionPage,
  ComponentInput,
  ComponentValues,
  Emission,
} from '../../../packages/plugin-sdk/src/index.js';
import { appearanceEvent, effectiveLocale } from '../../shared/appearance-editor.js';
import { observeBookmark, retryBookmark } from '../../shared/bookmark.js';
import type { AgentScreenProps, Theme } from '../../shared/contracts.js';
import { settingsCategory } from '../../shared/settings.js';
import { resolveLocale } from '../../shared/locale.js';
import { type Node, type Presentation } from '../../shared/view.js';

import { follow, select } from './actions.js';
import { handleComponent } from './component-events.js';
import { frame } from './frame.js';
import { handleRaw } from './raw-events.js';
import { delivery, queryFailed, querySucceeded, receipt } from './results.js';
import { navigationBookmark, navigationKey, type Navigation, type State } from './state.js';

export function handleEvent<N extends Node>(
  presentation: Presentation<N>,
  configuredLocale: string,
  previous: State,
  input: ComponentInput,
  props: AgentScreenProps,
  values: ComponentValues,
) {
  const target = presentation.target;

  // Callbacks receive detached committed state. Mutations here belong only to
  // the candidate frame and publish with its intents in one UI transaction.
  const state = previous;
  state.embedding = props;
  if (target === 'tui') {
    if (input.kind === 'raw' && input.key === 'navigation-checkpoint-retry')
      retryBookmark(state.bookmark);
    const restored = observeBookmark(
      state.bookmark,
      input,
      navigationKey(state),
      navigationBookmark(state),
      (value) => {
        const item = value as Navigation | null;
        return item &&
          Array.isArray(item.pages) &&
          item.pages.length <= 3 &&
          item.pages.every((page) =>
            [
              'settings',
              'connections',
              'deployment',
              'projects',
              'imports',
              'apps',
              'recall',
              'scheduler',
            ].includes(page),
          ) &&
          settingsCategory(item.category) !== null
          ? { ...item, category: settingsCategory(item.category)! }
          : null;
      },
    );
    if (restored) {
      state.pages = restored.pages;
      state.settingsCategory = restored.category;
    }
  }
  if (input.kind === 'component' && input.child === 'attachment-list' && input.name === 'input')
    input = input.event as ComponentInput;
  if (input.kind === 'component' && input.child === 'composer' && input.name === 'attachments')
    input = { kind: 'raw', key: 'attachments-open', event: {} };
  if (input.kind === 'raw' && input.key === 'attachments-summary')
    input = { ...input, key: 'attachments-open' };
  if (input.kind === 'raw' && input.key === 'attachment-browser-close')
    input = { ...input, key: 'attachment-close' };
  state.revision++;
  const actions: Action[] = [];
  const events: Emission[] = [];
  const locale =
    target === 'tui'
      ? effectiveLocale(state.preferences, configuredLocale, values.locale?.[0])
      : resolveLocale(configuredLocale, values.locale?.[0]);
  if (
    target === 'tui' &&
    appearanceEvent(
      state.preferences,
      input,
      values.appearance?.[0] as Theme,
      configuredLocale,
      actions,
    )
  )
    return frame(presentation, configuredLocale, state, values, actions);
  if (input.kind === 'action') {
    const outcome = input.event as ActionOutcome;
    if (state.scheduleWatch === input.key) {
      if (outcome.status === 'updated') state.scheduleAlerts = outcome.value as AgentScheduleAlerts;
      else if (outcome.status === 'failed' || outcome.status === 'not_accepted')
        state.scheduleWatch = null;
    } else if (state.catalogWatch?.key === input.key) {
      if (outcome.status === 'updated') {
        state.catalogWatch.loading = false;
        state.catalogFailed = false;
        state.catalog = outcome.value as AgentSessionPage;
        if (
          !state.selected &&
          !state.embedding.session &&
          !state.embedding.fixed &&
          state.catalog.sessions.length
        )
          select(state, actions, state.catalog.sessions[0]);
        else if (state.selected) {
          const selected = state.catalog.sessions.find(
            (session) => session.id === state.selected!.id,
          );
          if (selected && selected.revision >= state.selected.revision) state.selected = selected;
        }
      } else if (outcome.status === 'failed' || outcome.status === 'not_accepted') {
        state.catalogWatch = null;
        state.catalogFailed = true;
      }
    } else if (state.watch?.key === input.key) {
      if (outcome.status === 'updated')
        delivery(state, outcome.value as AgentDeliveryPage, actions);
      else if (outcome.status === 'failed' || outcome.status === 'not_accepted') {
        state.watch = null;
        state.notice = 'unavailable';
      }
    } else {
      const request = state.pending[input.key];
      if (request) {
        delete state.pending[input.key];
        if (
          outcome.status === 'operation' &&
          (request.kind === 'create' ||
            request.kind === 'send' ||
            request.kind === 'stop' ||
            request.kind === 'queue' ||
            request.kind === 'respond' ||
            request.kind === 'markRead' ||
            request.kind === 'uploadAttachment')
        )
          receipt(state, actions, request, outcome.reply);
        else if (outcome.status === 'succeeded')
          querySucceeded(state, actions, request, outcome.value);
        else queryFailed(state, request);
        // A newer original paint/intent may have arrived while this read
        // was pending. The last attempted basis prevents self-retry.
        if (request.kind === 'history' && request.implicit) follow(state, actions);
      }
    }
  } else if (input.kind === 'component') {
    return handleComponent(
      presentation,
      configuredLocale,
      state,
      input,
      values,
      actions,
      events,
      locale,
    );
  } else if (input.kind === 'raw') {
    return handleRaw(presentation, configuredLocale, state, input, values, actions, events, locale);
  }
  return frame(presentation, configuredLocale, state, values, actions, events);
}
