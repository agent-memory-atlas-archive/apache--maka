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

import { type AppearanceEditor } from '../../shared/appearance-editor.js';
import { type SettingsCategory } from '../../shared/settings.js';
import type {
  ModelSelection,
  AgentExecutorSelection,
  ThinkingLevel,
  AgentQueue,
  AgentPendingInteractions,
  InteractionRecord,
  AgentQueueMessage,
  AgentHistoryPage,
  AgentHistoryChunk,
  SourceField,
  SourceFormat,
  SourcePosition,
  HistoryPosition,
  AgentPart,
  AgentRun,
  AgentSession,
  AgentDataSource,
  AgentSessionPage,
  AgentList,
  AgentScheduleAlerts,
  FileCapture,
  RecoveryRef,
  EditorCommand,
  EditorObservation,
  TerminalPaneCommand,
  ReadingCommand,
} from '../../../packages/plugin-sdk/src/index.js';
import type {
  AgentScreenProps,
  InsightsProps,
  PageChrome,
  SelectedModel,
  ModelSetup,
  TimelineFileContext,
  MessageCopyObservation,
} from '../../shared/contracts.js';
import { type Bookmark } from '../../shared/bookmark.js';
import { type Copy } from '../../shared/locale.js';

import { type AttachmentDrafts } from '../../shared/attachments.js';

import { type DraftRecord, type DraftStep } from '../../shared/drafts.js';

type Notice = keyof Copy | null;

export type Backend =
  | { kind: 'model'; model: ModelSelection; thinkingLevel: ThinkingLevel | null }
  | { kind: 'executor'; selection: AgentExecutorSelection };

export type Request =
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
  | { kind: 'openSession'; selection: NonNullable<AgentScreenProps['session']> }
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

export type Mutation =
  | Extract<
      Request,
      { kind: 'create' | 'send' | 'stop' | 'queue' | 'respond' | 'markRead' | 'uploadAttachment' }
    >
  | { kind: 'restored' };

export interface DisplayPart extends AgentPart {
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

export type Page =
  | 'settings'
  | 'connections'
  | 'deployment'
  | 'projects'
  | 'imports'
  | 'apps'
  | 'recall'
  | 'scheduler';

export type Navigation = { pages: State['pages']; category: SettingsCategory };

export function navigationBookmark(state: State): Navigation {
  return { pages: state.pages, category: state.settingsCategory };
}

export function navigationKey(state: State): string {
  return (
    'navigation/' +
    (state.embedding.fixed ? (state.embedding.session?.id ?? 'pending') : 'agent-screen')
  );
}

// Bounded visited workspaces keep their original native reading owners. These
// are presentation snapshots only; run, queue and permission facts are refreshed.
export interface Visit {
  id: string;
  dataSource: AgentDataSource;
  key: string;
  window: State['window'];
  live: State['live'];
  following: boolean;
  source: State['source'];
  image: State['image'];
  landing: State['landing'];
  searchOpen: boolean;
  browse: number | null;
  inspector: State['inspector'];
  output: string | null;
  outputFocus: ReadingCommand | null;
  session: AgentSession;
  epoch: number;
  progress: {
    through: number;
    goal: string;
    action: string;
    outcome: 'completed' | 'failed' | 'aborted' | null;
    toolIssue: boolean;
  } | null;
}

export function readingKey(state: State): string {
  return state.visits.find((visit) => visit.id === state.selected?.id)?.key ?? 'timeline-empty';
}

export interface State {
  visits: Visit[];
  bookmark: Bookmark<Navigation>;
  embedding: AgentScreenProps;
  externalSelection: AgentScreenProps['session'] | null;
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
  connectionSetupRequest: number | null;
  catalogEpoch: number;
  modelRefreshing: boolean;
  searchOpen: boolean;
  browse: number | null;
  pages: Page[];
  openedPages: Page[];
  pageChrome: Partial<Record<Page, PageChrome>>;
  pageBack: Partial<Record<Page, number>>;
  recallAnchor: number | null;
  scheduledTasks: boolean;
  web: boolean;
  jev: boolean;
  scheduleAlerts: AgentScheduleAlerts;
  scheduleWatch: string | null;
  scheduleRequested: { sequence: number; taskId: string } | null;
  settingsCategory: SettingsCategory;
  insightsScope: InsightsProps['scope'];
  sidebar: { open: boolean } | { command: TerminalPaneCommand | null };
  inspector: 'details' | 'terminal' | 'outputs' | null;
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

export type RootCommand =
  | 'browse'
  | 'outputs-open'
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
  | 'usage-open'
  | 'create-workspace'
  | 'navigation-toggle'
  | 'settings'
  | 'projects-open'
  | 'imports-open'
  | 'recall-open'
  | 'scheduler-open'
  | 'apps-open'
  | 'workhub-open'
  | 'deployment-open'
  | 'reload'
  | 'stop'
  | 'check';
