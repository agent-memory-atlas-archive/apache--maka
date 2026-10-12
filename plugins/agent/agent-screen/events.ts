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

import { effectiveLocale, appearanceEvent } from '../../shared/appearance-editor.js';
import { settingsEvent } from '../../shared/settings.js';
import type {
  Action,
  ActionOutcome,
  Emission,
  AgentExecutorSelection,
  AgentDeliveryPage,
  InteractionAnswer,
  SourceField,
  SourcePosition,
  AgentSession,
  AgentSessionPage,
  AgentDirectorySelection,
  AgentProjectRef,
  AgentScheduleAlerts,
  FileObservation,
  AgentQueueAction,
  RowActivated,
  ComponentInput,
  ComponentValues,
  EditorObservation,
} from '../../../packages/plugin-sdk/src/index.js';
import type {
  AgentScreenProps,
  PageChrome,
  Theme,
  QuestionDraft,
  SelectedModel,
  ModelSetup,
  TimelineFileContext,
  MessageCopyObservation,
} from '../../shared/contracts.js';
import { sameValue } from '../../shared/value.js';
import { observeBookmark, retryBookmark } from '../../shared/bookmark.js';
import { copy, resolveLocale } from '../../shared/locale.js';
import { type Presentation, type Node } from '../../shared/view.js';

import {
  attachmentFiles,
  attachmentCapacity,
  attachmentChild,
  pastedFiles,
  type DraftFilePaste,
  selectedFile,
} from '../../shared/attachments.js';

import { observeDraft, draftReady, retryDraft, type DraftCheckpoint } from '../../shared/drafts.js';

import {
  type Page,
  type Navigation,
  navigationBookmark,
  navigationKey,
  type State,
} from './state.js';
import { utf8Bytes } from './messages.js';
import {
  intent,
  canExecute,
  selectedBackend,
  catalogPending,
  catalog,
  observe,
  history,
  follow,
  select,
  readHistorySource,
  mutationPending,
  executionPending,
  conversationRoute,
  openInteraction,
  rootCommands,
  openUsage,
  controlSidebar,
  rootCommand,
  loadQueueEdit,
  initializeQueueEdit,
  questionChild,
  respondInteraction,
  submitQuestion,
  checkInteraction,
  focusAttachments,
  closeDialog,
} from './actions.js';
import { receipt, queryFailed, querySucceeded, delivery } from './results.js';
import { frame } from './frame.js';

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
          [
            'appearance',
            'interface',
            'models',
            'sessions',
            'host',
            'plugins',
            'assistant',
            'insights',
            ...(state.web ? ['web'] : []),
            ...(state.jev ? ['jev'] : []),
            ...(props.workhub ? ['subagents'] : []),
          ].includes(item.category)
          ? item
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
    if (input.name === 'chrome' && state.openedPages.includes(input.child as Page)) {
      state.pageChrome[input.child as Page] = input.event as PageChrome;
      return frame(presentation, configuredLocale, state, values, actions);
    }

    if (input.child === 'composer' && input.name === 'checkpoint') {
      state.drafts = observeDraft(state.drafts, input.event as DraftCheckpoint, state.selected);
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'composer' && input.name === 'draftCheck') {
      retryDraft(state.drafts, (input.event as { owner: string }).owner);
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'composer' && input.name === 'files' && target === 'tui') {
      pastedFiles(state.attachments, input.event as DraftFilePaste);
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === attachmentChild(state.attachments) && input.name === 'changed') {
      if (state.dialog === 'attachments' && state.attachments.selection && !mutationPending(state))
        state.attachments.selection.file = input.event as FileObservation;
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === `permission-approval-${state.interaction?.attempt}`) {
      if (state.dialog === 'approval') {
        if (input.name === 'closed') closeDialog(state);
        else if (input.name === 'check') checkInteraction(state, actions);
        else if (input.name === 'submit') {
          const event = input.event as {
            requestId: string;
            decision: 'deny' | 'allow';
            scope: 'once' | 'turn' | 'session';
          };
          const request = state.interaction?.record?.request;
          if (event.requestId === state.interaction?.requestId && request?.kind === 'permissions')
            respondInteraction(state, actions, {
              kind: 'permissions',
              decision:
                event.decision === 'deny'
                  ? { decision: 'deny' }
                  : {
                      decision: 'allow',
                      scope: event.scope,
                      permissions: request.request.permissions,
                    },
            });
        }
      }
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === `service-approval-${state.interaction?.attempt}`) {
      if (state.dialog === 'approval') {
        if (input.name === 'closed') closeDialog(state);
        else if (input.name === 'check') checkInteraction(state, actions);
        else if (input.name === 'submit') {
          const event = input.event as { requestId: string; decision: 'allow' | 'deny' };
          if (event.requestId === state.interaction?.requestId)
            respondInteraction(state, actions, {
              kind: 'service_approval',
              decision: event.decision,
            });
        }
      }
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === questionChild(state)) {
      if (state.dialog === 'question') {
        if (input.name === 'closed') closeDialog(state);
        else if (input.name === 'check') checkInteraction(state, actions);
        else if (input.name === 'submit' && state.interaction?.record?.request.kind === 'form') {
          const event = input.event as { requestId: string; answer: InteractionAnswer };
          if (event.requestId === state.interaction.requestId && event.answer.kind === 'form')
            respondInteraction(state, actions, event.answer);
        } else if (input.name === 'submit')
          submitQuestion(
            state,
            actions,
            input.event as { requestId: string; answers: QuestionDraft[] },
          );
      }
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'queue-editor') {
      const edit = state.queueEdit;
      if (input.name === 'changed') {
        const draft = input.event as EditorObservation;
        state.queueDraft = draft;
        if (
          edit?.command &&
          draft.receipt?.owner === edit.command.owner &&
          draft.receipt.sequence === edit.command.sequence
        ) {
          edit.phase = draft.receipt.outcome.status === 'applied' ? 'ready' : 'failed';
          edit.command = null;
          if (edit.phase === 'failed') state.notice = 'draftKept';
        }
        initializeQueueEdit(state);
      } else if (state.dialog === 'queue' && edit) {
        if (input.name === 'closed') closeDialog(state);
        else if (
          input.name === 'check' &&
          state.uncertain &&
          !Object.values(state.pending).some((request) => request.kind === 'receipt')
        )
          intent(
            state,
            actions,
            { kind: 'receipt' },
            'receipts',
            'read',
            state.uncertain.reference,
          );
        else if (!mutationPending(state)) {
          if (
            input.name === 'reload' &&
            state.selected?.id === edit.target.sessionId &&
            state.queue
          ) {
            loadQueueEdit(state, actions, {
              ...edit.target,
              expectedRevision: state.queue.revision,
            });
          } else if (input.name === 'save' && edit.phase === 'ready' && !edit.command) {
            const draft = input.event as EditorObservation;
            if (!sameValue(draft, state.queueDraft) || draft.snapshot.marked)
              state.notice = 'draftKept';
            else if (draft.snapshot.text.bytes > 65536) state.notice = 'tooLarge';
            else
              intent(
                state,
                actions,
                { kind: 'queueRead', attempt: edit.attempt, draft },
                'reader',
                'read',
                { start: 0, end: draft.snapshot.text.bytes },
                {
                  text: {
                    child: 'queue-editor',
                    method: 'snapshot',
                    input: {
                      kind: 'at_revision',
                      owner: draft.snapshot.text.owner,
                      revision: draft.snapshot.text.revision,
                    },
                  },
                },
              );
          }
        }
      }
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'skill-library') {
      if (input.name === 'closed' && state.dialog === 'skills') state.dialog = null;
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'help') {
      if (input.name === 'closed' && state.dialog === 'help') state.dialog = null;
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'terminal-panel') {
      if (input.name === 'closed' && state.dialog === 'terminal') state.dialog = null;
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'session-details') {
      if (input.name === 'closed' && state.dialog === 'details') state.dialog = null;
      else if (input.name === 'changed') {
        if (state.selected?.id === (input.event as { sessionId: string }).sessionId)
          observe(state, actions);
      }
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (
      [
        'assistant-preferences',
        'session-defaults',
        'network-settings',
        'insights',
        'web-settings',
        'jev-settings',
        'subagent-settings',
      ].includes(input.child) &&
      input.name === 'closed' &&
      state.pages.at(-1) === 'settings'
    )
      state.pages.pop();
    if (input.child === 'apps') {
      if (input.name === 'closed' && state.pages.at(-1) === 'apps') state.pages.pop();
      if (input.name === 'manage' && state.pages.at(-1) === 'apps') state.pages.push('deployment');
    }
    if (input.child === 'scheduler') {
      if (input.name === 'closed' && state.pages.at(-1) === 'scheduler') state.pages.pop();
      else if (input.name === 'connections' && state.pages.at(-1) === 'scheduler')
        state.pages.push('connections');
      else if (
        input.name === 'selected' &&
        state.pages.at(-1) === 'scheduler' &&
        !state.embedding.fixed &&
        !mutationPending(state) &&
        !catalogPending(state)
      ) {
        select(state, actions, input.event as AgentSession);
        state.pages = [];
        catalog(state);
      }
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'recall') {
      if (input.name === 'closed' && state.pages.at(-1) === 'recall') state.pages.pop();
      else if (
        input.name === 'selected' &&
        state.pages.at(-1) === 'recall' &&
        !state.embedding.fixed &&
        !mutationPending(state) &&
        !catalogPending(state)
      ) {
        const selected = input.event as { session: AgentSession; anchor: number | null };
        select(state, actions, selected.session);
        state.recallAnchor = selected.anchor;
        state.pages = [];
      }
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'imports') {
      if (input.name === 'closed' && state.pages.at(-1) === 'imports') state.pages.pop();
      else if (input.name === 'changed') catalog(state);
      else if (
        input.name === 'selected' &&
        state.pages.at(-1) === 'imports' &&
        !state.embedding.fixed &&
        !mutationPending(state) &&
        !catalogPending(state)
      ) {
        select(state, actions, (input.event as { session: AgentSession }).session);
        state.pages = [];
        catalog(state);
      }
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'projects') {
      if (input.name === 'closed' && state.pages.at(-1) === 'projects') state.pages.pop();
      else if (input.name === 'changed') {
        catalog(state);
        if (state.selected) observe(state, actions);
      } else if (
        input.name === 'selected' &&
        state.pages.at(-1) === 'projects' &&
        !state.embedding.fixed &&
        !mutationPending(state) &&
        !catalogPending(state)
      ) {
        intent(state, actions, { kind: 'create' }, 'agent', 'create', {
          title: copy(locale).new,
          labels: [],
          workspace: {
            kind: 'project',
            ...(input.event as {
              project: AgentProjectRef;
              selection: AgentDirectorySelection;
            }),
          },
        });
        state.pages = [];
      }
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'directory-picker') {
      if (state.dialog === 'directories' && conversationRoute(state)) {
        if (input.name === 'closed') state.dialog = null;
        else if (
          input.name === 'selected' &&
          !state.embedding.fixed &&
          !mutationPending(state) &&
          !catalogPending(state)
        ) {
          intent(state, actions, { kind: 'create' }, 'agent', 'create', {
            title: copy(locale).new,
            labels: [],
            workspace: { kind: 'directory', selection: input.event as AgentDirectorySelection },
          });
          state.dialog = null;
        }
      }
      return frame(presentation, configuredLocale, state, values, actions);
    }
    if (input.child === 'command-palette') {
      if (state.dialog === 'palette') {
        if (input.name === 'closed') state.dialog = null;
        else if (input.name === 'invoked') {
          const command = rootCommands(state, locale, target).find(
            (command) => command.id === (input.event as { id: string }).id && !command.disabled,
          );
          if (
            command &&
            rootCommand(state, actions, locale, command.kind, target) &&
            state.dialog === 'palette'
          ) {
            state.dialog = null;
            if (command.kind === 'workhub-open') events.push({ name: 'workhub', value: {} });
          }
        } else if (
          input.name === 'selected' &&
          !state.embedding.fixed &&
          !mutationPending(state) &&
          !catalogPending(state)
        ) {
          select(state, actions, input.event as AgentSession);
          state.pages = [];
          state.dialog = null;
        }
      }
      return frame(presentation, configuredLocale, state, values, actions, events);
    }

    if (target === 'tui' && input.child === 'queue' && input.name === 'activated') {
      const event = input.event as RowActivated;
      const queue = state.queue;
      const row = queue?.entries.find((row) => row.messageId === event.row);
      if (
        conversationRoute(state) &&
        !state.dialog &&
        !mutationPending(state) &&
        !executionPending(state) &&
        state.selected &&
        queue &&
        event.revision === queue.revision &&
        row &&
        row.kind !== 'in_flight'
      ) {
        let edit: AgentQueueAction | null = null;
        if (event.action === 'edit')
          loadQueueEdit(state, actions, {
            sessionId: state.selected.id,
            messageId: row.messageId,
            expectedRevision: event.revision,
          });
        else if (event.action === 'retract') edit = { kind: 'retract', messageId: row.messageId };
        else if (event.action === 'promote' && row.kind === 'followup' && state.run)
          edit = { kind: 'promote', messageId: row.messageId };
        else if (event.action === 'up' || event.action === 'down') {
          const ids = queue.entries
            .filter((entry) => entry.kind === row.kind)
            .map((entry) => entry.messageId);
          const index = ids.indexOf(row.messageId),
            next = index + (event.action === 'up' ? -1 : 1);
          if (next >= 0 && next < ids.length) {
            [ids[index], ids[next]] = [ids[next], ids[index]];
            edit = { kind: 'reorder', messageIds: [ids[0], ...ids.slice(1)] };
          }
        }
        if (edit)
          intent(
            state,
            actions,
            { kind: 'queue', sessionId: state.selected.id },
            'agent',
            'editQueue',
            { sessionId: state.selected.id, expectedRevision: event.revision, edit },
          );
      }
    } else if (input.child === 'timeline' && input.name === 'messageCopy') {
      const copied = input.event as MessageCopyObservation | null;
      if (
        !copied ||
        (state.dialog === 'palette' &&
          copied.request === state.copyRequest &&
          copied.sessionId === state.selected?.id &&
          copied.through === state.window?.page.through &&
          copied.landing === state.landing.sequence)
      )
        state.messageCopy = copied;
    } else if (input.child === 'timeline' && input.name === 'fileContext') {
      const context = input.event as TimelineFileContext | null;
      if (
        !context ||
        (context.sessionId === state.selected?.id &&
          context.through === state.window?.page.through &&
          context.landing === state.landing.sequence)
      )
        state.fileContext = context;
    } else if (input.child === 'timeline' && input.name === 'browseClosed') {
      if ((input.event as { request: number }).request === state.browse) state.browse = null;
    } else if (input.child === 'timeline' && input.name === 'searchClosed') {
      state.searchOpen = false;
    } else if (input.child === 'timeline' && input.name === 'tailReady') {
      const receipt = input.event as NonNullable<State['tailReceipt']>;
      if (receipt.landing === state.landing.sequence) {
        state.tailReceipt = receipt;
        follow(state, actions);
      }
    } else if (input.name === 'closed' && input.child === state.pages.at(-1)) state.pages.pop();
    else if (input.child === 'connections' && input.name === 'changed') {
      state.catalogEpoch++;
      state.modelSetup = 'loading';
      state.modelRefreshing = !!state.model;
    } else if (input.child === 'composer' && input.name === 'changed') {
      const observation = input.event as EditorObservation;
      if (
        state.command &&
        observation.receipt?.owner === state.command.owner &&
        observation.receipt.sequence === state.command.sequence
      ) {
        if (observation.receipt.outcome.status === 'conflict') state.notice = 'draftKept';
        state.command = null;
      }
    } else if (
      input.child === 'composer' &&
      input.name === 'send' &&
      draftReady(state.drafts, state.selected) &&
      !state.attachments.paste &&
      conversationRoute(state) &&
      !state.dialog &&
      !state.image &&
      !state.source &&
      !executionPending(state) &&
      state.notice !== 'unavailable' &&
      !mutationPending(state) &&
      state.selected &&
      !state.selected.archived &&
      canExecute(state) &&
      !state.loading &&
      (!state.run || (target === 'tui' && state.queue !== null)) &&
      !state.command
    ) {
      const draft = input.event as EditorObservation;
      if (draft.snapshot.marked) state.notice = 'draftKept';
      else if (draft.snapshot.text.bytes > 65536) state.notice = 'tooLarge';
      else
        intent(
          state,
          actions,
          {
            kind: 'read',
            dataSource: state.selected.dataSource,
            attachments: attachmentFiles(
              state.attachments,
              state.selected.id,
              state.selected.dataSource,
            ).map((file) => file.id),
            draft,
            sessionId: state.selected.id,
            backend: selectedBackend(state),
          },
          'reader',
          'read',
          { start: 0, end: draft.snapshot.text.bytes },
          {
            text: {
              child: 'composer',
              method: 'snapshot',
              input: {
                kind: 'at_revision',
                owner: draft.snapshot.text.owner,
                revision: draft.revision,
              },
            },
          },
        );
    } else if (input.child === 'composer' && input.name === 'stop') {
      rootCommand(state, actions, locale, 'stop', target);
    } else if (input.child === 'models' && input.name === 'backends') {
      state.hasBackends = (input.event as { available: boolean }).available;
    } else if (
      input.child === 'models' &&
      input.name === 'executor' &&
      !mutationPending(state) &&
      !executionPending(state)
    ) {
      state.executor = input.event as AgentExecutorSelection | null;
    } else if (input.child === 'models' && input.name === 'setup') {
      const setup = input.event as ModelSetup;
      if (setup.catalogEpoch === state.catalogEpoch) state.modelSetup = setup.kind;
    } else if (
      input.child === 'models' &&
      input.name === 'connections' &&
      conversationRoute(state) &&
      !state.dialog
    ) {
      state.pages = ['connections'];
    } else if (
      input.child === 'models' &&
      input.name === 'selected' &&
      !state.executor &&
      !mutationPending(state) &&
      !executionPending(state)
    ) {
      state.model = input.event as SelectedModel;
      state.modelRefreshing = false;
    } else if (
      input.child === 'navigation' &&
      !state.embedding.fixed &&
      input.name === 'select' &&
      state.dialog === null &&
      !mutationPending(state) &&
      !catalogPending(state)
    ) {
      const { sessionId, details, usage } = input.event as {
        sessionId: string;
        details?: boolean;
        usage?: boolean;
      };
      const session = state.catalog?.sessions.find((item) => item.id === sessionId);
      if (session && usage && target === 'tui') openUsage(state, session);
      else if (session) {
        state.pages = [];
        if (state.selected?.id !== session.id) select(state, actions, session);
        if (details && !executionPending(state)) state.dialog = 'details';
        if (target === 'tui') controlSidebar(state, 'dismiss_transient');
      }
    } else if (input.child === 'image-viewer' && input.name === 'closed') {
      state.image = null;
      state.followAttempt = null;
      follow(state, actions);
    } else if (
      input.child === 'timeline' &&
      input.name === 'viewImage' &&
      state.selected &&
      state.window &&
      !state.loading
    ) {
      const sequence = (input.event as { sequence: number }).sequence;
      const chunk = state.window.page.chunks.find(
        (c) => c.sequence === sequence && c.tool?.result?.image,
      );
      if (chunk)
        state.image = {
          sessionId: state.selected.id,
          through: state.window.page.through,
          sequence: chunk.tool!.result!.sequence,
        };
    } else if (input.child === 'timeline' && state.source) {
      const source = state.source;
      if (input.name === 'sourceBack') state.source = null;
      else if (!source.loading && input.name === 'sourceField') {
        const field = (input.event as { field: SourceField }).field;
        if (field !== source.field && source.fields.includes(field)) {
          source.field = field;
          source.ready = false;
          source.format = null;
          source.chunk = { ...source.chunk, offset: 0, totalBytes: 0, text: '' };
          readHistorySource(state, actions, { kind: 'start' });
        }
      } else if (!source.loading) {
        const chunk = source.chunk;
        const end = chunk.offset + utf8Bytes(chunk.text);
        const position: SourcePosition | null =
          input.name === 'sourceRetry' && source.failed
            ? source.position
            : input.name === 'sourceFirst' && chunk.offset > 0
              ? { kind: 'start' }
              : input.name === 'sourceLast' && end < chunk.totalBytes
                ? { kind: 'tail' }
                : input.name === 'sourcePrevious' && chunk.offset > 0
                  ? { kind: 'before', offset: chunk.offset }
                  : input.name === 'sourceNext' && end < chunk.totalBytes
                    ? { kind: 'after', offset: end }
                    : null;
        if (position) readHistorySource(state, actions, position);
      }
    } else if (input.child === 'timeline' && !state.loading && state.window) {
      const page = state.window.page;
      if (
        input.name === 'markRead' &&
        state.selected?.hasUnread &&
        !state.selected.archived &&
        !mutationPending(state) &&
        conversationRoute(state) &&
        state.dialog === null &&
        !state.image &&
        !state.searchOpen
      ) {
        const event = input.event as {
          sessionId: string;
          sequence: number;
          field: 'body' | 'thinking';
        };
        const chunk =
          event.sessionId === state.selected.id &&
          page.chunks.find(
            (chunk) =>
              chunk.sequence === event.sequence &&
              (chunk.field ?? 'body') === event.field &&
              (chunk.role === 'user' || chunk.role === 'assistant'),
          );
        if (chunk)
          intent(
            state,
            actions,
            { kind: 'markRead', sessionId: state.selected.id },
            'agent',
            'markRead',
            {
              sessionId: state.selected.id,
              messageId: chunk.messageId,
            },
          );
      } else if (input.name === 'source') {
        const chunk = page.chunks.find(
          (chunk) =>
            chunk.sequence === (input.event as { sequence: number }).sequence &&
            (chunk.field ?? 'body') === ((input.event as { field?: string }).field ?? 'body'),
        );
        if (chunk)
          state.source = {
            through: page.through,
            chunk: {
              sequence: chunk.sequence,
              offset: chunk.offset,
              totalBytes: chunk.totalBytes,
              text: chunk.text,
            },
            navigation: ++state.navigation,
            landing: { sequence: state.navigation, edge: 'start' },
            loading: false,
            failed: false,
            field: chunk.field ?? 'body',
            fields: [chunk.field ?? 'body'],
            format: 'text',
            ready: true,
            position: { kind: 'after', offset: chunk.offset },
          };
      } else if (input.name === 'toolSource') {
        const event = input.event as { sequence: number; field: 'arguments' | 'result' };
        const chunk = page.chunks.find((chunk) => chunk.sequence === event.sequence);
        const reference =
          event.field === 'arguments'
            ? chunk?.tool?.call
            : event.field === 'result'
              ? chunk?.tool?.result
              : null;
        if (reference) {
          state.source = {
            through: page.through,
            chunk: { sequence: reference.sequence, offset: 0, totalBytes: 0, text: '' },
            navigation: state.navigation,
            landing: { sequence: state.navigation, edge: 'start' },
            loading: false,
            failed: false,
            field: event.field,
            fields: [],
            format: null,
            ready: false,
            position: { kind: 'start' },
          };
          readHistorySource(state, actions, { kind: 'start' });
        }
      } else if (
        input.name === 'older' &&
        page.before &&
        !Object.values(state.pending).some(
          (request) => request.kind === 'history' && request.epoch === state.epoch,
        )
      )
        history(
          state,
          actions,
          { kind: 'before', cursor: page.before },
          page.through,
          false,
          false,
          target === 'tui',
        );
      else if (
        input.name === 'newer' &&
        page.after &&
        !Object.values(state.pending).some(
          (request) => request.kind === 'history' && request.epoch === state.epoch,
        )
      )
        history(
          state,
          actions,
          { kind: 'after', cursor: page.after },
          page.through,
          false,
          false,
          target === 'tui',
        );
      else if (input.name === 'latest')
        history(state, actions, { kind: 'tail' }, state.latestKnown, true);
    }
  } else if (input.kind === 'raw') {
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
      const files = attachmentFiles(
        state.attachments,
        state.selected.id,
        state.selected.dataSource,
      );
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
        if (
          ['activate', 'submit'].includes((input.event as { kind?: string } | null)?.kind ?? '')
        ) {
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
        (input.key.startsWith('approval-') &&
          (kind === 'service_approval' || kind === 'permissions'))
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
  }
  return frame(presentation, configuredLocale, state, values, actions, events);
}
