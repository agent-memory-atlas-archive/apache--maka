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
  ComponentInput,
  EditorObservation,
  EditorTextRef,
  ReadingCommand,
  ReadingObservation,
  TextChunk,
  AgentHistoryMatch,
  AgentHistorySearchPage,
  AgentHistorySourcePage,
  HistorySource,
  SourcePosition,
} from '../../plugin-sdk/src/index.js';
import type { TimelineProps } from './contracts.js';
import { copy } from './locale.js';
import { fill, compactLabel, type Node, type Presentation } from './view.js';

export interface ConversationSearch {
  context: NonNullable<TimelineProps['search']>;
  key: string;
  closing: boolean;
  serial: number;
  history: HistorySearch | null;
  observation: EditorObservation | null;
  basis: EditorTextRef | null;
  query: string;
  pending: { key: string; basis: EditorTextRef } | null;
  waiting: number | null;
  failed: boolean;
}
export interface SearchReading {
  revision: number;
  expanded: string[];
  command: ReadingCommand | null;
  observation: ReadingObservation | null;
  search: ConversationSearch | null;
}
function command(state: SearchReading, target: ReadingCommand['target']) {
  const sequence = (state.command?.sequence ?? 0) + 1;
  state.command = { sequence, target };
  return sequence;
}
function find(state: SearchReading, step: 'first' | 'next' | 'previous') {
  const search = state.search!;
  search.waiting = command(state, { kind: 'find', query: search.query, step });
}
export function syncSearch(state: SearchReading, context: TimelineProps['search']) {
  if (!context) {
    if (state.search && !state.search.closing) command(state, { kind: 'clear_find' });
    state.search = null;
  } else if (
    !state.search ||
    state.search.context.epoch !== context.epoch ||
    state.search.context.sessionId !== context.sessionId
  ) {
    state.search = {
      context,
      key: `find-query-${state.revision}`,
      closing: false,
      serial: 0,
      history: null,
      observation: null,
      basis: null,
      query: '',
      pending: null,
      waiting: null,
      failed: false,
    };
    find(state, 'first');
  }
}
function same(a: EditorTextRef | null, b: EditorTextRef | null) {
  return a?.owner === b?.owner && a?.revision === b?.revision && a?.bytes === b?.bytes;
}
export function searchReady(search: ConversationSearch) {
  return (
    !search.closing &&
    !search.pending &&
    !search.failed &&
    !!search.observation &&
    same(search.basis, search.observation.snapshot.text)
  );
}
function capture(state: SearchReading, actions: Action[]) {
  const search = state.search!,
    basis = search.observation?.snapshot.text;
  if (search.pending || !basis || same(basis, search.basis)) return;
  if (basis.bytes > 512) {
    search.failed = true;
    return;
  }
  const key = `${search.key}-${++search.serial}`;
  search.pending = { key, basis };
  actions.push({
    key,
    binding: 'reader',
    member: 0,
    method: 'read',
    input: { start: 0, end: basis.bytes },
    resources: {
      text: {
        child: search.key,
        method: 'snapshot',
        input: { kind: 'at_revision', owner: basis.owner, revision: basis.revision },
      },
    },
  });
}
export function searchEvent(
  state: SearchReading,
  input: ComponentInput,
  props: TimelineProps,
  actions: Action[],
): 'closed' | 'handled' | null {
  const search = state.search;
  if (!search || search.closing) return null;
  if (input.kind === 'action' && input.key === search.pending?.key) {
    const request = search.pending;
    search.pending = null;
    const result = input.event as ActionOutcome;
    if (!same(request.basis, search.observation?.snapshot.text ?? null)) capture(state, actions);
    else if (result.status !== 'succeeded') search.failed = true;
    else {
      const chunk = result.value as TextChunk;
      const bytes = utf8Bytes(chunk.text);
      if (
        !same(chunk.metadata as EditorTextRef, request.basis) ||
        bytes !== request.basis.bytes ||
        bytes > 512
      )
        search.failed = true;
      else {
        search.basis = request.basis;
        search.failed = false;
        if (chunk.text !== search.query) {
          search.query = chunk.text;
          if (search.history) restartHistory(state, props, actions);
          else find(state, 'first');
        }
      }
    }
    return 'handled';
  }
  if (input.kind === 'component' && input.child === search.key) {
    if (input.name === 'changed') {
      search.observation = input.event as EditorObservation;
      search.failed = false;
      capture(state, actions);
    } else if (input.name === 'submitted' && searchReady(search)) {
      navigate(state, true, actions);
    }
    return 'handled';
  }
  if (historyEvent(state, input, props, actions)) return 'handled';
  if (input.kind === 'component' && input.child === 'reading' && input.name === 'observed') {
    const observation = state.observation,
      hit = observation?.find;
    if (
      hit &&
      hit.sequence === search.waiting &&
      observation?.receipt?.sequence === search.waiting &&
      observation.receipt.status === 'applied' &&
      searchReady(search) &&
      !search.history
    ) {
      search.waiting = null;
      if (hit.active) {
        const message = props.messages.find(
          (message) =>
            hit.active!.path.length === 1 && hit.active!.path[0] === 'text-' + message.contentKey,
        );
        if (message) {
          if (!state.expanded.includes(message.id)) state.expanded.push(message.id);
          command(state, { kind: 'reveal_source', ...hit.active, find_sequence: hit.sequence });
        }
      }
    }
    return null;
  }
  if (input.kind === 'raw') {
    if (input.key === 'find-scope') {
      if (search.history) {
        search.history = null;
        find(state, 'first');
      } else {
        command(state, { kind: 'clear_find' });
        restartHistory(state, props, actions);
      }
      return 'handled';
    }
    if (input.key === 'find-close') {
      search.closing = true;
      search.waiting = null;
      command(state, { kind: 'clear_find' });
      return 'closed';
    }
    if (input.key === 'find-next' || input.key === 'find-previous') {
      if (searchReady(search)) navigate(state, input.key === 'find-next', actions);
      return 'handled';
    }
  }
  return null;
}

interface HistorySearch {
  through: number;
  start: number;
  next: number | null;
  offset: number;
  back: { start: number; offset: number }[];
  matches: AgentHistoryMatch[];
  selected: number;
  reveal: ReadingCommand | null;
  selectLast: boolean;
  scanning: boolean;
  failed: boolean;
  pending: { key: string; after: number } | null;
  preview: {
    key: string;
    hit: AgentHistoryMatch;
    source: HistorySource | null;
    pending: { key: string; position: SourcePosition } | null;
    failed: boolean;
    command: ReadingCommand | null;
    highlight: { sequence: number; stage: 'land' | 'find'; reveal: boolean } | null;
  } | null;
}
function action(search: ConversationSearch, method: string, input: unknown): Action {
  return {
    key: `${search.key}-${++search.serial}`,
    binding: 'agent',
    member: 0,
    method,
    input,
    resources: {},
  };
}
function scan(search: ConversationSearch, actions: Action[]) {
  const history = search.history!;
  if (
    !search.query ||
    history.pending ||
    history.next === null ||
    history.matches.length >= 32 ||
    history.failed
  ) {
    history.scanning = false;
    return;
  }
  const after = history.next;
  const request = action(search, 'search', {
    sessionId: search.context.sessionId,
    through: history.through,
    after,
    query: search.query,
    includeInternal: false,
  });
  history.scanning = true;
  history.pending = { key: request.key, after };
  actions.push(request);
}
function restartHistory(state: SearchReading, props: TimelineProps, actions: Action[]) {
  const search = state.search!;
  search.waiting = null;
  search.history = {
    through: props.search!.through,
    start: 0,
    next: search.query ? 0 : null,
    offset: 0,
    back: [],
    matches: [],
    selected: -1,
    reveal: null,
    selectLast: false,
    scanning: false,
    failed: false,
    pending: null,
    preview: null,
  };
  scan(search, actions);
}
function readPreview(search: ConversationSearch, actions: Action[], position: SourcePosition) {
  const history = search.history!,
    preview = history.preview!;
  if (preview.pending) return;
  const request = action(search, 'source', {
    sessionId: search.context.sessionId,
    through: history.through,
    source: { kind: 'sequence', sequence: preview.hit.cursor.sequence },
    field: preview.hit.field,
    position,
  });
  preview.highlight = null;
  preview.pending = { key: request.key, position };
  preview.failed = false;
  actions.push(request);
}
function pick(search: ConversationSearch, index: number, actions: Action[]) {
  const history = search.history!,
    hit = history.matches[index];
  if (!hit || history.selected === index) return;
  history.selected = index;
  history.reveal = {
    sequence: (history.reveal?.sequence ?? 0) + 1,
    target: { kind: 'reveal_child', key: `find-hit-${index}` },
  };
  const sequence = (history.preview?.command?.sequence ?? 0) + 1;
  history.preview = {
    key: `find-preview-reading-${hit.cursor.sequence}-${hit.field}`,
    hit,
    source: null,
    pending: null,
    failed: false,
    highlight: null,
    command: { sequence, target: { kind: 'start' } },
  };
  readPreview(search, actions, { kind: 'start' });
}
function navigate(state: SearchReading, forward: boolean, actions: Action[]) {
  const search = state.search!,
    history = search.history;
  if (!history) {
    find(state, forward ? 'next' : 'previous');
    return;
  }
  const index = history.selected + (forward ? 1 : -1);
  if (index >= 0 && index < history.matches.length) {
    pick(search, index, actions);
    return;
  }
  if (history.scanning || history.failed) return;
  if (forward && history.next !== null) {
    history.back.push({ start: history.start, offset: history.offset });
    if (history.back.length > 64) history.back.shift();
    history.start = history.next;
    history.offset += history.matches.length;
    history.selectLast = false;
  } else if (!forward && history.back.length) {
    const previous = history.back.pop()!;
    history.start = previous.start;
    history.next = previous.start;
    history.offset = previous.offset;
    history.selectLast = true;
  } else return;
  history.matches = [];
  history.selected = -1;
  history.preview = null;
  scan(search, actions);
}
function utf8Bytes(text: string) {
  let bytes = 0;
  for (const character of text) {
    const point = character.codePointAt(0)!;
    bytes += point <= 0x7f ? 1 : point <= 0x7ff ? 2 : point <= 0xffff ? 3 : 4;
  }
  return bytes;
}
function historyEvent(
  state: SearchReading,
  input: ComponentInput,
  props: TimelineProps,
  actions: Action[],
) {
  const search = state.search!,
    history = search.history;
  if (!history) return false;
  if (input.kind === 'action' && input.key === history.pending?.key) {
    const request = history.pending;
    history.pending = null;
    const result = input.event as ActionOutcome;
    if (result.status !== 'succeeded') {
      history.failed = true;
      history.scanning = false;
      return true;
    }
    const page = result.value as AgentHistorySearchPage;
    if (page.through !== history.through) {
      history.failed = true;
      history.scanning = false;
      return true;
    }
    if (page.kind === 'preparing') {
      scan(search, actions);
      return true;
    }
    let after = request.after;
    if (
      page.matches.length > 16 ||
      page.matches.some((hit) => {
        const invalid =
          hit.cursor.sequence <= after ||
          hit.cursor.offset !== 0 ||
          hit.cursor.sequence > history.through * 256 + 255;
        after = hit.cursor.sequence;
        return invalid;
      }) ||
      (page.nextAfter !== null && (page.nextAfter <= request.after || page.nextAfter < after))
    ) {
      history.failed = true;
      history.scanning = false;
      return true;
    }
    const remaining = 32 - history.matches.length;
    history.matches.push(...page.matches.slice(0, remaining));
    history.next =
      page.matches.length > remaining ? history.matches.at(-1)!.cursor.sequence : page.nextAfter;
    if (!history.selectLast && history.selected < 0 && history.matches.length)
      pick(search, 0, actions);
    scan(search, actions);
    if (!history.scanning && history.selectLast && history.matches.length) {
      history.selectLast = false;
      pick(search, history.matches.length - 1, actions);
    }
    return true;
  }
  const preview = history.preview;
  if (input.kind === 'action' && preview && input.key === preview.pending?.key) {
    const request = preview.pending;
    preview.pending = null;
    const result = input.event as ActionOutcome;
    if (result.status !== 'succeeded') {
      preview.failed = true;
      return true;
    }
    const page = result.value as AgentHistorySourcePage;
    if (page.through !== history.through) {
      preview.failed = true;
      return true;
    }
    if (page.kind === 'preparing') {
      readPreview(search, actions, request.position);
      return true;
    }
    const source = page.source,
      end = source.offset + utf8Bytes(source.text),
      position = request.position;
    if (
      source.sequence !== preview.hit.cursor.sequence ||
      source.field !== preview.hit.field ||
      (!source.text && source.totalBytes > 0) ||
      end > source.totalBytes ||
      (preview.source &&
        (source.totalBytes !== preview.source.totalBytes ||
          source.format !== preview.source.format)) ||
      (position.kind === 'start' && source.offset !== 0) ||
      (position.kind === 'tail' && end !== source.totalBytes) ||
      (position.kind === 'after' && source.offset !== position.offset) ||
      (position.kind === 'before' && end !== position.offset)
    ) {
      preview.failed = true;
      return true;
    }
    const first = preview.source === null;
    preview.source = source;
    preview.command = {
      sequence: (preview.command?.sequence ?? 0) + 1,
      target: first
        ? { kind: 'find', query: search.query, step: 'first' }
        : { kind: position.kind === 'tail' || position.kind === 'before' ? 'end' : 'start' },
    };
    preview.highlight = {
      sequence: preview.command.sequence,
      stage: first ? 'find' : 'land',
      reveal: first,
    };
    return true;
  }
  if (
    input.kind === 'component' &&
    input.child === preview?.key &&
    input.name === 'observed' &&
    preview?.highlight
  ) {
    const observation = input.event as ReadingObservation;
    const highlight = preview.highlight;
    if (observation.receipt?.sequence === highlight.sequence) {
      preview.highlight = null;
      if (observation.receipt.status === 'applied') {
        if (highlight.stage === 'land') {
          preview.command = {
            sequence: highlight.sequence + 1,
            target: { kind: 'find', query: search.query, step: 'first' },
          };
          preview.highlight = {
            sequence: preview.command.sequence,
            stage: 'find',
            reveal: highlight.reveal,
          };
        } else if (
          highlight.reveal &&
          observation.find?.active &&
          observation.find.sequence === highlight.sequence
        ) {
          preview.command = {
            sequence: highlight.sequence + 1,
            target: {
              kind: 'reveal_source',
              ...observation.find.active,
              find_sequence: highlight.sequence,
            },
          };
        }
      }
    }
    return true;
  }
  if (input.kind !== 'raw') return false;
  if (input.key === 'find-restart') {
    if (searchReady(search)) restartHistory(state, props, actions);
    return true;
  }
  if (input.key.startsWith('find-hit-')) {
    if (searchReady(search)) pick(search, Number(input.key.slice('find-hit-'.length)), actions);
    return true;
  }
  if (input.key === 'find-page-up' || input.key === 'find-page-down') {
    if (preview) preview.highlight = null;
    if (preview?.source)
      preview.command = {
        sequence: (preview.command?.sequence ?? 0) + 1,
        target: { kind: 'scroll_rows', rows: input.key === 'find-page-up' ? -8 : 8 },
      };
    return true;
  }
  if (preview && !preview.pending) {
    const source = preview.source;
    const position: SourcePosition | null =
      input.key === 'find-source-retry'
        ? { kind: 'start' }
        : source && input.key === 'find-source-first' && source.offset > 0
          ? { kind: 'start' }
          : source && input.key === 'find-source-previous' && source.offset > 0
            ? { kind: 'before', offset: source.offset }
            : source &&
                input.key === 'find-source-next' &&
                source.offset + utf8Bytes(source.text) < source.totalBytes
              ? { kind: 'after', offset: source.offset + utf8Bytes(source.text) }
              : source &&
                  input.key === 'find-source-last' &&
                  source.offset + utf8Bytes(source.text) < source.totalBytes
                ? { kind: 'tail' }
                : null;
    if (position) {
      readPreview(search, actions, position);
      return true;
    }
  }
  return false;
}

export function historyView<N extends Node>(
  presentation: Presentation<N>,
  props: TimelineProps,
  search: ConversationSearch,
): N {
  const history = search.history!,
    ui = presentation.paint(props.theme),
    text = copy(props.locale),
    preview = history.preview;
  const source = preview?.source,
    pending = !!preview?.pending;
  const rows = history.matches.map((hit, index) => {
    const label = `${index === history.selected ? '› ' : ''}${history.offset + index + 1}. ${compactLabel(hit.preview, 96)}`;
    return ui.button(`find-hit-${index}`, label, !searchReady(search), false, {
      width: fill,
      justify: 'start',
      truncate: true,
      ...(index === history.selected ? { background: props.theme.selected } : {}),
    });
  });
  return ui.box(
    'find-history',
    [
      ui.box(
        'find-results',
        [
          presentation.reading(
            'find-results-reading',
            {
              content: ui.box(
                'find-result-list',
                rows.length
                  ? rows
                  : [
                      ui.text(
                        'find-empty',
                        history.scanning ? text.loading : text.findNoResults,
                        13,
                        props.theme.muted,
                      ),
                    ],
                {
                  scroll: 'manual',
                  gap: 0,
                  terminal: { grow: 1, basis: 0 },
                  style: { width: fill, height: fill },
                },
              ),
              command: history.reveal,
            },
            { input: 'find-result-input' },
          ),
        ],
        { terminal: { grow: 1, basis: 0 }, style: { width: fill, grow: 1, minHeight: 0 } },
      ),
      ...(history.failed ? [ui.button('find-restart', text.findFailed)] : []),
      ...(preview
        ? [
            ui.text(
              'find-preview-label',
              {
                body: text.findFieldBody,
                thinking: text.findFieldThinking,
                tool_name: text.findFieldTool,
                arguments: text.findFieldArguments,
                result: text.findFieldResult,
                failure_message: text.findFieldFailure,
                error_class: text.findFieldError,
              }[preview.hit.field],
              13,
              props.theme.muted,
            ),
            ui.text(
              'find-preview-hit',
              compactLabel(preview.hit.preview, 160),
              13,
              props.theme.muted,
            ),
            ui.box(
              'find-preview',
              [
                presentation.reading(
                  preview.key,
                  {
                    content: ui.box(
                      'find-preview-document',
                      source
                        ? [
                            presentation.slot(
                              `find-preview-text-${source.sequence}-${source.field}-${source.offset}`,
                              'source',
                              {
                                source: source.text,
                                focusOnPointer: false,
                                appearance:
                                  source.format === 'text'
                                    ? props.theme.markdown.body
                                    : props.theme.markdown.code,
                                selection: props.theme.markdown.selection,
                                disabled: false,
                              },
                              {},
                            ),
                          ]
                        : [
                            ui.text(
                              'find-preview-loading',
                              preview.failed ? text.sourceFailed : text.loading,
                              13,
                              props.theme.muted,
                            ),
                          ],
                      {
                        scroll: 'anchor',
                        gap: 0,
                        terminal: { grow: 1, basis: 0 },
                        style: { width: fill, height: fill },
                      },
                    ),
                    command: preview.command,
                  },
                  { input: 'find-preview-input', observed: 'find-preview-observed' },
                ),
              ],
              { terminal: { grow: 2, basis: 0 }, style: { width: fill, grow: 2, minHeight: 0 } },
            ),
            ...((source &&
              (source.offset > 0 || source.offset + utf8Bytes(source.text) < source.totalBytes)) ||
            preview.failed
              ? [
                  ui.box(
                    'find-source-controls',
                    [
                      ...(source && source.offset > 0
                        ? [
                            ui.button('find-source-first', text.sourceFirst, pending),
                            ui.button('find-source-previous', text.sourcePrevious, pending),
                          ]
                        : []),
                      ...(source && source.offset + utf8Bytes(source.text) < source.totalBytes
                        ? [
                            ui.button('find-source-next', text.sourceNext, pending),
                            ui.button('find-source-last', text.sourceLast, pending),
                          ]
                        : []),
                      ...(preview.failed
                        ? [ui.button('find-source-retry', text.sourceRetry, pending)]
                        : []),
                    ],
                    { row: true, gap: 1, terminal: { wrap: true } },
                  ),
                ]
              : []),
          ]
        : []),
    ],
    { gap: 0, terminal: { grow: 1, basis: 0 }, style: { width: fill, height: fill } },
  );
}
export function searchCount(state: SearchReading) {
  const search = state.search!;
  if (search.pending) return '…';
  const history = search.history;
  if (history)
    return `${history.selected < 0 ? 0 : history.offset + history.selected + 1}/${history.offset + history.matches.length}${history.scanning || history.next !== null ? '+' : ''}`;
  const find = state.observation?.find;
  return `${find?.index ?? 0}/${find?.count ?? 0}${find?.limited ? '+' : ''}`;
}
