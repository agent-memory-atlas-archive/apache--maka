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
  AgentExecutorSelection,
  AgentExecutors,
  Component,
  EditorObservation,
  EditorCommand,
  InputProps,
  EditorTextRef,
  Emission,
  TextChunk,
  ThinkingLevel,
} from '../../plugin-sdk/src/index.js';
import { executionPickerContract, type ExecutionPickerProps } from './contracts.js';
import { copy, type Copy } from './locale.js';
import { output, type Node, type Presentation } from './view.js';
interface State {
  revision: number;
  serial: number;
  openRequest: number | null;
  modelRequest: number | null;
  open: boolean;
  names: string[];
  listing: string | null;
  failed: boolean;
  draft: AgentExecutorSelection | null;
  editor: string;
  observation: EditorObservation | null;
  prefill: string | null;
  command: EditorCommand | null;
  capture: { key: string; basis: EditorTextRef; selection: AgentExecutorSelection } | null;
  error: boolean;
}
const levels: Record<ThinkingLevel, keyof Copy> = {
  off: 'thinkingOff',
  minimal: 'thinkingMinimal',
  low: 'thinkingLow',
  medium: 'thinkingMedium',
  high: 'thinkingHigh',
  xhigh: 'thinkingXhigh',
  max: 'thinkingMax',
  ultra: 'thinkingUltra',
};
function list(state: State): Action[] {
  if (state.listing) return [];
  state.failed = false;
  state.listing = `backends-${++state.serial}`;
  return [
    {
      key: state.listing,
      binding: 'agent',
      member: 0,
      method: 'executors',
      input: {},
      resources: {},
    },
  ];
}
function close(state: State) {
  state.open = false;
  state.capture = null;
}
function draft(state: State, selection: AgentExecutorSelection | null) {
  state.draft = selection ? { ...selection, settings: { ...selection.settings } } : null;
  state.editor = `backend-model-${++state.serial}`;
  state.observation = null;
  state.prefill = selection?.settings.model || null;
  state.command = null;
  state.capture = null;
  state.error = false;
}
function open(state: State, props: ExecutionPickerProps) {
  state.open = true;
  draft(state, props.executor);
}
function utf8Bytes(text: string) {
  let bytes = 0;
  for (const point of text) {
    const code = point.codePointAt(0)!;
    bytes += code <= 0x7f ? 1 : code <= 0x7ff ? 2 : code <= 0xffff ? 3 : 4;
  }
  return bytes;
}
function same(a: EditorTextRef, b: EditorTextRef | undefined) {
  return !!b && a.owner === b.owner && a.revision === b.revision && a.bytes === b.bytes;
}
function capture(state: State): Action[] {
  const observation = state.observation,
    selection = state.draft;
  if (!selection || state.capture || state.prefill !== null || state.command || !observation)
    return [];
  const basis = observation.snapshot.text;
  if (observation.snapshot.marked || basis.bytes > 512) {
    state.error = true;
    return [];
  }
  const key = `settings-${++state.serial}`;
  state.capture = { key, basis, selection: { ...selection, settings: { ...selection.settings } } };
  return [
    {
      key,
      binding: 'reader',
      member: 0,
      method: 'read',
      input: { start: 0, end: basis.bytes },
      resources: {
        text: {
          child: state.editor,
          method: 'snapshot',
          input: { kind: 'at_revision', owner: basis.owner, revision: basis.revision },
        },
      },
    },
  ];
}
function frame<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  props: ExecutionPickerProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const ui = presentation.paint(props.theme),
    text = copy(props.locale);
  const disabled = props.busy || !!state.capture;
  const model = presentation.slot(
    'models',
    'models',
    {
      catalogEpoch: props.catalogEpoch,
      openRequest: state.modelRequest,
      theme: props.theme,
      locale: props.locale,
      selection: props.selection,
      busy: props.busy || state.open || props.executor !== null,
    },
    { selected: 'selected', setup: 'setup', connections: 'connections' },
  );
  const background = ui.box(
    'backend-control',
    [
      ui.box('connected-model', [model], { visible: props.executor === null, gap: 0 }),
      ...(state.names.length || state.failed || props.executor
        ? [ui.button('backends', props.executor?.executor ?? text.executionBackend, props.busy)]
        : []),
    ],
    { row: true, gap: presentation.target === 'tui' ? 1 : 8 },
  );
  const options = [
    { key: 'default', label: text.executionDefault },
    ...Object.entries(levels).map(([key, value]) => ({ key, label: text[value] })),
  ];
  const current = state.draft?.settings.thinkingLevel ?? 'default';
  const content = ui.box(
    'backend-content',
    [
      ui.button('connected', text.executionModel, disabled),
      ...state.names.map((name, i) =>
        ui.button(
          `backend-${i}`,
          `${state.draft?.executor === name ? (props.theme.ascii ? '> ' : '✓ ') : ''}${name}`,
          disabled,
        ),
      ),
      ...(state.failed
        ? [ui.text('backend-error', text.executionUnavailable, 13, props.theme.error)]
        : []),
      ...(state.draft
        ? [
            ui.text('backend-model-label', text.executionModelName, 13, props.theme.text),
            presentation.slot<InputProps>(
              state.editor,
              'input',
              {
                appearance: props.theme.input,
                accessibilityLabel: text.executionModelName,
                placeholder: text.executionDefault,
                command: state.command,
                tabIndex: 0,
                maxBytes: 512,
                readOnly: state.prefill !== null || state.command !== null,
                disabled,
                autoFocus: true,
              },
              { changed: 'model', submitted: 'apply' },
            ),
            ui.text('backend-model-hint', text.executionModelHint, 13, props.theme.muted),
            ui.choice?.('thinking', text.thinking, options, current, disabled) ??
              ui.box(
                'thinking-options',
                options.map((option) =>
                  ui.button(
                    `thinking-${option.key}`,
                    `${current === option.key ? '✓ ' : ''}${option.label}`,
                    disabled,
                  ),
                ),
                { row: true, style: { wrap: true }, terminal: { wrap: true } },
              ),
            ui.button(
              'apply',
              text.modelApply,
              disabled ||
                state.prefill !== null ||
                !!state.command ||
                !state.observation ||
                !state.names.includes(state.draft.executor),
            ),
          ]
        : []),
      ...(state.error
        ? [ui.text('settings-error', text.executionSettingsFailed, 13, props.theme.error)]
        : []),
      ui.box(
        'backend-actions',
        [
          ui.button('refresh', text.refreshModels, disabled || !!state.listing),
          ui.button('close', text.closeModels),
        ],
        { row: true, gap: 1 },
      ),
    ],
    {
      gap: presentation.target === 'tui' ? 0 : 8,
      scroll: 'manual',
      autoFocus: true,
      terminal: { grow: 1 },
    },
  );
  const view = ui.dialog(
    'backend-popup',
    text.executionBackend,
    background,
    content,
    state.open,
    undefined,
    { width: 64 },
  );
  const out = output(state, view, events);
  out.actions = actions;
  return out;
}
export function executionPicker<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ExecutionPickerProps, N> {
  return {
    mount(props) {
      const state: State = {
        revision: 0,
        serial: 0,
        openRequest: props.openRequest,
        modelRequest: null,
        open: false,
        names: [],
        listing: null,
        failed: false,
        draft: null,
        editor: 'backend-model',
        observation: null,
        prefill: null,
        command: null,
        capture: null,
        error: false,
      };
      return frame(presentation, state, props, list(state));
    },
    update(previous, props) {
      const state = { ...previous, revision: previous.revision + 1 };
      const actions: Action[] = [];
      if (!props.busy && props.openRequest !== null && props.openRequest !== state.openRequest) {
        state.openRequest = props.openRequest;
        if (props.executor || state.names.length) {
          open(state, props);
          actions.push(...list(state));
        } else state.modelRequest = ++state.serial;
      }
      return frame(presentation, state, props, actions);
    },
    event(state, input, props) {
      state.revision++;
      const actions: Action[] = [],
        events: Emission[] = [];
      if (input.kind === 'action' && input.key === state.listing) {
        state.listing = null;
        const result = input.event as ActionOutcome;
        if (result.status === 'succeeded') {
          state.names = (result.value as AgentExecutors).executors;
          events.push({ name: 'backends', value: { available: state.names.length > 0 } });
        } else state.failed = true;
      } else if (input.kind === 'action' && input.key === state.capture?.key) {
        const request = state.capture;
        state.capture = null;
        const result = input.event as ActionOutcome;
        const chunk = result.status === 'succeeded' ? (result.value as TextChunk) : null;
        if (
          !props.busy &&
          state.open &&
          chunk &&
          same(request.basis, chunk.metadata as EditorTextRef) &&
          same(request.basis, state.observation?.snapshot.text) &&
          !state.observation?.snapshot.marked &&
          utf8Bytes(chunk.text) === request.basis.bytes &&
          state.names.includes(request.selection.executor)
        ) {
          events.push({
            name: 'executor',
            value: {
              ...request.selection,
              settings: { ...request.selection.settings, model: chunk.text.trim() || null },
            },
          });
          close(state);
        } else state.error = true;
      } else if (input.kind === 'component' && input.child === 'models') {
        if (input.name !== 'selected' || (!props.executor && !state.open && !props.busy))
          events.push({ name: input.name, value: input.event });
      } else if (input.kind === 'component' && input.child === state.editor && state.open) {
        if (input.name === 'changed') {
          const observed = input.event as EditorObservation;
          state.observation = observed;
          if (
            state.command &&
            observed.receipt?.owner === state.command.owner &&
            observed.receipt.sequence === state.command.sequence
          ) {
            state.error = observed.receipt.outcome.status !== 'applied';
            state.command = null;
            state.prefill = null;
          } else if (state.prefill !== null && !state.command) {
            state.command = {
              owner: observed.snapshot.text.owner,
              sequence: ++state.serial,
              basedOn: observed.revision,
              edit: { kind: 'reset', text: state.prefill },
            };
          }
        } else if (input.name === 'submitted' && !props.busy) actions.push(...capture(state));
      } else if (input.kind === 'raw') {
        if (input.key === 'close' || input.key === 'backend-popup') close(state);
        else if (!props.busy && input.key === 'backends') {
          open(state, props);
          actions.push(...list(state));
        } else if (state.open && !props.busy && !state.capture) {
          if (input.key === 'connected') {
            events.push({ name: 'executor', value: null });
            close(state);
            state.modelRequest = ++state.serial;
          } else if (input.key === 'refresh') actions.push(...list(state));
          else if (input.key === 'apply') actions.push(...capture(state));
          else if (/^backend-(0|[1-9][0-9]*)$/.test(input.key)) {
            const name = state.names[Number(input.key.slice(8))];
            if (name && name !== state.draft?.executor)
              draft(
                state,
                props.executor?.executor === name
                  ? props.executor
                  : { executor: name, settings: {} },
              );
          } else if (
            state.draft &&
            (input.key === 'thinking' || input.key.startsWith('thinking-'))
          ) {
            const level =
              input.key === 'thinking' ? (input.event as { key?: string }).key : input.key.slice(9);
            if (level === 'default' || (level && Object.hasOwn(levels, level)))
              state.draft.settings.thinkingLevel =
                level === 'default' ? null : (level as ThinkingLevel);
          }
        }
      }
      const out = frame(presentation, state, props, actions, events);
      if (input.kind === 'raw' && input.key === 'backend-popup')
        out.decision = { preventDefault: true, stopPropagation: true };
      return out;
    },
  };
}
export function activate() {
  return {
    ExecutionPicker: {
      kind: 'component',
      contract: executionPickerContract,
      component: ['component'],
    },
  };
}
