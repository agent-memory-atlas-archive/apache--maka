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
  Component,
  Emission,
  SettingsModelChoice,
  SettingsModelCursor,
  SettingsModelsPage,
  SettingsDefaultModel,
  EditorObservation,
  EditorTextRef,
  TextChunk,
  ComponentInput,
  ReadingCommand,
} from '../../plugin-sdk/src/index.js';
import type { ModelsProps } from './contracts.js';
import { modelsContract } from './contracts.js';
import { copy, type Copy } from './locale.js';
import { fill, output, pixels, type Presentation, type Node } from './view.js';
type Request =
  | { kind: 'page'; key: string; query: string }
  | { kind: 'text'; key: string; basis: EditorTextRef; choice: SettingsModelChoice | null };
interface State {
  catalogEpoch: number;
  defaultModel: SettingsDefaultModel | null;
  defaultsOpen: boolean;
  staleSelection: boolean;
  suggestDefault: boolean;
  revision: number;
  serial: number;
  pending: Request | null;
  query: string;
  queryBasis: EditorTextRef | null;
  observation: EditorObservation | null;
  queryError: keyof Copy | null;
  active: number;
  reveal: ReadingCommand | null;
  open: boolean;
  choices: SettingsModelChoice[];
  pageQuery: string | null;
  catalogRevision: number | null;
  next: SettingsModelCursor | null;
  failed: boolean;
  offset: number;
}
function candidate(state: State, props: ModelsProps) {
  if (props.busy || state.pending || state.failed || state.queryError || dirty(state)) return null;
  if (state.staleSelection && props.selection)
    return [
      ...state.choices,
      ...(state.defaultModel?.kind === 'ready' ? [state.defaultModel.choice] : []),
    ].find(
      (choice) =>
        choice.selection.connectionId === props.selection!.selection.connectionId &&
        choice.selection.modelId === props.selection!.selection.modelId,
    );
  return !props.selection && state.suggestDefault && state.defaultModel?.kind === 'ready'
    ? state.defaultModel.choice
    : null;
}

function sameText(a: EditorTextRef | null, b: EditorTextRef | null) {
  return a?.owner === b?.owner && a?.revision === b?.revision;
}
function dirty(state: State) {
  return !!state.observation && !sameText(state.observation.snapshot.text, state.queryBasis);
}
function interactive(state: State, props: ModelsProps) {
  return state.open && !state.defaultsOpen && !props.busy;
}
function resume(state: State, props: ModelsProps): Action[] {
  if (!interactive(state, props) || state.pending || state.queryError) return [];
  if (dirty(state)) return capture(state);
  if (state.failed) return [];
  if (
    state.pageQuery !== state.query ||
    state.catalogRevision === null ||
    (!state.choices.length && state.next)
  )
    return read(state, !dirty(state) && !!state.next);
  return [];
}
function defaultsInteractive(state: State, props: ModelsProps) {
  return state.open && state.defaultsOpen && !props.busy;
}
function reveal(state: State) {
  state.reveal = {
    sequence: (state.reveal?.sequence ?? 0) + 1,
    target: { kind: 'reveal_child', key: `choose-${state.active}` },
  };
}
function editableChoices(state: State, props: ModelsProps) {
  return (
    state.open &&
    !state.defaultsOpen &&
    !props.busy &&
    !state.pending &&
    !state.failed &&
    !state.queryError &&
    !dirty(state)
  );
}
function capture(state: State, choice: SettingsModelChoice | null = null): Action[] {
  const basis = state.observation?.snapshot.text;
  if (state.pending || !basis) return [];
  if (basis.bytes > 1024) {
    state.queryError = 'searchTooLong';
    return [];
  }
  const key = `models-${++state.serial}`;
  state.pending = { kind: 'text', key, basis, choice };
  state.queryError = null;
  return [
    {
      key,
      binding: 'reader',
      member: 0,
      method: 'read',
      input: { start: 0, end: basis.bytes },
      resources: {
        text: {
          child: 'model-query',
          method: 'snapshot',
          input: { kind: 'at_revision', owner: basis.owner, revision: basis.revision },
        },
      },
    },
  ];
}
function read(state: State, more = false): Action[] {
  if (state.pending) return [];
  if (dirty(state)) return capture(state);
  return readPage(state, more);
}
function readPage(state: State, more = false): Action[] {
  if (state.pending) return [];
  const key = `models-${++state.serial}`;
  state.pending = { kind: 'page', key, query: state.query };
  state.failed = false;
  return [
    {
      key,
      binding: 'settings',
      member: 0,
      method: 'models',
      input:
        more && state.next && state.catalogRevision !== null
          ? { kind: 'continue', revision: state.catalogRevision, cursor: state.next }
          : { kind: 'start', query: state.query },
      resources: {},
    },
  ];
}
function close(state: State) {
  state.open = false;
  if (state.pending?.kind === 'text') state.pending.choice = null;
}
function frame<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  props: ModelsProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const target = presentation.target;
  const ui = presentation.paint(props.theme);
  const text = copy(props.locale);
  const chosen = state.staleSelection ? null : props.selection;
  const compact = (label: string) =>
    [...label].length > 44 ? [...label].slice(0, 43).join('') + '…' : label;
  const trigger = ui.button(
    'toggle',
    chosen ? (target === 'desktop' ? chosen.label : compact(chosen.label)) : text.chooseModel,
    props.busy,
    false,
    {
      width: fill,
      minWidth: 0,
      justify: 'start',
      truncate: true,
    },
  );

  const modelRows = (
    target === 'desktop' ? state.choices : state.choices.slice(state.offset, state.offset + 3)
  ).map((choice, index) => {
    const at = index + (target === 'tui' ? state.offset : 0);
    const active = at === state.active;
    const button = ui.button(
      `choose-${at}`,
      `${target === 'tui' && active ? '› ' : ''}${target === 'desktop' ? choice.label : compact(choice.label)} · ${target === 'desktop' ? choice.connectionName : compact(choice.connectionName)}`,
      !editableChoices(state, props),
      false,
      {
        width: fill,
        minWidth: 0,
        truncate: true,
        justify: 'start',
        ...(active ? { background: props.theme.selected } : {}),
      },
    );

    return button;
  });
  const content = ui.box(
    'model-list',
    [
      ui.box(
        'model-choices',
        [
          ui.text('model-heading', text.models, 13, props.theme.muted),
          presentation.slot(
            'model-query',
            'input',
            {
              appearance: props.theme.input,
              accessibilityLabel: text.searchModels,
              placeholder: text.searchModels,
              autoFocus: true,
              tabIndex: 1,
              readOnly: false,
              disabled: props.busy || !state.open || state.defaultsOpen,
            },
            { changed: 'query', submitted: 'choose-active' },
          ),
          ...(state.queryError
            ? [ui.text('query-error', text[state.queryError], 13, props.theme.muted)]
            : []),
          ...(state.pending
            ? [ui.text('model-loading', text.loadingModels, 13, props.theme.muted)]
            : []),
          ...(state.failed
            ? [ui.text('model-error', text.modelsUnavailable, 13, props.theme.muted)]
            : []),
          ...(!state.pending &&
          !state.failed &&
          !state.queryError &&
          !dirty(state) &&
          !state.choices.length
            ? [
                ui.text(
                  'no-models',
                  state.query ? text.noMatchingModels : text.noModels,
                  13,
                  props.theme.muted,
                ),
              ]
            : []),
          ui.box(
            'model-actions',
            [
              ...(target === 'tui' && state.offset > 0
                ? [ui.button('previous', text.previousModels, props.busy || !!state.pending)]
                : []),
              ...((target === 'tui' && state.offset + 3 < state.choices.length) || state.next
                ? [ui.button('more', text.moreModels, !!state.pending || props.busy)]
                : []),
              ui.button('refresh', text.refreshModels, !!state.pending || props.busy),
              ui.button('defaults', text.defaultModel),
              ui.button('close', text.closeModels, false),
            ],
            { row: true, gap: target === 'desktop' ? 8 : 1, style: { wrap: true } },
          ),
          ...(target === 'desktop'
            ? [
                presentation.reading(
                  'model-rows',
                  {
                    content: ui.box('model-viewport', modelRows, {
                      scroll: 'manual',
                      gap: 2,
                      style: { width: fill, height: pixels(160), minWidth: 0, minHeight: 0 },
                    }),
                    command: state.reveal,
                  },
                  { input: 'row-input', observed: 'row-viewport' },
                ),
              ]
            : modelRows),
        ],
        {
          visible: !state.defaultsOpen,
          gap: target === 'desktop' ? 8 : 0,
          shortcuts: [
            {
              key: 'down',
              modifiers: [],
              input: 'next-choice',
              enabled: editableChoices(state, props),
            },
            {
              key: 'up',
              modifiers: [],
              input: 'previous-choice',
              enabled: editableChoices(state, props),
            },
            { key: 'escape', modifiers: [], input: 'close', enabled: true },
          ],
        },
      ),
      ui.box(
        'model-default-pane',
        [
          ui.box(
            'default-navigation',
            [
              ui.button('default-back', text.models),
              ui.button('default-refresh', text.refreshModels, !!state.pending),
              ui.button('default-close', text.closeModels),
            ],
            { row: target === 'desktop', gap: 0, style: { wrap: true } },
          ),
          presentation.slot(
            'default-settings',
            'defaults',
            {
              theme: props.theme,
              locale: props.locale,
              selection: chosen,
              defaultModel: state.defaultModel,
              catalogRevision: state.catalogRevision,
              busy: props.busy || !!state.pending || state.failed,
            },
            { changed: 'default-changed' },
          ),
        ],
        { visible: state.defaultsOpen, gap: target === 'desktop' ? 8 : 0 },
      ),
    ],
    {
      visible: target === 'desktop' || state.open,
      surface: true,
      padding: target === 'desktop' ? 12 : 0,
      style: { width: fill, minWidth: 0, background: 0 },
    },
  );
  const view = presentation.popup
    ? presentation.popup({
        key: 'model-popup',
        label: text.models,
        trigger,
        content,
        open: state.open,
        anchor: 'top_right',
        offset: 8,
        style: {
          width: pixels(360),
          maxHeight: 320,
          minWidth: 0,
          background: props.theme.menu.background,
          border: props.theme.menu.border,
          borderWidth: props.theme.menu.borderWidth,
          radius: props.theme.menu.radius,
        },
      })
    : ui.box('models', [trigger, content], { gap: 0 });
  const out = output(state, view, events);
  out.actions = actions;
  return out;
}
export function models<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ModelsProps, N> {
  const target = presentation.target;
  return {
    mount(props) {
      const state: State = {
        revision: 0,
        catalogEpoch: props.catalogEpoch,
        defaultModel: null,
        defaultsOpen: false,
        staleSelection: false,
        suggestDefault: true,
        serial: 0,
        pending: null,
        query: '',
        queryBasis: null,
        observation: null,
        queryError: null,
        active: 0,
        reveal: null,
        open: false,
        choices: [],
        pageQuery: null,
        catalogRevision: null,
        next: null,
        failed: false,
        offset: 0,
      };
      return frame(presentation, state, props, read(state));
    },
    update(previous, props) {
      if (previous.catalogEpoch !== props.catalogEpoch) {
        const state = {
          ...previous,
          revision: previous.revision + 1,
          catalogEpoch: props.catalogEpoch,
          staleSelection: true,
          suggestDefault: false,
          choices: [],
          pageQuery: null,
          pending: null,
          catalogRevision: null,
          defaultModel: null,
          next: null,
          offset: 0,
        };
        return frame(
          presentation,
          state,
          props,
          state.defaultsOpen ? readPage(state) : read(state),
        );
      }
      if (props.selection && !previous.staleSelection) previous.suggestDefault = false;
      const initial = candidate(previous, props);
      if (initial && previous.staleSelection) previous.staleSelection = false;
      const state = { ...previous, revision: previous.revision + 1 };
      return frame(
        presentation,
        state,
        props,
        defaultsInteractive(state, props) && state.catalogRevision === null && !state.failed
          ? readPage(state)
          : resume(state, props),
        initial ? [{ name: 'selected', value: initial }] : [],
      );
    },
    event(state, input, props) {
      state.revision++;
      const actions: Action[] = [];
      const events: Emission[] = [];
      if (input.kind === 'component' && input.child === 'model-rows' && input.name === 'input')
        input = input.event as ComponentInput;
      if (input.kind === 'action' && input.key === state.pending?.key) {
        const request = state.pending;
        state.pending = null;
        const result = input.event as ActionOutcome;
        if (request.kind === 'text') {
          if (!sameText(request.basis, state.observation?.snapshot.text ?? null)) {
            if (interactive(state, props)) actions.push(...read(state));
          } else if (result.status !== 'succeeded') state.queryError = 'modelsUnavailable';
          else {
            const query = (result.value as TextChunk).text.trim();
            if ([...query].length > 256) state.queryError = 'searchTooLong';
            else if (request.choice) {
              if (editableChoices(state, props) && query === state.query) {
                state.staleSelection = false;
                state.suggestDefault = false;
                close(state);
                events.push({ name: 'selected', value: request.choice });
              }
            } else {
              const changed = query !== state.query;
              state.query = query;
              state.queryBasis = request.basis;
              state.queryError = null;
              if (changed) {
                state.failed = false;
                state.offset = state.active = 0;
                state.choices = [];
                state.next = null;
                state.pageQuery = null;
                if (query) state.suggestDefault = false;
              }
              actions.push(...resume(state, props));
            }
          }
        } else if (result.status !== 'succeeded') {
          state.failed = request.query === state.query;
          actions.push(...resume(state, props));
        } else {
          const page = result.value as SettingsModelsPage;
          if (page.kind === 'changed') {
            state.choices = [];
            state.next = null;
            state.pageQuery = null;
            state.catalogRevision = null;
            state.defaultModel = null;
            if (interactive(state, props)) actions.push(...read(state));
            else if (defaultsInteractive(state, props)) actions.push(...readPage(state));
          } else {
            state.catalogRevision = page.revision;
            state.defaultModel = page.defaultModel;
            state.offset = state.active = 0;
            // A page's choices and cursor always travel with its original
            // revision, even while the native search input has newer metadata.
            const current = request.query === state.query;
            state.choices = current ? page.choices : [];
            state.next = current ? (page.next ?? null) : null;
            state.pageQuery = current ? request.query : null;
            if (state.choices.length) reveal(state);
            actions.push(...resume(state, props));
            const initial = candidate(state, props);
            if (initial) {
              state.staleSelection = false;
              events.push({ name: 'selected', value: initial });
            }
          }
        }
      } else if (input.kind === 'component' && input.child === 'model-query') {
        if (input.name === 'changed') {
          state.observation = input.event as EditorObservation;
          state.queryError = null;
          if (!state.observation.snapshot.text.bytes) {
            const changed = state.query !== '';
            state.query = '';
            state.queryBasis = state.observation.snapshot.text;
            if (changed) {
              state.failed = false;
              state.choices = [];
              state.next = null;
              state.pageQuery = null;
              state.offset = state.active = 0;
              if (interactive(state, props)) actions.push(...read(state));
            }
          } else if (interactive(state, props)) actions.push(...read(state));
        } else if (input.name === 'submitted' && editableChoices(state, props)) {
          const choice = state.choices[state.active];
          if (choice) actions.push(...capture(state, choice));
        }
      } else if (
        input.kind === 'component' &&
        input.child === 'default-settings' &&
        input.name === 'changed'
      ) {
        state.suggestDefault = false;
        state.pending = null;
        state.catalogRevision = null;
        state.defaultModel = null;
        actions.push(...readPage(state));
      } else if (input.kind === 'raw') {
        if (
          input.key === 'model-popup' &&
          target === 'desktop' &&
          (input.event as { kind?: string })?.kind === 'dismiss'
        )
          close(state);
        else if (input.key === 'close' || input.key === 'default-close') close(state);
        else if (input.key === 'defaults') state.defaultsOpen = true;
        else if (input.key === 'default-back') {
          state.defaultsOpen = false;
          actions.push(...resume(state, props));
        } else if (!props.busy) {
          if (input.key === 'toggle') {
            if (state.open) close(state);
            else {
              state.open = true;
              actions.push(...resume(state, props));
            }
          } else if (input.key === 'default-refresh') actions.push(...readPage(state));
          else if (input.key === 'refresh') actions.push(...read(state));
          else if (input.key === 'previous' && target === 'tui' && !state.pending) {
            state.offset = Math.max(0, state.offset - 3);
            state.active = state.offset;
          } else if (input.key === 'more' && !state.pending) {
            if (target === 'tui' && state.offset + 3 < state.choices.length) {
              state.offset += 3;
              state.active = state.offset;
            } else if (state.next) actions.push(...read(state, true));
          } else if (
            (input.key === 'next-choice' || input.key === 'previous-choice') &&
            editableChoices(state, props)
          ) {
            state.active = Math.max(
              0,
              Math.min(
                state.choices.length - 1,
                state.active + (input.key === 'next-choice' ? 1 : -1),
              ),
            );
            if (target === 'tui') state.offset = Math.floor(state.active / 3) * 3;
            reveal(state);
          } else if (editableChoices(state, props) && /^choose-(0|[1-9][0-9]*)$/.test(input.key)) {
            const choice = state.choices[Number(input.key.slice(7))];
            if (choice) actions.push(...capture(state, choice));
          }
        }
      }
      if (defaultsInteractive(state, props) && state.catalogRevision === null && !state.failed)
        actions.push(...readPage(state));
      const out = frame(presentation, state, props, actions, events);
      if (
        input.kind === 'raw' &&
        input.key === 'model-popup' &&
        (input.event as { reason?: string })?.reason === 'escape'
      ) {
        out.decision = { preventDefault: true, stopPropagation: true };
      }
      return out;
    },
  };
}
export function activate() {
  return { Models: { kind: 'component', contract: modelsContract, component: ['component'] } };
}
