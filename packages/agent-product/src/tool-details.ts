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
  AgentHistorySourcePage,
  Component,
  DiffProps,
  HistorySource,
  SourcePosition,
} from '../../plugin-sdk/src/index.js';
import { toolDetailsContract, type ToolDetailsProps } from './contracts.js';
import { copy } from './locale.js';
import { sameValue } from './value.js';
import { fill, output, type Node, type Presentation } from './view.js';

/** These keys explicitly name display contracts. Names and JSON shapes do not. */
export function toolPresentation(key: string | null | undefined) {
  switch (key) {
    case 'maka.workspace.Files@1/readText':
      return 'read';
    case 'maka.workspace.Files@1/write':
      return 'write';
    case 'maka.workspace.Files@1/edit':
      return 'edit';
    case 'maka.workspace.Files@1/applyPatch':
      return 'patch';
    default:
      return null;
  }
}
type Part = {
  field: 'arguments' | 'result';
  path: string[];
  format: 'text' | 'json';
  page: HistorySource | null;
  failed: boolean;
  pending: { key: string; position: SourcePosition } | null;
  position: SourcePosition;
};
type State = {
  revision: number;
  serial: number;
  context: ToolDetailsProps['context'];
  tool: ToolDetailsProps['tool'];
  parts: Record<string, Part>;
};
function bytes(text: string) {
  let size = 0;
  for (const point of text) {
    const value = point.codePointAt(0)!;
    size += value <= 0x7f ? 1 : value <= 0x7ff ? 2 : value <= 0xffff ? 3 : 4;
  }
  return size;
}
function sameIdentity(state: State, props: ToolDetailsProps) {
  return (
    state.context.sessionId === props.context.sessionId &&
    state.tool.toolUseId === props.tool.toolUseId &&
    state.tool.call?.sequence === props.tool.call?.sequence &&
    state.tool.call?.presentation === props.tool.call?.presentation &&
    state.tool.result?.sequence === props.tool.result?.sequence
  );
}
function add(
  state: State,
  id: string,
  field: Part['field'],
  path: string[],
  format: Part['format'] = 'text',
) {
  state.parts[id] ??= {
    field,
    path,
    format,
    page: null,
    failed: false,
    pending: null,
    position: { kind: 'start' },
  };
}
function request(state: State, part: Part, position: SourcePosition, actions: Action[]) {
  const sequence =
    part.field === 'arguments' ? state.tool.call?.sequence : state.tool.result?.sequence;
  if (!sequence || part.pending) return;
  const key = 'tool-value-' + ++state.serial;
  part.pending = { key, position };
  part.position = position;
  part.failed = false;
  actions.push({
    key,
    binding: 'agent',
    member: 0,
    method: 'source',
    resources: {},
    input: {
      ...state.context,
      source: { kind: 'sequence', sequence },
      field: part.field,
      path: part.path,
      position,
    },
  });
}
function complete(part: Part) {
  const page = part.page;
  return !!page && page.offset === 0 && bytes(page.text) === page.totalBytes && !part.failed;
}
function scalar(state: State, id: string): string | number | null {
  const part = state.parts[id];
  if (!part || !complete(part)) return null;
  if (part.format === 'text') return part.page!.text;
  try {
    const value: unknown = JSON.parse(part.page!.text);
    return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0 ? value : null;
  } catch {
    return null;
  }
}
function accept(state: State, part: Part, value: AgentHistorySourcePage, position: SourcePosition) {
  if (value.through !== state.context.through) {
    part.failed = true;
    return;
  }
  if (value.kind === 'preparing') return;
  const page = value.source,
    end = page.offset + bytes(page.text);
  const sequence =
    part.field === 'arguments' ? state.tool.call?.sequence : state.tool.result?.sequence;
  if (
    page.sequence !== sequence ||
    page.field !== part.field ||
    !sameValue(page.path, part.path) ||
    page.format !== part.format ||
    page.tool?.toolUseId !== state.tool.toolUseId ||
    page.tool.call?.sequence !== state.tool.call?.sequence ||
    page.tool.call?.presentation !== state.tool.call?.presentation ||
    end > page.totalBytes ||
    (!page.text && page.totalBytes > 0) ||
    (part.page && part.page.totalBytes !== page.totalBytes) ||
    (position.kind === 'start' && page.offset !== 0) ||
    (position.kind === 'tail' && end !== page.totalBytes) ||
    (position.kind === 'after' && page.offset !== position.offset) ||
    (position.kind === 'before' && end !== position.offset)
  ) {
    part.failed = true;
    return;
  }
  part.page = page;
}
export function toolDetails<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ToolDetailsProps, N> {
  function frame(state: State, props: ToolDetailsProps, actions: Action[] = []) {
    if (!sameIdentity(state, props)) {
      state.context = { ...props.context };
      state.tool = props.tool;
      state.parts = {};
    }
    const kind = toolPresentation(state.tool.call?.presentation);
    if (kind && props.expanded && state.tool.call) {
      add(state, 'path', 'arguments', kind === 'patch' ? ['operation', 'path'] : ['path']);
      if (state.tool.result) add(state, 'status', 'result', ['kind']);
      if (kind === 'edit') {
        add(state, 'before', 'arguments', ['old_string']);
        add(state, 'after', 'arguments', ['new_string']);
      } else if (kind === 'write') add(state, 'after', 'arguments', ['content']);
      else if (kind === 'patch') {
        add(state, 'operation', 'arguments', ['operation', 'type']);
        const operation = scalar(state, 'operation');
        if (operation === 'create_file' || operation === 'update_file')
          add(state, 'patch', 'arguments', ['operation', 'diff']);
      }
      if (scalar(state, 'status') === 'completed') {
        if (kind === 'read') {
          add(state, 'content', 'result', ['value', 'content']);
          for (const name of ['offset', 'returnedLines', 'totalLines'])
            add(state, name, 'result', ['value', name], 'json');
        } else if (kind === 'write') add(state, 'before', 'result', ['value', 'previousContent']);
      } else if (scalar(state, 'status') === 'failed') add(state, 'failure', 'result', ['message']);
      for (const part of Object.values(state.parts))
        if (!part.page && !part.pending && !part.failed)
          request(state, part, part.position, actions);
    }
    const ui = presentation.paint(props.theme),
      text = copy(props.locale);
    const labels: Record<string, string> = {
      path: text.toolFilePath,
      before: kind === 'write' ? text.toolRecordedBefore : text.toolRequestedBefore,
      after: text.toolRequestedAfter,
      content: text.toolReadContent,
      patch: text.toolPatch,
      operation: text.toolPatchOperation,
      failure: text.toolBusinessFailure,
    };
    const sections = Object.entries(state.parts)
      .filter(([id]) => id in labels)
      .map(([id, part]) => {
        const page = part.page,
          end = page ? page.offset + bytes(page.text) : 0;
        const change =
          id === 'before'
            ? 'removed'
            : id === 'after'
              ? kind === 'edit'
                ? 'added'
                : 'content'
              : null;
        const source = page ? page.text : '';
        const path = scalar(state, 'path');
        const hint =
          typeof path === 'string' ? path.split(/[\\/]/).at(-1)?.split('.').at(-1) : null;
        const language = hint && bytes(hint) <= 128 ? hint : null;
        const body =
          change && presentation.target === 'tui'
            ? presentation.slot(
                id + '-body',
                'diff',
                {
                  source,
                  rows: [{ start: 0, end: bytes(source), kind: change, language }],
                  ascii: false,
                  disabled: false,
                  focusOnPointer: props.focusOnPointer,
                  appearance: {
                    text: props.theme.markdown.code,
                    background: props.theme.markdown.codeBackground,
                    added: props.theme.markdown.dark ? 0x55cc88ff : 0x246c48ff,
                    removed: props.theme.markdown.dark ? 0xff6677ff : 0xbb2233ff,
                    selection: props.theme.markdown.selection,
                    dark: props.theme.markdown.dark,
                  },
                } satisfies DiffProps,
                {},
              )
            : presentation.slot(
                id + '-body',
                'source',
                {
                  source,
                  appearance: props.theme.markdown.code,
                  selection: props.theme.markdown.selection,
                  disabled: false,
                  focusOnPointer: props.focusOnPointer,
                },
                {},
              );
        return ui.box(
          id,
          [
            ui.text(id + '-label', labels[id]!, 13, props.theme.muted, 550),
            ...(page ? [body] : []),
            ...(part.pending
              ? [ui.text(id + '-loading', text.loading, 13, props.theme.muted)]
              : []),
            ...(part.failed
              ? [
                  ui.text(
                    id + '-failed',
                    id === 'before' && kind === 'write'
                      ? text.toolOriginalUnavailable
                      : text.sourceFailed,
                    13,
                    props.theme.muted,
                  ),
                ]
              : []),
            ...(page && (page.offset > 0 || end < page.totalBytes)
              ? [
                  ui.text(
                    id + '-range',
                    `${page.offset}–${end} / ${page.totalBytes} ${text.toolByteRange}`,
                    12,
                    props.theme.muted,
                  ),
                ]
              : []),
            ui.box(
              id + '-controls',
              [
                ...(part.failed
                  ? [ui.button(id + '-retry', text.sourceRetry, !!part.pending)]
                  : []),
                ...(page && page.offset > 0
                  ? [
                      ui.button(id + '-first', text.sourceFirst, !!part.pending),
                      ui.button(id + '-previous', text.sourcePrevious, !!part.pending),
                    ]
                  : []),
                ...(page && end < page.totalBytes
                  ? [
                      ui.button(id + '-next', text.sourceNext, !!part.pending),
                      ui.button(id + '-last', text.sourceLast, !!part.pending),
                    ]
                  : []),
              ],
              { row: true, gap: 1, terminal: { wrap: true }, style: { wrap: true } },
            ),
          ],
          { gap: 0, style: { width: fill } },
        );
      });
    const offset = scalar(state, 'offset'),
      count = scalar(state, 'returnedLines'),
      total = scalar(state, 'totalLines');
    if (typeof offset === 'number' && typeof count === 'number' && typeof total === 'number')
      sections.push(
        ui.text(
          'read-range',
          `${text.toolReadLines}: ${count ? offset + 1 : offset}–${offset + count} / ${total}`,
          12,
          props.theme.muted,
        ),
      );
    if (state.parts.status?.failed)
      sections.push(ui.button('status-retry', text.sourceRetry, !!state.parts.status.pending));
    const out = output(state, ui.box('tool-details', sections, { gap: 1, style: { width: fill } }));
    out.actions = actions;
    return out;
  }
  return {
    mount(props) {
      return frame(
        { revision: 0, serial: 0, context: { ...props.context }, tool: props.tool, parts: {} },
        props,
      );
    },
    update(state, props) {
      state.revision++;
      return frame(state, props);
    },
    event(state, input, props) {
      state.revision++;
      if (!sameIdentity(state, props)) return frame(state, props);
      const actions: Action[] = [];
      if (input.kind === 'action') {
        const part = Object.values(state.parts).find((part) => part.pending?.key === input.key);
        if (part?.pending) {
          const position = part.pending.position;
          part.pending = null;
          const result = input.event as ActionOutcome;
          if (result.status !== 'succeeded') part.failed = true;
          else {
            const value = result.value as AgentHistorySourcePage;
            accept(state, part, value, position);
            if (value.kind === 'preparing' && !part.failed && props.expanded)
              request(state, part, position, actions);
          }
        }
      } else if (props.expanded && input.kind === 'raw') {
        const split = input.key.lastIndexOf('-'),
          id = input.key.slice(0, split),
          event = input.key.slice(split + 1),
          part = state.parts[id];
        if (part && !part.pending) {
          const page = part.page,
            end = page ? page.offset + bytes(page.text) : 0;
          if (event === 'retry' && part.failed) request(state, part, part.position, actions);
          else if (page && page.offset > 0 && event === 'first')
            request(state, part, { kind: 'start' }, actions);
          else if (page && page.offset > 0 && event === 'previous')
            request(state, part, { kind: 'before', offset: page.offset }, actions);
          else if (page && end < page.totalBytes && event === 'next')
            request(state, part, { kind: 'after', offset: end }, actions);
          else if (page && end < page.totalBytes && event === 'last')
            request(state, part, { kind: 'tail' }, actions);
        }
      }
      return frame(state, props, actions);
    },
  };
}
export function activate() {
  return {
    ToolDetails: { kind: 'component', contract: toolDetailsContract, component: ['component'] },
  };
}
