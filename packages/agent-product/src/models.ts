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
} from '../../plugin-sdk/src/index.js';
import type { ModelsProps } from './contracts.js';
import { modelsContract } from './contracts.js';
import { copy } from './locale.js';
import { fill, output, paint, pixels, slot, type Target } from './view.js';
interface State {
  catalogEpoch: number;
  defaultModel: SettingsDefaultModel | null;
  defaultsOpen: boolean;
  staleSelection: boolean;
  suggestDefault: boolean;
  revision: number;
  serial: number;
  pending: string | null;
  open: boolean;
  choices: SettingsModelChoice[];
  catalogRevision: number | null;
  next: SettingsModelCursor | null;
  failed: boolean;
  offset: number;
}
function candidate(state: State, props: ModelsProps) {
  if (props.busy || state.pending || state.failed) return null;
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

function read(state: State, more = false): Action[] {
  if (state.pending) return [];
  const key = `models-${++state.serial}`;
  state.pending = key;
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
          : { kind: 'start' },
      resources: {},
    },
  ];
}
function frame(
  target: Target,
  state: State,
  props: ModelsProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const ui = paint(target, props.theme);
  const text = copy(props.locale);
  const chosen = state.staleSelection ? null : props.selection;
  const compact = (label: string) =>
    [...label].length > 44 ? [...label].slice(0, 43).join('') + '…' : label;
  const trigger = ui.button(
    'toggle',
    chosen ? (target === 'desktop' ? chosen.label : compact(chosen.label)) : text.chooseModel,
    props.busy,
  );
  if (target === 'desktop')
    trigger.style = {
      ...(trigger.style as object),
      width: fill,
      minWidth: 0,
      justify: 'start',
      truncate: true,
    };
  const content = ui.box(
    'model-list',
    [
      ui.box(
        'model-choices',
        [
          ui.text('model-heading', text.models, 13, props.theme.muted),
          ...(state.pending
            ? [ui.text('model-loading', text.loadingModels, 13, props.theme.muted)]
            : []),
          ...(state.failed
            ? [ui.text('model-error', text.modelsUnavailable, 13, props.theme.muted)]
            : []),
          ...(!state.pending && !state.failed && !state.choices.length
            ? [ui.text('no-models', text.noModels, 13, props.theme.muted)]
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
            ? state.choices
            : state.choices.slice(state.offset, state.offset + 3)
          ).map((choice, index) =>
            ui.button(
              `choose-${index + (target === 'tui' ? state.offset : 0)}`,
              `${compact(choice.label)} · ${compact(choice.connectionName)}`,
              props.busy || !!state.pending || state.failed,
            ),
          ),
        ],
        { visible: !state.defaultsOpen, gap: target === 'desktop' ? 8 : 0 },
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
          slot(
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
      scroll: target === 'tui',
      style: { width: fill, minWidth: 0, background: 0 },
    },
  );
  const view =
    target === 'desktop'
      ? {
          kind: 'popup',
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
        }
      : ui.box('models', [trigger, content], { gap: 0 });
  const out = output(state, view, events);
  out.actions = actions;
  return out;
}
export function models(target: Target): Component<State, ModelsProps> {
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
        open: false,
        choices: [],
        catalogRevision: null,
        next: null,
        failed: false,
        offset: 0,
      };
      return frame(target, state, props, read(state));
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
          pending: null,
          catalogRevision: null,
          defaultModel: null,
          next: null,
          offset: 0,
        };
        return frame(target, state, props, read(state));
      }
      if (props.selection && !previous.staleSelection) previous.suggestDefault = false;
      const initial = candidate(previous, props);
      if (initial && previous.staleSelection) previous.staleSelection = false;
      return frame(
        target,
        { ...previous, revision: previous.revision + 1 },
        props,
        [],
        initial ? [{ name: 'selected', value: initial }] : [],
      );
    },
    event(state, input, props) {
      state.revision++;
      const actions: Action[] = [];
      const events: Emission[] = [];
      if (input.kind === 'action' && input.key === state.pending) {
        state.pending = null;
        const result = input.event as ActionOutcome;
        if (result.status !== 'succeeded') state.failed = true;
        else {
          const page = result.value as SettingsModelsPage;
          if (page.kind === 'changed') {
            state.choices = [];
            state.next = null;
            state.catalogRevision = null;
            state.defaultModel = null;
            actions.push(...read(state));
          } else {
            state.offset = 0;
            state.choices = page.choices;
            state.next = page.next ?? null;
            state.catalogRevision = page.revision;
            state.defaultModel = page.defaultModel;
            const initial = candidate(state, props);
            if (initial) {
              state.staleSelection = false;
              events.push({ name: 'selected', value: initial });
            }
          }
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
        actions.push(...read(state));
      } else if (input.kind === 'raw') {
        if (
          input.key === 'model-popup' &&
          target === 'desktop' &&
          (input.event as { kind?: string })?.kind === 'dismiss'
        )
          state.open = false;
        else if (input.key === 'close' || input.key === 'default-close') state.open = false;
        else if (input.key === 'defaults') state.defaultsOpen = true;
        else if (input.key === 'default-back') state.defaultsOpen = false;
        else if (!props.busy) {
          if (input.key === 'toggle') state.open = !state.open;
          else if (input.key === 'refresh' || input.key === 'default-refresh')
            actions.push(...read(state));
          else if (input.key === 'previous' && target === 'tui' && !state.pending)
            state.offset = Math.max(0, state.offset - 3);
          else if (input.key === 'more' && !state.pending) {
            if (target === 'tui' && state.offset + 3 < state.choices.length) state.offset += 3;
            else if (state.next) actions.push(...read(state, true));
          } else if (
            !state.pending &&
            !state.failed &&
            /^choose-(0|[1-9][0-9]*)$/.test(input.key)
          ) {
            const choice = state.choices[Number(input.key.slice(7))];
            if (choice) {
              state.open = false;
              state.staleSelection = false;
              state.suggestDefault = false;
              events.push({ name: 'selected', value: choice });
            }
          }
        }
      }
      const out = frame(target, state, props, actions, events);
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
