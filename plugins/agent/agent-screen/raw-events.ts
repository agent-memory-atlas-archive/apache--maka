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
  ComponentInput,
  ComponentValues,
  Emission,
} from '../../../packages/plugin-sdk/src/index.js';
import type { Theme } from '../../shared/contracts.js';
import { settingsEvent } from '../../shared/settings.js';
import { sameValue } from '../../shared/value.js';
import { type Node, type Presentation } from '../../shared/view.js';

import {
  attachmentCapacity,
  attachmentChild,
  attachmentFiles,
  selectedFile,
} from '../../shared/attachments.js';

import { draftReady } from '../../shared/drafts.js';

import {
  catalog,
  catalogPending,
  checkInteraction,
  closeDialog,
  controlSidebar,
  conversationRoute,
  focusAttachments,
  intent,
  mutationPending,
  openInteraction,
  rootCommand,
  rootCommands,
} from './actions.js';
import { frame } from './frame.js';
import { type State } from './state.js';

export function handleRaw<N extends Node>(
  presentation: Presentation<N>,
  configuredLocale: string,
  state: State,
  input: Extract<ComponentInput, { kind: 'raw' }>,
  values: ComponentValues,
  actions: Action[],
  events: Emission[],
  locale: string,
) {
  const target = presentation.target;
  if (
    input.key === 'schedule-reminder' &&
    !state.dialog &&
    !state.embedding.fixed &&
    state.scheduleAlerts.reminders[0]
  ) {
    state.scheduleRequested = {
      sequence: ++state.serial,
      taskId: state.scheduleAlerts.reminders[0].taskId,
    };
    rootCommand(state, actions, locale, 'scheduler-open', target);
  }
  if (input.key === 'catalog-search') input = { ...input, key: 'palette-open' };
  if (input.key === 'home-projects') rootCommand(state, actions, locale, 'projects-open', target);
  else if (input.key === 'home-imports')
    rootCommand(state, actions, locale, 'imports-open', target);
  else if (input.key === 'home-new') rootCommand(state, actions, locale, 'create', target);
  if (
    input.key.startsWith('file-paste-') &&
    state.attachments.paste &&
    !mutationPending(state) &&
    !state.attachments.paste.command
  ) {
    const paste = state.attachments.paste,
      file = paste.observed.selection;
    if (input.key === 'file-paste-cancel')
      paste.command = { selection: selectedFile(file), action: 'cancel' };
    else if (input.key === 'file-paste-skip' && ['ready', 'failed'].includes(file.phase))
      paste.command = { selection: selectedFile(file), action: 'next' };
    else if (input.key === 'file-paste-retry') {
      if (file.phase === 'failed' && paste.observed.retryable)
        paste.command = { selection: selectedFile(file), action: 'retry' };
      else if (file.phase === 'ready') paste.attempted = null;
    }
  }
  if (
    input.key === 'attachments-open' &&
    draftReady(state.drafts, state.selected) &&
    !state.attachments.paste &&
    conversationRoute(state) &&
    !state.dialog &&
    state.selected &&
    !state.loading &&
    !state.source &&
    !state.image &&
    !mutationPending(state)
  ) {
    const files = attachmentFiles(state.attachments, state.selected.id, state.selected.dataSource);
    state.attachments.selection = {
      attempt: ++state.serial,
      sessionId: state.selected.id,
      dataSource: state.selected.dataSource,
      file: null,
      browse: files.length === 0,
      selected: files[0]?.id ?? null,
      focus: null,
    };
    focusAttachments(state);
    state.dialog = 'attachments';
  } else if (input.key === 'attachment-close' && state.dialog === 'attachments') {
    closeDialog(state);
  } else if (
    (input.key === 'attachment-check' || input.key === 'attachment-browser-check') &&
    state.dialog === 'attachments'
  ) {
    if (
      state.uncertain &&
      !Object.values(state.pending).some((request) => request.kind === 'receipt')
    )
      intent(state, actions, { kind: 'receipt' }, 'receipts', 'read', state.uncertain.reference);
  } else if (
    state.dialog === 'attachments' &&
    state.attachments.selection &&
    draftReady(state.drafts, state.selected) &&
    state.selected?.id === state.attachments.selection.sessionId &&
    sameValue(state.selected.dataSource, state.attachments.selection.dataSource) &&
    !mutationPending(state)
  ) {
    const selection = state.attachments.selection;
    const files = attachmentFiles(state.attachments, selection.sessionId, selection.dataSource);
    if (input.key === 'attachment-list-back') {
      selection.browse = false;
      focusAttachments(state);
    } else if (/^attachment-select-[0-7]$/.test(input.key)) {
      selection.selected =
        files[Number(input.key.slice('attachment-select-'.length))]?.id ?? selection.selected;
    } else if (
      input.key === 'attachment-browse' &&
      !state.selected.archived &&
      attachmentCapacity(state.attachments, selection.sessionId, selection.dataSource)
    ) {
      if (['activate', 'submit'].includes((input.event as { kind?: string } | null)?.kind ?? '')) {
        selection.selected = null;
        selection.browse = true;
        selection.focus = null;
      }
    } else if (input.key.startsWith('attachment-remove-')) {
      const id = input.key.slice('attachment-remove-'.length);
      state.attachments.files = state.attachments.files.filter(
        (file) =>
          file.sessionId !== selection.sessionId ||
          !sameValue(file.dataSource, selection.dataSource) ||
          file.id !== id,
      );
      const remaining = attachmentFiles(
        state.attachments,
        selection.sessionId,
        selection.dataSource,
      );
      const removed = files.findIndex((file) => file.id === id);
      if (removed >= 0) {
        selection.selected = remaining[Math.min(removed, remaining.length - 1)]?.id ?? null;
        focusAttachments(state);
      }
    } else if (
      input.key === 'attachment-upload' &&
      !state.selected.archived &&
      attachmentCapacity(state.attachments, selection.sessionId, selection.dataSource) &&
      selection.file?.phase === 'ready' &&
      selection.file.file
    ) {
      const file = selection.file;
      state.notice = null;
      intent(
        state,
        actions,
        {
          kind: 'uploadAttachment',
          dataSource: selection.dataSource,
          sessionId: selection.sessionId,
          attempt: selection.attempt,
        },
        'agent',
        'uploadAttachment',
        { sessionId: selection.sessionId, ...file.file },
        {
          file: {
            child: attachmentChild(state.attachments),
            method: 'snapshot',
            input: { owner: file.owner, revision: file.revision },
          },
        },
      );
    }
  }

  if (
    (input.key.startsWith('question-open-') || input.key.startsWith('approval-open-')) &&
    !state.dialog &&
    !mutationPending(state) &&
    state.selected &&
    !state.selected.archived
  ) {
    const requestId = input.key.slice(14);
    const kind = state.interactions?.entries.find((entry) => entry.requestId === requestId)?.kind;
    if (
      (input.key.startsWith('question-') && (kind === 'question' || kind === 'form')) ||
      (input.key.startsWith('approval-') && (kind === 'service_approval' || kind === 'permissions'))
    ) {
      openInteraction(state, actions, requestId, kind);
    }
  } else if (
    input.key === 'question-close' &&
    (state.dialog === 'question' || state.dialog === 'approval')
  )
    closeDialog(state);
  else if (
    input.key === 'question-check' &&
    (state.dialog === 'question' || state.dialog === 'approval')
  )
    checkInteraction(state, actions);
  else if (input.key === 'catalog-filter-all' || input.key === 'catalog-filter-needs_you') {
    const filter = input.key === 'catalog-filter-all' ? 'all' : 'needs_you';
    if (!mutationPending(state) && !catalogPending(state) && state.catalogFilter !== filter) {
      state.catalogFilter = filter;
      state.catalog = null;
      catalog(state);
    }
  } else if (['catalog-refresh', 'catalog-more', 'catalog-previous'].includes(input.key)) {
    if (!mutationPending(state) && !catalogPending(state)) {
      const page = state.catalog;
      if (input.key === 'catalog-refresh') catalog(state);
      else if (!state.catalogFailed && page) {
        if (input.key === 'catalog-more' && page.after)
          catalog(state, {
            kind: 'after',
            revision: page.revision,
            cursor: page.after,
          });
        else if (input.key === 'catalog-previous' && page.before)
          catalog(state, {
            kind: 'before',
            revision: page.revision,
            cursor: page.before,
          });
      }
    }
  } else if (input.key === 'palette-open' && state.dialog === null) {
    state.dialog = 'palette';
    state.copyRequest = ++state.serial;
    state.messageCopy = null;
  } else if (input.key === 'help-open') {
    if (state.dialog === null) rootCommand(state, actions, locale, 'help-open', target);
  } else if (input.key === 'agent-screen-dialog') {
    const selection = state.attachments.selection;
    if (
      target === 'tui' &&
      state.dialog === 'attachments' &&
      selection?.browse &&
      attachmentFiles(state.attachments, selection.sessionId, selection.dataSource).length
    ) {
      selection.browse = false;
      focusAttachments(state);
    } else closeDialog(state);
  } else if (input.key === 'page-back' && state.pages.length > 0 && !state.dialog) {
    const page = state.pages.at(-1)!;
    if (page === 'settings') state.pages.pop();
    else if (state.pageChrome[page]?.backEnabled) state.pageBack[page] = ++state.serial;
  } else if (
    state.pages.at(-1) === 'settings' &&
    settingsEvent(state, input, values.appearance?.[0] as Theme)
  ) {
    // Settings write through the existing preference checkpoint.
  } else if (input.key === 'navigation-back' && state.dialog === null) {
    if ('open' in state.sidebar) state.sidebar.open = false;
    else controlSidebar(state, 'toggle');
  } else {
    const command = rootCommands(state, locale, target).find(
      (command) => command.kind === input.key,
    );
    if (
      command &&
      rootCommand(state, actions, locale, command.kind, target) &&
      command.kind === 'workhub-open'
    )
      events.push({ name: 'workhub', value: {} });
  }
  return frame(presentation, configuredLocale, state, values, actions, events);
}
