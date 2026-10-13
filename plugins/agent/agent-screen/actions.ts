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

import { tuiText } from '../../shared/tui-locale.js';
import type {
  Action,
  InteractionAnswer,
  AgentQueueMessage,
  SourcePosition,
  HistoryPosition,
  AgentSession,
  AgentList,
  TerminalPaneCommand,
} from '../../../packages/plugin-sdk/src/index.js';
import type { PaletteCommand, QuestionDraft } from '../../shared/contracts.js';
import { sameValue } from '../../shared/value.js';
import { copy } from '../../shared/locale.js';
import { compactLabel } from '../../shared/view.js';

import {
  attachmentFiles,
  attachmentCapacity,
  sameFile,
  selectedFile,
} from '../../shared/attachments.js';

import { draftReady } from '../../shared/drafts.js';

import { type Backend, type Request, type State, type RootCommand } from './state.js';
import { toolCards } from '../../shared/tool-cards.js';
import { readingKey } from './state.js';
import { messages } from './messages.js';

export function intent(
  state: State,
  actions: Action[],
  request: Request,
  binding: string,
  method: string,
  input: unknown,
  resources: Action['resources'] = {},
  continuations: Action['continuations'] = [],
) {
  const key = 'request-' + ++state.serial;
  state.pending[key] = request;
  actions.push({
    key,
    binding,
    member: 0,
    method,
    input,
    resources,
    continuations,
    ...(request.kind === 'send'
      ? {
          recovery: {
            format: 'maka.agent.draft-send@1',
            sessionId: request.sessionId,
            editor: { owner: request.draft.snapshot.text.owner, revision: request.draft.revision },
            attachments: request.attachments,
          },
        }
      : {}),
  });
}

export function canExecute(state: State) {
  return !!state.executor || (!!state.model && !state.modelRefreshing);
}

export function selectedBackend(state: State): Backend {
  return state.executor
    ? {
        kind: 'executor',
        selection: { ...state.executor, settings: { ...state.executor.settings } },
      }
    : {
        kind: 'model',
        model: { ...state.model!.choice.selection },
        thinkingLevel: state.model!.thinkingLevel,
      };
}

function createSession(state: State, actions: Action[], locale: string) {
  if (mutationPending(state) || catalogPending(state)) return;
  const title = copy(locale).new;
  if (state.selected?.archived) state.dialog = 'directories';
  else if (state.selected)
    intent(state, actions, { kind: 'create' }, 'agent', 'create', {
      title,
      labels: [],
      workspace: { kind: 'session', sessionId: state.selected.id },
    });
  else
    intent(state, actions, { kind: 'createDirectory', title }, 'agent', 'directories', {
      kind: 'roots',
    });
}

export function openModelSetup(state: State) {
  if (state.modelSetup === 'connect' && !state.hasBackends) {
    state.connectionSetupRequest = ++state.serial;
    state.pages = ['connections'];
  } else state.modelOpenRequest = ++state.serial;
}

export function catalogPending(state: State): boolean {
  return state.catalogWatch?.loading ?? false;
}

export function catalog(state: State, position?: AgentList['position']) {
  state.catalogFailed = false;
  const anchor = state.catalog?.sessions[0]?.id;
  state.catalogWatch = {
    key: `catalog-${++state.catalogGeneration}`,
    input: {
      filter: state.catalogFilter,
      position: position ?? (anchor ? { kind: 'at', cursor: anchor } : { kind: 'start' }),
    },
    loading: true,
  };
}

export function observe(state: State, actions: Action[]) {
  if (state.selected)
    intent(state, actions, { kind: 'observe', epoch: state.epoch }, 'agent', 'observe', {
      sessionId: state.selected.id,
    });
}

export function history(
  state: State,
  actions: Action[],
  position: HistoryPosition,
  through: number,
  latest: boolean,
  implicit = false,
  extend = false,
) {
  if (!state.selected) return;
  if (latest && !implicit) {
    const visit = state.visits.find((entry) => entry.id === state.selected?.id);
    if (visit) visit.following = true;
  }
  if (!implicit && !extend) state.loading = true;
  const navigation = ++state.navigation;
  intent(
    state,
    actions,
    {
      kind: 'history',
      epoch: state.epoch,
      navigation,
      through,
      latest,
      implicit,
      position,
      extend,
    },
    'agent',
    'history',
    { sessionId: state.selected.id, through, position, includeThinking: true },
    {},
    implicit ? [{ child: readingKey(state), method: 'tail', input: null }] : [],
  );
}

export function follow(state: State, actions: Action[]) {
  const window = state.window;
  if (
    state.source ||
    state.inspector === 'outputs' ||
    state.searchOpen ||
    state.browse !== null ||
    state.image ||
    state.loading ||
    state.notice === 'unavailable' ||
    !window?.latest ||
    !(
      state.latestKnown > window.page.through ||
      window.page.after !== null ||
      state.live.some(
        (fragment) => fragment.closedAt !== null && fragment.closedAt <= window.page.through,
      )
    ) ||
    Object.values(state.pending).some(
      (request) =>
        request.kind === 'history' &&
        request.epoch === state.epoch &&
        request.navigation === state.navigation,
    )
  )
    return;
  // The introductory view has no mounted reader or reading anchor. New facts
  // must still appear, including failures that finish before a first text delta.
  if (!window.page.chunks.length && !window.page.before && !window.page.after) {
    history(state, actions, { kind: 'tail' }, state.latestKnown, true);
    return;
  }
  const receipt = state.tailReceipt;
  if (
    receipt?.landing === state.landing.sequence &&
    state.visits.find((visit) => visit.id === state.selected?.id)?.following !== false &&
    !(
      state.followAttempt?.through === state.latestKnown &&
      state.followAttempt.landing === receipt.landing &&
      state.followAttempt.readingRevision === receipt.readingRevision
    )
  ) {
    state.followAttempt = { ...receipt, through: state.latestKnown };
    const last = window.page.chunks.at(-1)!;
    history(
      state,
      actions,
      {
        kind: 'after',
        cursor: window.page.after ?? {
          sequence: last.sequence,
          // Refresh the boundary segment's recorded timing/tool metadata at
          // the new fence, then append the following immutable segments.
          offset: last.offset,
          field: last.field,
        },
      },
      window.page.after ? window.page.through : state.latestKnown,
      true,
      true,
      true,
    );
  }
}

export function select(state: State, actions: Action[], session: AgentSession | null) {
  const previous = state.visits.find((visit) => visit.id === state.selected?.id);
  if (previous)
    Object.assign(previous, {
      session: state.selected!,
      window: state.window,
      live: state.live,
      source: state.source
        ? { ...state.source, loading: false, failed: state.source.failed || state.source.loading }
        : null,
      image: state.image,
      landing: state.landing,
      searchOpen: state.searchOpen,
      browse: state.browse,
      inspector: state.inspector,
    });
  let visit = session
    ? state.visits.find(
        (entry) => entry.id === session.id && sameValue(entry.dataSource, session.dataSource),
      )
    : undefined;
  if (session && !visit) {
    visit = {
      id: session.id,
      dataSource: session.dataSource,
      key: state.visits.length === 0 ? 'timeline' : `timeline-${++state.serial}`,
      window: null,
      live: [],
      following: true,
      source: null,
      image: null,
      landing: { sequence: 0, edge: 'end' },
      searchOpen: false,
      browse: null,
      inspector: null,
      output: null,
      outputFocus: null,
      session,
      epoch: state.epoch + 1,
      progress: null,
    };
  }
  if (visit)
    state.visits = [
      ...state.visits.filter((entry) => entry !== visit && entry.id !== visit.id),
      visit,
    ].slice(-3);
  state.searchOpen = visit?.searchOpen ?? false;
  state.browse = visit?.browse ?? null;
  state.inspector = visit?.inspector ?? null;
  state.recallAnchor = null;
  state.selected = session;
  state.epoch++;
  state.checklist = { watch: null, page: null, failed: false };
  if (session) watchChecklist(state);
  state.window = visit?.window ?? null;
  if (visit?.window) visit.following = false;
  state.source = visit?.source ?? null;
  state.image = visit?.image ?? null;
  state.live = visit?.live ?? [];
  if (visit) {
    visit.window = null;
    visit.source = null;
    visit.live = [];
  }
  state.landing = visit?.landing ?? { sequence: 0, edge: 'end' };
  state.tailReceipt = null;
  state.fileContext = null;
  state.followAttempt = null;
  state.navigation++;
  state.parts = [];
  state.run = null;
  state.queue = null;
  state.interactions = null;
  state.watch = null;
  state.runThrough = 0;
  state.latestKnown = state.window?.page.through ?? 0;
  state.loading = session !== null;
  state.notice = null;
  if (session) observe(state, actions);
}

export function watchChecklist(state: State, offset = 0) {
  if (!state.selected) return;
  state.checklist = {
    watch: {
      key: `checklist-${++state.serial}`,
      input: { sessionId: state.selected.id, offset },
    },
    page: null,
    failed: false,
  };
}

export function readHistorySource(state: State, actions: Action[], position: SourcePosition) {
  const source = state.source;
  if (!source || !state.selected) return;
  source.navigation = ++state.navigation;
  source.loading = true;
  source.failed = false;
  source.position = position;
  intent(
    state,
    actions,
    {
      kind: 'source',
      epoch: state.epoch,
      navigation: source.navigation,
      through: source.through,
      sequence: source.chunk.sequence,
      field: source.field,
      position,
    },
    'agent',
    'source',
    {
      sessionId: state.selected.id,
      through: source.through,
      source: { kind: 'sequence', sequence: source.chunk.sequence },
      field: source.field,
      position,
    },
  );
}

export function mutationPending(state: State) {
  return (
    state.restoring ||
    state.uncertain !== null ||
    Object.values(state.pending).some((request) =>
      [
        'read',
        'queueRead',
        'questionRead',
        'respond',
        'markRead',
        'uploadAttachment',
        'create',
        'createDirectory',
        'send',
        'stop',
        'queue',
        'receipt',
      ].includes(request.kind),
    )
  );
}

export function uploadPastedFile(state: State, actions: Action[]) {
  const paste = state.attachments.paste,
    observed = paste?.observed,
    file = observed?.selection;
  if (
    !paste ||
    !observed ||
    !file ||
    file.phase !== 'ready' ||
    !file.file ||
    paste.command ||
    mutationPending(state) ||
    sameFile(paste.attempted, file)
  )
    return;
  if (
    state.selected?.id === observed.context &&
    sameValue(state.selected.dataSource, observed.dataSource) &&
    state.selected.archived
  )
    return;
  const dataSource = observed.dataSource;
  if (state.selected && !sameValue(state.selected.dataSource, dataSource)) return;
  if (!dataSource || !draftReady(state.drafts, { id: observed.context, dataSource })) return;
  if (!attachmentCapacity(state.attachments, observed.context, dataSource)) {
    state.notice = 'attachmentLimit';
    return;
  }
  paste.attempted = selectedFile(file);
  state.notice = null;
  intent(
    state,
    actions,
    {
      kind: 'uploadAttachment',
      dataSource,
      sessionId: observed.context,
      attempt: ++state.serial,
      pasted: paste.attempted,
    },
    'agent',
    'uploadAttachment',
    { sessionId: observed.context, ...file.file },
    {
      file: { child: 'composer', method: 'file', input: paste.attempted },
    },
  );
}

export function executionPending(state: State) {
  return Object.values(state.pending).some(
    (request) => request.kind === 'observe' && request.epoch === state.epoch,
  );
}

export function conversationRoute(state: State) {
  return state.pages.length === 0;
}

export function reviewTarget(state: State) {
  const retained = state.interaction,
    original = state.uncertain?.request;
  // Keep an unresolved answer attached to its original review even after the
  // canonical pending list advances. Opening never repeats that answer.
  if (
    retained?.record &&
    retained.sessionId === state.selected?.id &&
    original?.kind === 'respond' &&
    original.sessionId === retained.sessionId &&
    original.attempt === retained.attempt &&
    original.requestId === retained.requestId
  )
    return {
      requestId: retained.requestId,
      kind: retained.record.request.kind,
      retained: true,
    };
  const entry = state.interactions?.entries.find(
    (entry) =>
      ['question', 'form', 'service_approval', 'permissions', 'client_capability'].includes(
        entry.kind,
      ) &&
      !(
        retained?.sessionId === state.selected?.id &&
        retained?.requestId === entry.requestId &&
        retained.record?.outcome
      ),
  );
  return entry ? { ...entry, retained: false } : null;
}

export function openInteraction(state: State, actions: Action[], requestId: string, kind: string) {
  if (!state.selected) return;
  if (
    state.interaction?.requestId !== requestId ||
    state.interaction.sessionId !== state.selected.id
  )
    state.interaction = {
      attempt: ++state.serial,
      sessionId: state.selected.id,
      requestId,
      record: null,
      notice: null,
    };
  state.dialog = kind === 'question' || kind === 'form' ? 'question' : 'approval';
  readInteraction(state, actions);
}

export function rootCommands(
  state: State,
  locale: string,
  target: 'tui' | 'desktop',
): (PaletteCommand & { kind: RootCommand })[] {
  const text = copy(locale);
  const shellRoute = state.dialog === null || state.dialog === 'palette';
  const route = conversationRoute(state) && shellRoute;
  const review = reviewTarget(state);
  const file = state.fileContext;
  const filePath =
    file &&
    file.sessionId === state.selected?.id &&
    file.through === state.window?.page.through &&
    file.landing === state.landing.sequence &&
    !state.loading &&
    !state.source &&
    !state.image &&
    !state.searchOpen
      ? file.path
      : null;
  const copied = state.messageCopy;
  const readyCopy =
    copied &&
    state.dialog === 'palette' &&
    copied.request === state.copyRequest &&
    copied.sessionId === state.selected?.id &&
    copied.through === state.window?.page.through &&
    copied.landing === state.landing.sequence &&
    !state.loading &&
    !state.source &&
    !state.image &&
    !state.searchOpen
      ? copied
      : null;
  const commands: (PaletteCommand & { kind: RootCommand })[] = [
    ...(target === 'tui'
      ? [
          {
            kind: 'outputs-open' as const,
            id: 'outputs-open',
            label: tuiText(locale, 'task-outputs'),
            hint: 'F4',
            disabled: !route || !state.selected || state.loading || !!state.source || !!state.image,
          },
          {
            kind: 'browse' as const,
            id: 'browse',
            label: text.browseMessages,
            hint: '',
            disabled:
              !route ||
              !state.selected ||
              state.loading ||
              !!state.source ||
              !!state.image ||
              !messages(state).length,
          },
          {
            kind: 'search-open' as const,
            id: 'search-open',
            label: text.findConversation,
            hint: 'Ctrl F',
            disabled: !route || !state.selected || state.loading || !!state.source || !!state.image,
          },
          ...(['copy-message', 'copy-source'] as const).map((kind) => {
            const value = kind === 'copy-message' ? readyCopy?.message : readyCopy?.source;
            const result = readyCopy?.field === 'result',
              args = readyCopy?.field === 'arguments';
            return {
              kind,
              id: kind,
              label:
                kind === 'copy-message'
                  ? text.copyMessage
                  : result
                    ? text.copyToolResultSource
                    : args
                      ? text.copyToolArgumentsSource
                      : text.copySource,
              hint: value
                ? ''
                : readyCopy?.status === 'loading'
                  ? text.copyLoading
                  : readyCopy?.status === 'too_large'
                    ? text.copyTooLarge
                    : readyCopy?.status === 'empty'
                      ? text.copyEmpty
                      : readyCopy?.status === 'streaming'
                        ? text.copyStreaming
                        : text.copyUnavailable,
              disabled: !route || !value,
              ...(value ? { copyText: value } : {}),
            };
          }),
          {
            kind: 'copy-file-path' as const,
            id: 'copy-file-path',
            label: text.toolCopyPath,
            hint: filePath
              ? compactLabel(filePath.split(/[/\\]/u).filter(Boolean).at(-1) || filePath, 32)
              : '',
            disabled: !route || !filePath,
            ...(filePath ? { copyText: filePath } : {}),
          },
        ]
      : []),
    {
      kind: 'help-open',
      id: 'help-open',
      label: text.helpTitle,
      hint: 'F1',
      disabled: state.dialog !== null && state.dialog !== 'palette',
    },
    {
      kind: 'interaction-open',
      id: 'interaction-open',
      label: text.reviewRequest,
      hint: 'Ctrl/⌘ G',
      disabled:
        !route ||
        !state.selected ||
        state.selected.archived ||
        !review ||
        (mutationPending(state) && !review.retained),
    },
    {
      kind: 'model-setup',
      id: 'model-setup',
      label: state.hasBackends
        ? text.executionBackend
        : state.modelSetup === 'connect'
          ? text.configureModels
          : text.chooseModel,
      hint: '',
      disabled: !route || mutationPending(state) || executionPending(state),
    },
    {
      kind: 'create',
      id: 'create',
      label: text.new,
      hint: 'Ctrl/⌘ N',
      disabled: !shellRoute || mutationPending(state) || catalogPending(state),
    },
    {
      kind: 'create-workspace',
      id: 'create-workspace',
      label: text.newWorkspace,
      hint: '',
      disabled: !shellRoute || mutationPending(state) || catalogPending(state),
    },
    ...(state.skills
      ? [
          {
            kind: 'skills-open' as const,
            id: 'skills-open',
            label: text.skillsTitle,
            hint: '',
            disabled: !route,
          },
        ]
      : []),
    {
      kind: 'terminal-open',
      id: 'terminal-open',
      label: text.terminalTitle,
      hint: '',
      disabled: !route || !state.selected || state.loading,
    },
    ...(target === 'tui'
      ? [
          {
            kind: 'usage-open' as const,
            id: 'usage-open',
            label: tuiText(locale, 'insights-this-session'),
            hint: '',
            disabled: !route || !state.selected,
          },
        ]
      : []),
    {
      kind: 'checklist-open',
      id: 'checklist-open',
      label: text.checklistTitle,
      hint: '',
      disabled: !route || !state.selected || state.checklist.page?.enabled === false,
    },
    {
      kind: 'details-open',
      id: 'details-open',
      label: text.detailsTitle,
      hint: '',
      disabled:
        !route ||
        !state.selected ||
        state.loading ||
        executionPending(state) ||
        mutationPending(state) ||
        catalogPending(state),
    },
    {
      kind: 'navigation-toggle',
      id: 'navigation-toggle',
      label: sidebarLabel(state, text),
      hint: 'Ctrl/⌘ B',
      disabled: !shellRoute,
    },
    ...(target === 'tui'
      ? [
          {
            kind: 'appearance-open' as const,
            id: 'appearance-open',
            label: tuiText(locale, 'settings-appearance'),
            hint: '',
            disabled: !shellRoute,
          },
        ]
      : []),
    ...(state.embedding.workhub && !state.embedding.fixed
      ? [
          {
            kind: 'workhub-open' as const,
            id: 'workhub-open',
            label: 'WorkHub',
            hint: '',
            disabled: !shellRoute,
          },
        ]
      : []),
    {
      kind: 'apps-open',
      id: 'apps-open',
      label: text.appsTitle,
      hint: '',
      disabled: !shellRoute || !!state.embedding.fixed,
    },
    {
      kind: 'scheduler-open',
      id: 'scheduler-open',
      label: text.scheduleTitle,
      hint: '',
      disabled: !shellRoute || !!state.embedding.fixed || !state.scheduledTasks,
    },
    {
      kind: 'recall-open',
      id: 'recall-open',
      label: text.recallTitle,
      hint: '',
      disabled: !shellRoute || !!state.embedding.fixed,
    },
    {
      kind: 'imports-open',
      id: 'imports-open',
      label: text.importsTitle,
      hint: '',
      disabled: !shellRoute || !!state.embedding.fixed,
    },
    {
      kind: 'projects-open',
      id: 'projects-open',
      label: text.projectsTitle,
      hint: '',
      disabled: !shellRoute || !!state.embedding.fixed,
    },
    {
      kind: 'settings',
      id: 'settings',
      label: target === 'tui' ? tuiText(locale, 'route-settings') : text.settings,
      hint: '',
      disabled: !shellRoute,
    },
    {
      kind: 'deployment-open',
      id: 'deployment-open',
      label: text.deploymentTitle,
      hint: '',
      disabled: !shellRoute,
    },
    {
      kind: 'reload',
      id: `reload:${state.epoch}`,
      label: text.refreshSessions,
      hint: '',
      disabled: !shellRoute || state.loading,
    },
  ];
  if (state.run)
    commands.push({
      kind: 'stop',
      id: `stop:${state.epoch}:${state.run.invocationId}`,
      label: text.stop,
      hint: '',
      disabled: !route || executionPending(state) || mutationPending(state) || !state.selected,
    });
  if (state.uncertain)
    commands.push({
      kind: 'check',
      id: `check:${state.uncertain.reference.operation}:${state.uncertain.reference.fingerprint}`,
      label: text.check,
      hint: '',
      disabled:
        !route || Object.values(state.pending).some((request) => request.kind === 'receipt'),
    });
  return state.embedding.fixed
    ? commands.filter(
        (command) => !['create', 'create-workspace', 'navigation-toggle'].includes(command.kind),
      )
    : commands;
}

export function openUsage(state: State, session: AgentSession) {
  state.insightsScope = { sequence: ++state.serial, sessionId: session.id, label: session.title };
  state.settingsCategory = 'insights';
  state.pages = ['settings'];
  controlSidebar(state, 'dismiss_transient');
}

export function sidebarLabel(state: State, text: ReturnType<typeof copy>) {
  return 'open' in state.sidebar
    ? state.sidebar.open
      ? text.hideSidebar
      : text.showSidebar
    : text.toggleSidebar;
}

export function controlSidebar(state: State, op: TerminalPaneCommand['op']) {
  if ('open' in state.sidebar) {
    if (op === 'toggle') state.sidebar.open = !state.sidebar.open;
  } else {
    state.sidebar.command = { sequence: (state.sidebar.command?.sequence ?? 0) + 1, op };
  }
}

export function rootCommand(
  state: State,
  actions: Action[],
  locale: string,
  kind: RootCommand,
  target: 'tui' | 'desktop',
) {
  if (
    !rootCommands(state, locale, target).some(
      (command) => command.kind === kind && !command.disabled,
    )
  )
    return false;
  if (kind === 'outputs-open') {
    state.inspector = 'outputs';
    const visit = state.visits.find((entry) => entry.id === state.selected?.id);
    if (visit) {
      const first = toolCards(messages(state)).find(
        ({ message }) => message.role === 'assistant' || message.tool,
      )?.message;
      visit.outputFocus = {
        sequence: ++state.serial,
        target: first ? { kind: 'focus_item', key: 'output-item-' + first.id } : { kind: 'start' },
      };
    }
  } else if (kind === 'browse') {
    state.searchOpen = false;
    state.browse = ++state.serial;
    state.tailReceipt = null;
  } else if (kind === 'search-open') {
    state.searchOpen = true;
    state.browse = null;
  } else if (kind === 'help-open') state.dialog = 'help';
  else if (kind === 'interaction-open') {
    const review = reviewTarget(state)!;
    openInteraction(state, actions, review.requestId, review.kind);
  } else if (kind === 'create') {
    state.pages = [];
    createSession(state, actions, locale);
  } else if (kind === 'appearance-open') state.preferences.open = true;
  else if (kind === 'model-setup') openModelSetup(state);
  else if (kind === 'create-workspace') {
    state.pages = [];
    state.dialog = 'directories';
    if (target === 'tui') controlSidebar(state, 'dismiss_transient');
  } else if (kind === 'terminal-open') {
    if (target === 'tui') state.inspector = state.inspector === 'terminal' ? null : 'terminal';
    else state.dialog = 'terminal';
  } else if (kind === 'skills-open') state.dialog = 'skills';
  else if (kind === 'checklist-open') {
    if (!state.checklist.watch) watchChecklist(state);
    if (target === 'tui') state.inspector = state.inspector === 'checklist' ? null : 'checklist';
    else state.dialog = 'checklist';
  } else if (kind === 'details-open') {
    if (target === 'tui') state.inspector = state.inspector === 'details' ? null : 'details';
    else state.dialog = 'details';
  } else if (kind === 'usage-open' && state.selected) openUsage(state, state.selected);
  else if (kind === 'navigation-toggle') controlSidebar(state, 'toggle');
  else if (kind === 'settings') {
    state.insightsScope = { sequence: ++state.serial, sessionId: null, label: '' };
    state.pages = [target === 'tui' ? 'settings' : 'connections'];
    if (target === 'tui') controlSidebar(state, 'dismiss_transient');
  } else if (kind === 'workhub-open') {
    if (target === 'tui') controlSidebar(state, 'dismiss_transient');
  } else if (kind === 'apps-open') {
    state.pages = ['apps'];
    if (target === 'tui') controlSidebar(state, 'dismiss_transient');
  } else if (kind === 'scheduler-open') {
    state.pages = ['scheduler'];
    if (!state.scheduleWatch) state.scheduleWatch = 'schedule-alerts-' + ++state.serial;
    if (target === 'tui') controlSidebar(state, 'dismiss_transient');
  } else if (kind === 'recall-open') {
    state.pages = ['recall'];
    if (target === 'tui') controlSidebar(state, 'dismiss_transient');
  } else if (kind === 'imports-open') {
    state.pages = ['imports'];
    if (target === 'tui') controlSidebar(state, 'dismiss_transient');
  } else if (kind === 'projects-open') {
    state.pages = ['projects'];
    if (target === 'tui') controlSidebar(state, 'dismiss_transient');
  } else if (kind === 'deployment-open') {
    state.pages = ['deployment'];
    if (target === 'tui') controlSidebar(state, 'dismiss_transient');
  } else if (kind === 'stop' && state.selected && state.run)
    intent(state, actions, { kind: 'stop', sessionId: state.selected.id }, 'agent', 'stop', {
      sessionId: state.selected.id,
      invocationId: state.run.invocationId,
    });
  else if (kind === 'check' && state.uncertain)
    intent(state, actions, { kind: 'receipt' }, 'receipts', 'read', state.uncertain.reference);
  else if (kind === 'reload') {
    if (state.restoring)
      intent(state, actions, { kind: 'restorePage' }, 'history', 'page', {
        after: state.restoreAfter,
        limit: 1,
      });
    else if (!state.selected && state.embedding.session) state.externalSelection = null;
    else if (!state.selected) catalog(state);
    else {
      const window = state.window;
      select(state, actions, state.selected);
      state.window = window;
    }
  }
  return true;
}

export function loadQueueEdit(state: State, actions: Action[], target: AgentQueueMessage) {
  const attempt = ++state.serial;
  state.queueEdit = {
    attempt,
    target,
    phase: 'loading',
    hasAttachments: false,
    body: null,
    command: null,
  };
  state.dialog = 'queue';
  state.notice = null;
  intent(state, actions, { kind: 'queueBody', attempt }, 'agent', 'queueMessage', target);
}

export function initializeQueueEdit(state: State) {
  const edit = state.queueEdit,
    draft = state.queueDraft;
  if (!edit || edit.body === null || !draft || edit.command) return;
  edit.command = {
    owner: draft.snapshot.text.owner,
    sequence: ++state.serial,
    basedOn: draft.revision,
    edit: { kind: 'reset', text: edit.body },
  };
  edit.body = null;
}

export function questionChild(state: State) {
  return `question-form-${state.interaction?.attempt ?? 0}`;
}

export function readInteraction(state: State, actions: Action[]) {
  const question = state.interaction;
  if (
    !question ||
    Object.values(state.pending).some(
      (r) => r.kind === 'interaction' && r.attempt === question.attempt,
    )
  )
    return;
  intent(
    state,
    actions,
    {
      kind: 'interaction',
      attempt: question.attempt,
      sessionId: question.sessionId,
      requestId: question.requestId,
    },
    'agent',
    'interaction',
    { sessionId: question.sessionId, requestId: question.requestId },
  );
}

export function respondQuestion(state: State, actions: Action[]) {
  const submission = state.questionSubmission,
    question = state.interaction;
  if (!submission || submission.waiting.length) return;
  state.questionSubmission = null;
  if (
    !question ||
    submission.attempt !== question.attempt ||
    state.dialog !== 'question' ||
    state.selected?.id !== question.sessionId ||
    state.selected.archived ||
    !question.record ||
    question.record.outcome
  )
    return;
  respondInteraction(state, actions, { kind: 'question', answers: submission.answers });
}

export function respondInteraction(state: State, actions: Action[], answer: InteractionAnswer) {
  const interaction = state.interaction;
  if (
    !interaction?.record ||
    interaction.record.outcome ||
    interaction.record.request.kind !== answer.kind ||
    state.selected?.id !== interaction.sessionId ||
    state.selected.archived ||
    mutationPending(state)
  )
    return;
  intent(
    state,
    actions,
    {
      kind: 'respond',
      attempt: interaction.attempt,
      sessionId: interaction.sessionId,
      requestId: interaction.requestId,
    },
    'agent',
    'respond',
    {
      sessionId: interaction.sessionId,
      requestId: interaction.requestId,
      answer,
    },
  );
}

export function submitQuestion(
  state: State,
  actions: Action[],
  event: { requestId: string; answers: QuestionDraft[] },
) {
  const question = state.interaction;
  if (
    !question ||
    !question.record ||
    question.record.request.kind !== 'question' ||
    question.record.outcome ||
    state.dialog !== 'question' ||
    state.selected?.id !== question.sessionId ||
    state.selected.archived ||
    event.requestId !== question.requestId ||
    mutationPending(state)
  )
    return;
  const questions = question.record.request.questions;
  if (event.answers.length !== questions.length) return;
  const answers: (string | null)[] = [],
    waiting: number[] = [];
  for (const [index, answer] of event.answers.entries()) {
    if (answer.kind === 'skip') answers.push(null);
    else if (answer.kind === 'option' && questions[index].options[answer.index])
      answers.push(questions[index].options[answer.index].label);
    else if (
      answer.kind === 'text' &&
      !answer.observation.snapshot.marked &&
      answer.observation.snapshot.text.bytes > 0 &&
      answer.observation.snapshot.text.bytes <= 2048
    ) {
      answers.push(null);
      waiting.push(index);
    } else {
      question.notice = 'questionInvalid';
      return;
    }
  }
  question.notice = null;
  state.questionSubmission = { attempt: question.attempt, answers, waiting };
  for (const index of waiting) {
    const answer = event.answers[index];
    if (answer.kind !== 'text') continue;
    const draft = answer.observation;
    intent(
      state,
      actions,
      { kind: 'questionRead', attempt: question.attempt, index, draft },
      'reader',
      'read',
      { start: 0, end: draft.snapshot.text.bytes },
      {
        text: {
          child: questionChild(state),
          method: 'snapshot',
          input: {
            requestId: question.requestId,
            index,
            snapshot: {
              kind: 'at_revision',
              owner: draft.snapshot.text.owner,
              revision: draft.snapshot.text.revision,
            },
          },
        },
      },
    );
  }
  respondQuestion(state, actions);
}

export function checkInteraction(state: State, actions: Action[]) {
  if (state.uncertain && !Object.values(state.pending).some((r) => r.kind === 'receipt'))
    intent(state, actions, { kind: 'receipt' }, 'receipts', 'read', state.uncertain.reference);
  readInteraction(state, actions);
}

export function focusAttachments(state: State) {
  const selection = state.attachments.selection;
  if (!selection || selection.browse) return;
  const files = attachmentFiles(state.attachments, selection.sessionId, selection.dataSource);
  const index = files.findIndex((file) => file.id === selection.selected);
  selection.focus = {
    sequence: ++state.serial,
    target: {
      kind: 'focus_item',
      key: index < 0 ? 'attachment-browse' : 'attachment-select-' + index,
    },
  };
}

export function closeDialog(state: State) {
  // Cancel follows the original queue sheet: discard this edit, not an accepted operation.
  if (state.dialog === 'queue') state.queueEdit = null;
  if (state.dialog === 'question') state.questionSubmission = null;
  state.dialog = null;
}
