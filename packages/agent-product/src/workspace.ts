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
  AgentHistoryChunk,
  AgentToolSourcePage,
  ToolSourceField,
  SourceFormat,
  SourcePosition,
  HistoryPosition,
  AgentObservation,
  AgentPart,
  AgentRun,
  AgentSession,
  AgentSessionPage,
  AgentList,
  AgentStarted,
  CallerIntent,
  Component,
  ComponentValues,
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
  | { kind: 'list'; generation: number }
  | { kind: 'observe'; epoch: number }
  | {
      kind: 'history';
      epoch: number;
      through: number;
      navigation: number;
      latest: boolean;
      implicit: boolean;
      position: HistoryPosition;
    }
  | {
      kind: 'source';
      epoch: number;
      navigation: number;
      through: number;
      position: HistoryPosition;
    }
  | { kind: 'read'; draft: EditorObservation; sessionId: string; model: ModelSelection }
  | {
      kind: 'toolSource';
      epoch: number;
      navigation: number;
      through: number;
      sequence: number;
      field: ToolSourceField;
      position: SourcePosition;
    }
  | { kind: 'create' }
  | { kind: 'send'; draft: EditorObservation; sessionId: string; model: ModelSelection }
  | { kind: 'stop'; sessionId: string }
  | { kind: 'receipt' }
  | { kind: 'restorePage' }
  | { kind: 'restoreReceipt'; reference: RecoveryRef; cursor: number };
type Mutation = Extract<Request, { kind: 'create' | 'send' | 'stop' }> | { kind: 'restored' };
interface DisplayPart extends AgentPart {
  invocationId: string | null;
}
interface Uncertain {
  reference: RecoveryRef;
  request: Mutation;
}
export interface State {
  revision: number;
  serial: number;
  epoch: number;
  catalog: AgentSessionPage | null;
  catalogGeneration: number;
  catalogFailed: boolean;
  selected: AgentSession | null;
  model: SettingsModelChoice | null;
  catalogEpoch: number;
  modelRefreshing: boolean;
  settingsOpen: boolean;
  deploymentOpen: boolean;
  navigationOpen: boolean;
  window: { page: Extract<AgentHistoryPage, { kind: 'ready' }>; latest: boolean } | null;
  navigation: number;
  image: { sessionId: string; through: number; sequence: number } | null;
  source: {
    through: number;
    chunk: AgentHistoryChunk;
    navigation: number;
    landing: { sequence: number; edge: 'start' | 'end' };
    loading: boolean;
    failed: boolean;
    position: HistoryPosition | null;
    toolSource: {
      field: ToolSourceField;
      format: SourceFormat | null;
      ready: boolean;
      position: SourcePosition;
    } | null;
  } | null;
  landing: { sequence: number; edge: 'start' | 'end' };
  tailReceipt: { landing: number; readingRevision: number } | null;
  followAttempt: { through: number; landing: number; readingRevision: number } | null;
  parts: DisplayPart[];
  run: AgentRun | null;
  runThrough: number;
  loading: boolean;
  live: {
    sequence: number;
    messageId: string;
    invocationId: string;
    stepId: string;
    text: string;
    closedAt: number | null;
  }[];
  latestKnown: number;
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
  if (
    state.window &&
    sequence <= state.window.page.through &&
    state.window.page.chunks.some((chunk) => chunk.messageId === messageId)
  )
    return;
  const ids = new Set(state.live.map((fragment) => fragment.messageId));
  const remaining = 16000 - state.live.reduce((total, fragment) => total + fragment.text.length, 0);
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
  const canonical = (state.window?.page.chunks ?? []).map((chunk) => ({
    id: chunk.messageId,
    contentKey: `canonical-${chunk.sequence}-${chunk.offset}`,
    role: chunk.role,
    text: chunk.text,
    tool: chunk.tool,
    continued: chunk.offset !== 0 || utf8Bytes(chunk.text) !== chunk.totalBytes,
    source: chunk.sequence,
    image: chunk.tool?.kind === 'result' && chunk.tool.image,
  }));
  if (!state.window?.latest) return canonical;
  const live = new Map<string, Message>();
  for (const fragment of state.live) {
    if (
      fragment.sequence <= state.window.page.through &&
      canonical.some((message) => message.id === fragment.messageId)
    )
      continue;
    let message = live.get(fragment.messageId);
    if (!message) {
      message = {
        id: 'live-' + fragment.messageId,
        contentKey: 'live-' + fragment.messageId,
        role: 'assistant',
        text: '',
        tool: null,
        continued: true,
        source: null,
        image: false,
      };
      live.set(fragment.messageId, message);
    }
    message.text += fragment.text;
  }
  return [...canonical, ...live.values()];
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
  actions.push({ key, binding, member: 0, method, input, resources, continuations });
}
function createSession(state: State, actions: Action[], locale: string) {
  if (!mutationPending(state) && !catalogPending(state))
    intent(state, actions, { kind: 'create' }, 'agent', 'create', { title: copy(locale).new });
}
function catalogPending(state: State): boolean {
  return Object.values(state.pending).some(
    (request) => request.kind === 'list' && request.generation === state.catalogGeneration,
  );
}
function catalog(state: State, actions: Action[], position: AgentList = { kind: 'start' }) {
  state.catalogFailed = false;
  intent(
    state,
    actions,
    { kind: 'list', generation: ++state.catalogGeneration },
    'agent',
    'list',
    position,
  );
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
) {
  if (!state.selected) return;
  if (!implicit) state.loading = true;
  const navigation = ++state.navigation;
  intent(
    state,
    actions,
    { kind: 'history', epoch: state.epoch, navigation, through, latest, implicit, position },
    'agent',
    'history',
    { sessionId: state.selected.id, through, position },
    {},
    implicit ? [{ child: 'timeline', method: 'tail', input: null }] : [],
  );
}
function follow(state: State, actions: Action[]) {
  const receipt = state.tailReceipt;
  if (
    receipt?.landing === state.landing.sequence &&
    !state.source &&
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
function select(state: State, actions: Action[], session: AgentSession) {
  state.selected = session;
  state.epoch++;
  state.window = null;
  state.source = null;
  state.image = null;
  state.tailReceipt = null;
  state.followAttempt = null;
  state.navigation++;
  state.parts = [];
  state.run = null;
  state.watch = null;
  state.runThrough = 0;
  state.live = [];
  state.latestKnown = 0;
  state.loading = true;
  state.notice = null;
  observe(state, actions);
}
function readToolSource(state: State, actions: Action[], position: SourcePosition) {
  const source = state.source;
  if (!source?.toolSource || !state.selected) return;
  source.navigation = ++state.navigation;
  source.loading = true;
  source.failed = false;
  source.toolSource.position = position;
  intent(
    state,
    actions,
    {
      kind: 'toolSource',
      epoch: state.epoch,
      navigation: source.navigation,
      through: source.through,
      sequence: source.chunk.sequence,
      field: source.toolSource.field,
      position,
    },
    'agent',
    'toolSource',
    {
      sessionId: state.selected.id,
      through: source.through,
      sequence: source.chunk.sequence,
      field: source.toolSource.field,
      position,
    },
  );
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
    catalog(state, actions);
  } else if (request.kind === 'create') {
    const session = value as AgentSession;
    select(state, actions, session);
    catalog(state, actions);
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
    settled(state, actions, request, reply.receipt.outcome.value);
  } else {
    state.uncertain = { reference: reply.reference, request };
    state.notice = 'unknown';
  }
}
function queryFailed(state: State, request: Request) {
  // Optional presentation work does not disable input or change execution facts.
  if (request.kind === 'list') {
    if (request.generation === state.catalogGeneration) state.catalogFailed = true;
    return;
  }
  if (request.kind === 'history' && request.implicit) return;
  if (request.kind === 'source' || request.kind === 'toolSource') {
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
      if (request.generation !== state.catalogGeneration) break;
      state.catalog = value as AgentSessionPage;
      state.catalogFailed = false;
      if (!state.selected && state.catalog.sessions.length)
        select(state, actions, state.catalog.sessions[0]);
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
      state.selected = observation.session;
      if (observation.through >= state.runThrough) {
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
      if (request.epoch !== state.epoch || !source || request.navigation !== source.navigation)
        break;
      const page = value as AgentHistoryPage;
      if (page.through !== request.through) {
        queryFailed(state, request);
        break;
      }
      if (page.kind === 'preparing') {
        intent(state, actions, request, 'agent', 'history', {
          sessionId: state.selected!.id,
          through: request.through,
          position: request.position,
        });
        break;
      }
      const chunk = page.chunks.find((chunk) => chunk.sequence === source.chunk.sequence);
      const end = chunk ? chunk.offset + utf8Bytes(chunk.text) : 0;
      const position = request.position;
      if (
        !chunk ||
        chunk.messageId !== source.chunk.messageId ||
        chunk.totalBytes !== source.chunk.totalBytes ||
        !chunk.text ||
        end > chunk.totalBytes ||
        (position.kind === 'after' && chunk.offset !== position.cursor.offset) ||
        (position.kind === 'before' && end !== position.cursor.offset)
      ) {
        queryFailed(state, request);
        break;
      }
      source.chunk = chunk;
      source.loading = false;
      source.failed = false;
      source.landing = {
        sequence: request.navigation,
        edge: position.kind === 'before' ? 'end' : 'start',
      };
      break;
    }
    case 'toolSource': {
      const source = state.source;
      if (
        request.epoch !== state.epoch ||
        !source?.toolSource ||
        request.navigation !== source.navigation ||
        request.field !== source.toolSource.field
      )
        break;
      const page = value as AgentToolSourcePage;
      if (page.through !== request.through) {
        queryFailed(state, request);
        break;
      }
      if (page.kind === 'preparing') {
        intent(state, actions, request, 'agent', 'toolSource', {
          sessionId: state.selected!.id,
          through: request.through,
          sequence: request.sequence,
          field: request.field,
          position: request.position,
        });
        break;
      }
      const next = page.source;
      const end = next.offset + utf8Bytes(next.text);
      if (
        next.sequence !== request.sequence ||
        next.field !== request.field ||
        end > next.totalBytes ||
        (source.toolSource.ready &&
          (next.totalBytes !== source.chunk.totalBytes ||
            next.format !== source.toolSource.format)) ||
        (request.position.kind === 'after' && next.offset !== request.position.offset) ||
        (request.position.kind === 'before' && end !== request.position.offset) ||
        (request.position.kind === 'start' && next.offset !== 0) ||
        (request.position.kind === 'tail' && end !== next.totalBytes)
      ) {
        queryFailed(state, request);
        break;
      }
      source.chunk = { ...source.chunk, ...next };
      source.toolSource.format = next.format;
      source.toolSource.ready = true;
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
          },
          {},
          request.implicit ? [{ child: 'timeline', method: 'tail', input: null }] : [],
        );
        break;
      }
      state.window = { page, latest: request.latest };
      state.landing = {
        sequence: state.landing.sequence + 1,
        edge: request.position.kind === 'before' || request.latest ? 'end' : 'start',
      };
      state.loading = false;
      if (!request.implicit) state.notice = null;
      state.live = state.live.filter(
        (fragment) =>
          fragment.sequence > page.through ||
          ((fragment.closedAt === null || fragment.closedAt > page.through) &&
            !page.chunks.some((chunk) => chunk.messageId === fragment.messageId)),
      );
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
function delivery(state: State, page: AgentDeliveryPage, actions: Action[]) {
  const watch = state.watch;
  if (!watch) return;
  state.latestKnown = Math.max(state.latestKnown, page.through);
  let canonicalChanged = false;
  for (const event of page.events) {
    if (event.sequence <= watch.last) continue;
    const fact = event.fact;
    if (
      ['opened', 'transcript_changed', 'part_finished', 'step_ended', 'ended'].includes(fact.kind)
    )
      canonicalChanged = true;
    if (fact.kind === 'opened') {
      if (event.sequence > state.runThrough) {
        state.run = { invocationId: event.invocationId, state: 'running' };
        state.runThrough = event.sequence;
      }
    } else if (fact.kind === 'part_started') {
      state.parts = state.parts.filter(
        (part) =>
          part.invocationId !== event.invocationId ||
          part.stepId !== fact.stepId ||
          part.partId !== fact.partId,
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
        invocationId: event.invocationId,
      });
    } else if (fact.kind === 'delta') {
      const part = state.parts.find(
        (part) =>
          part.invocationId === event.invocationId &&
          part.stepId === fact.stepId &&
          part.partId === fact.partId,
      );
      if (part?.textKind === 'text') {
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
      sessions: state.catalog?.sessions ?? [],
      selected: state.selected?.id ?? null,
      busy: busy || catalogPending(state),
      previous: !!state.catalog?.before,
      more: !!state.catalog?.after,
      failed: state.catalogFailed,
    },
    { create: 'create', select: 'select', more: 'more', previous: 'previous', refresh: 'refresh' },
  );
  const timeline = slot(
    'timeline',
    'timeline',
    {
      theme,
      locale,
      messages: messages(state),
      source: state.source
        ? {
            kind: state.source.toolSource?.field ?? 'message',
            format: state.source.toolSource?.format ?? 'text',
            ready: state.source.toolSource?.ready ?? true,
            key: `${state.source.toolSource ? state.source.toolSource.field + '-' : ''}${state.source.through}-${state.source.chunk.sequence}-${state.source.chunk.offset}`,
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
      live: !!state.run && !!state.window?.latest,
    },
    {
      older: 'older',
      newer: 'newer',
      latest: 'latest',
      source: 'source',
      toolSource: 'tool-source',
      viewImage: 'view-image',
      sourceFirst: 'source-first',
      sourcePrevious: 'source-previous',
      sourceNext: 'source-next',
      sourceLast: 'source-last',
      sourceBack: 'source-back',
      sourceRetry: 'source-retry',
      tailReady: 'tail-ready',
    },
  );
  const imageViewer = slot(
    'image-viewer',
    'imageViewer',
    {
      theme,
      locale,
      image: state.image,
      active:
        !!state.image &&
        !state.settingsOpen &&
        !state.deploymentOpen &&
        (target === 'desktop' || !state.navigationOpen),
    },
    { closed: 'image-closed' },
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
  const title = ui.text('title', state.selected?.title ?? 'maka', 17, theme.text, 550);
  const models = slot(
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
  );
  if (target === 'desktop')
    title.style = {
      ...(title.style as object),
      width: pixels(0),
      grow: 1,
      minWidth: 0,
      truncate: true,
    };
  const main = ui.box(
    'main',
    [
      ui.box(
        'header',
        [
          ...(target === 'desktop'
            ? [
                ui.box(
                  'context',
                  [
                    title,
                    ui.button(
                      'navigation-toggle',
                      state.navigationOpen ? text.hideSidebar : text.showSidebar,
                    ),
                  ],
                  { row: true, gap: 8, style: { width: fill, align: 'center' } },
                ),
                ui.box(
                  'toolbar',
                  [
                    ui.box('model-control', [models], {
                      style: { width: pixels(0), grow: 1, minWidth: 0 },
                    }),
                    ui.button('settings', text.settings),
                    ui.button('deployment-open', text.deploymentTitle),
                  ],
                  { row: true, gap: 8, style: { width: fill, align: 'center' } },
                ),
              ]
            : [
                title,
                models,
                ui.box(
                  'toolbar',
                  [
                    ui.button('settings', text.settings),
                    ui.button('deployment-open', text.deploymentTitle),
                    ui.button('navigation-toggle', text.showSidebar),
                  ],
                  { row: true, gap: 1 },
                ),
              ]),
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
          gap: target === 'desktop' ? 8 : 1,
          style: {
            width: fill,
            shrink: 0,
            padding: { top: 16, right: 24, bottom: 16, left: 24 },
          },
        },
      ),
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
                style: { width: fill, height: pixels(0), grow: 1, shrink: 1, minHeight: 0 },
              },
            ),
            ui.box('input-area', [composer], {
              visible: !state.image,
              padding: 24,
              style: { width: fill, maxWidth: 808, shrink: 0 },
            }),
          ]
        : [
            ui.box('retained-composer', [composer], { visible: !state.source && !state.image }),
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
              { terminal: { grow: 1 } },
            ),
          ]),
    ],
    {
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
    target === 'desktop'
      ? [
          ui.box('sidebar', [navigation], {
            visible: state.navigationOpen,
            style: { width: pixels(240), height: fill, shrink: 0 },
          }),
          main,
        ]
      : [
          ui.box('main-route', [main], { visible: !state.navigationOpen, terminal: { grow: 1 } }),
          ui.box('navigation-route', [ui.button('navigation-back', text.hideSidebar), navigation], {
            visible: state.navigationOpen,
            terminal: { grow: 1 },
          }),
        ],
    {
      row: target === 'desktop',
      gap: 0,
      visible: !state.settingsOpen && !state.deploymentOpen,
      terminal: { grow: 1 },
      style: { width: fill, height: fill },
    },
  );
  const out = output(
    state,
    ui.box(
      'product',
      [
        conversation,
        ...(state.deploymentOpen
          ? [slot('deployment', 'deployment', { theme, locale }, { closed: 'deployment-closed' })]
          : []),
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
      {
        gap: 0,
        style: { width: fill, height: fill, align: 'center' },
        terminal: { grow: 1 },
        shortcuts: [
          {
            key: 'b',
            modifiers: ['primary'],
            input: 'navigation-toggle',
            enabled: !state.settingsOpen && !state.deploymentOpen,
          },
          {
            key: 'n',
            modifiers: ['primary'],
            input: 'create',
            enabled:
              !state.settingsOpen &&
              !state.deploymentOpen &&
              !mutationPending(state) &&
              !catalogPending(state),
          },
          ...(state.navigationOpen && !state.settingsOpen && !state.deploymentOpen
            ? [
                {
                  key: 'escape',
                  modifiers: [],
                  input: 'navigation-back',
                  enabled: true,
                } satisfies import('../../plugin-sdk/src/index.js').Shortcut,
              ]
            : []),
        ],
      },
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
        catalog: null,
        catalogGeneration: 0,
        catalogFailed: false,
        selected: null,
        model: null,
        catalogEpoch: 0,
        modelRefreshing: false,
        settingsOpen: false,
        deploymentOpen: false,
        navigationOpen: target === 'desktop',
        image: null,
        window: null,
        source: null,
        navigation: 0,
        landing: { sequence: 0, edge: 'end' },
        tailReceipt: null,
        followAttempt: null,
        parts: [],
        run: null,
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
      catalog(state, actions);
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
              (request.kind === 'create' || request.kind === 'send' || request.kind === 'stop')
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
        if (input.child === 'timeline' && input.name === 'tailReady') {
          const receipt = input.event as NonNullable<State['tailReceipt']>;
          if (receipt.landing === state.landing.sequence) {
            state.tailReceipt = receipt;
            follow(state, actions);
          }
        } else if (input.child === 'deployment' && input.name === 'closed')
          state.deploymentOpen = false;
        else if (input.child === 'connections' && input.name === 'closed')
          state.settingsOpen = false;
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
              { kind: 'read', draft, sessionId: state.selected.id, model: state.model.selection },
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
          intent(state, actions, { kind: 'stop', sessionId: state.selected.id }, 'agent', 'stop', {
            sessionId: state.selected.id,
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
        } else if (input.name === 'create') createSession(state, actions, locale);
        else if (input.name === 'select' && !mutationPending(state) && !catalogPending(state)) {
          const sessionId = (input.event as { sessionId: string }).sessionId;
          const session = state.catalog?.sessions.find((item) => item.id === sessionId);
          if (session) select(state, actions, session);
        } else if (
          input.child === 'navigation' &&
          ['refresh', 'more', 'previous'].includes(input.name) &&
          !mutationPending(state) &&
          !catalogPending(state)
        ) {
          const page = state.catalog;
          if (input.name === 'refresh') catalog(state, actions);
          else if (!state.catalogFailed && page) {
            if (input.name === 'more' && page.after)
              catalog(state, actions, {
                kind: 'after',
                revision: page.revision,
                cursor: page.after,
              });
            else if (input.name === 'previous' && page.before)
              catalog(state, actions, {
                kind: 'before',
                revision: page.revision,
                cursor: page.before,
              });
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
            (c) =>
              c.sequence === sequence &&
              c.role === 'tool_result' &&
              c.tool?.kind === 'result' &&
              c.tool.image,
          );
          if (chunk)
            state.image = {
              sessionId: state.selected.id,
              through: state.window.page.through,
              sequence,
            };
        } else if (input.child === 'timeline' && state.source) {
          const source = state.source;
          if (input.name === 'sourceBack') state.source = null;
          else if (!source.loading && source.toolSource) {
            const tool = source.toolSource;
            const chunk = source.chunk;
            const end = chunk.offset + utf8Bytes(chunk.text);
            const position: SourcePosition | null =
              input.name === 'sourceRetry' && source.failed
                ? tool.position
                : input.name === 'sourceFirst'
                  ? { kind: 'start' }
                  : input.name === 'sourceLast'
                    ? { kind: 'tail' }
                    : input.name === 'sourcePrevious' && chunk.offset > 0
                      ? { kind: 'before', offset: chunk.offset }
                      : input.name === 'sourceNext' && end < chunk.totalBytes
                        ? { kind: 'after', offset: end }
                        : null;
            if (position) readToolSource(state, actions, position);
          } else if (!source.loading) {
            const chunk = source.chunk;
            const end = chunk.offset + utf8Bytes(chunk.text);
            const offset =
              input.name === 'sourceFirst'
                ? 0
                : input.name === 'sourcePrevious'
                  ? chunk.offset
                  : input.name === 'sourceNext'
                    ? end
                    : chunk.totalBytes;
            const position: HistoryPosition | null =
              input.name === 'sourceRetry' && source.failed
                ? source.position
                : (input.name === 'sourceFirst' || input.name === 'sourcePrevious') &&
                    chunk.offset > 0
                  ? {
                      kind: input.name === 'sourceFirst' ? 'after' : 'before',
                      cursor: { sequence: chunk.sequence, offset },
                    }
                  : (input.name === 'sourceNext' || input.name === 'sourceLast') &&
                      end < chunk.totalBytes
                    ? {
                        kind: input.name === 'sourceNext' ? 'after' : 'before',
                        cursor: { sequence: chunk.sequence, offset },
                      }
                    : null;
            if (position) {
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
                  position,
                },
                'agent',
                'history',
                { sessionId: state.selected!.id, through: source.through, position },
              );
            }
          }
        } else if (input.child === 'timeline' && !state.loading && state.window) {
          const page = state.window.page;
          if (input.name === 'source') {
            const chunk = page.chunks.find(
              (chunk) => chunk.sequence === (input.event as { sequence: number }).sequence,
            );
            if (chunk)
              state.source = {
                through: page.through,
                chunk,
                navigation: ++state.navigation,
                landing: { sequence: state.navigation, edge: 'start' },
                loading: false,
                failed: false,
                position: null,
                toolSource: null,
              };
          } else if (input.name === 'toolSource') {
            const chunk = page.chunks.find(
              (chunk) => chunk.sequence === (input.event as { sequence: number }).sequence,
            );
            if (chunk && (chunk.role === 'tool_call' || chunk.role === 'tool_result')) {
              state.source = {
                through: page.through,
                chunk: { ...chunk, offset: 0, totalBytes: 0, text: '' },
                navigation: state.navigation,
                landing: { sequence: state.navigation, edge: 'start' },
                loading: false,
                failed: false,
                position: null,
                toolSource: {
                  field: chunk.role === 'tool_call' ? 'arguments' : 'result',
                  format: null,
                  ready: false,
                  position: { kind: 'start' },
                },
              };
              readToolSource(state, actions, { kind: 'start' });
            }
          } else if (input.name === 'older' && page.before)
            history(state, actions, { kind: 'before', cursor: page.before }, page.through, false);
          else if (input.name === 'newer' && page.after)
            history(state, actions, { kind: 'after', cursor: page.after }, page.through, false);
          else if (input.name === 'latest')
            history(state, actions, { kind: 'tail' }, state.latestKnown, true);
        }
      } else if (input.kind === 'raw') {
        if (input.key === 'settings') {
          state.settingsOpen = true;
          state.deploymentOpen = false;
        }
        if (input.key === 'deployment-open') {
          state.deploymentOpen = true;
          state.settingsOpen = false;
        } else if (input.key === 'navigation-toggle') state.navigationOpen = !state.navigationOpen;
        else if (input.key === 'navigation-back') state.navigationOpen = false;
        else if (input.key === 'create') createSession(state, actions, locale);
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
          else if (!state.selected) catalog(state, actions);
          else {
            const window = state.window;
            select(state, actions, state.selected);
            state.window = window;
          }
        }
      }
      return frame(target, locale, state, values, actions);
    },
  };
}
