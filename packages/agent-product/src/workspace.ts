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
  ModelSelection,
  SettingsModelChoice,
  AgentDeliveryPage,
  AgentHistoryPage,
  AgentHistoryPosition,
  AgentObservation,
  AgentPart,
  AgentRun,
  AgentSession,
  AgentSessionPage,
  AgentStarted,
  CallerIntent,
  Component,
  ComponentValues,
  Cursor,
  OperationReply,
  RecoveryRef,
  EditorCommand,
  EditorObservation,
  TextChunk,
} from '../../plugin-sdk/src/index.js';
import type { Message, Theme } from './contracts.js';
import { sameValue } from './value.js';
import { copy, type Copy } from './locale.js';
import { fill, output, paint, pixels, slot, type Target } from './view.js';

type Notice = keyof Copy | null;
type Request =
  | { kind: 'list'; append: boolean }
  | { kind: 'observe'; epoch: number }
  | {
      kind: 'history';
      epoch: number;
      through: number;
      refresh: boolean;
      position: AgentHistoryPosition;
    }
  | { kind: 'read'; draft: EditorObservation; sessionId: string; model: ModelSelection }
  | { kind: 'create' }
  | { kind: 'send'; draft: EditorObservation; sessionId: string; model: ModelSelection }
  | { kind: 'stop'; sessionId: string }
  | { kind: 'receipt' }
  | { kind: 'restorePage' }
  | { kind: 'restoreReceipt'; reference: RecoveryRef; cursor: number };
type Mutation = Extract<Request, { kind: 'create' | 'send' | 'stop' }> | { kind: 'restored' };
interface DisplayPart extends AgentPart {
  continued: boolean;
  invocationId: string | null;
  sealed: boolean;
}
interface Uncertain {
  reference: RecoveryRef;
  request: Mutation;
}
export interface State {
  revision: number;
  serial: number;
  epoch: number;
  sessions: AgentSession[];
  listRevision: string | null;
  listCursor: string | null;
  selected: string | null;
  model: SettingsModelChoice | null;
  catalogEpoch: number;
  modelRefreshing: boolean;
  settingsOpen: boolean;
  messages: Message[];
  through: number | null;
  cursor: Cursor | null;
  parts: DisplayPart[];
  run: AgentRun | null;
  runThrough: number;
  loading: boolean;
  liveMessages: Message[];
  canonicalDone: number;
  refreshTarget: number;
  refreshed: Record<string, { sequence: number; end: number }>;
  watch: { key: string; input: { sessionId: string; after: number }; last: number } | null;
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
const WINDOW_CHARS = 16000;
const WINDOW_MESSAGES = 16;
// A bounded disposable display window. Canonical history remains in the service.
function trim(messages: Message[]): Message[] {
  while (messages.length > WINDOW_MESSAGES) messages.shift();
  let remaining = WINDOW_CHARS;
  for (let index = messages.length - 1; index >= 0; index--) {
    const message = messages[index];
    if (message.text.length > remaining) {
      const tail = message.text.slice(-remaining || message.text.length);
      message.text =
        tail.charCodeAt(0) >= 0xdc00 && tail.charCodeAt(0) <= 0xdfff ? tail.slice(1) : tail;
      message.continued = true;
    }
    remaining = Math.max(0, remaining - message.text.length);
  }
  return messages.filter((message) => message.text.length > 0);
}
function append(state: State, id: string, role: Message['role'], text: string, continued: boolean) {
  let message = state.messages.find((message) => message.id === id);
  if (!message) {
    message = { id, role, text: '', continued };
    state.messages.push(message);
  }
  message.text += text;
  message.continued ||= continued;
  state.messages = trim(state.messages);
}
function intent(
  state: State,
  actions: Action[],
  request: Request,
  binding: string,
  method: string,
  input: unknown,
  resources: Action['resources'] = {},
) {
  const key = 'request-' + ++state.serial;
  state.pending[key] = request;
  actions.push({ key, binding, member: 0, method, input, resources });
}
function observe(state: State, actions: Action[]) {
  if (state.selected)
    intent(state, actions, { kind: 'observe', epoch: state.epoch }, 'agent', 'observe', {
      sessionId: state.selected,
    });
}
function history(state: State, actions: Action[]) {
  if (state.selected && state.through !== null)
    intent(
      state,
      actions,
      {
        kind: 'history',
        epoch: state.epoch,
        through: state.through,
        refresh: false,
        position: state.cursor ? { kind: 'cursor', cursor: state.cursor } : { kind: 'start' },
      },
      'agent',
      'history',
      {
        sessionId: state.selected,
        through: state.through,
        position: state.cursor ? { kind: 'cursor', cursor: state.cursor } : { kind: 'start' },
      },
    );
}
function refresh(state: State, actions: Action[]) {
  if (
    !state.selected ||
    state.refreshTarget <= state.canonicalDone ||
    state.loading ||
    Object.values(state.pending).some(
      (request) => request.kind === 'history' && request.epoch === state.epoch,
    )
  )
    return;
  const through = state.refreshTarget;
  const start = Math.min(state.canonicalDone + 1, ...state.parts.map((part) => part.startSequence));
  intent(
    state,
    actions,
    {
      kind: 'history',
      epoch: state.epoch,
      through,
      refresh: true,
      position: { kind: 'event', sequence: Math.min(start, through) },
    },
    'agent',
    'history',
    {
      sessionId: state.selected,
      through,
      position: { kind: 'event', sequence: Math.min(start, through) },
    },
  );
}
function select(state: State, actions: Action[], sessionId: string) {
  state.selected = sessionId;
  state.epoch++;
  state.messages = [];
  state.through = null;
  state.cursor = null;
  state.parts = [];
  state.run = null;
  state.watch = null;
  state.runThrough = 0;
  state.liveMessages = [];
  state.canonicalDone = 0;
  state.refreshTarget = 0;
  state.refreshed = {};
  state.loading = true;
  state.notice = null;
  observe(state, actions);
}
function mutationPending(state: State) {
  return (
    state.restoring ||
    state.uncertain !== null ||
    Object.values(state.pending).some((request) =>
      ['read', 'create', 'send', 'stop', 'receipt'].includes(request.kind),
    )
  );
}
function executionPending(state: State) {
  return Object.values(state.pending).some(
    (request) => request.kind === 'observe' && request.epoch === state.epoch,
  );
}

function settled(state: State, actions: Action[], request: Mutation, value: unknown) {
  state.uncertain = null;
  if (request.kind === 'restored') {
    state.restoring = true;
    intent(state, actions, { kind: 'restorePage' }, 'history', 'page', {
      after: state.restoreAfter,
      limit: 1,
    });
    intent(state, actions, { kind: 'list', append: false }, 'agent', 'list', {});
  } else if (request.kind === 'create') {
    const session = value as AgentSession;
    state.sessions = [session, ...state.sessions.filter((item) => item.id !== session.id)].slice(
      0,
      128,
    );
    select(state, actions, session.id);
  } else if (request.kind === 'send') {
    const started = value as AgentStarted;
    if (started.sessionId !== request.sessionId) {
      state.notice = 'unknown';
      return;
    }
    // Clear only the exact captured draft after a durable returned admission.
    // The native editor rejects this command if a newer draft or IME composition exists.
    state.command = {
      owner: request.draft.snapshot.text.owner,
      sequence: ++state.commandSequence,
      basedOn: request.draft.revision,
      edit: { kind: 'reset', text: '' },
    };
    state.notice = null;
    if (state.selected === started.sessionId) {
      observe(state, actions);
    }
  } else {
    state.notice = 'stopping';
    if (state.selected === request.sessionId) observe(state, actions);
  }
}
function receipt(state: State, actions: Action[], request: Mutation, reply: OperationReply) {
  if (reply.receipt.kind === 'recorded' && reply.receipt.outcome.kind === 'returned') {
    settled(state, actions, request, reply.receipt.outcome.value);
  } else {
    state.uncertain = { reference: reply.reference, request };
    state.notice = 'unknown';
  }
}
function queryFailed(state: State, request: Request) {
  if (request.kind === 'history' || request.kind === 'observe') {
    if (request.epoch === state.epoch) {
      state.loading = false;
      state.notice = 'unavailable';
    }
  } else {
    if (request.kind === 'restoreReceipt') {
      state.uncertain = { reference: request.reference, request: { kind: 'restored' } };
      state.restoring = false;
      state.notice = 'unknown';
    } else state.notice = request.kind === 'read' ? 'draftKept' : 'unavailable';
  }
}
function querySucceeded(state: State, actions: Action[], request: Request, value: unknown) {
  switch (request.kind) {
    case 'list': {
      const page = value as AgentSessionPage;
      state.sessions = (request.append ? [...state.sessions, ...page.sessions] : page.sessions)
        .filter(
          (session, index, list) => list.findIndex((item) => item.id === session.id) === index,
        )
        .slice(0, 128);
      state.listRevision = page.revision;
      state.listCursor = page.nextCursor ?? null;
      if (!state.selected && state.sessions.length) select(state, actions, state.sessions[0].id);
      break;
    }
    case 'observe': {
      if (request.epoch !== state.epoch) break;
      const observation = value as AgentObservation;
      if (observation.session.id !== state.selected) {
        state.loading = false;
        state.notice = 'unavailable';
        break;
      }
      if (observation.through >= state.runThrough) {
        state.run = observation.run ?? null;
        state.runThrough = observation.through;
      }
      if (!state.watch)
        state.parts = observation.parts.map((part) => ({
          ...part,
          continued: false,
          invocationId: observation.run?.invocationId ?? null,
          sealed: false,
        }));
      state.refreshTarget = Math.max(state.refreshTarget, observation.through);
      // Keep an existing watch's original cursor. Re-observation must not drop
      // intervening text or restart the subscription with every frame.
      if (!state.watch) {
        state.through = observation.through;
        state.cursor = null;
        history(state, actions);
      }
      break;
    }
    case 'history': {
      if (request.epoch !== state.epoch) break;
      const page = value as AgentHistoryPage;
      if (page.through !== request.through) {
        state.loading = false;
        state.notice = 'unavailable';
        break;
      }
      if (page.kind === 'preparing') {
        intent(state, actions, request, 'agent', 'history', {
          sessionId: state.selected,
          through: request.through,
          position: request.position,
        });
        break;
      }
      for (const chunk of page.chunks) {
        const prior = state.refreshed[chunk.messageId];
        if (!prior || chunk.offset >= prior.end) {
          append(
            state,
            chunk.messageId,
            chunk.role,
            chunk.text,
            chunk.offset !== 0 && !state.messages.some((message) => message.id === chunk.messageId),
          );
          state.refreshed[chunk.messageId] = {
            sequence: chunk.sequence,
            end: chunk.offset + utf8Bytes(chunk.text),
          };
        }
        state.liveMessages = state.liveMessages.filter((message) => message.id !== chunk.messageId);
      }
      for (const id of Object.keys(state.refreshed))
        if (!state.messages.some((message) => message.id === id)) delete state.refreshed[id];
      if (request.refresh) {
        if (page.next)
          intent(
            state,
            actions,
            { ...request, position: { kind: 'cursor', cursor: page.next } },
            'agent',
            'history',
            {
              sessionId: state.selected,
              through: request.through,
              position: { kind: 'cursor', cursor: page.next },
            },
          );
        else {
          state.canonicalDone = request.through;
          state.parts = state.parts.filter(
            (part) =>
              !part.sealed && !state.messages.some((message) => message.id === part.messageId),
          );
        }
        break;
      }
      state.cursor = page.next ?? null;
      if (
        state.cursor &&
        state.messages.length < WINDOW_MESSAGES &&
        state.messages.reduce((n, message) => n + message.text.length, 0) < WINDOW_CHARS
      )
        history(state, actions);
      else {
        state.loading = false;
        if (!state.watch && state.selected) {
          const after = Math.min(
            page.through,
            ...state.parts.map((part) => part.startSequence - 1),
          );
          state.watch = {
            key: 'watch-' + state.epoch,
            input: { sessionId: state.selected, after },
            last: after,
          };
          state.canonicalDone = page.through;
        }
      }
      break;
    }
    case 'read': {
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
      if (!chunk.text.trim()) {
        state.notice = 'empty';
        break;
      }
      intent(
        state,
        actions,
        { kind: 'send', draft: request.draft, sessionId: request.sessionId, model: request.model },
        'agent',
        'send',
        { sessionId: request.sessionId, message: chunk.text, model: request.model },
      );
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
function delivery(state: State, page: AgentDeliveryPage) {
  const watch = state.watch;
  if (!watch) return;
  for (const event of page.events) {
    if (event.sequence <= watch.last) continue;
    const fact = event.fact;
    if (fact.kind === 'opened') {
      if (event.sequence > state.runThrough) {
        state.run = { invocationId: event.invocationId, state: 'running' };
        state.runThrough = event.sequence;
      }
      state.refreshTarget = Math.max(state.refreshTarget, page.through);
    } else if (fact.kind === 'part_started') {
      state.parts = state.parts.filter(
        (part) => part.stepId !== fact.stepId || part.partId !== fact.partId,
      );
      if (state.parts.length >= 128) {
        state.watch = null;
        state.notice = 'unavailable';
        return;
      }
      state.parts.push({
        startSequence: event.sequence,
        messageId: fact.messageId,
        stepId: fact.stepId,
        partId: fact.partId,
        textKind: fact.textKind,
        continued: false,
        invocationId: event.invocationId,
        sealed: false,
      });
    } else if (fact.kind === 'delta') {
      const part = state.parts.find(
        (part) => part.stepId === fact.stepId && part.partId === fact.partId,
      );
      if (part?.textKind === 'text') {
        let message = state.liveMessages.find((message) => message.id === part.messageId);
        if (!message) {
          message = { id: part.messageId, role: 'assistant', text: '', continued: false };
          state.liveMessages.push(message);
        }
        message.text += fact.text;
        state.liveMessages = trim(state.liveMessages);
      }
    } else if (fact.kind === 'step_ended') {
      state.parts = state.parts.map((part) =>
        part.stepId === fact.stepId ? { ...part, sealed: true } : part,
      );
      state.refreshTarget = Math.max(state.refreshTarget, page.through);
    } else if (fact.kind === 'transcript_changed')
      state.refreshTarget = Math.max(state.refreshTarget, page.through);
    else if (fact.kind === 'ended') {
      state.parts = state.parts.map((part) =>
        part.invocationId === event.invocationId ? { ...part, sealed: true } : part,
      );
      if (event.sequence > state.runThrough && state.run?.invocationId === event.invocationId) {
        state.run = null;
        state.runThrough = event.sequence;
        state.notice = fact.failed ? 'failed' : 'completed';
      }
      state.refreshTarget = Math.max(state.refreshTarget, page.through);
    }
  }
  watch.last = Math.max(watch.last, page.nextAfter);
}
function frame(
  target: Target,
  locale: string,
  state: State,
  values: ComponentValues,
  actions: Action[] = [],
) {
  const theme = values.appearance?.[0] as Theme;
  const text = copy(locale),
    ui = paint(target, theme),
    busy = mutationPending(state);
  const navigation = slot(
    'navigation',
    'navigation',
    {
      theme,
      locale,
      sessions: state.sessions,
      selected: state.selected,
      busy,
      more: !!state.listCursor && state.sessions.length < 128,
    },
    { create: 'create', select: 'select', more: 'sessions' },
  );
  const timeline = slot(
    'timeline',
    'timeline',
    {
      theme,
      locale,
      messages: [
        ...state.messages,
        ...state.liveMessages.filter(
          (message) => !state.messages.some((canonical) => canonical.id === message.id),
        ),
      ].slice(-32),
      loading: state.loading,
      more: !!state.cursor && !state.run,
      live: !!state.run,
    },
    { more: 'history' },
  );
  const composer = slot(
    'composer',
    'composer',
    {
      theme,
      locale,
      busy: busy || executionPending(state),
      running: !!state.run,
      enabled:
        !!state.selected &&
        !!state.model &&
        !state.modelRefreshing &&
        !state.loading &&
        state.notice !== 'unavailable',
      command: state.command,
    },
    { send: 'send', stop: 'stop', changed: 'draft' },
  );
  const status = state.notice
    ? text[state.notice]
    : state.run
      ? state.run.state === 'waiting_for_user'
        ? text.waiting
        : text.working
      : '';
  const main = ui.box(
    'main',
    [
      ui.box(
        'header',
        [
          ui.text(
            'title',
            state.sessions.find((session) => session.id === state.selected)?.title ?? 'maka',
            17,
            theme.text,
            550,
          ),
          slot(
            'models',
            'models',
            {
              theme,
              locale,
              selection: state.model,
              catalogEpoch: state.catalogEpoch,
              busy: busy || executionPending(state),
            },
            { selected: 'model' },
          ),
          ui.button('settings', text.settings),
          ...(status ? [ui.text('status', status, 13, theme.muted)] : []),
          ...(state.uncertain
            ? [
                ui.button(
                  'check',
                  text.check,
                  Object.values(state.pending).some((request) => request.kind === 'receipt'),
                ),
              ]
            : []),
          ...(state.notice === 'unavailable'
            ? [ui.button('reload', text.loading, state.loading)]
            : []),
        ],
        {
          row: target === 'desktop',
          style: {
            width: fill,
            justify: 'between',
            align: 'center',
            padding: { top: 16, right: 24, bottom: 16, left: 24 },
          },
        },
      ),
      ...(target === 'desktop'
        ? [
            ui.box('conversation', [timeline], {
              style: { width: fill, height: pixels(0), grow: 1, shrink: 1, minHeight: 0 },
            }),
            ui.box('input-area', [composer], {
              padding: 24,
              style: { width: fill, maxWidth: 808, shrink: 0 },
            }),
          ]
        : [composer, timeline]),
    ],
    { style: { width: fill, height: fill, align: 'center', minWidth: 0 } },
  );
  const conversation = ui.box(
    'workspace',
    target === 'desktop'
      ? [
          ui.box('sidebar', [navigation], {
            style: { width: pixels(240), height: fill, shrink: 0 },
          }),
          main,
        ]
      : [main, navigation],
    {
      row: target === 'desktop',
      gap: 0,
      visible: !state.settingsOpen,
      style: { width: fill, height: fill },
    },
  );
  const out = output(
    state,
    ui.box(
      'product',
      [
        conversation,
        ...(state.settingsOpen
          ? [
              slot(
                'connections',
                'connections',
                { theme, locale },
                { closed: 'settings-closed', changed: 'settings-changed' },
              ),
            ]
          : []),
      ],
      { gap: 0, style: { width: fill, height: fill, align: 'center' } },
    ),
  );
  out.frame.surface =
    target === 'desktop'
      ? { kind: 'window', binding: 'surface', title: 'maka', width: 1100, height: 760 }
      : { kind: 'terminal', binding: 'surface' };
  out.actions = actions;
  out.subscriptions = state.watch
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
    : [];
  return out;
}
export function workspace(target: Target, locale: string): Component<State, Record<string, never>> {
  return {
    mount(_props, values) {
      const state: State = {
        revision: 0,
        serial: 0,
        epoch: 0,
        sessions: [],
        listRevision: null,
        listCursor: null,
        selected: null,
        model: null,
        catalogEpoch: 0,
        modelRefreshing: false,
        settingsOpen: false,
        messages: [],
        through: null,
        cursor: null,
        parts: [],
        run: null,
        runThrough: 0,
        loading: false,
        liveMessages: [],
        canonicalDone: 0,
        refreshTarget: 0,
        refreshed: {},
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
      intent(state, actions, { kind: 'list', append: false }, 'agent', 'list', {});
      intent(state, actions, { kind: 'restorePage' }, 'history', 'page', { after: 0, limit: 1 });
      return frame(target, locale, state, values, actions);
    },
    update(state, _props, values) {
      return frame(target, locale, { ...state, revision: state.revision + 1 }, values);
    },
    event(previous, input, _props, values) {
      // Callbacks receive detached committed state. Mutations here belong only to
      // the candidate frame and publish with its intents in one UI transaction.
      const state = previous;
      state.revision++;
      const actions: Action[] = [];
      if (input.kind === 'action') {
        const outcome = input.event as ActionOutcome;
        if (state.watch?.key === input.key) {
          if (outcome.status === 'updated') delivery(state, outcome.value as AgentDeliveryPage);
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
              (request.kind === 'create' || request.kind === 'send' || request.kind === 'stop')
            )
              receipt(state, actions, request, outcome.reply);
            else if (outcome.status === 'succeeded')
              querySucceeded(state, actions, request, outcome.value);
            else queryFailed(state, request);
          }
        }
      } else if (input.kind === 'component') {
        if (input.child === 'connections' && input.name === 'closed') state.settingsOpen = false;
        else if (input.child === 'connections' && input.name === 'changed') {
          state.catalogEpoch++;
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
          input.name === 'send' &&
          !executionPending(state) &&
          state.notice !== 'unavailable' &&
          !mutationPending(state) &&
          state.selected &&
          state.model &&
          !state.modelRefreshing &&
          !state.loading &&
          !state.run &&
          !state.command
        ) {
          const draft = input.event as EditorObservation;
          if (draft.snapshot.marked) state.notice = 'draftKept';
          else if (draft.snapshot.text.bytes > 65536) state.notice = 'tooLarge';
          else
            intent(
              state,
              actions,
              { kind: 'read', draft, sessionId: state.selected, model: state.model.selection },
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
        } else if (
          input.name === 'stop' &&
          !executionPending(state) &&
          !mutationPending(state) &&
          state.selected &&
          state.run
        ) {
          intent(state, actions, { kind: 'stop', sessionId: state.selected }, 'agent', 'stop', {
            sessionId: state.selected,
            invocationId: state.run.invocationId,
          });
        } else if (
          input.child === 'models' &&
          input.name === 'selected' &&
          !mutationPending(state) &&
          !executionPending(state)
        ) {
          state.model = input.event as SettingsModelChoice;
          state.modelRefreshing = false;
        } else if (input.name === 'create' && !mutationPending(state))
          intent(state, actions, { kind: 'create' }, 'agent', 'create', {
            title: copy(locale).new,
          });
        else if (input.name === 'select' && !mutationPending(state)) {
          const sessionId = (input.event as { sessionId: string }).sessionId;
          if (state.sessions.some((session) => session.id === sessionId))
            select(state, actions, sessionId);
        } else if (
          input.child === 'navigation' &&
          input.name === 'more' &&
          state.listCursor &&
          !Object.values(state.pending).some((request) => request.kind === 'list')
        )
          intent(state, actions, { kind: 'list', append: true }, 'agent', 'list', {
            revision: state.listRevision,
            cursor: state.listCursor,
          });
        else if (
          input.child === 'timeline' &&
          input.name === 'more' &&
          state.cursor &&
          !state.loading &&
          !state.run
        ) {
          state.messages = [];
          state.loading = true;
          history(state, actions);
        }
      } else if (input.kind === 'raw') {
        if (input.key === 'settings') state.settingsOpen = true;
        else if (
          input.key === 'check' &&
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
        else if (input.key === 'reload' && !state.loading) {
          if (state.restoring)
            intent(state, actions, { kind: 'restorePage' }, 'history', 'page', {
              after: state.restoreAfter,
              limit: 1,
            });
          else if (!state.selected)
            intent(state, actions, { kind: 'list', append: false }, 'agent', 'list', {});
          else {
            select(state, actions, state.selected);
          }
        }
      }
      refresh(state, actions);
      return frame(target, locale, state, values, actions);
    },
  };
}
