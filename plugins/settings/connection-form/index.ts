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
  ComponentInput,
  ReadingCommand,
  Emission,
  EditorObservation,
  EditorCommand,
  SettingsConnectionDetail,
  SettingsProvider,
  SettingsProvidersPage,
  TextChunk,
  SettingsConnectionsPage,
  ConnectionVersionBasis,
  ConnectionCatalogEntryUpdate,
  CatalogMutationResult,
  RecoveryRef,
  OperationReply,
  CallerIntent,
  Identity,
} from '../../../packages/plugin-sdk/src/index.js';
import { connectionFormContract, type ConnectionFormProps } from '../../shared/contracts.js';
import { pageHeader, withPageShell, type PageState } from '../../shared/page-shell.js';
import { copy, type Copy } from '../../shared/locale.js';
import { sameValue } from '../../shared/value.js';
import {
  compactLabel,
  fill,
  pixels,
  output,
  type Presentation,
  type Node,
} from '../../shared/view.js';

type Detail = Extract<SettingsConnectionDetail, { kind: 'ready' }>;
export interface SettingsValidation {
  create(input: unknown): boolean;
  update(input: unknown): boolean;
}
const names = [
  'name',
  'slug',
  'endpoint',
  'models',
  'configuration',
  'overrides',
  'overlay',
] as const;
type FieldName = (typeof names)[number];
const advancedFields: readonly FieldName[] = ['configuration', 'overrides', 'overlay'];
function formPanes(state: State) {
  return names.filter(
    (name) =>
      (name !== 'slug' || !state.baseline) &&
      (!advancedFields.includes(name) || state.advanced || state.fields[name].enabled),
  );
}
interface Field {
  enabled: boolean;
  observation: EditorObservation | null;
  command: EditorCommand | null;
  sequence: number;
  initialized: boolean;
  failed: boolean;
}
type Request =
  | { kind: 'providers' }
  | { kind: 'read'; field: FieldName; captured: EditorObservation; attempt: number }
  | { kind: 'mutate' }
  | { kind: 'receipt'; reference: RecoveryRef }
  | { kind: 'review'; basis: ConnectionVersionBasis; retried: boolean }
  | { kind: 'reviewCatalog' }
  | { kind: 'history'; after: number }
  | { kind: 'earlier'; reference: RecoveryRef; cursor: number };
interface State extends PageState {
  revision: number;
  serial: number;
  opened: Detail | null;
  baseline: Detail | null;
  catalogRevision: number;
  provider: SettingsProvider | null;
  providers: SettingsProvider[];
  providerNext: number | null;
  providerActive: number;
  providerFocus: ReadingCommand | null;
  providersFailed: boolean;
  fields: Record<FieldName, Field>;
  enabled: boolean;
  enabledEdited: boolean;
  pane: number;
  advanced: boolean;
  pending: Record<string, Request>;
  current: {
    kind: 'save' | 'remove';
    attempt: number;
    values: Partial<Record<FieldName, string>>;
    remaining: FieldName[];
    reference: RecoveryRef | null;
  } | null;
  notice: keyof Copy | null;
  conflict: boolean;
  reviewed: Detail | null;
  reviewedCatalog: number | null;
  reviewPage: number;
  confirmation: 'leave' | 'rebase' | 'remove' | null;
  historyOpen: boolean;
  earlier: { cursor: number; reference: RecoveryRef; result: OperationReply | null }[];
  historyNext: number | null;
  historyFailed: boolean;
}
function request(
  state: State,
  actions: Action[],
  r: Request,
  binding: string,
  method: string,
  input: unknown,
  resources: Action['resources'] = {},
) {
  const key = 'connection-form-' + ++state.serial;
  if (r.kind === 'providers') state.providersFailed = false;
  state.pending[key] = r;
  actions.push({ key, binding, member: 0, method, input, resources });
}
function active(state: State) {
  return names.filter((name) => state.fields[name].enabled);
}
function busy(state: State) {
  return (
    !!state.current ||
    Object.values(state.pending).some((r) =>
      ['providers', 'review', 'reviewCatalog'].includes(r.kind),
    )
  );
}
function fresh(state: State, props: ConnectionFormProps) {
  return sameValue(state.opened, props.connection);
}
function sameConnection(state: State, props: ConnectionFormProps) {
  return state.opened?.target.connectionId === props.connection?.target.connectionId;
}
function configuration(state: State): Record<string, unknown> {
  return (
    state.baseline?.target.configuration ??
    (isObject(state.provider?.descriptor.configurationDefaults)
      ? (state.provider!.descriptor.configurationDefaults as Record<string, unknown>)
      : {})
  );
}
function initial(state: State, name: FieldName) {
  switch (name) {
    case 'name':
      return state.baseline?.name ?? state.provider?.descriptor.label ?? '';
    case 'slug':
      return '';
    case 'endpoint':
      return typeof configuration(state).baseUrl === 'string'
        ? (configuration(state).baseUrl as string)
        : '';
    case 'configuration':
      return JSON.stringify(configuration(state));
    case 'models':
      return '';
    case 'overrides':
    case 'overlay':
      return 'null';
  }
}
function prefill(state: State, name: FieldName, empty = false) {
  const field = state.fields[name];
  if (!field.observation || field.command || busy(state)) return;
  field.initialized = false;
  field.failed = false;
  field.command = {
    owner: field.observation.snapshot.text.owner,
    sequence: ++field.sequence,
    basedOn: field.observation.revision,
    edit: { kind: 'reset', text: empty ? '' : initial(state, name) },
  };
}
function canSave(state: State, props: ConnectionFormProps) {
  return (
    fresh(state, props) &&
    !busy(state) &&
    !state.conflict &&
    !state.confirmation &&
    !state.historyOpen &&
    !!(state.baseline || state.provider) &&
    active(state).every((name) => {
      const f = state.fields[name];
      return (
        f.initialized &&
        !f.failed &&
        !f.command &&
        !!f.observation &&
        !f.observation.snapshot.marked &&
        f.observation.snapshot.text.bytes <= 65536
      );
    })
  );
}
function canRemove(state: State, props: ConnectionFormProps) {
  return !!state.baseline && fresh(state, props) && !busy(state) && !state.conflict;
}
function remove(state: State, props: ConnectionFormProps, actions: Action[]) {
  if (state.confirmation !== 'remove' || !canRemove(state, props)) return;
  const target = state.baseline!.target;
  state.confirmation = null;
  state.current = {
    kind: 'remove',
    attempt: ++state.serial,
    values: {},
    remaining: [],
    reference: null,
  };
  state.notice = null;
  request(state, actions, { kind: 'mutate' }, 'settings', 'remove', {
    expected: { connectionId: target.connectionId, revision: target.revision },
  });
}
function save(
  state: State,
  props: ConnectionFormProps,
  actions: Action[],
  validation: SettingsValidation,
) {
  if (!canSave(state, props)) return;
  const fields = active(state);
  const attempt = ++state.serial;
  state.current = { kind: 'save', attempt, values: {}, remaining: fields, reference: null };
  state.notice = null;
  for (const name of fields) {
    const captured = state.fields[name].observation!;
    request(
      state,
      actions,
      { kind: 'read', field: name, captured, attempt },
      'reader',
      'read',
      { start: 0, end: captured.snapshot.text.bytes },
      {
        text: {
          child: 'field-' + name,
          method: 'snapshot',
          input: {
            kind: 'at_revision',
            owner: captured.snapshot.text.owner,
            revision: captured.revision,
          },
        },
      },
    );
  }
  if (!fields.length) mutate(state, actions, validation);
}
function isObject(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}
function utf8Bytes(value: string) {
  let bytes = 0;
  for (const character of value) {
    const code = character.codePointAt(0)!;
    bytes += code <= 0x7f ? 1 : code <= 0x7ff ? 2 : code <= 0xffff ? 3 : 4;
  }
  return bytes;
}
function json(text: string, nullable = false) {
  // Public input must fit inside the native JSON callback envelope as well as
  // its domain schema. Count containers bottom-up before constructing actions.
  const depths = new WeakMap<object, number>();
  const value: unknown = JSON.parse(text, (_, value: unknown) => {
    if (typeof value === 'number' && !Number.isFinite(value))
      throw new Error('finite number required');
    if (value && typeof value === 'object') {
      let depth = 1;
      for (const child of Object.values(value))
        if (child && typeof child === 'object')
          depth = Math.max(depth, 1 + (depths.get(child) ?? 0));
      if (depth > 64) throw new Error('JSON input exceeds editor nesting limit');
      depths.set(value, depth);
    }
    return value;
  });
  if ((!nullable || value !== null) && !isObject(value)) throw new Error('object required');
  return value as Record<string, unknown> | null;
}
function mutate(state: State, actions: Action[], validation: SettingsValidation) {
  const current = state.current!;
  const values = current.values;
  try {
    const changes: ConnectionCatalogEntryUpdate = {};
    if (values.name !== undefined) {
      if (!values.name.length || values.name.length > 256) throw new Error('name');
      if (!state.baseline || values.name !== state.baseline.name) changes.name = values.name;
    }
    let cfg: Record<string, unknown> | undefined;
    if (values.configuration !== undefined) cfg = json(values.configuration)!;
    else if (
      values.endpoint !== undefined &&
      (state.baseline || values.endpoint !== initial(state, 'endpoint'))
    ) {
      cfg = { ...configuration(state), baseUrl: values.endpoint };
    }
    if (cfg && (!state.baseline || !sameValue(cfg, state.baseline.target.configuration)))
      changes.configuration = cfg;
    if (!state.baseline || state.enabled !== state.baseline.enabled)
      changes.enabled = state.enabled;
    if (values.models !== undefined) {
      const ids = values.models
        .split(/\r?\n/u)
        .map((id) => id.trim())
        .filter(Boolean);
      if (ids.length > 256 || new Set(ids).size !== ids.length) throw new Error('models');
      changes.enabledModelIds = ids;
    }
    if (values.overrides !== undefined)
      changes.modelOverrides = json(
        values.overrides,
        true,
      ) as ConnectionCatalogEntryUpdate['modelOverrides'];
    if (values.overlay !== undefined) changes.requestBodyOverlay = json(values.overlay, true);
    if (!state.baseline) {
      if (
        !values.slug ||
        !/^[a-z0-9]+(?:[-_][a-z0-9]+)*$/.test(values.slug) ||
        values.slug.length > 64
      )
        throw new Error('slug');
      const input = {
        expectedCatalogRevision: state.catalogRevision,
        connection: {
          slug: values.slug,
          name: changes.name,
          provider: state.provider!.identity,
          configuration: changes.configuration ?? configuration(state),
          enabled: state.enabled,
          enabledModelIds: changes.enabledModelIds ?? [],
          ...(changes.modelOverrides ? { modelOverrides: changes.modelOverrides } : {}),
          ...(changes.requestBodyOverlay ? { requestBodyOverlay: changes.requestBodyOverlay } : {}),
        },
      };
      if (!validation.create(input)) throw new Error('invalid create input');
      request(state, actions, { kind: 'mutate' }, 'settings', 'create', input);
    } else if (!Object.keys(changes).length) {
      state.current = null;
      state.notice = 'noConnectionChanges';
    } else {
      const input = {
        expected: {
          connectionId: state.baseline.target.connectionId,
          revision: state.baseline.target.revision,
        },
        changes,
      };
      if (!validation.update(input)) throw new Error('invalid update input');
      request(state, actions, { kind: 'mutate' }, 'settings', 'update', input);
    }
    // Read results are immutable attempt data, never the writable editor state.
    current.values = {};
  } catch {
    state.current = null;
    state.notice = 'invalidConnectionFields';
  }
}
function settle(state: State, reply: OperationReply, events: Emission[]) {
  const current = state.current;
  if (!current || (current.reference && !sameValue(current.reference, reply.reference))) return;
  current.reference = reply.reference;
  if (reply.receipt.kind !== 'recorded' || reply.receipt.outcome.kind !== 'returned') {
    state.notice = 'connectionUnknown';
    return;
  }
  const value = reply.receipt.outcome.value as CatalogMutationResult;
  if (value.kind === 'committed') {
    if (
      current.kind === 'remove'
        ? value.connection !== undefined
        : !value.connection ||
          (state.baseline && value.connection.connectionId !== state.baseline.target.connectionId)
    ) {
      state.notice = 'connectionUnknown';
      return;
    }
    state.current = null;
    events.push({ name: 'changed', value: {} }, { name: 'closed', value: {} });
  } else {
    state.current = null;
    state.conflict = ['connection_stale', 'revision_conflict'].includes(value.kind);
    state.notice = state.conflict ? 'connectionConflict' : 'connectionRejected';
  }
}
function history(state: State, actions: Action[], after: number) {
  if (Object.values(state.pending).some((r) => ['history', 'earlier'].includes(r.kind))) return;
  state.historyFailed = false;
  request(state, actions, { kind: 'history', after }, 'history', 'page', { after, limit: 4 });
}
function fieldLabel(name: FieldName, text: Copy) {
  return {
    name: text.connectionName,
    slug: text.connectionSlug,
    endpoint: text.connectionEndpoint,
    models: text.connectionModels,
    configuration: text.connectionConfiguration,
    overrides: text.connectionOverrides,
    overlay: text.connectionOverlay,
  }[name];
}
function frame<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  props: ConnectionFormProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const target = presentation.target;
  const ui = presentation.paint(props.theme),
    text = copy(props.locale);
  const locked = busy(state);
  const identity: Identity | undefined =
    state.baseline?.target.provider ?? state.provider?.identity;
  const panes = formPanes(state);
  const currentPane = panes[Math.min(state.pane, panes.length - 1)];
  const form = ui.box(
    'form',
    [
      ...pageHeader(
        presentation,
        state,
        props,
        events,
        state.baseline ? text.editConnection : text.createConnection,
        'form-back',
        false,
      ),
      ui.box(
        'form-actions',
        [
          ui.button('form-save', text.saveConnection, !canSave(state, props), true),
          ...(state.current?.reference
            ? [
                ui.button(
                  'form-check',
                  text.check,
                  Object.values(state.pending).some(
                    (r) => r.kind === 'receipt' || r.kind === 'mutate',
                  ),
                ),
              ]
            : []),
          ...(state.conflict || !fresh(state, props)
            ? [
                ui.button(
                  'form-review',
                  text.reviewConnection,
                  locked || !sameConnection(state, props),
                ),
              ]
            : []),
          ui.button(
            'form-history',
            text.earlierConnectionOperations,
            false,
            false,
            target === 'tui' ? { background: 0, align: 'start' } : {},
          ),
        ],
        { row: true, gap: target === 'desktop' ? 8 : 1, terminal: { wrap: true } },
      ),
      ...(state.baseline
        ? [ui.button('form-remove', text.removeConnection, !canRemove(state, props))]
        : []),
      ...(state.notice ? [ui.text('form-notice', text[state.notice], 13, props.theme.muted)] : []),
      ...(!fresh(state, props)
        ? [ui.text('form-target-changed', text.connectionChanged, 13, props.theme.muted)]
        : []),
      ...(!state.baseline && !state.provider
        ? [
            ui.text('provider-heading', text.chooseProvider, 16, props.theme.text, 550),
            ui.box(
              'provider-actions',
              [
                ui.button(
                  'provider-refresh',
                  target === 'tui' ? (props.theme.ascii ? 'r' : '↻') : text.refreshConnections,
                  locked,
                  false,
                  target === 'tui' ? { background: 0 } : {},
                  undefined,
                  text.refreshConnections,
                ),
                ...(state.providerNext !== null
                  ? [ui.button('provider-more', text.moreProviders, locked)]
                  : []),
              ],
              { row: true, gap: target === 'desktop' ? 8 : 1, terminal: { wrap: true } },
            ),
            ...(Object.values(state.pending).some((r) => r.kind === 'providers')
              ? [ui.text('provider-loading', text.connecting, 13, props.theme.muted)]
              : []),
            ...(state.providersFailed
              ? [ui.text('provider-failed', text.providersUnavailable)]
              : []),
            ...(state.providers.length === 0 && !locked && !state.providersFailed
              ? [ui.text('providers-empty', text.noProviders)]
              : []),
            presentation.reading(
              'provider-list',
              {
                command: state.providerFocus,
                content: ui.box(
                  'provider-rows',
                  state.providers.map((provider, at) =>
                    ui.item
                      ? ui.item(
                          'provider-' + at,
                          ui.text(
                            'provider-label-' + at,
                            provider.descriptor.label,
                            14,
                            props.theme.text,
                            400,
                            { truncate: true },
                          ),
                          state.providerActive === at,
                          locked,
                          { followFocus: true, submit: true },
                        )
                      : ui.button('provider-' + at, provider.descriptor.label, locked),
                  ),
                  {
                    focusGroup: target === 'tui',
                    scroll: 'manual',
                    gap: target === 'tui' ? 0 : 8,
                    terminal: { grow: 1, basis: 8, shrink: true },
                    style: { width: fill, height: pixels(240), minHeight: 0 },
                  },
                ),
              },
              { input: 'provider-input' },
            ),
          ]
        : [
            ui.box(
              'provider-summary',
              [
                ui.text(
                  'provider-identity',
                  compactLabel(identity?.name ?? '', 36),
                  13,
                  props.theme.muted,
                ),
                ui.button(
                  'connection-enabled',
                  state.enabled ? text.connectionEnabled : text.connectionDisabled,
                  locked,
                ),
              ],
              { row: true, gap: target === 'desktop' ? 8 : 1, terminal: { wrap: true } },
            ),
            ...(target === 'tui'
              ? [
                  ui.box(
                    'field-navigation',
                    [
                      ...(state.pane > 0 ? [ui.button('field-previous', text.previousModels)] : []),
                      ...(state.pane + 1 < panes.length
                        ? [ui.button('field-next', text.nextField)]
                        : []),
                    ],
                    { row: true, gap: 1, terminal: { wrap: true } },
                  ),
                ]
              : []),
          ]),
      ...(state.baseline || state.provider
        ? [
            ui.button(
              'form-advanced',
              state.advanced ? text.basicSettings : text.advancedSettings,
              locked,
            ),
          ]
        : []),
      // All original native owners stay mounted across provider selection, panes and confirmations.
      ...names.map((name) => {
        const f = state.fields[name],
          label = fieldLabel(name, text);
        const available =
          !!(state.baseline || state.provider) && (name !== 'slug' || !state.baseline);
        return ui.box(
          'pane-' + name,
          [
            ui.box(
              'field-heading-' + name,
              [
                ui.text('label-' + name, label, 15, props.theme.text, 550),
                ...(state.baseline ||
                ['endpoint', 'configuration', 'overrides', 'overlay'].includes(name)
                  ? [
                      ui.button(
                        'edit-' + name,
                        (f.enabled
                          ? state.baseline
                            ? text.keepSavedField
                            : text.useDefaultField
                          : text.replaceField) +
                          ': ' +
                          label,
                        locked ||
                          (name === 'configuration' && state.fields.endpoint.enabled) ||
                          (name === 'endpoint' && state.fields.configuration.enabled),
                      ),
                    ]
                  : []),
              ],
              { row: true, gap: target === 'desktop' ? 8 : 1, terminal: { wrap: true } },
            ),
            ...(['models', 'overrides', 'overlay'].includes(name)
              ? [
                  ui.text(
                    'replace-help-' + name,
                    state.baseline ? text.replaceCollectionHelp : text.newCollectionHelp,
                    12,
                    props.theme.muted,
                  ),
                ]
              : []),
            ...(name === 'configuration'
              ? [ui.text('configuration-help', text.configurationHelp, 12, props.theme.muted)]
              : []),
            ...(f.failed
              ? [
                  ui.text(
                    'field-failed-' + name,
                    state.baseline ? text.fieldPrefillFailed : text.defaultPrefillFailed,
                    12,
                    props.theme.muted,
                  ),
                  ui.button('empty-' + name, text.replaceFromEmpty, locked),
                ]
              : []),
            ui.box(
              'owner-pane-' + name,
              [
                presentation.slot(
                  'field-' + name,
                  name === 'slug' ? 'input' : 'editor',
                  {
                    appearance: name === 'slug' ? props.theme.input : props.theme.textarea,
                    accessibilityLabel: label,
                    placeholder: text.fieldPlaceholder + label,
                    readOnly: locked || !f.enabled,
                    disabled: false,
                    tabIndex: f.enabled ? 0 : -1,
                    autoFocus: target === 'tui' && name === currentPane,
                    command: f.command,
                    ...(name === 'slug' ? {} : { softWrap: true }),
                  },
                  {
                    changed: 'field-changed',
                    ...(name === 'slug' ? { submitted: 'field-submitted' } : {}),
                  },
                ),
              ],
              {
                visible: f.enabled,
                gap: 0,
                style: {
                  width: fill,
                  height: pixels(
                    name === 'slug'
                      ? 36
                      : ['configuration', 'overrides', 'overlay'].includes(name)
                        ? 180
                        : 72,
                  ),
                },
              },
            ),
            ...(!f.enabled
              ? [
                  ui.text(
                    'kept-' + name,
                    state.baseline ? text.savedFieldKept : text.defaultFieldKept,
                    12,
                    props.theme.muted,
                  ),
                ]
              : []),
          ],
          {
            visible:
              available && panes.includes(name) && (target === 'desktop' || currentPane === name),
            gap: target === 'desktop' ? 8 : 1,
            padding: 0,
            style: { width: fill },
          },
        );
      }),
    ],
    {
      visible: !state.historyOpen && !state.confirmation,
      scroll: target === 'tui' ? 'anchor' : 'none',
      gap: target === 'desktop' ? 16 : 1,
      terminal: { grow: 1 },
    },
  );
  const reviewValue =
    state.confirmation === 'remove'
      ? [
          text.connectionName + ': ' + state.baseline!.name,
          text.connectionSlug + ': ' + state.baseline!.target.slug,
          text.connectionProvider + ': ' + state.baseline!.target.provider.name,
        ].join('\n')
      : state.reviewed
        ? JSON.stringify(state.reviewed)
        : state.reviewedCatalog !== null
          ? JSON.stringify({
              catalogRevision: state.reviewedCatalog,
              provider: state.provider?.identity,
            })
          : '';
  // Full human-readable identity, with bounded terminal lines. Native version
  // and package internals remain in the frozen request, not confirmation copy.
  const reviewLines = reviewValue.split(/\r?\n/u).flatMap((line) => {
    if (state.confirmation === 'remove' && target === 'desktop') return [line];
    const characters = [...line],
      lines: string[] = [];
    for (let at = 0; at < characters.length; at += 36)
      lines.push(characters.slice(at, at + 36).join(''));
    return lines.length ? lines : [''];
  });
  const pageSize = target === 'tui' ? 7 : 24;
  const pages = Math.max(1, Math.ceil(reviewLines.length / pageSize));
  const page = Math.min(state.reviewPage, pages - 1);
  const confirmation = ui.box(
    'form-confirmation',
    [
      ui.text(
        'confirm-question',
        state.confirmation === 'leave'
          ? state.current
            ? text.connectionLeaveUnknown
            : text.discardConnection
          : state.confirmation === 'remove'
            ? text.confirmRemoveConnection
            : text.connectionRebaseHelp,
        14,
      ),
      ui.box(
        'confirm-actions',
        [
          ui.button('form-cancel', text.keepEditing),
          ...(state.confirmation === 'leave'
            ? [ui.button('form-discard', text.discard)]
            : [
                ...(state.confirmation === 'remove'
                  ? [
                      ui.button(
                        'form-confirm-remove',
                        text.removeConnection,
                        !canRemove(state, props),
                      ),
                    ]
                  : [
                      ui.button(
                        'form-rebase',
                        text.useConnection,
                        (!state.reviewed && state.reviewedCatalog === null) || locked,
                      ),
                    ]),
                ...(page > 0 ? [ui.button('review-prev', text.previousModels)] : []),
                ...(page + 1 < pages ? [ui.button('review-next', text.moreTarget)] : []),
              ]),
        ],
        { row: true, gap: target === 'desktop' ? 8 : 1, terminal: { wrap: true } },
      ),
      ...(state.confirmation === 'rebase' || state.confirmation === 'remove'
        ? [
            ui.text('review-page', page + 1 + ' / ' + pages, 12, props.theme.muted),
            ...reviewLines
              .slice(page * pageSize, (page + 1) * pageSize)
              .map((line, at) => ui.text('review-line-' + at, line, 13)),
          ]
        : []),
    ],
    { visible: !!state.confirmation, gap: target === 'desktop' ? 12 : 1 },
  );
  const earlier = ui.box(
    'form-earlier',
    [
      ui.button(
        'form-history-back',
        text.back,
        false,
        false,
        target === 'tui' ? { background: 0, align: 'start' } : {},
      ),
      ui.text('earlier-heading', text.earlierConnectionOperations, 20, props.theme.text, 550),
      ui.text('earlier-help', text.earlierConnectionHelp, 13, props.theme.muted),
      ui.box(
        'earlier-actions',
        [
          ui.button(
            'form-history-refresh',
            target === 'tui' ? (props.theme.ascii ? 'r' : '↻') : text.refreshConnections,
            false,
            false,
            target === 'tui' ? { background: 0 } : {},
            undefined,
            text.refreshConnections,
          ),
          ...(state.historyNext !== null
            ? [ui.button('form-history-more', text.moreOperations)]
            : []),
        ],
        { row: true, gap: target === 'desktop' ? 8 : 1, terminal: { wrap: true } },
      ),
      ...(state.historyFailed ? [ui.text('earlier-failed', text.historyUnavailable)] : []),
      ...state.earlier.map((entry, at) => {
        const receipt = entry.result?.receipt;
        const value =
          receipt?.kind === 'recorded' && receipt.outcome.kind === 'returned'
            ? (receipt.outcome.value as CatalogMutationResult)
            : null;
        return ui.box(
          'earlier-entry-' + at,
          [
            ui.text(
              'earlier-result-' + at,
              value?.kind === 'committed'
                ? (entry.reference.method === 'remove'
                    ? text.connectionRemoved
                    : text.connectionSaved) +
                    ' ' +
                    (value.connection?.connectionId ?? '')
                : value
                  ? text.historyRejected
                  : text.historyUnknown,
              13,
            ),
            ui.button(
              'form-earlier-' + at,
              text.check,
              Object.values(state.pending).some(
                (r) => r.kind === 'earlier' && r.cursor === entry.cursor,
              ),
            ),
          ],
          { gap: 0 },
        );
      }),
    ],
    { visible: state.historyOpen, gap: target === 'desktop' ? 12 : 1 },
  );
  const out = output(
    state,
    ui.box(
      'connection-form',
      target !== 'tui'
        ? [form, confirmation, earlier]
        : [
            ui.dialog(
              'form-auxiliary',
              state.historyOpen ? text.earlierConnectionOperations : text.editConnection,
              form,
              ui.box('form-auxiliary-content', [confirmation, earlier], { gap: 0 }),
              !!state.confirmation || state.historyOpen,
              undefined,
              { width: 72, height: state.confirmation === 'leave' ? 8 : 24 },
            ),
          ],
      {
        autoFocus: true,
        shortcuts:
          target === 'tui'
            ? [
                {
                  key: 'escape',
                  modifiers: [],
                  input: state.confirmation
                    ? 'form-cancel'
                    : state.historyOpen
                      ? 'form-history-back'
                      : 'form-back',
                  enabled: true,
                },
              ]
            : [],
        gap: 0,
        padding: target === 'desktop' ? 24 : props.shell ? 0 : 1,
        scroll: target === 'tui' ? 'none' : 'anchor',
        terminal: { grow: 1 },
        style: { width: fill, height: fill, maxWidth: 760, minWidth: 0, minHeight: 0 },
      },
    ),
    events,
  );
  out.actions = actions;
  return out;
}
export function connectionForm<N extends Node>(
  presentation: Presentation<N>,
  validation: SettingsValidation,
): Component<State, ConnectionFormProps, N> {
  const target = presentation.target;
  return withPageShell<State, ConnectionFormProps, N>(
    {
      mount(props) {
        const state: State = {
          revision: 0,
          serial: 0,
          opened: props.connection,
          baseline: props.connection,
          catalogRevision: props.catalogRevision,
          provider: null,
          providers: [],
          providerNext: null,
          providerActive: 0,
          providerFocus: null,
          providersFailed: false,
          fields: Object.fromEntries(
            names.map((name) => [
              name,
              {
                enabled: false,
                observation: null,
                command: null,
                sequence: 0,
                initialized: false,
                failed: false,
              },
            ]),
          ) as State['fields'],
          enabled: props.connection?.enabled ?? true,
          enabledEdited: false,
          pane: 0,
          advanced: false,
          pending: {},
          current: null,
          notice: null,
          conflict: false,
          reviewed: null,
          reviewedCatalog: null,
          reviewPage: 0,
          confirmation: null,
          historyOpen: false,
          earlier: [],
          historyNext: null,
          historyFailed: false,
        };
        const actions: Action[] = [];
        if (!props.connection)
          request(state, actions, { kind: 'providers' }, 'settings', 'providers', { offset: 0 });
        return frame(presentation, state, props, actions);
      },
      update(state, props) {
        state.revision++;
        return frame(presentation, state, props);
      },
      event(state, input, props) {
        if (input.kind === 'component' && input.child === 'provider-list' && input.name === 'input')
          input = input.event as ComponentInput;
        state.revision++;
        const actions: Action[] = [],
          events: Emission[] = [];
        if (input.kind === 'component' && input.child.startsWith('field-')) {
          const name = input.child.slice(6) as FieldName,
            field = state.fields[name];
          if (!field) return frame(presentation, state, props);
          if (input.name === 'changed') {
            field.observation = input.event as EditorObservation;
            const r = field.observation.receipt;
            if (
              field.command &&
              r?.owner === field.command.owner &&
              r.sequence === field.command.sequence
            ) {
              field.failed = r.outcome.status !== 'applied';
              field.initialized = !field.failed;
              field.command = null;
            } else if (field.enabled && !field.initialized && !field.failed && !field.command)
              prefill(state, name);
          } else if (input.name === 'submitted') save(state, props, actions, validation);
        } else if (input.kind === 'action') {
          const r = state.pending[input.key];
          if (!r) return frame(presentation, state, props);
          delete state.pending[input.key];
          const outcome = input.event as ActionOutcome;
          if (r.kind === 'providers') {
            if (outcome.status !== 'succeeded') state.providersFailed = true;
            else {
              const page = outcome.value as SettingsProvidersPage;
              state.providers = page.providers;
              state.providerNext = page.next;
              state.providerActive = 0;
              if (target === 'tui' && state.providers.length)
                state.providerFocus = {
                  sequence: (state.providerFocus?.sequence ?? 0) + 1,
                  target: { kind: 'focus_item', key: 'provider-0' },
                };
            }
          } else if (r.kind === 'read') {
            const current = state.current;
            if (current?.attempt === r.attempt && current.remaining.includes(r.field)) {
              const chunk = outcome.status === 'succeeded' ? (outcome.value as TextChunk) : null;
              if (
                !chunk ||
                !sameValue(chunk.metadata, r.captured.snapshot.text) ||
                utf8Bytes(chunk.text) !== r.captured.snapshot.text.bytes
              ) {
                state.current = null;
                state.notice = 'connectionDraftKept';
              } else {
                current.values[r.field] = chunk.text;
                current.remaining = current.remaining.filter((name) => name !== r.field);
                if (!current.remaining.length) mutate(state, actions, validation);
              }
            }
          } else if (r.kind === 'mutate') {
            if (outcome.status === 'operation') settle(state, outcome.reply, events);
            else if (outcome.status === 'not_accepted') {
              state.current = null;
              state.notice = 'connectionRejected';
            } else state.notice = 'connectionUnknown';
          } else if (r.kind === 'receipt') {
            if (
              outcome.status === 'succeeded' &&
              sameValue((outcome.value as OperationReply).reference, r.reference)
            )
              settle(state, outcome.value as OperationReply, events);
            else state.notice = 'connectionUnknown';
          } else if (r.kind === 'review') {
            const detail =
              outcome.status === 'succeeded' ? (outcome.value as SettingsConnectionDetail) : null;
            if (
              detail?.kind === 'stale' &&
              detail.actual &&
              !r.retried &&
              detail.actual.connectionId === state.baseline?.target.connectionId
            )
              request(
                state,
                actions,
                { kind: 'review', basis: detail.actual, retried: true },
                'settings',
                'connection',
                detail.actual,
              );
            else if (
              detail?.kind === 'ready' &&
              detail.target.connectionId === r.basis.connectionId &&
              detail.target.revision === r.basis.revision
            ) {
              state.reviewed = detail;
              state.reviewPage = 0;
              state.confirmation = 'rebase';
            } else state.notice = 'connectionConflict';
          } else if (r.kind === 'reviewCatalog') {
            const page =
              outcome.status === 'succeeded' ? (outcome.value as SettingsConnectionsPage) : null;
            if (page?.kind === 'ready') {
              state.reviewedCatalog = page.revision;
              state.reviewPage = 0;
              state.confirmation = 'rebase';
            } else state.notice = 'connectionConflict';
          } else if (r.kind === 'history') {
            if (outcome.status !== 'succeeded') state.historyFailed = true;
            else {
              const page = outcome.value as CallerIntent[];
              state.historyNext = page.length === 4 ? page[page.length - 1].cursor : null;
              state.earlier = page
                .filter(
                  (entry) =>
                    entry.reference.contract.name === 'maka.settings.Settings' &&
                    ['create', 'update', 'remove'].includes(entry.reference.method),
                )
                .map((entry) => ({
                  cursor: entry.cursor,
                  reference: entry.reference,
                  result: null,
                }));
            }
          } else if (
            r.kind === 'earlier' &&
            outcome.status === 'succeeded' &&
            sameValue((outcome.value as OperationReply).reference, r.reference)
          ) {
            const entry = state.earlier.find(
              (entry) => entry.cursor === r.cursor && sameValue(entry.reference, r.reference),
            );
            if (entry) entry.result = outcome.value as OperationReply;
          }
        } else if (input.kind === 'raw') {
          const key =
            input.key === 'form-auxiliary'
              ? state.historyOpen
                ? 'form-history-back'
                : 'form-cancel'
              : input.key;
          if (key === 'form-save') save(state, props, actions, validation);
          else if (
            key === 'form-remove' &&
            canRemove(state, props) &&
            !state.confirmation &&
            !state.historyOpen
          ) {
            state.confirmation = 'remove';
            state.reviewPage = 0;
          } else if (key === 'form-confirm-remove') remove(state, props, actions);
          else if (key === 'form-back') {
            if (!state.provider && !state.baseline && !state.current)
              events.push({ name: 'closed', value: {} });
            else state.confirmation = 'leave';
          } else if (key === 'form-cancel') {
            state.confirmation = null;
            state.reviewed = null;
            state.reviewedCatalog = null;
          } else if (key === 'form-discard' && state.confirmation === 'leave')
            events.push({ name: 'closed', value: {} });
          else if (
            key === 'form-check' &&
            state.current?.reference &&
            !Object.values(state.pending).some((r) => ['receipt', 'mutate'].includes(r.kind))
          )
            request(
              state,
              actions,
              { kind: 'receipt', reference: state.current.reference },
              'receipts',
              'read',
              state.current.reference,
            );
          else if (key === 'form-review' && !busy(state) && sameConnection(state, props)) {
            if (state.baseline) {
              const basis = {
                connectionId: state.baseline.target.connectionId,
                revision: state.baseline.target.revision,
              };
              request(
                state,
                actions,
                { kind: 'review', basis, retried: false },
                'settings',
                'connection',
                basis,
              );
            } else
              request(state, actions, { kind: 'reviewCatalog' }, 'settings', 'connections', {
                kind: 'start',
              });
          } else if (
            key === 'review-next' &&
            ['rebase', 'remove'].includes(state.confirmation ?? '')
          )
            state.reviewPage++;
          else if (key === 'review-prev' && ['rebase', 'remove'].includes(state.confirmation ?? ''))
            state.reviewPage = Math.max(0, state.reviewPage - 1);
          else if (
            key === 'form-rebase' &&
            state.confirmation === 'rebase' &&
            (state.reviewed || state.reviewedCatalog !== null) &&
            !busy(state) &&
            sameConnection(state, props)
          ) {
            // Explicitly accept complete replacement of every opted-in field, including remote config keys.
            if (state.reviewed) {
              state.baseline = state.reviewed;
              if (!state.enabledEdited) state.enabled = state.reviewed.enabled;
            } else state.catalogRevision = state.reviewedCatalog!;
            state.opened = props.connection;
            state.reviewed = null;
            state.reviewedCatalog = null;
            state.confirmation = null;
            state.conflict = false;
            state.notice = null;
          } else if (key === 'form-history') {
            state.historyOpen = true;
            history(state, actions, 0);
          } else if (key === 'form-history-back') {
            state.historyOpen = false;
            if (state.providerFocus)
              state.providerFocus = {
                sequence: state.providerFocus.sequence + 1,
                target: { kind: 'focus_item', key: 'provider-' + state.providerActive },
              };
          } else if (key === 'form-history-refresh') history(state, actions, 0);
          else if (key === 'form-history-more' && state.historyNext !== null)
            history(state, actions, state.historyNext);
          else if (/^form-earlier-(0|[1-9][0-9]*)$/.test(key) && state.historyOpen) {
            const entry = state.earlier[Number(key.slice(13))];
            if (
              entry &&
              !Object.values(state.pending).some(
                (r) => r.kind === 'earlier' && r.cursor === entry.cursor,
              )
            )
              request(
                state,
                actions,
                { kind: 'earlier', reference: entry.reference, cursor: entry.cursor },
                'receipts',
                'read',
                entry.reference,
              );
          } else if (
            !busy(state) &&
            !state.confirmation &&
            !state.historyOpen &&
            fresh(state, props)
          ) {
            if (key === 'connection-enabled') {
              state.enabled = !state.enabled;
              state.enabledEdited = true;
            } else if (key === 'form-advanced') {
              state.advanced = !state.advanced;
              state.pane = 0;
            } else if (key === 'field-next')
              state.pane = Math.min(state.pane + 1, formPanes(state).length - 1);
            else if (key === 'field-previous') state.pane = Math.max(0, state.pane - 1);
            else if (key === 'provider-refresh' && !state.provider && !state.baseline)
              request(state, actions, { kind: 'providers' }, 'settings', 'providers', {
                offset: 0,
              });
            else if (key === 'provider-more' && !state.provider && !state.baseline) {
              if (state.providerNext !== null)
                request(state, actions, { kind: 'providers' }, 'settings', 'providers', {
                  offset: state.providerNext,
                });
            } else if (
              /^provider-(0|[1-9][0-9]*)$/.test(key) &&
              !state.provider &&
              !state.baseline
            ) {
              const provider = state.providers[Number(key.slice(9))];
              if (provider) {
                state.providerActive = Number(key.slice(9));
                if (target === 'tui' && (input.event as { kind?: string }).kind !== 'submit')
                  return frame(presentation, state, props, actions, events);
                state.provider = provider;
                for (const name of ['name', 'slug', 'endpoint', 'models'] as FieldName[]) {
                  state.fields[name].enabled = true;
                  prefill(state, name);
                }
              }
            } else if (key.startsWith('edit-')) {
              const name = key.slice(5) as FieldName,
                f = state.fields[name];
              if (
                f &&
                (name !== 'slug' || !state.baseline) &&
                !(name === 'configuration' && state.fields.endpoint.enabled) &&
                !(name === 'endpoint' && state.fields.configuration.enabled)
              ) {
                f.enabled = !f.enabled;
                if (f.enabled && !f.initialized) prefill(state, name);
              }
            } else if (key.startsWith('empty-')) {
              const name = key.slice(6) as FieldName;
              if (state.fields[name]?.enabled && state.fields[name].failed)
                prefill(state, name, true);
            }
          }
        }
        return frame(presentation, state, props, actions, events);
      },
    },
    () => 'form-back',
  );
}
export function activate() {
  return {
    ConnectionForm: {
      kind: 'component',
      contract: connectionFormContract,
      component: ['component'],
    },
  };
}
