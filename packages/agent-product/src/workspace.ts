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

import {
  initialAppearance,
  effectiveAppearance,
  effectiveLocale,
  appearanceEvent,
  appearanceView,
  type AppearanceEditor,
} from './appearance-editor.js';
import { settingsView, settingsEvent, type SettingsCategory } from './settings.js';
import { tuiText } from './tui-locale.js';
import type {
  Action,
  ActionOutcome,
  Shortcut,
  ModelSelection,
  AgentExecutorSelection,
  ThinkingLevel,
  AgentDeliveryPage,
  AgentQueue,
  AgentPendingInteractions,
  InteractionRecord,
  InteractionAnswer,
  AgentInteractionResult,
  AgentQueueMessage,
  AgentQueueMessageResult,
  AgentHistoryPage,
  AgentHistoryChunk,
  AgentHistorySourcePage,
  SourceField,
  SourceFormat,
  SourcePosition,
  HistoryPosition,
  AgentObservation,
  AgentPart,
  AgentRun,
  AgentSession,
  AgentDataSource,
  AgentSessionPage,
  AgentCreated,
  AgentMarkReadResult,
  AgentDirectoryPage,
  AgentDirectorySelection,
  AgentProjectRef,
  AgentList,
  AgentSendResult,
  AgentAttachmentUploadResult,
  FileObservation,
  FileCapture,
  AgentQueueMutationResult,
  AgentQueueAction,
  RowsProps,
  RowActivated,
  CallerIntent,
  Component,
  ComponentInput,
  ComponentValues,
  OperationReply,
  RecoveryRef,
  EditorCommand,
  EditorObservation,
  TextChunk,
  TerminalPaneCommand,
} from '../../plugin-sdk/src/index.js';
import type {
  WorkspaceProps,
  PaletteCommand,
  Message,
  Theme,
  QueueEditorProps,
  QuestionDraft,
  QuestionEditorProps,
  ServiceApprovalProps,
  SelectedModel,
  ModelSetup,
  TimelineProps,
  TimelineFileContext,
  MessageCopyObservation,
} from './contracts.js';
import { sameValue } from './value.js';
import {
  bookmark,
  bookmarkReady,
  bookmarkView,
  observeBookmark,
  retryBookmark,
  type Bookmark,
} from './bookmark.js';
import { copy, resolveLocale, type Copy } from './locale.js';
import { compactLabel, fill, output, pixels, type Presentation, type Node } from './view.js';

import {
  attachmentFiles,
  attachmentCapacity,
  attachmentChild,
  attachmentView,
  attachmentSummary,
  pastedFiles,
  type DraftFilePaste,
  pasteView,
  sameFile,
  selectedFile,
  type AttachmentDrafts,
} from './attachments.js';

import {
  observeDraft,
  draftReady,
  retainSelectedDraft,
  draftStatuses,
  nextDraftStep,
  draftPage,
  draftReceipt,
  draftFailed,
  retryDraft,
  type DraftRecord,
  type DraftStep,
  type DraftCheckpoint,
} from './drafts.js';

type Notice = keyof Copy | null;
type Backend =
  | { kind: 'model'; model: ModelSelection; thinkingLevel: ThinkingLevel | null }
  | { kind: 'executor'; selection: AgentExecutorSelection };
type Request =
  | DraftStep
  | { kind: 'observe'; epoch: number }
  | {
      kind: 'history';
      epoch: number;
      through: number;
      navigation: number;
      latest: boolean;
      implicit: boolean;
      extend?: boolean;
      position: HistoryPosition;
    }
  | {
      kind: 'read';
      dataSource: AgentDataSource;
      attachments: string[];
      draft: EditorObservation;
      sessionId: string;
      backend: Backend;
    }
  | {
      kind: 'source';
      epoch: number;
      navigation: number;
      through: number;
      sequence: number;
      field: SourceField;
      position: SourcePosition;
    }
  | { kind: 'interaction'; attempt: number; sessionId: string; requestId: string }
  | { kind: 'questionRead'; attempt: number; index: number; draft: EditorObservation }
  | { kind: 'respond'; attempt: number; sessionId: string; requestId: string }
  | { kind: 'openSession'; selection: NonNullable<WorkspaceProps['session']> }
  | { kind: 'create' }
  | { kind: 'createDirectory'; title: string }
  | {
      kind: 'send';
      dataSource: AgentDataSource;
      attachments: string[];
      draft: EditorObservation;
      sessionId: string;
      backend: Backend;
    }
  | {
      kind: 'uploadAttachment';
      sessionId: string;
      dataSource: AgentDataSource;
      attempt: number;
      pasted?: FileCapture;
    }
  | { kind: 'markRead'; sessionId: string }
  | { kind: 'stop'; sessionId: string }
  | { kind: 'queue'; sessionId: string; saved?: { attempt: number; draft: EditorObservation } }
  | { kind: 'queueBody'; attempt: number }
  | { kind: 'queueRead'; attempt: number; draft: EditorObservation }
  | { kind: 'receipt' }
  | { kind: 'restorePage' }
  | { kind: 'restoreReceipt'; reference: RecoveryRef; cursor: number };
type Mutation =
  | Extract<
      Request,
      { kind: 'create' | 'send' | 'stop' | 'queue' | 'respond' | 'markRead' | 'uploadAttachment' }
    >
  | { kind: 'restored' };
interface DisplayPart extends AgentPart {
  invocationId: string | null;
}
interface Uncertain {
  reference: RecoveryRef;
  request: Mutation;
}
interface QueueEdit {
  hasAttachments: boolean;
  attempt: number;
  target: AgentQueueMessage;
  phase: 'loading' | 'initializing' | 'ready' | 'failed';
  body: string | null;
  command: EditorCommand | null;
}
type Navigation = { pages: State['pages']; category: SettingsCategory };
function navigationBookmark(state: State): Navigation {
  return { pages: state.pages, category: state.settingsCategory };
}
function navigationKey(state: State): string {
  return (
    'navigation/' +
    (state.embedding.fixed ? (state.embedding.session?.id ?? 'pending') : 'workspace')
  );
}
export interface State {
  bookmark: Bookmark<Navigation>;
  embedding: WorkspaceProps;
  externalSelection: WorkspaceProps['session'] | null;
  preferences: AppearanceEditor;
  drafts: DraftRecord[];
  attachments: AttachmentDrafts;
  revision: number;
  serial: number;
  epoch: number;
  catalog: AgentSessionPage | null;
  catalogGeneration: number;
  catalogWatch: { key: string; input: AgentList; loading: boolean } | null;
  catalogFilter: AgentList['filter'];
  catalogFailed: boolean;
  selected: AgentSession | null;
  model: SelectedModel | null;
  executor: AgentExecutorSelection | null;
  hasBackends: boolean;
  modelSetup: ModelSetup['kind'];
  modelOpenRequest: number | null;
  catalogEpoch: number;
  modelRefreshing: boolean;
  searchOpen: boolean;
  browse: number | null;
  pages: ('settings' | 'connections' | 'deployment' | 'projects')[];
  projectsOpened: boolean;
  settingsCategory: SettingsCategory;
  sidebar: { open: boolean } | { command: TerminalPaneCommand | null };
  terminals: boolean;
  skills: boolean;
  place: string;
  dialog:
    | 'attachments'
    | 'terminal'
    | 'skills'
    | 'palette'
    | 'directories'
    | 'details'
    | 'help'
    | 'queue'
    | 'question'
    | 'approval'
    | null;
  window: {
    page: Extract<AgentHistoryPage, { kind: 'ready' }>;
    latest: boolean;
  } | null;
  navigation: number;
  image: { sessionId: string; through: number; sequence: number } | null;
  source: {
    through: number;
    chunk: Pick<AgentHistoryChunk, 'sequence' | 'offset' | 'totalBytes' | 'text'>;
    navigation: number;
    landing: { sequence: number; edge: 'start' | 'end' };
    loading: boolean;
    failed: boolean;
    field: SourceField;
    fields: SourceField[];
    format: SourceFormat | null;
    ready: boolean;
    position: SourcePosition;
  } | null;
  landing: { sequence: number; edge: 'start' | 'end' };
  tailReceipt: { landing: number; readingRevision: number } | null;
  fileContext: TimelineFileContext | null;
  copyRequest: number | null;
  messageCopy: MessageCopyObservation | null;
  followAttempt: {
    through: number;
    landing: number;
    readingRevision: number;
  } | null;
  parts: DisplayPart[];
  run: AgentRun | null;
  queue: AgentQueue | null;
  queueEdit: QueueEdit | null;
  queueDraft: EditorObservation | null;
  interactions: AgentPendingInteractions | null;
  interaction: {
    attempt: number;
    sessionId: string;
    requestId: string;
    record: InteractionRecord | null;
    notice: Notice;
  } | null;
  questionSubmission: {
    attempt: number;
    answers: (string | null)[];
    waiting: number[];
  } | null;
  runThrough: number;
  loading: boolean;
  live: {
    sequence: number;
    messageId: string;
    invocationId: string;
    stepId: string;
    text: string;
    closedAt: number | null;
    field: 'body' | 'thinking';
  }[];
  latestKnown: number;
  watch: {
    key: string;
    input: { sessionId: string; after: number };
    last: number;
  } | null;
  pending: Record<string, Request>;
  uncertain: Uncertain | null;
  restoring: boolean;
  restoreAfter: number;
  command: EditorCommand | null;
  commandSequence: number;
  notice: Notice;
}
function utf8Bytes(text: string): number {
  let bytes = 0;
  for (const character of text) {
    const point = character.codePointAt(0)!;
    bytes += point <= 0x7f ? 1 : point <= 0x7ff ? 2 : point <= 0xffff ? 3 : 4;
  }
  return bytes;
}
// Raw fences can advance while model text is still open. Only an actual step
// closure covers its live prefix for both atomic and item-accepting providers.
function appendLive(
  state: State,
  sequence: number,
  part: DisplayPart,
  invocationId: string,
  text: string,
) {
  const messageId = part.messageId;
  const field = part.textKind === 'thinking' ? 'thinking' : 'body';
  if (
    state.window &&
    sequence <= state.window.page.through &&
    state.window.page.chunks.some(
      (chunk) => chunk.messageId === messageId && (chunk.field ?? 'body') === field,
    )
  )
    return;
  const ids = new Set(state.live.map((fragment) => fragment.messageId));
  let remaining = 16000 - state.live.reduce((total, fragment) => total + fragment.text.length, 0);
  // Thinking shares the existing prefix budget but leaves room for an answer
  // before atomic providers publish either field. Never evict a displayed prefix.
  if (field === 'thinking') {
    const thinking = state.live.filter((fragment) => fragment.field === 'thinking');
    const thinkingIds = new Set(thinking.map((fragment) => fragment.messageId));
    if (thinking.length >= 32 || (!thinkingIds.has(messageId) && thinkingIds.size >= 8)) return;
    remaining = Math.min(
      remaining,
      8000 - thinking.reduce((total, fragment) => total + fragment.text.length, 0),
    );
  }
  // A truncated surrogate can leave one unused code unit; keep the prefix closed.
  if (remaining <= 1 || state.live.length >= 64 || (!ids.has(messageId) && ids.size >= 16)) return;
  text = text.slice(0, remaining);
  const last = text.charCodeAt(text.length - 1);
  if (last >= 0xd800 && last <= 0xdbff) text = text.slice(0, -1);
  if (text)
    state.live.push({
      sequence,
      messageId,
      invocationId,
      stepId: part.stepId,
      text,
      closedAt: null,
      field,
    });
}
function closeLive(state: State, sequence: number, invocationId: string, stepId?: string) {
  for (const fragment of state.live)
    if (
      fragment.closedAt === null &&
      fragment.invocationId === invocationId &&
      (stepId === undefined || fragment.stepId === stepId)
    )
      fragment.closedAt = sequence;
}
function messages(state: State): Message[] {
  const canonical: Message[] = (state.window?.page.chunks ?? []).map((chunk) => ({
    id: chunk.field === 'thinking' ? 'thinking-' + chunk.messageId : chunk.messageId,
    contentKey: `canonical-${chunk.sequence}-${chunk.offset}${chunk.field === 'thinking' ? '-thinking' : ''}`,
    role: chunk.field === 'thinking' ? 'thinking' : chunk.role,
    timestamp: chunk.timestamp,
    text: chunk.text,
    tool: chunk.tool,
    continued: chunk.offset !== 0 || utf8Bytes(chunk.text) !== chunk.totalBytes,
    source: chunk.sequence,
    image: chunk.tool?.result?.image ?? false,
  }));
  if (!state.window?.latest) return canonical;
  const live = new Map<string, Message>();
  for (const fragment of state.live) {
    if (
      fragment.sequence <= state.window.page.through &&
      state.window.page.chunks.some(
        (chunk) =>
          chunk.messageId === fragment.messageId && (chunk.field ?? 'body') === fragment.field,
      )
    )
      continue;
    const key = fragment.messageId + (fragment.field === 'thinking' ? '-thinking' : '');
    let message = live.get(key);
    if (!message) {
      message = {
        id: 'live-' + key,
        contentKey: 'live-' + key,
        role: fragment.field === 'thinking' ? 'thinking' : 'assistant',
        timestamp: null,
        text: '',
        tool: null,
        continued: true,
        source: null,
        image: false,
      };
      live.set(key, message);
    }
    message.text += fragment.text;
  }
  return [...canonical, ...live.values()];
}
function turnTimings(state: State, displayed: Message[]): TimelineProps['timings'] {
  const turns = new Map<string, TimelineProps['timings'][number]>();
  for (const chunk of state.window?.page.chunks ?? []) {
    if (!chunk.timing) continue;
    if (!chunk.timing.finished && (!state.window?.latest || state.run?.turnId !== chunk.turnId))
      continue;
    turns.set(chunk.turnId, {
      ...chunk.timing,
      turnId: chunk.turnId,
      afterMessageId: chunk.field === 'thinking' ? 'thinking-' + chunk.messageId : chunk.messageId,
    });
  }
  if (state.window?.latest && state.run) {
    const timing = turns.get(state.run.turnId);
    if (timing && !timing.finished) {
      for (const fragment of state.live) {
        const id =
          'live-' + fragment.messageId + (fragment.field === 'thinking' ? '-thinking' : '');
        if (
          fragment.invocationId === state.run.invocationId &&
          displayed.some((message) => message.id === id)
        )
          timing.afterMessageId = id;
      }
    }
  }
  return [...turns.values()];
}
function intent(
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
function canExecute(state: State) {
  return !!state.executor || (!!state.model && !state.modelRefreshing);
}
function selectedBackend(state: State): Backend {
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
  if (!canExecute(state)) {
    openModelSetup(state);
    return;
  }
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
function openModelSetup(state: State) {
  if (state.modelSetup === 'connect' && !state.hasBackends) {
    state.pages = ['connections'];
  } else state.modelOpenRequest = ++state.serial;
}
function catalogPending(state: State): boolean {
  return state.catalogWatch?.loading ?? false;
}
function catalog(state: State, position?: AgentList['position']) {
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
function observe(state: State, actions: Action[]) {
  if (state.selected)
    intent(state, actions, { kind: 'observe', epoch: state.epoch }, 'agent', 'observe', {
      sessionId: state.selected.id,
    });
}
function history(
  state: State,
  actions: Action[],
  position: HistoryPosition,
  through: number,
  latest: boolean,
  implicit = false,
  extend = false,
) {
  if (!state.selected) return;
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
    implicit ? [{ child: 'timeline', method: 'tail', input: null }] : [],
  );
}
function follow(state: State, actions: Action[]) {
  const receipt = state.tailReceipt;
  if (
    receipt?.landing === state.landing.sequence &&
    !state.source &&
    !state.searchOpen &&
    state.browse === null &&
    !state.image &&
    !state.loading &&
    state.window?.latest &&
    (state.latestKnown > state.window.page.through ||
      state.live.some(
        (fragment) => fragment.closedAt !== null && fragment.closedAt <= state.window!.page.through,
      )) &&
    !Object.values(state.pending).some(
      (request) =>
        request.kind === 'history' &&
        request.epoch === state.epoch &&
        request.navigation === state.navigation,
    ) &&
    !(
      state.followAttempt?.through === state.latestKnown &&
      state.followAttempt.landing === receipt.landing &&
      state.followAttempt.readingRevision === receipt.readingRevision
    )
  ) {
    state.followAttempt = { ...receipt, through: state.latestKnown };
    history(state, actions, { kind: 'tail' }, state.latestKnown, true, true);
  }
}
function select(state: State, actions: Action[], session: AgentSession | null) {
  state.searchOpen = false;
  state.browse = null;
  state.selected = session;
  state.epoch++;
  state.window = null;
  state.source = null;
  state.image = null;
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
  state.live = [];
  state.latestKnown = 0;
  state.loading = session !== null;
  state.notice = null;
  if (session) observe(state, actions);
}
function readHistorySource(state: State, actions: Action[], position: SourcePosition) {
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
function mutationPending(state: State) {
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
function uploadPastedFile(state: State, actions: Action[]) {
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
function executionPending(state: State) {
  return Object.values(state.pending).some(
    (request) => request.kind === 'observe' && request.epoch === state.epoch,
  );
}

type RootCommand =
  | 'browse'
  | 'copy-file-path'
  | 'copy-message'
  | 'copy-source'
  | 'search-open'
  | 'help-open'
  | 'interaction-open'
  | 'create'
  | 'appearance-open'
  | 'model-setup'
  | 'terminal-open'
  | 'skills-open'
  | 'details-open'
  | 'create-workspace'
  | 'navigation-toggle'
  | 'settings'
  | 'projects-open'
  | 'deployment-open'
  | 'reload'
  | 'stop'
  | 'check';
function conversationRoute(state: State) {
  return state.pages.length === 0;
}
function reviewTarget(state: State) {
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
      ['question', 'form', 'service_approval', 'permissions'].includes(entry.kind) &&
      !(
        retained?.sessionId === state.selected?.id &&
        retained?.requestId === entry.requestId &&
        retained.record?.outcome
      ),
  );
  return entry ? { ...entry, retained: false } : null;
}
function openInteraction(state: State, actions: Action[], requestId: string, kind: string) {
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
function rootCommands(
  state: State,
  locale: string,
  target: 'tui' | 'desktop',
): (PaletteCommand & { kind: RootCommand })[] {
  const text = copy(locale);
  const route = conversationRoute(state) && (state.dialog === null || state.dialog === 'palette');
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
        state.loading ||
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
      disabled: !route || mutationPending(state) || catalogPending(state),
    },
    {
      kind: 'create-workspace',
      id: 'create-workspace',
      label: text.newWorkspace,
      hint: '',
      disabled: !route || mutationPending(state) || catalogPending(state),
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
      disabled: !route,
    },
    ...(target === 'tui'
      ? [
          {
            kind: 'appearance-open' as const,
            id: 'appearance-open',
            label: tuiText(locale, 'settings-appearance'),
            hint: '',
            disabled: !route,
          },
        ]
      : []),
    {
      kind: 'projects-open',
      id: 'projects-open',
      label: text.projectsTitle,
      hint: '',
      disabled: !route || !!state.embedding.fixed,
    },
    {
      kind: 'settings',
      id: 'settings',
      label: target === 'tui' ? tuiText(locale, 'route-settings') : text.settings,
      hint: '',
      disabled: !route,
    },
    {
      kind: 'deployment-open',
      id: 'deployment-open',
      label: text.deploymentTitle,
      hint: '',
      disabled: !route,
    },
    {
      kind: 'reload',
      id: `reload:${state.epoch}`,
      label: text.refreshSessions,
      hint: '',
      disabled: !route || state.loading,
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
function sidebarLabel(state: State, text: ReturnType<typeof copy>) {
  return 'open' in state.sidebar
    ? state.sidebar.open
      ? text.hideSidebar
      : text.showSidebar
    : text.toggleSidebar;
}
function controlSidebar(state: State, op: TerminalPaneCommand['op']) {
  if ('open' in state.sidebar) {
    if (op === 'toggle') state.sidebar.open = !state.sidebar.open;
  } else {
    state.sidebar.command = { sequence: (state.sidebar.command?.sequence ?? 0) + 1, op };
  }
}
function rootCommand(
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
  if (kind === 'browse') {
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
  } else if (kind === 'create') createSession(state, actions, locale);
  else if (kind === 'appearance-open') state.preferences.open = true;
  else if (kind === 'model-setup') openModelSetup(state);
  else if (kind === 'create-workspace') {
    if (!canExecute(state)) openModelSetup(state);
    else state.dialog = 'directories';
    if (target === 'tui') controlSidebar(state, 'dismiss_transient');
  } else if (kind === 'terminal-open') state.dialog = 'terminal';
  else if (kind === 'skills-open') state.dialog = 'skills';
  else if (kind === 'details-open') state.dialog = 'details';
  else if (kind === 'navigation-toggle') controlSidebar(state, 'toggle');
  else if (kind === 'settings') {
    state.pages = [target === 'tui' ? 'settings' : 'connections'];
  } else if (kind === 'projects-open') {
    state.pages = ['projects'];
    state.projectsOpened = true;
    if (target === 'tui') controlSidebar(state, 'dismiss_transient');
  } else if (kind === 'deployment-open') {
    state.pages = ['deployment'];
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

function loadQueueEdit(state: State, actions: Action[], target: AgentQueueMessage) {
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
function initializeQueueEdit(state: State) {
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
function questionChild(state: State) {
  return `question-form-${state.interaction?.attempt ?? 0}`;
}
function readInteraction(state: State, actions: Action[]) {
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
function respondQuestion(state: State, actions: Action[]) {
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
function respondInteraction(state: State, actions: Action[], answer: InteractionAnswer) {
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
function submitQuestion(
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
function checkInteraction(state: State, actions: Action[]) {
  if (state.uncertain && !Object.values(state.pending).some((r) => r.kind === 'receipt'))
    intent(state, actions, { kind: 'receipt' }, 'receipts', 'read', state.uncertain.reference);
  readInteraction(state, actions);
}
function focusAttachments(state: State) {
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
function closeDialog(state: State) {
  // Cancel follows the original queue sheet: discard this edit, not an accepted operation.
  if (state.dialog === 'queue') state.queueEdit = null;
  if (state.dialog === 'question') state.questionSubmission = null;
  state.dialog = null;
}
function settled(state: State, actions: Action[], request: Mutation, value: unknown) {
  state.uncertain = null;
  if (request.kind === 'restored') {
    state.restoring = true;
    intent(state, actions, { kind: 'restorePage' }, 'history', 'page', {
      after: state.restoreAfter,
      limit: 1,
    });
    catalog(state);
  } else if (request.kind === 'create') {
    const created = value as AgentCreated;
    if (created.kind === 'rejected') {
      if (state.notice !== 'unavailable') state.notice = 'createRejected';
      return;
    }
    // The live catalog may have selected this first conversation before its
    // original creation receipt arrived. Keep that observation and its owners.
    if (state.selected?.id !== created.session.id) select(state, actions, created.session);
    else if (created.session.revision >= state.selected.revision) state.selected = created.session;
    // Explicit creation reveals its selected row; background catalog changes
    // continue to retain the current directory location.
    if (
      state.catalogFilter !== 'all' ||
      !state.catalog?.sessions.some((session) => session.id === created.session.id)
    ) {
      state.catalogFilter = 'all';
      catalog(state, { kind: 'at', cursor: created.session.id });
    }
  } else if (request.kind === 'markRead') {
    const result = value as AgentMarkReadResult;
    if (result.kind === 'observed' && result.session.id === request.sessionId) {
      if (
        state.selected?.id === request.sessionId &&
        result.session.revision >= state.selected.revision
      )
        state.selected = result.session;
      if (state.catalog)
        state.catalog.sessions = state.catalog.sessions.map((session) =>
          session.id === request.sessionId && result.session.revision >= session.revision
            ? result.session
            : session,
        );
      state.notice = null;
    } else state.notice = 'unavailable';
  } else if (request.kind === 'queue') {
    const result = value as AgentQueueMutationResult;
    state.notice = result.kind === 'committed' ? null : 'queueChanged';
    if (
      result.kind === 'committed' &&
      request.saved &&
      state.queueEdit?.attempt === request.saved.attempt &&
      state.queueDraft?.snapshot.text.owner === request.saved.draft.snapshot.text.owner &&
      state.queueDraft.revision === request.saved.draft.revision
    )
      closeDialog(state);
  } else if (request.kind === 'respond') {
    const result = value as AgentInteractionResult;
    const question = state.interaction;
    if (question?.attempt === request.attempt) {
      if (
        result.kind === 'recorded' &&
        result.record.sessionId === request.sessionId &&
        result.record.requestId === request.requestId &&
        sameValue(result.record.request, question.record?.request)
      ) {
        question.record = result.record;
        question.notice = null;
      } else
        question.notice =
          result.kind === 'rejected' &&
          result.reason === 'invalid_answer' &&
          question.record?.request.kind === 'form'
            ? 'formInvalid'
            : 'interactionChanged';
    }
    if (state.selected?.id === request.sessionId) observe(state, actions);
  } else if (request.kind === 'uploadAttachment') {
    const result = value as AgentAttachmentUploadResult;
    if (result.kind === 'uploaded') {
      if (
        !state.attachments.files.some(
          (file) =>
            file.id === result.attachment.id &&
            file.sessionId === request.sessionId &&
            sameValue(file.dataSource, request.dataSource),
        )
      )
        state.attachments.files.push({ ...result.attachment, dataSource: request.dataSource });
      if (
        request.pasted &&
        state.attachments.paste &&
        sameFile(request.pasted, state.attachments.paste.observed.selection)
      ) {
        state.attachments.paste.command = { selection: request.pasted, action: 'next' };
      }
      if (!request.pasted && state.attachments.selection?.attempt === request.attempt) {
        if (state.dialog === 'attachments') state.dialog = null;
        state.attachments.selection = null;
      }
      state.notice = null;
    } else state.notice = 'attachmentFailed';
  } else if (request.kind === 'send') {
    const started = value as AgentSendResult;
    if (started.sessionId !== request.sessionId) {
      state.notice = 'unknown';
      return;
    }
    if (started.kind === 'rejected') {
      state.notice =
        started.reason === 'input_too_large' ? 'skillInputTooLarge' : 'skillInputUnavailable';
      return;
    }
    state.attachments.files = state.attachments.files.filter(
      (file) =>
        file.sessionId !== request.sessionId ||
        !sameValue(file.dataSource, request.dataSource) ||
        !request.attachments.includes(file.id),
    );
    // Clear only the exact captured draft after a durable returned admission.
    // The native editor rejects this command if a newer draft or IME composition exists.
    state.command = {
      owner: request.draft.snapshot.text.owner,
      sequence: ++state.commandSequence,
      basedOn: request.draft.revision,
      edit: { kind: 'reset', text: '' },
    };
    state.notice = null;
    if (state.selected?.id === started.sessionId) {
      observe(state, actions);
    }
  } else {
    state.notice = 'stopping';
    if (state.selected?.id === request.sessionId) observe(state, actions);
  }
}
function receipt(state: State, actions: Action[], request: Mutation, reply: OperationReply) {
  if (reply.receipt.kind === 'recorded' && reply.receipt.outcome.kind === 'returned') {
    const value = reply.receipt.outcome.value;
    const returned = value as AgentAttachmentUploadResult;
    if (
      (request.kind === 'send' && (value as AgentSendResult).sessionId !== request.sessionId) ||
      (request.kind === 'uploadAttachment' &&
        returned.kind === 'uploaded' &&
        returned.attachment.sessionId !== request.sessionId)
    ) {
      state.uncertain = { reference: reply.reference, request };
      state.notice = 'unknown';
      return;
    }
    settled(state, actions, request, value);
  } else {
    state.uncertain = { reference: reply.reference, request };
    state.notice = 'unknown';
  }
}
function queryFailed(state: State, request: Request) {
  if (request.kind === 'draftPage' || request.kind === 'draftReceipt') {
    draftFailed(state.drafts, request);
    return;
  }
  if (
    request.kind === 'interaction' ||
    request.kind === 'questionRead' ||
    request.kind === 'respond'
  ) {
    if (state.interaction?.attempt === request.attempt)
      state.interaction.notice =
        request.kind === 'questionRead'
          ? 'questionInvalid'
          : request.kind === 'respond'
            ? 'interactionUnknown'
            : 'interactionUnavailable';
    if (state.questionSubmission?.attempt === request.attempt) state.questionSubmission = null;
    return;
  }
  if (request.kind === 'queueBody' || request.kind === 'queueRead') {
    if (state.queueEdit?.attempt === request.attempt) {
      if (request.kind === 'queueBody') state.queueEdit.phase = 'failed';
      state.notice = 'draftKept';
    }
    return;
  }
  // Optional presentation work does not disable input or change execution facts.
  if (request.kind === 'history' && request.implicit) return;
  if (request.kind === 'source') {
    if (request.epoch === state.epoch && request.navigation === state.source?.navigation) {
      state.source.loading = false;
      state.source.failed = true;
    }
  } else if (request.kind === 'history' || request.kind === 'observe') {
    if (
      request.epoch === state.epoch &&
      (request.kind !== 'history' || request.navigation === state.navigation)
    ) {
      state.loading = false;
      state.notice = 'unavailable';
    }
  } else {
    if (request.kind === 'openSession') {
      if (sameValue(request.selection, state.externalSelection)) {
        state.loading = false;
        state.notice = 'unavailable';
      }
    } else if (request.kind === 'createDirectory') {
      if (state.notice !== 'unavailable') state.notice = 'createRejected';
    } else if (request.kind === 'restoreReceipt') {
      state.uncertain = { reference: request.reference, request: { kind: 'restored' } };
      state.restoring = false;
      state.notice = 'unknown';
    } else
      state.notice =
        request.kind === 'read'
          ? 'draftKept'
          : request.kind === 'uploadAttachment'
            ? 'attachmentFailed'
            : 'unavailable';
  }
}
function querySucceeded(state: State, actions: Action[], request: Request, value: unknown) {
  switch (request.kind) {
    case 'openSession': {
      if (!sameValue(request.selection, state.externalSelection)) break;
      const observed = value as AgentObservation;
      if (observed.session.id !== request.selection.id) {
        state.loading = false;
        state.notice = 'unavailable';
        break;
      }
      select(state, actions, observed.session);
      break;
    }
    case 'interaction': {
      const question = state.interaction,
        record = value as InteractionRecord;
      if (!question || question.attempt !== request.attempt) break;
      if (
        record.sessionId !== request.sessionId ||
        record.requestId !== request.requestId ||
        !['question', 'form', 'service_approval', 'permissions'].includes(record.request.kind) ||
        (question.record && !sameValue(question.record.request, record.request))
      ) {
        question.notice = 'interactionUnavailable';
        break;
      }
      if (!question.record?.outcome) question.record = record;
      question.notice = null;
      break;
    }
    case 'questionRead': {
      const submission = state.questionSubmission,
        question = state.interaction;
      if (
        !submission ||
        !question ||
        submission.attempt !== request.attempt ||
        question.attempt !== request.attempt
      )
        break;
      const chunk = value as TextChunk;
      if (
        !sameValue(chunk.metadata, request.draft.snapshot.text) ||
        utf8Bytes(chunk.text) !== request.draft.snapshot.text.bytes ||
        !chunk.text.trim()
      ) {
        state.questionSubmission = null;
        question.notice = 'questionInvalid';
        break;
      }
      submission.answers[request.index] = chunk.text;
      submission.waiting = submission.waiting.filter((index) => index !== request.index);
      respondQuestion(state, actions);
      break;
    }
    case 'createDirectory': {
      const page = value as AgentDirectoryPage;
      if (page.kind !== 'roots' || !page.roots.length) {
        state.notice = 'createRejected';
        break;
      }
      intent(state, actions, { kind: 'create' }, 'agent', 'create', {
        title: request.title,
        labels: [],
        workspace: { kind: 'directory', selection: page.roots[0].selection },
      });
      break;
    }
    case 'observe': {
      if (request.epoch !== state.epoch) break;
      const observation = value as AgentObservation;
      if (observation.session.id !== state.selected?.id) {
        state.loading = false;
        state.notice = 'unavailable';
        break;
      }
      if (observation.session.revision >= state.selected!.revision)
        state.selected = observation.session;
      if (
        observation.through >= state.runThrough &&
        observation.session.revision >= state.selected!.revision
      ) {
        state.run = observation.run ?? null;
        state.runThrough = observation.through;
      }
      if (!state.watch)
        state.parts = observation.parts.map((part) => ({
          ...part,
          invocationId: observation.run?.invocationId ?? null,
        }));
      state.latestKnown = Math.max(state.latestKnown, observation.through);
      // Watch/run facts are independent of the fixed-fence reading window.
      if (!state.watch) {
        const after = Math.min(
          observation.through,
          ...state.parts.map((part) => part.startSequence - 1),
        );
        state.watch = {
          key: 'watch-' + state.epoch,
          input: { sessionId: state.selected!.id, after },
          last: after,
        };
      }
      if (
        !state.window &&
        !Object.values(state.pending).some(
          (pending) => pending.kind === 'history' && pending.epoch === state.epoch,
        )
      )
        history(state, actions, { kind: 'tail' }, observation.through, true);
      else if (
        !Object.values(state.pending).some(
          (pending) =>
            pending.kind === 'history' &&
            pending.epoch === state.epoch &&
            pending.navigation === state.navigation,
        )
      )
        state.loading = false;
      follow(state, actions);
      break;
    }
    case 'source': {
      const source = state.source;
      if (
        request.epoch !== state.epoch ||
        !source ||
        request.navigation !== source.navigation ||
        request.field !== source.field
      )
        break;
      const page = value as AgentHistorySourcePage;
      if (page.through !== request.through) {
        queryFailed(state, request);
        break;
      }
      if (page.kind === 'preparing') {
        intent(state, actions, request, 'agent', 'source', {
          sessionId: state.selected!.id,
          through: request.through,
          source: { kind: 'sequence', sequence: request.sequence },
          field: request.field,
          position: request.position,
        });
        break;
      }
      const next = page.source;
      const end = next.offset + utf8Bytes(next.text);
      if (
        next.sequence !== request.sequence ||
        next.path != null ||
        next.field !== request.field ||
        !next.fields.includes(next.field) ||
        (!next.text && next.totalBytes > 0) ||
        end > next.totalBytes ||
        (source.ready &&
          (next.totalBytes !== source.chunk.totalBytes || next.format !== source.format)) ||
        (request.position.kind === 'after' && next.offset !== request.position.offset) ||
        (request.position.kind === 'before' && end !== request.position.offset) ||
        (request.position.kind === 'start' && next.offset !== 0) ||
        (request.position.kind === 'tail' && end !== next.totalBytes)
      ) {
        queryFailed(state, request);
        break;
      }
      source.chunk = {
        ...source.chunk,
        sequence: next.sequence,
        offset: next.offset,
        totalBytes: next.totalBytes,
        text: next.text,
      };
      source.fields = next.fields;
      source.format = next.format;
      source.ready = true;
      source.loading = false;
      source.failed = false;
      source.landing = {
        sequence: request.navigation,
        edge:
          request.position.kind === 'before' || request.position.kind === 'tail' ? 'end' : 'start',
      };
      break;
    }
    case 'history': {
      if (request.epoch !== state.epoch || request.navigation !== state.navigation) break;
      const page = value as AgentHistoryPage;
      if (page.through !== request.through) {
        queryFailed(state, request);
        break;
      }
      if (page.kind === 'preparing') {
        intent(
          state,
          actions,
          request,
          'agent',
          'history',
          {
            sessionId: state.selected!.id,
            through: request.through,
            position: request.position,
            includeThinking: true,
          },
          {},
          request.implicit ? [{ child: 'timeline', method: 'tail', input: null }] : [],
        );
        break;
      }
      const old = state.window;
      const splitMessage = old?.page.chunks.some((oldChunk) =>
        page.chunks.some(
          (chunk) => chunk.sequence === oldChunk.sequence && chunk.field === oldChunk.field,
        ),
      );
      if (request.extend && old?.page.through === page.through && !splitMessage) {
        const before = request.position.kind === 'before';
        const chunks = before
          ? [...page.chunks, ...old.page.chunks]
          : [...old.page.chunks, ...page.chunks];
        let bytes = chunks.reduce((total, chunk) => total + utf8Bytes(chunk.text), 0);
        let first = before ? page.before : old.page.before;
        let last = before ? old.page.after : page.after;
        // Adjacent canonical pages share one fence. Retain the displayed edge
        // and its native anchor. Keep two complete pages so loading an adjacent
        // page cannot evict the message that anchors the current viewport.
        while (chunks.length > 16 || bytes > 131072) {
          const removed = before ? chunks.pop()! : chunks.shift()!;
          bytes -= utf8Bytes(removed.text);
          if (before)
            last = { sequence: removed.sequence, offset: removed.offset, field: removed.field };
          else {
            const start = chunks[0]!;
            first = { sequence: start.sequence, offset: start.offset, field: start.field };
          }
        }
        state.window = {
          page: { ...page, chunks, before: first, after: last },
          latest: last === null,
        };
      } else {
        state.window = { page, latest: request.latest };
        state.landing = {
          sequence: state.landing.sequence + 1,
          edge: request.position.kind === 'before' || request.latest ? 'end' : 'start',
        };
      }
      state.loading = false;
      if (!request.implicit) state.notice = null;
      state.live = state.live.filter(
        (fragment) =>
          fragment.sequence > page.through ||
          ((fragment.closedAt === null || fragment.closedAt > page.through) &&
            !page.chunks.some(
              (chunk) =>
                chunk.messageId === fragment.messageId &&
                (chunk.field ?? 'body') === fragment.field,
            )),
      );
      break;
    }
    case 'queueBody': {
      const edit = state.queueEdit;
      if (!edit || edit.attempt !== request.attempt) break;
      const body = value as AgentQueueMessageResult;
      if (body.kind !== 'ready' || body.revision !== edit.target.expectedRevision) {
        edit.phase = 'failed';
        state.notice = 'queueChanged';
        break;
      }
      edit.body = body.message;
      edit.hasAttachments = body.hasAttachments;
      edit.phase = 'initializing';
      initializeQueueEdit(state);
      break;
    }
    case 'queueRead': {
      const edit = state.queueEdit;
      if (!edit || edit.attempt !== request.attempt || state.dialog !== 'queue') break;
      const chunk = value as TextChunk,
        captured = request.draft.snapshot.text;
      if (!sameValue(chunk.metadata, captured) || utf8Bytes(chunk.text) !== captured.bytes) {
        state.notice = 'draftKept';
        break;
      }
      if (!chunk.text.trim() && !edit.hasAttachments) {
        state.notice = 'empty';
        break;
      }
      intent(
        state,
        actions,
        {
          kind: 'queue',
          sessionId: edit.target.sessionId,
          saved: { attempt: edit.attempt, draft: request.draft },
        },
        'agent',
        'editQueue',
        {
          sessionId: edit.target.sessionId,
          expectedRevision: edit.target.expectedRevision,
          edit: { kind: 'update', messageId: edit.target.messageId, message: chunk.text },
        },
      );
      break;
    }
    case 'read': {
      if (state.selected?.id === request.sessionId && state.selected.archived) {
        state.notice = 'draftKept';
        break;
      }
      const chunk = value as TextChunk;
      const metadata = chunk.metadata as { owner?: string; revision?: number; bytes?: number };
      const captured = request.draft.snapshot.text;
      if (
        !metadata ||
        metadata.owner !== captured.owner ||
        metadata.revision !== captured.revision ||
        metadata.bytes !== captured.bytes
      ) {
        state.notice = 'draftKept';
        break;
      }
      if (!chunk.text.trim() && request.attachments.length === 0) {
        state.notice = 'empty';
        break;
      }
      intent(
        state,
        actions,
        {
          kind: 'send',
          dataSource: request.dataSource,
          attachments: request.attachments,
          draft: request.draft,
          sessionId: request.sessionId,
          backend: request.backend,
        },
        'agent',
        request.backend.kind === 'executor' ? 'execute' : 'send',
        request.backend.kind === 'executor'
          ? {
              sessionId: request.sessionId,
              message: chunk.text,
              executor: request.backend.selection,
              attachments: request.attachments,
            }
          : {
              sessionId: request.sessionId,
              message: chunk.text,
              attachments: request.attachments,
              model: request.backend.model,
              thinkingLevel: request.backend.thinkingLevel,
            },
      );
      break;
    }
    case 'draftPage': {
      draftPage(state.drafts, request, value as CallerIntent[]);
      break;
    }
    case 'draftReceipt': {
      draftReceipt(state.drafts, request, value as OperationReply);
      break;
    }
    case 'restorePage': {
      const page = value as CallerIntent[];
      if (!page.length) {
        state.restoring = false;
        break;
      }
      const original = page[0];
      state.restoreAfter = original.cursor;
      intent(
        state,
        actions,
        { kind: 'restoreReceipt', reference: original.reference, cursor: original.cursor },
        'receipts',
        'read',
        original.reference,
      );
      break;
    }
    case 'restoreReceipt': {
      const reply = value as OperationReply;
      if (!sameValue(request.reference, reply.reference)) {
        state.uncertain = { reference: request.reference, request: { kind: 'restored' } };
        state.notice = 'unknown';
        state.restoring = false;
        break;
      }
      if (reply.receipt.kind === 'recorded' && reply.receipt.outcome.kind === 'returned')
        intent(state, actions, { kind: 'restorePage' }, 'history', 'page', {
          after: request.cursor,
          limit: 1,
        });
      else {
        state.uncertain = { reference: request.reference, request: { kind: 'restored' } };
        state.notice = 'unknown';
        state.restoring = false;
      }
      break;
    }
    case 'receipt': {
      const original = state.uncertain;
      const reply = value as OperationReply;
      if (!original || !sameValue(original.reference, reply.reference)) {
        state.notice = 'unknown';
        break;
      }
      receipt(state, actions, original.request, reply);
      break;
    }
    default:
      state.notice = 'unknown';
  }
}
function delivery(state: State, page: AgentDeliveryPage, actions: Action[]) {
  if (page.kind === 'interactions') {
    if (!state.interactions || page.pending.revision > state.interactions.revision) {
      const changed =
        !state.interactions || !sameValue(state.interactions.entries, page.pending.entries);
      state.interactions = page.pending;
      if (changed) observe(state, actions);
      const question = state.interaction;
      if (
        question?.record &&
        !question.record.outcome &&
        question.sessionId === state.selected?.id &&
        !page.pending.entries.some((entry) => entry.requestId === question.requestId)
      )
        readInteraction(state, actions);
    }
    return;
  }
  if (page.kind === 'queue') {
    if (!state.queue || page.queue.revision > state.queue.revision) state.queue = page.queue;
    return;
  }
  const watch = state.watch;
  if (!watch) return;
  state.latestKnown = Math.max(state.latestKnown, page.through);
  let canonicalChanged = false;
  for (const event of page.events) {
    if (event.sequence <= watch.last) continue;
    const fact = event.fact;
    if (
      [
        'opened',
        'executor_started',
        'transcript_changed',
        'part_finished',
        'step_ended',
        'ended',
      ].includes(fact.kind)
    )
      canonicalChanged = true;
    if (fact.kind === 'opened') {
      if (event.sequence > state.runThrough) {
        state.run = { invocationId: event.invocationId, turnId: event.turnId, state: 'running' };
        state.runThrough = event.sequence;
      }
    } else if (fact.kind === 'part_started' || fact.kind === 'executor_started') {
      const started: DisplayPart[] =
        fact.kind === 'executor_started'
          ? (['text', 'thinking'] as const).map((textKind) => ({
              startSequence: event.sequence,
              messageId: fact.messageId,
              stepId: event.invocationId,
              partId: textKind,
              textKind,
              invocationId: event.invocationId,
            }))
          : [
              {
                startSequence: event.sequence,
                messageId: fact.messageId,
                stepId: fact.stepId,
                partId: fact.partId,
                textKind: fact.textKind,
                invocationId: event.invocationId,
              },
            ];
      state.parts = state.parts.filter(
        (part) =>
          !started.some(
            (next) =>
              part.invocationId === next.invocationId &&
              part.stepId === next.stepId &&
              part.partId === next.partId,
          ),
      );
      if (state.parts.length + started.length > 128) {
        state.watch = null;
        state.notice = 'unavailable';
        return;
      }
      state.parts.push(...started);
    } else if (fact.kind === 'delta') {
      const part = state.parts.find(
        (part) =>
          part.invocationId === event.invocationId &&
          part.stepId === fact.stepId &&
          part.partId === fact.partId,
      );
      if (part) {
        appendLive(state, event.sequence, part, event.invocationId, fact.text);
      }
    } else if (fact.kind === 'part_finished') {
      state.parts = state.parts.filter(
        (part) =>
          part.invocationId !== event.invocationId ||
          part.stepId !== fact.stepId ||
          part.partId !== fact.partId,
      );
    } else if (fact.kind === 'step_ended') {
      closeLive(state, event.sequence, event.invocationId, fact.stepId);
      state.parts = state.parts.filter(
        (part) => part.invocationId !== event.invocationId || part.stepId !== fact.stepId,
      );
    } else if (fact.kind === 'ended') {
      closeLive(state, event.sequence, event.invocationId);
      state.parts = state.parts.filter((part) => part.invocationId !== event.invocationId);
      if (event.sequence > state.runThrough && state.run?.invocationId === event.invocationId) {
        state.run = null;
        state.runThrough = event.sequence;
        state.notice = fact.failed ? 'failed' : 'completed';
      }
    }
  }
  watch.last = Math.max(watch.last, page.nextAfter);
  if (canonicalChanged) follow(state, actions);
}
function frame<N extends Node>(
  presentation: Presentation<N>,
  locale: string,
  state: State,
  values: ComponentValues,
  actions: Action[] = [],
) {
  const target = presentation.target;
  const requested = state.embedding.session;
  if (requested && !sameValue(requested, state.externalSelection) && !mutationPending(state)) {
    state.externalSelection = requested;
    select(state, actions, null);
    state.loading = true;
    closeDialog(state);
    intent(state, actions, { kind: 'openSession', selection: requested }, 'agent', 'observe', {
      sessionId: requested.id,
    });
  }
  if (state.dialog !== 'palette') {
    state.copyRequest = null;
    state.messageCopy = null;
  }
  retainSelectedDraft(state.drafts, state.selected);
  if (
    !Object.values(state.pending).some(
      (request) => request.kind === 'draftPage' || request.kind === 'draftReceipt',
    )
  ) {
    const step = nextDraftStep(state.drafts, state.attachments);
    if (step)
      intent(
        state,
        actions,
        step,
        step.kind === 'draftPage' ? 'history' : 'receipts',
        step.kind === 'draftPage' ? 'page' : 'read',
        step.kind === 'draftPage' ? { after: step.after, limit: 1 } : step.original.reference,
      );
  }
  uploadPastedFile(state, actions);
  if (!conversationRoute(state) || state.source || state.image || !state.selected) {
    state.searchOpen = false;
    state.browse = null;
  }
  const baseTheme = values.appearance?.[0] as Theme;
  if (target === 'tui' && state.preferences.open && !state.preferences.draft)
    appearanceEvent(
      state.preferences,
      { kind: 'raw', key: 'appearance-open', event: {} },
      baseTheme,
      locale,
      actions,
    );
  locale =
    target === 'tui'
      ? effectiveLocale(state.preferences, locale, values.locale?.[0])
      : resolveLocale(locale, values.locale?.[0]);
  const theme = target === 'tui' ? effectiveAppearance(state.preferences, baseTheme) : baseTheme;
  const text = copy(locale),
    ui = presentation.paint(theme),
    busy = mutationPending(state);
  const catalogBusy = busy || catalogPending(state);
  const commands = rootCommands(state, locale, target);
  const createButton = ui.button(
    'create',
    target === 'tui' ? '+ ' + tuiText(locale, 'sidebar-new-session') : text.new,
    !commands.some((command) => command.kind === 'create' && !command.disabled),
    false,
    target === 'tui' ? { background: 0, align: 'start' } : {},
  );
  const createEntry = ui.contextMenu
    ? ui.contextMenu('navigation-create-menu', text.new, createButton, [
        ui.button(
          'create-workspace',
          text.newWorkspace,
          !commands.some((command) => command.kind === 'create-workspace' && !command.disabled),
        ),
      ])
    : createButton;
  const navigation = ui.box(
    'navigation-panel',
    [
      target === 'tui'
        ? ui.box(
            'navigation-heading',
            [
              ui.box('navigation-create-cell', [createEntry], { terminal: { grow: 1 } }),
              ui.button(
                'navigation-back',
                theme.ascii ? 'x' : '×',
                false,
                false,
                { background: 0, align: 'start' },
                undefined,
                text.hideSidebar,
              ),
            ],
            { row: true, gap: 0 },
          )
        : ui.text('navigation-brand', 'Maka', 24, theme.text, 650),
      ...(target === 'desktop' ? [createEntry] : []),
      ...(target === 'desktop'
        ? [ui.text('navigation-heading', text.conversations, 12, theme.muted, 550)]
        : []),
      ui.box(
        'catalog-filters',
        [
          ...(['all', 'needs_you'] as const).map((filter) => {
            const label =
              filter === 'all'
                ? text.allSessions
                : state.catalog
                  ? `${text.needsYou} (${state.catalog.needsYouCount})`
                  : text.needsYou;
            const selected = state.catalogFilter === filter;
            return ui.item
              ? ui.item(
                  `catalog-filter-${filter}`,
                  ui.text(`catalog-filter-label-${filter}`, label, 12, theme.text),
                  selected,
                  catalogBusy,
                )
              : ui.button(
                  `catalog-filter-${filter}`,
                  label,
                  catalogBusy,
                  false,
                  selected ? { background: theme.selected } : {},
                );
          }),
          ...(target === 'tui'
            ? [
                ui.box('catalog-filter-space', [], { terminal: { grow: 1 } }),
                ui.box(
                  'catalog-refresh-cell',
                  [
                    ui.button(
                      'catalog-refresh',
                      theme.ascii ? 'r' : '↻',
                      catalogBusy,
                      false,
                      { background: 0 },
                      undefined,
                      text.refreshSessions,
                    ),
                  ],
                  { terminal: { basis: 3 } },
                ),
              ]
            : []),
        ],
        { row: true, focusGroup: target === 'tui', gap: 1 },
      ),
      ...(!state.catalogFailed && !catalogPending(state) && state.catalog?.sessions.length === 0
        ? [
            ui.text(
              'catalog-empty',
              state.catalogFilter === 'needs_you'
                ? text.noneNeedsYou
                : tuiText(locale, 'sidebar-empty'),
              12,
              theme.muted,
            ),
          ]
        : []),
      ...(state.catalogFailed
        ? [ui.text('catalog-error', text.sessionsUnavailable, 12, theme.muted)]
        : []),
      ui.box(
        'catalog-controls',
        [
          ...(state.catalog?.before
            ? [
                ui.button(
                  'catalog-previous',
                  text.previousSessions,
                  catalogBusy || state.catalogFailed,
                ),
              ]
            : []),
          ...(state.catalog?.after
            ? [ui.button('catalog-more', text.moreSessions, catalogBusy || state.catalogFailed)]
            : []),
          ...(target === 'desktop'
            ? [ui.button('catalog-refresh', text.refreshSessions, catalogBusy)]
            : []),
        ],
        { row: true, style: { wrap: true }, terminal: { wrap: true } },
      ),
      presentation.slot(
        'navigation',
        'navigation',
        {
          theme,
          locale,
          sessions: state.catalog?.sessions ?? [],
          selected: state.selected?.id ?? null,
          busy: catalogBusy,
        },
        { select: 'select' },
      ),
      ...(target === 'tui'
        ? [
            ui.box(
              'navigation-footer',
              [
                ...(!state.embedding.fixed
                  ? [
                      ui.button('projects-open', text.projectsTitle, false, false, {
                        background: 0,
                        align: 'start',
                      }),
                    ]
                  : []),
                ui.button(
                  'settings',
                  `${theme.ascii ? 'S' : '⛭'} ${tuiText(locale, 'route-settings')}`,
                  false,
                  false,
                  { background: 0, align: 'start' },
                ),
              ],
              { gap: 0 },
            ),
          ]
        : []),
    ],
    {
      surface: target === 'desktop',
      gap: target === 'tui' ? 0 : undefined,
      padding: target === 'desktop' ? 20 : 1,
      style: { height: fill, width: fill },
      terminal: { grow: 1 },
    },
  );
  const timelineMessages = messages(state);
  const timeline = presentation.slot(
    'timeline',
    'timeline',
    {
      theme,
      locale,
      messages: timelineMessages,
      timings: turnTimings(state, timelineMessages),
      copyRequest: target === 'tui' && !state.image ? state.copyRequest : null,
      browse: state.browse,
      markReadEnabled:
        !!state.selected?.hasUnread &&
        !state.selected.archived &&
        !busy &&
        conversationRoute(state) &&
        state.dialog === null &&
        !state.source &&
        !state.image,
      search:
        target === 'tui' && state.searchOpen && state.selected
          ? { sessionId: state.selected.id, epoch: state.epoch, through: state.latestKnown }
          : null,
      source: state.source
        ? {
            kind: state.source.field === 'body' ? 'message' : state.source.field,
            fields: state.source.fields,
            format: state.source.format ?? 'text',
            ready: state.source.ready,
            key: `${state.source.field}-${state.source.through}-${state.source.chunk.sequence}-${state.source.chunk.offset}`,
            text: state.source.chunk.text,
            atStart: state.source.chunk.offset === 0,
            atEnd:
              state.source.chunk.offset + utf8Bytes(state.source.chunk.text) ===
              state.source.chunk.totalBytes,
            loading: state.source.loading,
            failed: state.source.failed,
            navigation: state.source.landing.sequence,
            landing: state.source.landing.edge,
          }
        : null,
      loading: state.loading,
      older: !!state.window?.page.before,
      newer: !!state.window?.page.after,
      latest:
        !!state.window &&
        (!state.window.latest ||
          state.latestKnown > state.window.page.through ||
          state.live.some(
            (fragment) =>
              fragment.closedAt !== null && fragment.closedAt <= state.window!.page.through,
          )),
      landing: state.landing,
      toolContext:
        state.selected && state.window && state.window.page.through > 0
          ? { sessionId: state.selected.id, through: state.window.page.through }
          : null,
      liveTools:
        state.window?.latest && state.run
          ? [
              ...new Set(
                state.window.page.chunks.flatMap((chunk) =>
                  chunk.turnId === state.run!.turnId && chunk.tool ? [chunk.tool.toolUseId] : [],
                ),
              ),
            ]
          : [],
      waitingTools: state.window?.latest
        ? (state.interactions?.entries ?? [])
            .filter(
              (entry) =>
                !(
                  state.interaction?.record?.requestId === entry.requestId &&
                  state.interaction.record.outcome
                ),
            )
            .flatMap((entry) => (entry.toolUseId === null ? [] : [entry.toolUseId]))
        : [],
      live: !!state.run && !!state.window?.latest,
    },
    {
      older: 'older',
      newer: 'newer',
      latest: 'latest',
      markRead: 'mark-read',
      source: 'source',
      toolSource: 'tool-source',
      fileContext: 'fileContext',
      messageCopy: 'messageCopy',
      viewImage: 'view-image',
      sourceFirst: 'source-first',
      sourcePrevious: 'source-previous',
      sourceNext: 'source-next',
      sourceLast: 'source-last',
      sourceBack: 'source-back',
      sourceRetry: 'source-retry',
      sourceField: 'source-field',
      tailReady: 'tail-ready',
      searchClosed: 'search-closed',
      browseClosed: 'browse-closed',
    },
  );
  const imageViewer = presentation.slot(
    'image-viewer',
    'imageViewer',
    {
      theme,
      locale,
      image: state.image,
      active: !!state.image && conversationRoute(state),
    },
    { closed: 'image-closed' },
  );
  const attached = attachmentFiles(
    state.attachments,
    state.selected?.id,
    state.selected?.dataSource,
  );
  const attachmentsDisabled =
    !state.selected ||
    !draftReady(state.drafts, state.selected) ||
    busy ||
    state.loading ||
    !!state.attachments.paste;
  const attachmentsButton = ui.button(
    'attachments-open',
    `${text.attachmentTitle}${attached.length ? ` (${attached.length})` : ''}`,
    attachmentsDisabled,
  );
  const composer = presentation.slot(
    'composer',
    'composer',
    {
      context: state.selected?.id ?? null,
      dataSource: state.selected?.dataSource ?? null,
      attachments: state.attachments.files,
      drafts: draftStatuses(state.drafts),
      theme,
      locale,
      busy:
        busy ||
        !!state.attachments.paste ||
        executionPending(state) ||
        (target === 'tui' && !!state.run && !state.queue),
      filePaste:
        target === 'tui' &&
        state.selected &&
        !state.selected.archived &&
        conversationRoute(state) &&
        !state.dialog &&
        !state.source &&
        !state.image &&
        !state.loading &&
        (!busy || !!state.attachments.paste) &&
        (!state.attachments.paste ||
          (state.attachments.paste.observed.context === state.selected.id &&
            sameValue(state.attachments.paste.observed.dataSource, state.selected.dataSource)))
          ? {
              context: state.selected.id,
              maxFiles: Math.min(8 - attached.length, 64 - state.attachments.files.length),
            }
          : null,
      fileCommand: state.attachments.paste?.command ?? null,
      running: !!state.run,
      enabled:
        !!state.selected &&
        !state.selected.archived &&
        canExecute(state) &&
        !state.loading &&
        state.notice !== 'unavailable',
      command: state.command,
      autoFocus: !state.searchOpen && state.browse === null,
      hasAttachments: attached.length > 0,
    },
    {
      send: 'send',
      stop: 'stop',
      attachments: 'attachments',
      changed: 'draft',
      files: 'files',
      checkpoint: 'checkpoint',
      draftCheck: 'draft-check',
    },
  );
  const queueProps: RowsProps = {
    terminal: theme.terminal,
    entry: state.queue?.entries.at(-1)?.messageId ?? null,
    revision: state.queue?.revision ?? 0,
    rows: (state.queue?.entries ?? []).map((entry): RowsProps['rows'][number] => {
      const lane = state.queue!.entries.filter((row) => row.kind === entry.kind);
      const index = lane.findIndex((row) => row.messageId === entry.messageId);
      const enabled = !busy && !executionPending(state);
      const up = {
        key: 'up',
        label: theme.ascii ? '^' : '↑',
        enabled: enabled && index > 0,
      };
      const down = {
        key: 'down',
        label: theme.ascii ? 'v' : '↓',
        enabled: enabled && index + 1 < lane.length,
      };
      const edit = { key: 'edit', label: theme.ascii ? 'e' : '✎', enabled };
      const retract = {
        key: 'retract',
        label: theme.ascii ? 'x' : '×',
        enabled,
      };
      return {
        key: entry.messageId,
        text: ` ${entry.kind === 'steering' ? (theme.ascii ? '^' : '↗') : entry.kind === 'in_flight' ? (theme.ascii ? '...' : '…') : theme.ascii ? '+' : '↳'} ${entry.preview}`,
        actions:
          entry.kind === 'in_flight'
            ? []
            : entry.kind === 'followup'
              ? [
                  {
                    key: 'promote',
                    label: theme.ascii ? '^' : '↗',
                    enabled: enabled && !!state.run,
                  },
                  up,
                  down,
                  edit,
                  retract,
                ]
              : [up, down, edit, retract],
      };
    }),
    shortcuts: [
      { key: 'enter', modifiers: [], input: 'edit', enabled: true },
      { key: 'e', modifiers: [], input: 'edit', enabled: true },
      { key: 's', modifiers: [], input: 'promote', enabled: true },
      { key: 'delete', modifiers: [], input: 'retract', enabled: true },
      { key: 'x', modifiers: [], input: 'retract', enabled: true },
      { key: 'up', modifiers: ['alt'], input: 'up', enabled: true },
      { key: 'down', modifiers: ['alt'], input: 'down', enabled: true },
    ],
    maxRows: 3,
    compactBelow: { height: 12, maxRows: 1 },
    tabIndex: 2,
    foreground: theme.text,
    muted: theme.muted,
    selected: theme.selected,
    hovered: theme.surface,
    scrollbar: {
      thumb: theme.ascii ? '#' : '┃',
      track: theme.ascii ? '|' : '│',
      thumbColor: theme.text,
      trackColor: theme.border,
    },
  };
  const status = state.notice
    ? text[state.notice]
    : state.run
      ? state.run.state === 'waiting_for_user'
        ? text.waiting
        : text.working
      : state.selected?.archived
        ? text.archivedConversation
        : '';
  const title = ui.text(
    'title',
    state.pages.at(-1) === 'projects'
      ? text.projectsTitle
      : state.pages.at(-1) === 'settings'
        ? tuiText(locale, 'route-settings')
        : state.pages.at(-1) === 'connections'
          ? text.settings
          : state.pages.at(-1) === 'deployment'
            ? text.deploymentTitle
            : (state.selected?.title ?? tuiText(locale, 'route-workspace')),
    17,
    theme.text,
    650,
    {
      align: target === 'tui' ? 'start' : 'center',
      width: pixels(0),
      grow: 1,
      minWidth: 0,
      truncate: true,
    },
  );
  const models = presentation.slot(
    'models',
    'models',
    {
      theme,
      locale,
      selection: state.model,
      executor: state.executor,
      catalogEpoch: state.catalogEpoch,
      openRequest: state.modelOpenRequest,
      busy: busy || executionPending(state) || (target === 'tui' && !!state.run && !state.queue),
    },
    {
      selected: 'model',
      setup: 'model-setup',
      connections: 'model-connections',
      executor: 'executor',
      backends: 'backends',
    },
  );

  const header = ui.box(
    'header',
    [
      ...(target === 'desktop'
        ? [
            ui.box(
              'context',
              [
                title,
                ...(!state.embedding.fixed
                  ? [ui.button('navigation-toggle', sidebarLabel(state, text))]
                  : []),
              ],
              {
                row: true,
                gap: 8,
                style: { width: fill, align: 'center' },
              },
            ),
            ui.box(
              'toolbar',
              [
                ui.box('model-control', [models], {
                  style: { width: pixels(0), grow: 1, minWidth: 0 },
                }),
                ui.button('palette-open', text.paletteTitle),
                ui.button(
                  'details-open',
                  text.detailsOpen,
                  !rootCommands(state, locale, target).some(
                    (command) => command.kind === 'details-open' && !command.disabled,
                  ),
                ),
                ui.button('terminal-open', text.terminalTitle, !state.selected || state.loading),
                ...(!state.embedding.fixed ? [ui.button('projects-open', text.projectsTitle)] : []),
                ui.button('settings', text.settings),
                ui.button('deployment-open', text.deploymentTitle),
              ],
              { row: true, gap: 8, style: { width: fill, align: 'center' } },
            ),
          ]
        : [
            ui.box(
              'header-left',
              [
                ui.button(
                  conversationRoute(state) ? 'navigation-toggle' : 'page-back',
                  conversationRoute(state) ? (theme.ascii ? '=' : '≡') : theme.ascii ? '<' : '‹',
                  false,
                  false,
                  { background: 0 },
                  undefined,
                  conversationRoute(state) ? text.toggleSidebar + ' · Ctrl+B' : text.back,
                ),
              ],
              {
                visible: !state.embedding.fixed || !conversationRoute(state),
                row: true,
                gap: 0,
                terminal: { basis: 3 },
              },
            ),
            ui.box(
              'header-task',
              [
                ...(state.selected && conversationRoute(state)
                  ? [
                      state.run?.state === 'waiting_for_user'
                        ? ui.text('task-face', '-_-', 14, theme.warning)
                        : ui.activity('task-face', state.run ? 'spring' : 'familiar', theme.accent),
                    ]
                  : []),
                ui.box('header-title', [title], { terminal: { shrink: true } }),
                ui.box('header-title-trailing', [], { terminal: { grow: 1 } }),
              ],
              { row: true, gap: 1, terminal: { grow: 1, shrink: true } },
            ),
            ui.box(
              'toolbar',
              [
                ...(['details-open', 'terminal-open', 'palette-open'] as const).map((key, at) =>
                  ui.box(
                    `${key}-cell`,
                    [
                      ui.button(
                        key,
                        [theme.ascii ? 'i' : 'ⓘ', '>_', theme.ascii ? ':' : '⌘'][at]!,
                        key !== 'palette-open' && !state.selected,
                        false,
                        { background: 0 },
                        undefined,
                        [text.detailsOpen, text.terminalTitle, text.paletteTitle + ' · Ctrl+K'][at],
                      ),
                    ],
                    { terminal: { basis: 3 } },
                  ),
                ),
              ],
              { row: true, gap: 0, terminal: { basis: 9 } },
            ),
          ]),
      ...((target === 'desktop' ? state.interactions?.entries : [])
        ?.filter((entry) =>
          ['question', 'form', 'service_approval', 'permissions'].includes(entry.kind),
        )
        .map((entry, index) =>
          ui.button(
            `${entry.kind === 'question' || entry.kind === 'form' ? 'question' : 'approval'}-open-${entry.requestId}`,
            `${entry.kind === 'form' ? text.formTitle : entry.kind === 'question' ? text.questionsTitle : entry.kind === 'permissions' ? text.permissionApprovalTitle : text.approvalTitle} ${index + 1}`,
            !state.selected || state.selected.archived || mutationPending(state),
          ),
        ) ?? []),
      ...(target === 'desktop' && !canExecute(state)
        ? [
            ui.text(
              'model-setup-hint',
              state.hasBackends
                ? text.executionBackend
                : state.modelSetup === 'loading'
                  ? text.loadingModels
                  : state.modelSetup === 'unavailable'
                    ? text.modelsUnavailable
                    : state.modelSetup === 'connect'
                      ? text.setupConnectionHint
                      : text.setupModelHint,
              13,
              theme.muted,
            ),
            ...(state.modelSetup === 'connect'
              ? [
                  ui.button(
                    'model-setup',
                    state.hasBackends ? text.executionBackend : text.configureModels,
                    busy,
                  ),
                ]
              : []),
          ]
        : []),
      ...(target === 'desktop' && status ? [ui.text('status', status, 13, theme.muted)] : []),
      ...(state.uncertain
        ? [
            ui.button(
              'check',
              text.check,
              Object.values(state.pending).some((request) => request.kind === 'receipt'),
            ),
          ]
        : []),
      ...(state.notice === 'unavailable' ? [ui.button('reload', text.loading, state.loading)] : []),
    ],
    {
      row: target === 'tui',
      gap: target === 'desktop' ? 8 : 0,
      style: {
        width: fill,
        shrink: 0,
        padding: { top: 16, right: 24, bottom: 16, left: 24 },
      },
    },
  );
  const welcome =
    !!state.selected &&
    !state.selected.archived &&
    !state.loading &&
    !!state.window &&
    !state.window.page.chunks.length &&
    !state.live.length &&
    !state.run &&
    !state.source &&
    !state.image &&
    !state.searchOpen;
  const main = ui.box(
    'main',
    [
      ...(target === 'desktop' ? [header] : []),
      ...(target === 'tui'
        ? [
            ui.box(
              'home',
              [
                ui.box('home-top', [], { terminal: { grow: 1 } }),
                ui.box(
                  'home-center',
                  [
                    ui.box('home-left', [], { terminal: { grow: 1 } }),
                    ui.box(
                      'home-content',
                      [
                        ui.text('home-title', 'Maka', 24, theme.text, 650, {
                          align: 'center',
                        }),
                        ...(state.place
                          ? [
                              ui.text('home-place', state.place, 14, theme.subtle, 400, {
                                align: 'center',
                                truncate: true,
                              }),
                            ]
                          : []),
                        ui.box('home-gap', [], { terminal: { basis: 1 } }),
                        ...(!canExecute(state)
                          ? [
                              ui.text(
                                'home-setup',
                                state.hasBackends
                                  ? text.executionBackend
                                  : tuiText(
                                      locale,
                                      state.modelSetup === 'loading'
                                        ? 'connections-loading'
                                        : state.modelSetup === 'unavailable'
                                          ? 'home-setup-failed'
                                          : state.modelSetup === 'connect'
                                            ? 'home-needs-connection'
                                            : 'home-needs-default',
                                    ),
                                14,
                                state.modelSetup === 'unavailable' ? theme.warning : theme.subtle,
                                400,
                                { align: 'center' },
                              ),
                            ]
                          : []),
                        ui.button(
                          'home-new',
                          canExecute(state)
                            ? '+ ' + tuiText(locale, 'sidebar-new-session') + '   Ctrl+N'
                            : state.hasBackends
                              ? text.executionBackend
                              : tuiText(
                                  locale,
                                  state.modelSetup === 'connect'
                                    ? 'route-connections'
                                    : 'default-model-title',
                                ) + '   Ctrl+N',
                          catalogBusy ||
                            (!canExecute(state) &&
                              !state.hasBackends &&
                              state.modelSetup === 'loading'),
                          true,
                          { background: 0 },
                        ),
                      ],
                      { gap: 0, terminal: { basis: 52, shrink: true } },
                    ),
                    ui.box('home-right', [], { terminal: { grow: 1 } }),
                  ],
                  { row: true, gap: 0 },
                ),
                ui.box('home-bottom', [], { terminal: { grow: 1 } }),
              ],
              {
                gap: 0,
                visible: !state.selected && !state.embedding.fixed,
                viewport: true,
                terminal: { grow: 1 },
              },
            ),
          ]
        : []),
      ...(state.selected?.archived && attached.length && !state.source && !state.image
        ? [ui.box('archived-attachments', [attachmentsButton], { row: true })]
        : []),
      ...(target === 'desktop' && state.attachments.paste
        ? [pasteView(presentation, theme, locale, state.attachments.paste, busy)]
        : []),
      ...(target === 'desktop'
        ? [
            ui.box(
              'conversation',
              [
                ui.box('retained-timeline', [timeline], {
                  visible: !state.image,
                  style: { width: fill, height: fill },
                  terminal: { grow: 1 },
                }),
                imageViewer,
              ],
              {
                style: {
                  width: fill,
                  height: pixels(0),
                  grow: 1,
                  shrink: 1,
                  minHeight: 0,
                },
              },
            ),
            ui.box(
              'input-area',
              [...(!state.selected?.archived ? [attachmentsButton] : []), composer],
              {
                visible: !state.image && !state.selected?.archived,
                padding: 24,
                style: { width: fill, maxWidth: 808, shrink: 0 },
              },
            ),
          ]
        : [
            ui.box(
              'session-body',
              [
                ui.box(
                  'conversation',
                  [
                    ui.box(
                      'welcome-mark',
                      [
                        presentation.slot(
                          'welcome',
                          'welcome',
                          {
                            background: theme.terminal?.background ?? theme.background >>> 8,
                            foreground: theme.terminal?.foreground ?? theme.text >>> 8,
                            motion: theme.motion,
                            ascii: theme.ascii,
                          },
                          {},
                        ),
                      ],
                      { visible: welcome, terminal: { grow: 1 } },
                    ),
                    ui.box('retained-timeline', [timeline], {
                      visible: !state.image && !welcome,
                      style: { width: fill, height: fill },
                      terminal: { grow: 1 },
                    }),
                    imageViewer,
                  ],
                  { terminal: { basis: 1, grow: 1 } },
                ),
                ...(state.attachments.paste
                  ? [pasteView(presentation, theme, locale, state.attachments.paste, busy)]
                  : []),
                ui.box(
                  'retained-composer',
                  [
                    ...(reviewTarget(state)
                      ? [
                          ui.button(
                            'interaction-open',
                            `Ctrl G · ${text.reviewRequest}`,
                            !rootCommands(state, locale, target).some(
                              (command) => command.kind === 'interaction-open' && !command.disabled,
                            ),
                          ),
                        ]
                      : []),
                    presentation.slot('queue', 'rows', queueProps, {
                      activated: 'queue-action',
                    }),
                    ...(queueProps.rows.length && ui.focusButton
                      ? [
                          ui.box(
                            'queue-status',
                            [
                              ui.box('queue-status-space', [], {
                                terminal: { basis: 0, grow: 1 },
                              }),
                              ui.focusButton(
                                'queue-focus',
                                `${theme.ascii ? '=' : '≡'} ${queueProps.rows.length}`,
                                'queue',
                              ),
                            ],
                            { row: true, gap: 0 },
                          ),
                        ]
                      : []),
                    ...(attached.length
                      ? [
                          attachmentSummary(
                            presentation,
                            theme,
                            locale,
                            attached,
                            attachmentsDisabled,
                          ),
                        ]
                      : []),
                    ui.box('composer-navigation', [composer], {
                      terminal: { shrink: true },
                      focusShortcuts: [
                        {
                          key: 'up',
                          modifiers: [],
                          child: 'queue',
                          enabled: queueProps.rows.length > 0,
                        },
                      ],
                    }),
                  ],
                  {
                    gap: 0,
                    visible: !state.source && !state.image && !state.selected?.archived,
                    terminal: { shrink: true },
                  },
                ),
              ],
              {
                gap: 0,
                visible: !!state.selected,
                viewport: true,
                terminal: { grow: 1 },
              },
            ),
            ui.box(
              'session-metadata',
              [
                ui.box('model-metadata', [models], {
                  terminal: { grow: 1, shrink: true },
                }),
                ...(status
                  ? [
                      ui.text(
                        'status',
                        status,
                        12,
                        state.run?.state === 'waiting_for_user' ? theme.warning : theme.muted,
                      ),
                    ]
                  : []),
              ],
              { row: true, gap: 1 },
            ),
          ]),
    ],
    {
      gap: target === 'tui' ? 0 : undefined,
      terminal: { grow: 1 },
      style: {
        width: target === 'desktop' ? pixels(0) : fill,
        grow: 1,
        height: fill,
        align: 'center',
        minWidth: 0,
      },
    },
  );
  const conversation = ui.box(
    'workspace',
    state.embedding.fixed
      ? [main]
      : target === 'desktop'
        ? [
            ui.box('sidebar', [navigation], {
              visible: 'open' in state.sidebar && state.sidebar.open,
              style: { width: pixels(240), height: fill, shrink: 0 },
            }),
            main,
          ]
        : [
            ui.adaptivePane!(
              'navigation-pane',
              text.conversations,
              ui.box(
                'page',
                [
                  ui.box('page-inset-start', [], { terminal: { basis: 1 } }),
                  main,
                  ui.box('page-inset-end', [], { terminal: { basis: 1 } }),
                ],
                { row: true, gap: 0, terminal: { grow: 1 } },
              ),
              ui.box('navigation-content', [navigation], {
                gap: 0,
                terminal: { grow: 1 },
              }),
              'command' in state.sidebar ? state.sidebar.command : null,
            ),
          ],
    {
      row: target === 'desktop',
      gap: 0,
      visible: conversationRoute(state),
      terminal: { grow: 1 },
      style: { width: fill, height: fill },
    },
  );
  const background = ui.box(
    'product',
    [
      ...(target === 'tui' && (conversationRoute(state) || state.pages.at(-1) === 'settings')
        ? [header]
        : []),
      conversation,
      ...(target === 'tui' && (!state.embedding.fixed || state.embedding.session)
        ? [
            bookmarkView(
              presentation,
              state.bookmark,
              'navigation-checkpoint-' + navigationKey(state),
              navigationKey(state),
              navigationBookmark(state),
            ),
            ...(state.bookmark.observed?.state.kind === 'failed'
              ? [ui.button('navigation-checkpoint-retry', copy(locale).sourceRetry)]
              : []),
          ]
        : []),
      ...(state.pages.at(-1) === 'settings'
        ? [settingsView(presentation, state.preferences, theme, locale, state.settingsCategory)]
        : []),
      ...(state.pages.at(-1) === 'deployment'
        ? [
            presentation.slot(
              'deployment',
              'deployment',
              {
                theme,
                locale,
                ...(target === 'tui'
                  ? {
                      bookmarkKey: navigationKey(state) + '/deployment',
                      navigationReady: bookmarkReady(state.bookmark, navigationBookmark(state)),
                    }
                  : {}),
              },
              { closed: 'deployment-closed' },
            ),
          ]
        : []),
      ...(state.projectsOpened
        ? [
            presentation.slot(
              'projects',
              'projects',
              {
                theme,
                locale,
                open: state.pages.at(-1) === 'projects',
                busy: mutationPending(state) || catalogPending(state),
              },
              {
                closed: 'projects-closed',
                selected: 'project-selected',
                changed: 'projects-changed',
              },
            ),
          ]
        : []),
      ...(state.pages.at(-1) === 'connections'
        ? [
            presentation.slot(
              'connections',
              'connections',
              { theme, locale },
              { closed: 'settings-closed', changed: 'settings-changed' },
            ),
          ]
        : []),
    ],
    {
      gap: 0,
      background: theme.background,
      style: { width: fill, height: fill, align: 'center' },
      terminal: { grow: 1 },
      shortcuts: [
        ...(state.pages.at(-1) === 'settings' && state.dialog === null && !state.preferences.open
          ? ([{ key: 'escape', modifiers: [], input: 'page-back', enabled: true }] as Shortcut[])
          : []),
        {
          key: 'f',
          modifiers: ['primary'],
          input: 'search-open',
          enabled: rootCommands(state, locale, target).some(
            (command) => command.kind === 'search-open' && !command.disabled,
          ),
        },
        { key: 'f1', modifiers: [], input: 'help-open', enabled: true },
        {
          key: 'g',
          modifiers: ['primary'],
          input: 'interaction-open',
          enabled: rootCommands(state, locale, target).some(
            (command) => command.kind === 'interaction-open' && !command.disabled,
          ),
        },
        {
          key: 'k',
          modifiers: ['primary'],
          input: 'palette-open',
          enabled: conversationRoute(state),
        },
        {
          key: 'b',
          modifiers: ['primary'],
          input: 'navigation-toggle',
          enabled: !state.embedding.fixed && conversationRoute(state),
        },
        {
          key: 'n',
          modifiers: ['primary'],
          input: 'create',
          enabled:
            !state.embedding.fixed &&
            conversationRoute(state) &&
            !mutationPending(state) &&
            !catalogPending(state),
        },
      ],
    },
  );
  const out = output(
    state,
    ui.dialog(
      'workspace-dialog',
      state.dialog === 'attachments'
        ? target === 'tui' && state.attachments.selection?.browse
          ? tuiText(locale, 'attachments-local-files')
          : text.attachmentTitle
        : state.dialog === 'skills'
          ? text.skillsTitle
          : state.dialog === 'terminal'
            ? text.terminalTitle
            : state.dialog === 'question'
              ? state.interaction?.record?.request.kind === 'form'
                ? text.formTitle
                : text.questionsTitle
              : state.dialog === 'approval'
                ? state.interaction?.record?.request.kind === 'permissions'
                  ? text.permissionApprovalTitle
                  : text.approvalTitle
                : state.dialog === 'directories'
                  ? text.directoryTitle
                  : state.dialog === 'details'
                    ? text.detailsTitle
                    : state.dialog === 'queue'
                      ? text.queueEdit
                      : state.dialog === 'help'
                        ? text.helpTitle
                        : text.paletteTitle,
      background,
      ui.box(
        'dialog-content',
        [
          ...(state.dialog === 'attachments' && state.attachments.selection
            ? [
                attachmentView(
                  presentation,
                  theme,
                  locale,
                  state.attachments,
                  busy,
                  state.selected?.id === state.attachments.selection.sessionId &&
                    sameValue(state.selected.dataSource, state.attachments.selection.dataSource) &&
                    !state.selected.archived,
                  state.notice
                    ? text[state.notice]
                    : state.selected?.archived
                      ? text.archivedConversation
                      : null,
                  !!state.uncertain,
                  Object.values(state.pending).some((request) => request.kind === 'receipt'),
                ),
              ]
            : []),
          ...(state.interaction?.record?.request.kind === 'service_approval' ||
          state.interaction?.record?.request.kind === 'permissions'
            ? [
                presentation.slot(
                  `${state.interaction.record.request.kind === 'permissions' ? 'permission' : 'service'}-approval-${state.interaction.attempt}`,
                  state.interaction.record.request.kind === 'permissions'
                    ? 'permissionApproval'
                    : 'serviceApproval',
                  {
                    theme,
                    locale,
                    open: state.dialog === 'approval',
                    record: state.interaction.record,
                    busy:
                      mutationPending(state) ||
                      state.selected?.id !== state.interaction.sessionId ||
                      !!state.selected?.archived,
                    checking: Object.values(state.pending).some(
                      (r) =>
                        r.kind === 'interaction' || r.kind === 'receipt' || r.kind === 'respond',
                    ),
                    status: state.uncertain
                      ? text.interactionUnknown
                      : state.interaction.notice
                        ? text[state.interaction.notice]
                        : state.selected?.id !== state.interaction.sessionId
                          ? text.interactionChanged
                          : '',
                  } satisfies ServiceApprovalProps,
                  { closed: 'closed', submit: 'submit', check: 'check' },
                ),
              ]
            : state.interaction?.record
              ? [
                  presentation.slot(
                    questionChild(state),
                    state.interaction.record.request.kind === 'form'
                      ? 'formEditor'
                      : 'questionEditor',
                    {
                      theme,
                      locale,
                      open: state.dialog === 'question',
                      record: state.interaction.record,
                      busy:
                        mutationPending(state) ||
                        state.selected?.id !== state.interaction.sessionId ||
                        !!state.selected?.archived,
                      checking: Object.values(state.pending).some(
                        (r) =>
                          r.kind === 'interaction' || r.kind === 'receipt' || r.kind === 'respond',
                      ),
                      status: state.uncertain
                        ? text.interactionUnknown
                        : state.interaction.notice
                          ? text[state.interaction.notice]
                          : state.selected?.id !== state.interaction.sessionId
                            ? text.interactionChanged
                            : '',
                    } satisfies QuestionEditorProps,
                    { closed: 'closed', submit: 'submit', check: 'check' },
                  ),
                ]
              : [
                  ui.box(
                    'question-loading',
                    [
                      ui.button('question-close', text.closeModels),
                      ui.text(
                        'question-loading-text',
                        state.interaction?.notice
                          ? text[state.interaction.notice]
                          : text.questionLoading,
                      ),
                      ui.button(
                        'question-check',
                        text.check,
                        Object.values(state.pending).some((r) => r.kind === 'interaction'),
                      ),
                    ],
                    {
                      visible: state.dialog === 'question' || state.dialog === 'approval',
                      terminal: { grow: 1 },
                    },
                  ),
                ]),
          presentation.slot(
            'queue-editor',
            'queueEditor',
            {
              theme,
              locale,
              open: state.dialog === 'queue',
              ready: state.queueEdit?.phase === 'ready',
              hasAttachments: state.queueEdit?.hasAttachments ?? false,
              busy: mutationPending(state),
              uncertain: !!state.uncertain,
              checking: Object.values(state.pending).some((request) => request.kind === 'receipt'),
              command: state.queueEdit?.command ?? null,
              status: state.notice
                ? text[state.notice]
                : state.queueEdit?.phase === 'loading'
                  ? text.queueLoading
                  : '',
            } satisfies QueueEditorProps,
            {
              changed: 'changed',
              save: 'save',
              closed: 'closed',
              reload: 'reload',
              check: 'check',
            },
          ),
          ...(state.skills
            ? [
                presentation.slot(
                  'skill-library',
                  'skills',
                  {
                    theme,
                    locale,
                    open: state.dialog === 'skills',
                    workspace: state.selected?.workspace ?? null,
                  },
                  { closed: 'closed' },
                ),
              ]
            : []),
          presentation.slot(
            'help',
            'help',
            { theme, locale, open: state.dialog === 'help' },
            { closed: 'closed' },
          ),
          presentation.slot(
            'terminal-panel',
            'terminals',
            {
              theme,
              locale,
              open: state.dialog === 'terminal',
              enabled: state.terminals,
              busy: mutationPending(state),
              session: state.selected,
            },
            { closed: 'closed' },
          ),
          presentation.slot(
            'session-details',
            'details',
            {
              theme,
              locale,
              open: state.dialog === 'details',
              session: state.selected,
              busy: mutationPending(state),
            },
            { changed: 'changed', closed: 'closed' },
          ),
          presentation.slot(
            'directory-picker',
            'directories',
            {
              theme,
              locale,
              open: state.dialog === 'directories',
              busy: mutationPending(state) || catalogPending(state),
            },
            { selected: 'selected', closed: 'closed' },
          ),
          presentation.slot(
            'command-palette',
            'palette',
            {
              theme,
              locale,
              open: state.dialog === 'palette',
              busy: false,
              selectionDisabled:
                !!state.embedding.fixed ||
                !conversationRoute(state) ||
                mutationPending(state) ||
                catalogPending(state),
              commands: rootCommands(state, locale, target).map(
                ({ kind: _kind, ...command }) => command,
              ),
            },
            { invoked: 'invoked', selected: 'selected', closed: 'closed' },
          ),
        ],
        {
          gap: 0,
          terminal: { grow: state.dialog === 'queue' ? 0 : 1 },
          style: { width: fill, minWidth: 0 },
        },
      ),
      state.dialog !== null,
      state.dialog === 'terminal' ? { width: pixels(900) } : {},
      state.dialog === 'attachments'
        ? {
            width: 64,
            height: state.attachments.selection?.browse
              ? 20
              : Math.min(20, attached.length * 3 + 7),
          }
        : {},
    ),
  );
  if (target === 'tui')
    out.frame.view = appearanceView(
      presentation,
      state.preferences,
      theme,
      locale,
      out.frame.view,
      resolveLocale('auto', values.locale?.[0]),
    );
  if (!state.embedding.embedded)
    out.frame.surface =
      target === 'desktop'
        ? { kind: 'window', binding: 'surface', title: 'maka', width: 1100, height: 760 }
        : { kind: 'terminal', binding: 'surface' };
  out.actions = actions;
  out.subscriptions = [
    ...(state.catalogWatch
      ? [
          {
            key: state.catalogWatch.key,
            binding: 'agent',
            member: 0,
            method: 'catalog',
            input: state.catalogWatch.input,
            resources: {},
          },
        ]
      : []),
    ...(state.watch
      ? [
          {
            key: state.watch.key,
            binding: 'agent',
            member: 0,
            method: 'watch',
            input: state.watch.input,
            resources: {},
          },
        ]
      : []),
  ];
  return out;
}
export function workspace<N extends Node>(
  presentation: Presentation<N>,
  locale: string,
  terminals = false,
  place = '',
  skills = false,
): Component<State, WorkspaceProps, N> {
  const target = presentation.target;
  const configuredLocale = locale;
  return {
    mount(props, values) {
      const state: State = {
        embedding: props,
        externalSelection: null,
        place,
        preferences: initialAppearance(),
        bookmark: bookmark({ pages: [], category: 'appearance' }),
        revision: 0,
        attachments: { files: [], selection: null, paste: null },
        drafts: [],
        serial: 0,
        epoch: 0,
        catalog: null,
        catalogGeneration: 0,
        catalogWatch: null,
        catalogFilter: 'all',
        catalogFailed: false,
        selected: null,
        model: null,
        executor: null,
        hasBackends: false,
        modelSetup: 'loading',
        modelOpenRequest: null,
        catalogEpoch: 0,
        modelRefreshing: false,
        pages: [],
        projectsOpened: false,
        settingsCategory: 'appearance',
        searchOpen: false,
        browse: null,
        sidebar: target === 'desktop' ? { open: true } : { command: null },
        terminals,
        skills,
        dialog: null,
        image: null,
        window: null,
        source: null,
        navigation: 0,
        landing: { sequence: 0, edge: 'end' },
        tailReceipt: null,
        fileContext: null,
        copyRequest: null,
        messageCopy: null,
        followAttempt: null,
        parts: [],
        run: null,
        queue: null,
        queueEdit: null,
        queueDraft: null,
        interactions: null,
        interaction: null,
        questionSubmission: null,
        runThrough: 0,
        loading: false,
        live: [],
        latestKnown: 0,
        watch: null,
        pending: {},
        uncertain: null,
        restoring: true,
        restoreAfter: 0,
        command: null,
        commandSequence: 0,
        notice: null,
      };
      const actions: Action[] = [];
      catalog(state);
      intent(state, actions, { kind: 'restorePage' }, 'history', 'page', { after: 0, limit: 1 });
      return frame(presentation, configuredLocale, state, values, actions);
    },
    update(state, props, values) {
      if (state.embedding.fixed && state.embedding.session?.id !== props.session?.id) {
        state.bookmark = bookmark({ pages: [], category: 'appearance' });
        state.pages = [];
        state.settingsCategory = 'appearance';
      }
      return frame(
        presentation,
        configuredLocale,
        { ...state, embedding: props, revision: state.revision + 1 },
        values,
      );
    },
    event(previous, input, props, values) {
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
          'navigation-checkpoint-' + navigationKey(state),
          navigationBookmark(state),
          (value) => {
            const item = value as Navigation | null;
            return item &&
              Array.isArray(item.pages) &&
              item.pages.length <= 3 &&
              item.pages.every((page) =>
                ['settings', 'connections', 'deployment', 'projects'].includes(page),
              ) &&
              ['appearance', 'interface', 'models', 'plugins'].includes(item.category)
              ? item
              : null;
          },
        );
        if (restored) {
          state.pages = restored.pages;
          if (state.pages.includes('projects') && !state.embedding.fixed)
            state.projectsOpened = true;
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
        if (state.catalogWatch?.key === input.key) {
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
              if (selected && selected.revision >= state.selected.revision)
                state.selected = selected;
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
          if (
            state.dialog === 'attachments' &&
            state.attachments.selection &&
            !mutationPending(state)
          )
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
              if (
                event.requestId === state.interaction?.requestId &&
                request?.kind === 'permissions'
              )
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
            else if (
              input.name === 'submit' &&
              state.interaction?.record?.request.kind === 'form'
            ) {
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
          if (state.dialog === 'palette' && conversationRoute(state)) {
            if (input.name === 'closed') state.dialog = null;
            else if (input.name === 'invoked') {
              const command = rootCommands(state, locale, target).find(
                (command) => command.id === (input.event as { id: string }).id && !command.disabled,
              );
              if (
                command &&
                rootCommand(state, actions, locale, command.kind, target) &&
                state.dialog === 'palette'
              )
                state.dialog = null;
            } else if (
              input.name === 'selected' &&
              !state.embedding.fixed &&
              !mutationPending(state) &&
              !catalogPending(state)
            ) {
              select(state, actions, input.event as AgentSession);
              state.dialog = null;
            }
          }
          return frame(presentation, configuredLocale, state, values, actions);
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
            else if (event.action === 'retract')
              edit = { kind: 'retract', messageId: row.messageId };
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
          const { sessionId, details } = input.event as { sessionId: string; details?: boolean };
          const session = state.catalog?.sessions.find((item) => item.id === sessionId);
          if (session) {
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
        if (input.key === 'home-new') rootCommand(state, actions, locale, 'create', target);
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
            intent(
              state,
              actions,
              { kind: 'receipt' },
              'receipts',
              'read',
              state.uncertain.reference,
            );
        } else if (
          state.dialog === 'attachments' &&
          state.attachments.selection &&
          draftReady(state.drafts, state.selected) &&
          state.selected?.id === state.attachments.selection.sessionId &&
          sameValue(state.selected.dataSource, state.attachments.selection.dataSource) &&
          !mutationPending(state)
        ) {
          const selection = state.attachments.selection;
          const files = attachmentFiles(
            state.attachments,
            selection.sessionId,
            selection.dataSource,
          );
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
          const kind = state.interactions?.entries.find(
            (entry) => entry.requestId === requestId,
          )?.kind;
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
        } else if (
          input.key === 'palette-open' &&
          conversationRoute(state) &&
          state.dialog === null
        ) {
          state.dialog = 'palette';
          state.copyRequest = ++state.serial;
          state.messageCopy = null;
        } else if (input.key === 'help-open') {
          if (state.dialog === null) rootCommand(state, actions, locale, 'help-open', target);
        } else if (input.key === 'workspace-dialog') {
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
        } else if (
          input.key === 'page-back' &&
          state.pages.at(-1) === 'settings' &&
          !state.dialog
        ) {
          state.pages.pop();
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
          if (command) rootCommand(state, actions, locale, command.kind, target);
        }
      }
      return frame(presentation, configuredLocale, state, values, actions);
    },
  };
}
