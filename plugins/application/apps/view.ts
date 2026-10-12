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

import type { AppsConfiguration } from '../../shared/contracts.js';

import type {
  Action,
  Emission,
  FilePickerProps,
  InputProps,
  SettingValue,
  TextareaProps,
} from '../../../packages/plugin-sdk/src/index.js';
import { type AppsProps } from '../../shared/contracts.js';
import { settingOptions } from '../../shared/instance-settings.js';
import { copy, type Copy } from '../../shared/locale.js';
import { pageHeader } from '../../shared/page-shell.js';
import { sameValue } from '../../shared/value.js';
import { fill, output, type Node, type Presentation } from '../../shared/view.js';
import { complete, entries, locked, value } from './operations.js';
import { type State } from './state.js';

export function display(value: SettingValue | null, text: Copy) {
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

export function frame<N extends Node>(
  presentation: Presentation<N>,
  configuration: AppsConfiguration,
  state: State,
  props: AppsProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
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
      ...(state.notice ? [ui.text('apps-notice', text[state.notice], 13, props.theme.muted)] : []),
      ...(state.diagnostic
        ? [ui.text('apps-diagnostic', state.diagnostic, 13, props.theme.warning)]
        : []),
      ...(state.current
        ? [ui.button('receipt', text.executionReceipt, !!state.pending || !state.current.reference)]
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
