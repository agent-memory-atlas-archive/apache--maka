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
  CallerIntent,
  Component,
  EditorCommand,
  EditorObservation,
  Emission,
  InputProps,
  OperationReply,
  RecoveryRef,
  SettingsAssistantPreferences,
  SettingsPreferencesUpdate,
  SettingsPreferencesUpdated,
  TextChunk,
  TextareaProps,
} from '../../../packages/plugin-sdk/src/index.js';
import { assistantPreferencesContract, type ProjectsProps } from '../../shared/contracts.js';
import { withPageShell, pageHeader, type PageState } from '../../shared/page-shell.js';
import { sameValue } from '../../shared/value.js';
import { copy, tuiText } from './locale.js';
import { fill, output, type Node, type Presentation } from '../../shared/view.js';

type Name = 'name' | 'tone';
interface Field {
  observation: EditorObservation | null;
  command: EditorCommand | null;
  reset: string | null;
  sequence: number;
  ready: boolean;
}
type Request =
  | { kind: 'preferences'; reset: boolean }
  | { kind: 'mutate' | 'receipt' }
  | { kind: 'history'; after: number }
  | { kind: 'field'; name: Name; captured: EditorObservation };
interface State extends PageState {
  revision: number;
  serial: number;
  saved: SettingsAssistantPreferences | null;
  fields: Record<Name, Field>;
  pending: { key: string; request: Request } | null;
  current: { input: SettingsPreferencesUpdate; reference: RecoveryRef | null } | null;
  capture: { name: EditorObservation; tone: EditorObservation; displayName: string | null } | null;
  restoring: boolean;
  notice: string | null;
}
const field = (): Field => ({
  observation: null,
  command: null,
  reset: null,
  sequence: 0,
  ready: false,
});
function initialize(field: Field) {
  if (field.reset !== null && field.observation && !field.command)
    field.command = {
      owner: field.observation.snapshot.text.owner,
      sequence: ++field.sequence,
      basedOn: field.observation.revision,
      edit: { kind: 'reset', text: field.reset },
    };
}
function request(
  state: State,
  actions: Action[],
  request: Request,
  binding: string,
  method: string,
  input: unknown,
  resources: Action['resources'] = {},
) {
  if (state.pending) return;
  const key = 'assistant-preferences-' + ++state.serial;
  state.pending = { key, request };
  actions.push({ key, binding, member: 0, method, input, resources });
}
function read(state: State, actions: Action[], reset = true) {
  request(state, actions, { kind: 'preferences', reset }, 'settings', 'assistantPreferences', {});
}
function history(state: State, actions: Action[], after = 0) {
  state.restoring = true;
  if (!after) state.current = null;
  request(state, actions, { kind: 'history', after }, 'history', 'page', { after, limit: 16 });
}
function disabled(state: State, props: ProjectsProps) {
  return (
    !props.open ||
    props.busy ||
    !!state.pending ||
    !!state.current ||
    !!state.capture ||
    state.restoring
  );
}
function mutate(state: State, actions: Action[], input: SettingsPreferencesUpdate) {
  state.current = { input, reference: null };
  state.notice = null;
  request(state, actions, { kind: 'mutate' }, 'settings', 'updateAssistantPreferences', input);
  actions.at(-1)!.recovery = { kind: 'assistant-preferences/v1', input };
}
function settle(state: State, reply: OperationReply, actions: Action[]) {
  if (
    !state.current ||
    (state.current.reference && !sameValue(state.current.reference, reply.reference))
  )
    return;
  state.current.reference = reply.reference;
  if (reply.receipt.kind !== 'recorded' || reply.receipt.outcome.kind !== 'returned') {
    state.notice = 'assistant-unknown';
    return;
  }
  const result = reply.receipt.outcome.value as SettingsPreferencesUpdated;
  const reset = state.current.input.mutation.kind === 'personalization' || !state.saved;
  state.current = null;
  state.notice =
    result.kind === 'committed'
      ? 'assistant-saved'
      : result.kind === 'revision_conflict'
        ? 'assistant-conflict'
        : 'assistant-invalid';
  if (result.kind === 'committed' || !state.saved) read(state, actions, reset);
}
function capture(state: State, actions: Action[], name: Name) {
  const captured = state.capture![name];
  request(
    state,
    actions,
    { kind: 'field', name, captured },
    'reader',
    'read',
    { start: 0, end: captured.snapshot.text.bytes },
    {
      text: {
        child: 'assistant-' + name,
        method: 'snapshot',
        input: {
          kind: 'at_revision',
          owner: captured.snapshot.text.owner,
          revision: captured.snapshot.text.revision,
        },
      },
    },
  );
}
export function assistantPreferences<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ProjectsProps, N> {
  function frame(
    state: State,
    props: ProjectsProps,
    actions: Action[] = [],
    events: Emission[] = [],
  ) {
    const ui = presentation.paint(props.theme),
      t = (key: string) => tuiText(props.locale, key);
    const busy = disabled(state, props),
      text = copy(props.locale);
    const fields = (['name', 'tone'] as const).map((name) => {
      const common = {
        accessibilityLabel: t('assistant-' + name),
        placeholder: '',
        autoFocus: name === 'name',
        tabIndex: name === 'name' ? 1 : 2,
        readOnly: false,
        disabled: busy || !state.fields[name].ready || !!state.fields[name].command,
        command: state.fields[name].command,
      } satisfies Omit<InputProps, 'appearance'>;
      const editor =
        name === 'name'
          ? presentation.slot(
              'assistant-name',
              'input',
              { ...common, appearance: props.theme.input } satisfies InputProps,
              { changed: 'changed' },
            )
          : presentation.slot(
              'assistant-tone',
              'editor',
              {
                ...common,
                appearance: props.theme.textarea,
                softWrap: true,
                terminalRows: { min: 3, max: 5 },
              } satisfies TextareaProps,
              { changed: 'changed' },
            );
      return ui.box(
        'assistant-field-' + name,
        [ui.text('assistant-label-' + name, t('assistant-' + name), 14, props.theme.muted), editor],
        { gap: 0 },
      );
    });
    const content = ui.box(
      'assistant-preferences-content',
      [
        ...fields,
        ui.text(
          'assistant-notice',
          state.notice && !state.pending ? t(state.notice) : ' ',
          13,
          props.theme.muted,
        ),
        ui.button(
          'save',
          t('assistant-save'),
          busy ||
            (['name', 'tone'] as const).some(
              (name) => !state.fields[name].ready || !!state.fields[name].command,
            ),
        ),
        ui.text('assistant-help', t('assistant-instructions-help'), 13, props.theme.muted),
        ui.button(
          'instructions',
          `${t('assistant-instructions')}: ${state.saved?.workspaceInstructions ? text.settingOn : text.settingOff}`,
          busy || !state.saved,
          false,
          { background: 0, align: 'start' },
        ),
        ui.text(
          'assistant-shell-titles-help',
          t('assistant-shell-titles-help'),
          13,
          props.theme.muted,
        ),
        ui.button(
          'shell-titles',
          `${t('assistant-shell-titles')}: ${state.saved?.shellTitles ? text.settingOn : text.settingOff}`,
          busy || !state.saved,
          false,
          { background: 0, align: 'start' },
        ),
        ui.text('assistant-privacy-help', t('assistant-privacy-help'), 13, props.theme.muted),
        ui.button(
          'privacy',
          `${t('assistant-privacy')}: ${state.saved?.incognitoActive ? text.settingOn : text.settingOff}`,
          busy || !state.saved,
          false,
          { background: 0, align: 'start' },
        ),
        ...(state.current
          ? [
              ui.button(
                'receipt',
                text.executionReceipt,
                !!state.pending || !state.current.reference,
              ),
            ]
          : []),
        ...((state.restoring || (state.current && !state.current.reference)) && !state.pending
          ? [ui.button('history', text.refreshDefaultOperations)]
          : []),
        ui.button('reload', t('assistant-reload'), busy),
      ],
      {
        gap: 1,
        scroll: 'manual',
        terminal: { grow: 1 },
        style: { width: fill, height: fill },
      },
    );
    const result = output(
      state,
      ui.box(
        'assistant-preferences-root',
        [...pageHeader(presentation, state, props, events, t('settings-assistant')), content],
        {
          visible: props.open,
          gap: 1,
          terminal: { grow: 1 },
          style: { width: fill, height: fill },
          shortcuts: [{ key: 'escape', modifiers: [], input: 'back', enabled: true }],
        },
      ),
      events,
    );
    result.actions = actions;
    return result;
  }
  return withPageShell<State, ProjectsProps, N>(
    {
      mount(props) {
        const state: State = {
          revision: 0,
          serial: 0,
          saved: null,
          fields: { name: field(), tone: field() },
          pending: null,
          current: null,
          capture: null,
          restoring: true,
          notice: null,
        };
        const actions: Action[] = [];
        history(state, actions);
        return frame(state, props, actions);
      },
      update(state, props) {
        state.revision++;
        return frame(state, props);
      },
      event(state, input, props) {
        state.revision++;
        const actions: Action[] = [],
          events: Emission[] = [];
        if (input.kind === 'component') {
          const name = (['name', 'tone'] as const).find(
            (name) => input.kind === 'component' && input.child === 'assistant-' + name,
          );
          if (name && input.name === 'changed') {
            const field = state.fields[name],
              observed = input.event as EditorObservation;
            if (
              field.observation &&
              field.observation.snapshot.text.owner !== observed.snapshot.text.owner
            ) {
              field.command = null;
              field.ready = false;
              field.reset =
                name === 'name'
                  ? (state.saved?.displayName ?? '')
                  : (state.saved?.assistantTone ?? '');
            }
            field.observation = observed;
            const receipt = observed.receipt;
            if (
              field.command &&
              receipt?.owner === field.command.owner &&
              receipt.sequence === field.command.sequence
            ) {
              field.ready = receipt.outcome.status === 'applied';
              field.command = null;
              field.reset = null;
              if (!field.ready) state.notice = 'assistant-unavailable';
            } else initialize(field);
          }
        } else if (input.kind === 'action' && input.key === state.pending?.key) {
          const r = state.pending.request,
            result = input.event as ActionOutcome;
          state.pending = null;
          if (r.kind === 'history') {
            if (result.status !== 'succeeded') state.notice = 'assistant-unavailable';
            else {
              const items = result.value as CallerIntent[];
              for (const item of items) {
                const recovery = item.recovery as {
                  kind?: string;
                  input?: SettingsPreferencesUpdate;
                } | null;
                if (
                  item.reference.method === 'updateAssistantPreferences' &&
                  item.reference.contract.name === 'maka.settings.Settings' &&
                  recovery?.kind === 'assistant-preferences/v1' &&
                  recovery.input
                )
                  state.current = { input: recovery.input, reference: item.reference };
              }
              const last = items.at(-1);
              if (last && last.cursor > r.after) history(state, actions, last.cursor);
              else if (last) state.notice = 'assistant-unavailable';
              else {
                state.restoring = false;
                if (state.current?.reference)
                  request(
                    state,
                    actions,
                    { kind: 'receipt' },
                    'receipts',
                    'read',
                    state.current.reference,
                  );
                else read(state, actions);
              }
            }
          } else if (r.kind === 'preferences') {
            if (result.status !== 'succeeded') state.notice = 'assistant-unavailable';
            else {
              state.saved = result.value as SettingsAssistantPreferences;
              if (r.reset)
                for (const name of ['name', 'tone'] as const) {
                  state.fields[name].ready = false;
                  state.fields[name].reset =
                    name === 'name' ? state.saved.displayName : state.saved.assistantTone;
                  initialize(state.fields[name]);
                }
            }
          } else if (r.kind === 'mutate') {
            if (result.status === 'operation') settle(state, result.reply, actions);
            else if (result.status === 'not_accepted') {
              state.current = null;
              state.notice = 'assistant-unavailable';
            } else state.notice = 'assistant-unknown';
          } else if (r.kind === 'receipt') {
            if (result.status === 'succeeded')
              settle(state, result.value as OperationReply, actions);
            else state.notice = 'assistant-unknown';
          } else if (r.kind === 'field') {
            const chunk = result.status === 'succeeded' ? (result.value as TextChunk) : null;
            if (
              !state.capture ||
              !chunk ||
              !sameValue(chunk.metadata, r.captured.snapshot.text) ||
              !sameValue(
                state.fields[r.name].observation?.snapshot.text,
                r.captured.snapshot.text,
              ) ||
              [...chunk.text].length > (r.name === 'name' ? 256 : 4096)
            ) {
              state.capture = null;
              state.notice = 'assistant-invalid';
            } else if (r.name === 'name') {
              state.capture.displayName = chunk.text;
              capture(state, actions, 'tone');
            } else {
              const current = state.capture;
              state.capture = null;
              if (
                current.displayName !== null &&
                state.saved &&
                sameValue(state.fields.name.observation?.snapshot.text, current.name.snapshot.text)
              )
                mutate(state, actions, {
                  expectedRevision: state.saved.revision,
                  mutation: {
                    kind: 'personalization',
                    displayName: current.displayName,
                    assistantTone: chunk.text,
                  },
                });
              else state.notice = 'assistant-invalid';
            }
          }
        } else if (input.kind === 'raw' && props.open) {
          if (input.key === 'back') events.push({ name: 'closed', value: {} });
          else if (
            input.key === 'history' &&
            !state.pending &&
            (state.restoring || (state.current && !state.current.reference))
          )
            history(state, actions);
          else if (input.key === 'receipt' && !state.pending && state.current?.reference)
            request(
              state,
              actions,
              { kind: 'receipt' },
              'receipts',
              'read',
              state.current.reference,
            );
          else if (!disabled(state, props)) {
            if (input.key === 'reload') read(state, actions);
            else if (input.key === 'instructions' && state.saved)
              mutate(state, actions, {
                expectedRevision: state.saved.revision,
                mutation: {
                  kind: 'workspace_instructions',
                  enabled: !state.saved.workspaceInstructions,
                },
              });
            else if (input.key === 'privacy' && state.saved)
              mutate(state, actions, {
                expectedRevision: state.saved.revision,
                mutation: { kind: 'privacy', incognitoActive: !state.saved.incognitoActive },
              });
            else if (input.key === 'shell-titles' && state.saved)
              mutate(state, actions, {
                expectedRevision: state.saved.revision,
                mutation: { kind: 'shell_titles', enabled: !state.saved.shellTitles },
              });
            else if (input.key === 'save' && state.saved) {
              const name = state.fields.name.observation,
                tone = state.fields.tone.observation;
              if (
                name &&
                tone &&
                !name.snapshot.marked &&
                !tone.snapshot.marked &&
                name.snapshot.text.bytes <= 1024 &&
                tone.snapshot.text.bytes <= 16384 &&
                !state.fields.name.command &&
                !state.fields.tone.command &&
                state.fields.name.ready &&
                state.fields.tone.ready
              ) {
                state.capture = { name, tone, displayName: null };
                capture(state, actions, 'name');
              } else state.notice = 'assistant-invalid';
            }
          }
        }
        return frame(state, props, actions, events);
      },
    },
    () => 'back',
  );
}
export function activate() {
  return {
    AssistantPreferences: {
      kind: 'component',
      contract: assistantPreferencesContract,
      component: ['component'],
    },
  };
}
