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
  BindingCandidate,
  BindingCandidatesPage,
  BindingCandidatesRead,
  CallerIntent,
  Component,
  ComponentInput,
  CompositionEdit,
  DeploymentRef,
  EditorCommand,
  EditorObservation,
  Emission,
  FileObservation,
  FilePickerProps,
  InputProps,
  TextareaProps,
  ManagedImport,
  ManagementResultForDeploymentRef,
  ManagementResultForPackageInstalled,
  OperationReply,
  PackageSettingsPage,
  RecoveryRef,
  SettingChange,
  SettingField,
  SettingValue,
  Target,
  TextChunk,
} from '../../../packages/plugin-sdk/src/index.js';
import {
  appsContract,
  type AppsConfiguration,
  type AppsProps,
  type AppEntry,
} from '../../shared/contracts.js';
import { pageHeader, withPageShell, type PageState } from '../../shared/page-shell.js';
import { copy, type Copy } from '../../shared/locale.js';
import { settingOptions } from '../../shared/instance-settings.js';
import { sameValue } from '../../shared/value.js';
import { fill, output, type Node, type Presentation } from '../../shared/view.js';

type Mutation =
  | { kind: 'install'; source: DeploymentRef; file: FileObservation }
  | { kind: 'edit'; input: CompositionEdit; outcome: 'add' | 'remove' };
type Request =
  | { kind: 'catalog' | 'bindings'; input: BindingCandidatesRead }
  | { kind: 'settings'; path: string[]; after: string | null }
  | { kind: 'field'; name: 'name' | 'setting'; observation: EditorObservation }
  | { kind: 'mutation' | 'receipt' }
  | { kind: 'history'; after: number };
interface Field {
  observation: EditorObservation | null;
  command: EditorCommand | null;
  reset: string | null;
  sequence: number;
  ready: boolean;
}
interface Draft {
  candidate: BindingCandidate;
  id: string;
  label: string;
  changes: SettingChange[];
  bindings: Record<string, Target[]>;
  page: PackageSettingsPage | null;
  path: string[];
  after: string | null;
  previous: (string | null)[];
  setting: { descriptor: SettingField; path: string[] } | null;
  choosing: ManagedImport | null;
  choices: BindingCandidatesPage | null;
}
interface State extends PageState {
  revision: number;
  serial: number;
  open: boolean;
  route: 'list' | 'app' | 'choose' | 'configure' | 'bindings' | 'file';
  catalog: BindingCandidatesPage | null;
  selected: string | null;
  opened: string[];
  draft: Draft | null;
  fields: { name: Field; setting: Field };
  file: FileObservation | null;
  pending: { key: string; request: Request } | null;
  original: RecoveryRef | null;
  restoring: boolean;
  historyFailed: boolean;
  current: { input: Mutation; reference: RecoveryRef | null } | null;
  confirm: Mutation | null;
  notice: keyof Copy | null;
  diagnostic: string | null;
}
const field = (): Field => ({
  observation: null,
  command: null,
  reset: null,
  sequence: 0,
  ready: true,
});
function initialize(field: Field) {
  if (field.reset !== null && field.observation && !field.command)
    field.command = {
      owner: field.observation.snapshot.text.owner,
      basedOn: field.observation.revision,
      sequence: ++field.sequence,
      edit: { kind: 'reset', text: field.reset },
    };
}
function reset(state: State, name: 'name' | 'setting', text: string) {
  const field = state.fields[name];
  field.reset = text;
  field.ready = false;
  initialize(field);
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
  if (state.pending) return;
  const key = 'apps-' + ++state.serial;
  state.pending = { key, request: r };
  actions.push({ key, binding, member: 0, method, input, resources });
}
function catalog(
  state: State,
  actions: Action[],
  configuration: AppsConfiguration,
  source: DeploymentRef | null = null,
  after: string | null = null,
) {
  const input: BindingCandidatesRead = {
    source,
    consumer: { kind: 'root', name: configuration.root },
    import: 'pages',
    includePackages: true,
    after,
  };
  request(state, actions, { kind: 'catalog', input }, 'management', 'bindingCandidates', input);
}
function settings(state: State, actions: Action[]) {
  const d = state.draft,
    source = state.catalog?.source;
  if (!source || d?.candidate.kind !== 'package') return;
  request(
    state,
    actions,
    { kind: 'settings', path: d.path, after: d.after },
    'management',
    'packageSettings',
    { source, package: d.candidate.package, path: d.path, after: d.after, changes: d.changes },
  );
}
function bindings(state: State, actions: Action[], after: string | null = null) {
  const d = state.draft,
    source = state.catalog?.source;
  if (!source || d?.candidate.kind !== 'package' || !d.choosing) return;
  const input: BindingCandidatesRead = {
    source,
    consumer: { kind: 'package', package: d.candidate.package },
    import: d.choosing.name,
    includePackages: false,
    after,
  };
  request(state, actions, { kind: 'bindings', input }, 'management', 'bindingCandidates', input);
}
function history(state: State, actions: Action[], after = 0) {
  state.restoring = true;
  state.historyFailed = false;
  if (!after) state.original = null;
  request(state, actions, { kind: 'history', after }, 'history', 'page', { after, limit: 16 });
}
function locked(state: State, props: AppsProps) {
  return (
    !props.open ||
    props.busy ||
    !!state.pending ||
    !!state.current ||
    state.restoring ||
    state.historyFailed
  );
}
function capture(state: State, actions: Action[], name: 'name' | 'setting') {
  const field = state.fields[name],
    observation = field.observation;
  if (!observation || observation.snapshot.marked || field.command || !field.ready) return;
  request(
    state,
    actions,
    { kind: 'field', name, observation },
    'reader',
    'read',
    { start: 0, end: observation.snapshot.text.bytes },
    {
      text: {
        child: 'apps-' + name,
        method: 'snapshot',
        input: {
          kind: 'at_revision',
          owner: observation.snapshot.text.owner,
          revision: observation.snapshot.text.revision,
        },
      },
    },
  );
}
function value(draft: Draft, f: SettingField): SettingValue | null {
  const change = draft.changes.find((c) => sameValue(c.path, [...draft.path, f.key]));
  return (change ? change.value : f.value) ?? null;
}
function change(draft: Draft, path: string[], v: SettingValue | null) {
  const at = draft.changes.findIndex((c) => sameValue(c.path, path));
  const item: SettingChange = { path: path as SettingChange['path'], value: v };
  if (at < 0) draft.changes.push(item);
  else draft.changes[at] = item;
}
function display(value: SettingValue | null, text: Copy) {
  return value?.kind === 'text' || value?.kind === 'number'
    ? value.text || text.settingUnset
    : value?.kind === 'boolean'
      ? value.value
        ? text.settingOn
        : text.settingOff
      : value?.kind === 'null'
        ? text.settingNone
        : text.settingUnset;
}
function complete(d: Draft) {
  return (
    d.candidate.kind === 'bound' ||
    (!!d.page &&
      d.page.imports.every((i) => {
        const count = (d.bindings[i.name] ?? []).length;
        return i.cardinality.kind === 'one'
          ? count === 1
          : i.cardinality.kind === 'optional'
            ? count <= 1
            : count >= i.cardinality.min && count <= i.cardinality.max;
      }))
  );
}
function entries(state: State): AppEntry[] {
  return (state.catalog?.configuration.entries ?? []) as AppEntry[];
}
function edit(state: State, configuration: AppsConfiguration, label: string) {
  const page = state.catalog,
    draft = state.draft;
  if (!page || !draft || !complete(draft)) {
    state.notice = 'appsBindingsMissing';
    return;
  }
  const candidate = draft.candidate;
  const target: Target =
    candidate.kind === 'bound'
      ? candidate.target
      : { kind: 'instance', path: [draft.id], export: candidate.export };
  state.confirm = {
    kind: 'edit',
    outcome: 'add',
    input: {
      expectedRevision: page.source.revision,
      edits: [
        ...(candidate.kind === 'package'
          ? [
              {
                kind: 'insert' as const,
                name: draft.id,
                package: candidate.package,
                settings: draft.changes,
                bindings: draft.bindings,
              },
            ]
          : []),
        {
          kind: 'update',
          name: configuration.root,
          configuration: { entries: [...entries(state), { id: draft.id, label }] },
          bindings: { pages: [...page.current, target] },
        },
      ],
    },
  };
}
function mutate(state: State, actions: Action[]) {
  const input = state.confirm;
  if (!input) return;
  state.current = { input, reference: null };
  state.notice = null;
  state.diagnostic = null;
  request(
    state,
    actions,
    { kind: 'mutation' },
    'management',
    input.kind === 'install' ? 'install' : 'editComposition',
    input.kind === 'install' ? { expectedRevision: input.source.revision } : input.input,
    input.kind === 'install'
      ? {
          package: {
            child: 'apps-file',
            method: 'snapshot',
            input: { owner: input.file.owner, revision: input.file.revision },
          },
        }
      : {},
  );
  actions.at(-1)!.recovery = { kind: 'application-apps/v1', input };
}
function settle(
  state: State,
  reply: OperationReply,
  actions: Action[],
  configuration: AppsConfiguration,
) {
  if (
    !state.current ||
    (state.current.reference && !sameValue(reply.reference, state.current.reference))
  )
    return;
  state.current.reference = reply.reference;
  if (reply.receipt.kind !== 'recorded' || reply.receipt.outcome.kind !== 'returned') {
    state.notice = 'appsUnknown';
    return;
  }
  const result = reply.receipt.outcome.value as
    | ManagementResultForDeploymentRef
    | ManagementResultForPackageInstalled;
  const input = state.current.input;
  state.current = null;
  state.confirm = null;
  if (result.status === 'committed') {
    state.notice =
      input.kind === 'install'
        ? 'appsPackageAdded'
        : input.outcome === 'add'
          ? 'appsAdded'
          : 'appsRemoved';
    state.draft = null;
    state.route = input.kind === 'install' ? 'choose' : 'list';
    catalog(state, actions, configuration);
  } else {
    state.notice = result.status === 'conflict' ? 'appsChanged' : 'appsInvalid';
    state.diagnostic = result.status === 'rejected' ? (result.diagnostic ?? null) : null;
  }
}
export function apps<N extends Node>(
  presentation: Presentation<N>,
  configuration: AppsConfiguration,
): Component<State, AppsProps, N> {
  function frame(state: State, props: AppsProps, actions: Action[] = [], events: Emission[] = []) {
    const ui = presentation.paint(props.theme),
      text = copy(props.locale),
      disabled = locked(state, props);
    const suspended = disabled || !!state.confirm || !!state.draft?.setting;
    const draft = state.draft;
    const rows: N[] =
      state.route === 'list'
        ? configuration.entries.map((app, index) =>
            ui.box(
              'app-row-' + app.id,
              [
                ui.button('open-' + app.id, app.label, disabled, false, { align: 'start' }),
                ...(state.opened.includes(app.id)
                  ? [ui.button('close-' + app.id, text.appsClose, disabled)]
                  : []),
                ui.button(
                  'remove-' + index,
                  text.appsRemove,
                  disabled || !state.catalog || !sameValue(entries(state), configuration.entries),
                ),
              ],
              { row: true, terminal: { wrap: true }, style: { wrap: true } },
            ),
          )
        : state.route === 'choose'
          ? (state.catalog?.candidates ?? []).map((choice, index) =>
              ui.button('candidate-' + index, choice.label, disabled, false, { align: 'start' }),
            )
          : state.route === 'bindings'
            ? (draft?.choices?.candidates ?? []).map((choice, index) =>
                ui.button(
                  'binding-' + index,
                  choice.label,
                  disabled,
                  choice.kind === 'bound' &&
                    !!draft?.choosing &&
                    (draft.bindings[draft.choosing.name] ?? []).some((t) =>
                      sameValue(t, choice.target),
                    ),
                  { align: 'start' },
                ),
              )
            : [];
    const title =
      state.route === 'app'
        ? (configuration.entries.find((e) => e.id === state.selected)?.label ?? text.appsTitle)
        : state.route === 'choose'
          ? text.appsChoose
          : state.route === 'configure'
            ? text.appsConfigure
            : state.route === 'bindings'
              ? (draft?.choosing?.name ?? text.appsBindings)
              : state.route === 'file'
                ? text.appsInstall
                : text.appsTitle;
    const page = ui.box(
      'apps-page',
      [
        ...pageHeader(
          presentation,
          state,
          props,
          events,
          title,
          'back',
          !!state.pending && !state.current,
        ),
        ...(state.pending || state.restoring
          ? [ui.text('apps-loading', text.connecting, 13, props.theme.muted)]
          : []),
        ...(state.notice
          ? [ui.text('apps-notice', text[state.notice], 13, props.theme.muted)]
          : []),
        ...(state.diagnostic
          ? [ui.text('apps-diagnostic', state.diagnostic, 13, props.theme.warning)]
          : []),
        ...(state.current
          ? [
              ui.button(
                'receipt',
                text.executionReceipt,
                !!state.pending || !state.current.reference,
              ),
            ]
          : []),
        ...(state.historyFailed
          ? [ui.button('history-retry', text.sourceRetry, !!state.pending)]
          : []),
        ui.box(
          'apps-directory',
          [
            presentation.reading(
              'apps-list',
              {
                content: ui.box(
                  'apps-list-content',
                  [
                    ...rows,
                    ...(!rows.length &&
                    !state.pending &&
                    ['list', 'choose', 'bindings'].includes(state.route)
                      ? [
                          ui.text(
                            'apps-empty',
                            state.route === 'list' ? text.appsEmpty : text.appsNoCandidates,
                            14,
                            props.theme.muted,
                          ),
                        ]
                      : []),
                  ],
                  {
                    scroll: 'manual',
                    gap: 1,
                    terminal: { grow: 1, basis: 0 },
                    style: { width: fill, height: fill, minHeight: 0 },
                  },
                ),
                command: null,
              },
              { input: 'list-input' },
            ),
          ],
          {
            visible: ['list', 'choose', 'bindings'].includes(state.route),
            terminal: { grow: 1, basis: 0 },
            style: { width: fill, height: fill, minHeight: 0 },
          },
        ),
        ui.box(
          'apps-configuration',
          [
            ui.text(
              'apps-setup-help',
              draft?.candidate.kind === 'bound' ? text.appsExistingHelp : text.appsSetupHelp,
              13,
              props.theme.muted,
            ),
            ui.text('apps-name-label', text.appsName, 13, props.theme.muted),
            presentation.slot(
              'apps-name',
              'input',
              {
                appearance: props.theme.input,
                accessibilityLabel: text.appsName,
                placeholder: text.appsName,
                autoFocus: true,
                readOnly: false,
                tabIndex: 0,
                disabled: suspended || !!state.fields.name.command || !state.fields.name.ready,
                command: state.fields.name.command,
              } satisfies InputProps,
              { changed: 'name-changed', submitted: 'name-submit' },
            ),
            ...(draft?.page?.fields ?? []).map((f, index) => {
              const options = settingOptions(f),
                group = f.kind === 'object' || f.kind === 'array';
              const current = draft ? value(draft, f) : null;
              return ui.box(
                'app-setting-row-' + index,
                [
                  options.length && ui.choice
                    ? ui.choice(
                        'setting-choice-' + index,
                        f.label,
                        options.map((v, at) => ({ key: String(at), label: display(v, text) })),
                        options.some((v) => sameValue(v, current))
                          ? String(options.findIndex((v) => sameValue(v, current)))
                          : null,
                        suspended || !f.editable,
                      )
                    : ui.button(
                        'setting-' + index,
                        f.label + '  ' + (group ? '›' : display(current, text)),
                        suspended || !f.editable || (f.kind === 'array' && !f.present),
                        false,
                        { align: 'start' },
                      ),
                  ...(f.description
                    ? [
                        ui.text(
                          'app-setting-description-' + index,
                          f.description,
                          12,
                          props.theme.muted,
                        ),
                      ]
                    : []),
                ],
                { gap: 0 },
              );
            }),
            ...(draft?.page && !draft.page.fields.length
              ? [ui.text('apps-settings-empty', text.appsNoSettings, 13, props.theme.muted)]
              : []),
            ...(draft?.path.length
              ? [ui.button('settings-up', text.directoryParent, suspended)]
              : []),
            ...(draft?.previous.length
              ? [ui.button('settings-previous', text.skillsPrevious, suspended)]
              : []),
            ...(draft?.page?.next
              ? [ui.button('settings-next', text.deploymentMore, suspended)]
              : []),
            ...(draft?.candidate.kind === 'package'
              ? [
                  ui.text('apps-bindings-title', text.appsBindings, 15, props.theme.text, 500),
                  ui.text('apps-bindings-help', text.appsBindingHelp, 13, props.theme.muted),
                  ...(draft.page?.imports ?? []).map((binding, index) =>
                    ui.button(
                      'choose-binding-' + index,
                      binding.name +
                        ' · ' +
                        binding.contract +
                        ' · ' +
                        (draft.bindings[binding.name]?.length ?? 0),
                      suspended,
                      false,
                      { align: 'start' },
                    ),
                  ),
                ]
              : []),
          ],
          {
            visible: state.route === 'configure',
            scroll: 'manual',
            gap: 1,
            terminal: { grow: 1, basis: 0 },
            style: { width: fill, height: fill, minHeight: 0 },
          },
        ),
        ui.box(
          'apps-file-region',
          [
            ui.text('apps-file-help', text.deploymentInstallHelp, 13, props.theme.muted),
            presentation.slot(
              'apps-file',
              'file',
              {
                appearance: props.theme.input,
                label: text.deploymentChooseFile,
                pathLabel: text.deploymentFilePath,
                browseLabel: text.fileBrowse,
                parentLabel: text.directoryParent,
                emptyLabel: text.fileListEmpty,
                truncatedLabel: text.fileListTruncated,
                placeholder: text.deploymentFilePlaceholder,
                disabled: suspended,
                tabIndex: 0,
              } satisfies FilePickerProps,
              { changed: 'file-changed' },
            ),
            ...(state.file?.file ? [ui.text('apps-file-name', state.file.file.name, 14)] : []),
            ...(state.file?.phase === 'failed'
              ? [ui.text('apps-file-error', text.deploymentFileFailed, 13, props.theme.warning)]
              : []),
          ],
          {
            visible: state.route === 'file',
            gap: 1,
            terminal: { grow: 1 },
            style: { width: fill, height: fill },
          },
        ),
        ...configuration.entries.flatMap((app, member) =>
          state.opened.includes(app.id)
            ? [
                ui.box(
                  'application-' + app.id,
                  [
                    presentation.slot(
                      'app-' + app.id,
                      'pages',
                      {
                        theme: props.theme,
                        locale: props.locale,
                        open: props.open && state.route === 'app' && state.selected === app.id,
                      },
                      { closed: 'app-closed-' + app.id },
                      member,
                    ),
                  ],
                  {
                    visible: state.route === 'app' && state.selected === app.id,
                    terminal: { grow: 1, basis: 0 },
                    style: { width: fill, height: fill, minHeight: 0 },
                  },
                ),
              ]
            : [],
        ),
        ui.box(
          'apps-actions',
          state.route === 'list'
            ? [
                ui.button('add', text.appsAdd, disabled),
                ui.button('file', text.appsInstall, disabled),
                ui.button('refresh', text.appsReload, disabled),
                ui.button('manage', text.appsManage, disabled),
              ]
            : state.route === 'choose'
              ? [
                  ui.button('file', text.appsInstall, disabled),
                  ...(state.catalog?.next
                    ? [ui.button('catalog-more', text.deploymentMore, disabled)]
                    : []),
                ]
              : state.route === 'configure'
                ? [
                    ui.button(
                      'review',
                      text.appsReview,
                      suspended ||
                        !draft ||
                        !complete(draft) ||
                        !state.fields.name.ready ||
                        !!state.fields.name.command,
                    ),
                  ]
                : state.route === 'file'
                  ? [
                      ui.button(
                        'install',
                        text.appsInstall,
                        disabled ||
                          !state.catalog ||
                          state.file?.phase !== 'ready' ||
                          !state.file.file,
                      ),
                    ]
                  : state.route === 'bindings'
                    ? [
                        ui.button('bindings-done', text.appsContinue, disabled),
                        ui.button('bindings-clear', text.appsBindingClear, disabled),
                        ...(draft?.choices?.next
                          ? [ui.button('bindings-more', text.deploymentMore, disabled)]
                          : []),
                      ]
                    : [ui.button('close-current', text.appsClose, disabled)],
          { row: true, terminal: { wrap: true }, style: { wrap: true }, gap: 1 },
        ),
      ],
      {
        visible: props.open,
        gap: 1,
        autoFocus: true,
        terminal: { grow: 1 },
        style: { width: fill, height: fill },
        shortcuts: [{ key: 'escape', modifiers: [], input: 'back', enabled: true }],
      },
    );
    const setting = draft?.setting;
    const editor = ui.box(
      'apps-setting-dialog',
      [
        presentation.slot(
          'apps-setting',
          'editor',
          {
            appearance: props.theme.textarea,
            accessibilityLabel: setting?.descriptor.label ?? text.instanceConfiguration,
            placeholder: '',
            readOnly: false,
            tabIndex: 0,
            autoFocus: true,
            disabled: disabled || !!state.fields.setting.command || !state.fields.setting.ready,
            softWrap: true,
            terminalRows: { min: 3, max: 6 },
            command: state.fields.setting.command,
          } satisfies TextareaProps,
          { changed: 'setting-changed' },
        ),
        ui.box(
          'apps-setting-actions',
          [
            ui.button('setting-cancel', text.cancel, disabled),
            ui.button(
              'setting-apply',
              text.settingApply,
              disabled || !state.fields.setting.ready || !!state.fields.setting.command,
            ),
          ],
          { row: true },
        ),
      ],
      { gap: 1, terminal: { grow: 1 }, style: { width: fill, height: fill } },
    );
    const withEditor = ui.dialog(
      'apps-setting-overlay',
      setting?.descriptor.label ?? text.instanceConfiguration,
      page,
      editor,
      !!setting,
      {},
      { width: 64, height: 12 },
    );
    const confirmation = ui.box(
      'apps-confirmation',
      [
        ui.text(
          'apps-confirm-help',
          state.confirm?.kind === 'install'
            ? text.deploymentInstallHelp
            : state.confirm?.kind === 'edit' && state.confirm.outcome === 'remove'
              ? text.appsRemoveHelp
              : text.appsConfirmHelp,
          14,
        ),
        ...(state.confirm?.kind === 'install' && state.confirm.file.file
          ? [ui.text('apps-confirm-file', state.confirm.file.file.name, 15)]
          : []),
        ...(draft
          ? [
              ui.text('apps-confirm-name', draft.label, 15, props.theme.text, 600),
              ...Object.entries(draft.bindings).map(([name, targets], index) =>
                ui.text(
                  'apps-confirm-binding-' + index,
                  name +
                    ': ' +
                    targets
                      .map((t) =>
                        t.kind === 'instance'
                          ? t.path.join(' / ') + ' / ' + t.export
                          : t.kind === 'remote'
                            ? t.deployment + ' / ' + t.export
                            : t.name,
                      )
                      .join(', '),
                  13,
                  props.theme.muted,
                ),
              ),
            ]
          : []),
        ...(state.notice
          ? [ui.text('apps-confirm-notice', text[state.notice], 13, props.theme.muted)]
          : []),
        ...(state.diagnostic
          ? [ui.text('apps-confirm-diagnostic', state.diagnostic, 13, props.theme.warning)]
          : []),
        ...(state.current
          ? [
              ui.button(
                'confirm-receipt',
                text.executionReceipt,
                !!state.pending || !state.current.reference,
              ),
            ]
          : []),
        ui.box(
          'apps-confirm-actions',
          [
            ui.button('confirm-cancel', text.cancel, !!state.current || !!state.pending),
            ui.button(
              'confirm',
              state.confirm?.kind === 'install'
                ? text.appsInstall
                : state.confirm?.kind === 'edit' && state.confirm.outcome === 'remove'
                  ? text.appsRemove
                  : text.appsConfirm,
              disabled,
              true,
            ),
          ],
          { row: true, terminal: { wrap: true }, style: { wrap: true } },
        ),
      ],
      { gap: 1 },
    );
    const view = ui.dialog(
      'apps-confirm-overlay',
      text.confirmDefault,
      withEditor,
      confirmation,
      !!state.confirm,
      {},
      { width: 72 },
    );
    const result = output(
      state,
      ui.box('apps-root', [view], {
        visible: props.open,
        terminal: { grow: 1 },
        style: { width: fill, height: fill },
      }),
      events,
    );
    result.actions = actions;
    return result;
  }
  function choose(state: State, candidate: BindingCandidate, actions: Action[]) {
    state.notice = null;
    state.diagnostic = null;
    const label = candidate.label;
    state.draft = {
      candidate,
      id: 'app-' + state.catalog!.source.revision + '-' + ++state.serial,
      label,
      changes: [],
      bindings: {},
      page: null,
      path: [],
      after: null,
      previous: [],
      setting: null,
      choosing: null,
      choices: null,
    };
    state.route = 'configure';
    reset(state, 'name', label);
    if (candidate.kind === 'package') settings(state, actions);
  }
  return withPageShell<State, AppsProps, N>(
    {
      mount(props) {
        const state: State = {
          revision: 0,
          serial: 0,
          open: props.open,
          route: 'list',
          catalog: null,
          selected: null,
          opened: [],
          draft: null,
          fields: { name: field(), setting: field() },
          file: null,
          pending: null,
          original: null,
          restoring: false,
          historyFailed: false,
          current: null,
          confirm: null,
          notice: null,
          diagnostic: null,
        };
        const actions: Action[] = [];
        history(state, actions);
        return frame(state, props, actions);
      },
      update(state, props) {
        state.revision++;
        const actions: Action[] = [];
        if (
          props.open &&
          !state.open &&
          !state.pending &&
          !state.current &&
          !state.restoring &&
          !state.draft &&
          !state.confirm
        )
          catalog(state, actions, configuration);
        state.open = props.open;
        return frame(state, props, actions);
      },
      event(state, input, props) {
        state.revision++;
        const actions: Action[] = [],
          events: Emission[] = [];
        if (input.kind === 'component' && input.child === 'apps-list' && input.name === 'input')
          input = input.event as ComponentInput;
        if (input.kind === 'component') {
          const name =
            input.child === 'apps-name'
              ? 'name'
              : input.child === 'apps-setting'
                ? 'setting'
                : null;
          if (name && input.name === 'changed') {
            const f = state.fields[name];
            f.observation = input.event as EditorObservation;
            const receipt = f.observation.receipt;
            if (
              f.command &&
              receipt?.owner === f.command.owner &&
              receipt.sequence === f.command.sequence
            ) {
              f.ready = receipt.outcome.status === 'applied';
              f.command = null;
              f.reset = null;
              if (!f.ready) state.notice = 'appsUnavailable';
            } else initialize(f);
          } else if (name === 'name' && input.name === 'submitted')
            input = { kind: 'raw', key: 'review', event: {} } as ComponentInput;
          else if (input.child === 'apps-file' && input.name === 'changed')
            state.file = input.event as FileObservation;
          else if (input.name === 'closed' && input.child.startsWith('app-')) {
            const id = input.child.slice(4);
            state.opened = state.opened.filter((o) => o !== id);
            if (state.selected === id) {
              state.selected = null;
              state.route = 'list';
            }
          }
        }
        if (input.kind === 'action' && input.key === state.pending?.key) {
          const r = state.pending.request,
            result = input.event as ActionOutcome;
          state.pending = null;
          if (r.kind === 'history') {
            if (result.status !== 'succeeded') {
              state.restoring = false;
              state.historyFailed = true;
              state.notice = 'historyUnavailable';
            } else {
              const items = result.value as CallerIntent[];
              for (const item of items) {
                const recovery = item.recovery as { kind?: string; input?: Mutation } | null;
                if (
                  item.reference.contract.name === 'maka.application.DeploymentManager' &&
                  ['install', 'editComposition'].includes(item.reference.method) &&
                  recovery?.kind === 'application-apps/v1' &&
                  recovery.input
                ) {
                  state.original = item.reference;
                  state.current = { input: recovery.input, reference: item.reference };
                }
              }
              const last = items.at(-1);
              if (last && last.cursor > r.after) history(state, actions, last.cursor);
              else if (last) {
                state.restoring = false;
                state.historyFailed = true;
                state.notice = 'historyUnavailable';
              } else {
                state.restoring = false;
                if (state.original)
                  request(state, actions, { kind: 'receipt' }, 'receipts', 'read', state.original);
                else catalog(state, actions, configuration);
              }
            }
          } else if (r.kind === 'mutation') {
            if (result.status === 'operation') settle(state, result.reply, actions, configuration);
            else if (result.status === 'not_accepted') {
              state.current = null;
              state.notice = 'appsUnavailable';
            } else state.notice = 'appsUnknown';
          } else if (r.kind === 'receipt') {
            if (result.status === 'succeeded')
              settle(state, result.value as OperationReply, actions, configuration);
            else state.notice = 'appsUnknown';
          } else if (r.kind === 'catalog' || r.kind === 'bindings') {
            if (result.status !== 'succeeded') state.notice = 'appsUnavailable';
            else {
              const page = result.value as BindingCandidatesPage;
              if (r.input.source && !sameValue(page.source, r.input.source))
                state.notice = 'appsChanged';
              else if (r.kind === 'catalog') state.catalog = page;
              else if (state.draft) state.draft.choices = page;
            }
          } else if (r.kind === 'settings') {
            if (result.status !== 'succeeded') state.notice = 'appsUnavailable';
            else if (
              state.draft &&
              sameValue(state.draft.path, r.path) &&
              state.draft.after === r.after
            ) {
              const page = result.value as PackageSettingsPage;
              if (
                !sameValue(page.source, state.catalog?.source) ||
                state.draft.candidate.kind !== 'package' ||
                !sameValue(page.package, state.draft.candidate.package)
              )
                state.notice = 'appsChanged';
              else state.draft.page = page;
            }
          } else if (r.kind === 'field') {
            const chunk = result.status === 'succeeded' ? (result.value as TextChunk) : null;
            if (
              !chunk ||
              !sameValue(chunk.metadata, r.observation.snapshot.text) ||
              new TextEncoder().encode(chunk.text).length !== r.observation.snapshot.text.bytes
            )
              state.notice = 'appsUnavailable';
            else if (state.draft) {
              if (r.name === 'name') {
                const label = chunk.text.trim();
                if (
                  !label ||
                  [...label].length > 256 ||
                  /[\u0000-\u001f\u007f-\u009f]/u.test(label)
                )
                  state.notice = 'appsInvalid';
                else {
                  state.draft.label = label;
                  edit(state, configuration, label);
                }
              } else if (state.draft.setting) {
                const { descriptor, path } = state.draft.setting;
                const value = descriptor.kind === 'number' ? chunk.text.trim() : chunk.text;
                if (
                  [...value].length > 4096 ||
                  (descriptor.kind === 'number' &&
                    (value.length > 128 ||
                      !/^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$/.test(value)))
                )
                  state.notice = 'appsInvalid';
                else {
                  change(state.draft, path, {
                    kind: descriptor.kind === 'number' ? 'number' : 'text',
                    text: value,
                  });
                  state.draft.setting = null;
                  settings(state, actions);
                }
              }
            }
          }
        } else if (input.kind === 'raw' && props.open) {
          const key = input.key,
            d = state.draft;
          if (key === 'back' && !state.confirm && !d?.setting) {
            if (state.current || state.restoring || state.route === 'list')
              events.push({ name: 'closed', value: {} });
            else if (!state.pending) {
              if (state.route === 'bindings') state.route = 'configure';
              else {
                state.route = 'list';
                state.draft = null;
                state.selected = null;
              }
            }
          } else if (
            ['receipt', 'confirm-receipt'].includes(key) &&
            state.current?.reference &&
            !state.pending
          )
            request(
              state,
              actions,
              { kind: 'receipt' },
              'receipts',
              'read',
              state.current.reference,
            );
          else if (key === 'history-retry' && state.historyFailed && !state.pending)
            history(state, actions);
          else if (key === 'confirm-cancel' && !state.current && !state.pending)
            state.confirm = null;
          else if (!locked(state, props) && (!state.confirm || key === 'confirm')) {
            if (key === 'confirm') mutate(state, actions);
            else if (key === 'setting-cancel' && d) d.setting = null;
            else if (key === 'setting-apply' && d?.setting) capture(state, actions, 'setting');
            else if (!d?.setting) {
              if (key === 'refresh') {
                state.notice = null;
                state.diagnostic = null;
                catalog(state, actions, configuration);
              } else if (key === 'manage') events.push({ name: 'manage', value: {} });
              else if (key === 'add') {
                state.route = 'choose';
                state.notice = null;
                catalog(state, actions, configuration);
              } else if (key === 'file') {
                state.route = 'file';
                state.notice = null;
                if (!state.catalog) catalog(state, actions, configuration);
              } else if (
                key === 'install' &&
                state.catalog &&
                state.file?.phase === 'ready' &&
                state.file.file
              )
                state.confirm = { kind: 'install', source: state.catalog.source, file: state.file };
              else if (key.startsWith('open-')) {
                const id = key.slice(5);
                if (configuration.entries.some((e) => e.id === id)) {
                  state.selected = id;
                  state.route = 'app';
                  if (!state.opened.includes(id)) state.opened.push(id);
                }
              } else if (key.startsWith('close-')) {
                const id = key === 'close-current' ? state.selected : key.slice(6);
                state.opened = state.opened.filter((o) => o !== id);
                if (state.selected === id) {
                  state.selected = null;
                  state.route = 'list';
                }
              } else if (
                key.startsWith('remove-') &&
                state.catalog &&
                sameValue(entries(state), configuration.entries)
              ) {
                const index = Number(key.slice(7));
                if (configuration.entries[index])
                  state.confirm = {
                    kind: 'edit',
                    outcome: 'remove',
                    input: {
                      expectedRevision: state.catalog.source.revision,
                      edits: [
                        {
                          kind: 'update',
                          name: configuration.root,
                          configuration: {
                            entries: entries(state).filter((_, at) => at !== index),
                          },
                          bindings: {
                            pages: state.catalog.current.filter((_, at) => at !== index),
                          },
                        },
                      ],
                    },
                  };
              } else if (key.startsWith('candidate-') && state.route === 'choose') {
                const candidate = state.catalog?.candidates[Number(key.slice(10))];
                if (candidate) choose(state, candidate, actions);
              } else if (key === 'catalog-more' && state.catalog?.next)
                catalog(state, actions, configuration, state.catalog.source, state.catalog.next);
              else if (key === 'review' && state.route === 'configure')
                capture(state, actions, 'name');
              else if (key.startsWith('setting-choice-') && d?.page) {
                const f = d.page.fields[Number(key.slice(15))];
                const option =
                  f && settingOptions(f)[Number((input.event as { key?: string }).key)];
                if (option) {
                  change(d, [...d.path, f.key], option);
                  settings(state, actions);
                }
              } else if (key.startsWith('setting-') && d?.page) {
                const f = d.page.fields[Number(key.slice(8))];
                if (f?.editable) {
                  if (f.kind === 'object' || f.kind === 'array') {
                    d.path = [...d.path, f.key];
                    d.after = null;
                    d.previous = [];
                    settings(state, actions);
                  } else if (settingOptions(f).length) {
                    const options = settingOptions(f),
                      index = options.findIndex((v) => sameValue(v, value(d, f)));
                    change(d, [...d.path, f.key], options[(index + 1) % options.length]);
                    settings(state, actions);
                  } else {
                    d.setting = { descriptor: f, path: [...d.path, f.key] };
                    const v = value(d, f);
                    reset(
                      state,
                      'setting',
                      v?.kind === 'text' || v?.kind === 'number' ? v.text : '',
                    );
                  }
                }
              } else if (key === 'settings-up' && d?.path.length) {
                d.path = d.path.slice(0, -1);
                d.after = null;
                d.previous = [];
                settings(state, actions);
              } else if (key === 'settings-next' && d?.page?.next) {
                d.previous.push(d.after);
                d.after = d.page.next;
                settings(state, actions);
              } else if (key === 'settings-previous' && d?.previous.length) {
                d.after = d.previous.pop()!;
                settings(state, actions);
              } else if (key.startsWith('choose-binding-') && d?.page) {
                const i = d.page.imports[Number(key.slice(15))];
                if (i) {
                  d.choosing = i;
                  d.choices = null;
                  state.route = 'bindings';
                  bindings(state, actions);
                }
              } else if (key.startsWith('binding-') && d?.choosing && d.choices) {
                const candidate = d.choices.candidates[Number(key.slice(8))];
                if (candidate?.kind === 'bound') {
                  const i = d.choosing,
                    current = d.bindings[i.name] ?? [],
                    has = current.some((t) => sameValue(t, candidate.target));
                  d.bindings[i.name] = has
                    ? current.filter((t) => !sameValue(t, candidate.target))
                    : i.cardinality.kind === 'many'
                      ? [...current, candidate.target]
                      : [candidate.target];
                }
              } else if (key === 'bindings-clear' && d?.choosing) d.bindings[d.choosing.name] = [];
              else if (key === 'bindings-more' && d?.choices?.next)
                bindings(state, actions, d.choices.next);
              else if (key === 'bindings-done') state.route = 'configure';
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
  return { Apps: { kind: 'component', contract: appsContract, component: ['component'] } };
}
