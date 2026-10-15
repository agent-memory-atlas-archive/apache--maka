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

import type { Action, Emission, InputProps } from '../../../packages/plugin-sdk/src/index.js';
import { bookmarkReady, bookmarkView } from '../../shared/bookmark.js';
import { type DeploymentProps } from '../../shared/contracts.js';
import { documentView } from '../../shared/deployment-document.js';
import { importView } from '../../shared/deployment-import.js';
import { instanceSettingsView } from '../../shared/instance-settings.js';
import { copy } from './locale.js';
import { pluginDescription } from '../../shared/plugin-description.js';
import { pageHeader } from '../../shared/page-shell.js';
import { visibleRows } from '../../shared/scroll.js';
import { sameValue } from '../../shared/value.js';
import { fill, output, type Node, type Presentation } from '../../shared/view.js';
import { filteredInstances, page, request } from './operations.js';
import { settingsBookmark, type State } from './state.js';

export function frame<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  props: DeploymentProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const target = presentation.target;
  if (state.settings?.changes.length && state.notice === 'deploymentSaved') state.notice = null;
  if (state.restoreSettings && !state.pending) {
    state.restoreSettings = false;
    state.restoreOriginal = null;
    request(state, actions, { kind: 'restoreSettings', after: 0 }, 'history', 'page', {
      after: 0,
      limit: 16,
    });
  }
  const navigationReady =
    !state.restoreFailed &&
    (target !== 'tui' ||
      (props.navigationReady !== false && bookmarkReady(state.bookmark, settingsBookmark(state))));
  const ui = presentation.paint(props.theme),
    text = copy(props.locale);
  const loading = state.document?.phase === 'loading';
  const busy =
    !!state.pending ||
    !!state.current ||
    loading ||
    state.restoreFailed ||
    (target === 'tui' && !state.bookmark.restored);
  const basis = state.queryObservation?.snapshot.text;
  if (!state.pending && basis && !sameValue(basis, state.queryBasis) && basis.bytes <= 1024) {
    if (!basis.bytes) {
      state.query = '';
      state.queryBasis = basis;
      state.listWindow = 0;
    } else
      request(
        state,
        actions,
        { kind: 'filter', basis },
        'reader',
        'read',
        { start: 0, end: basis.bytes },
        {
          text: {
            child: 'deployment-query',
            method: 'snapshot',
            input: { kind: 'at_revision', owner: basis.owner, revision: basis.revision },
          },
        },
      );
  }
  if (state.query && !state.pending && !state.notice && state.page?.next)
    page(
      state,
      actions,
      { kind: 'continue', source: state.page.source, after: state.page.next },
      'next',
    );
  const entries = filteredInstances(state, props.locale);
  const current = state.confirmation;
  const history = state.historyOpen;
  const confirmationBody = current
    ? [
        ui.text(
          'deployment-question',
          current.enabled ? text.deploymentConfirmEnable : text.deploymentConfirmDisable,
          16,
          props.theme.text,
          550,
        ),
        ui.text('deployment-confirm-path', current.instance.path.join(' / '), 14),
        ui.text('deployment-confirm-package', current.instance.package.id, 13, props.theme.muted),
        ui.text('deployment-confirm-help', text.deploymentChangeHelp, 13, props.theme.muted),
      ]
    : [];
  const historyBody = [
    ...(target === 'tui' && state.notice
      ? [ui.text('deployment-history-notice', text[state.notice], 13, props.theme.muted)]
      : []),
    ...(target === 'tui' && state.pending
      ? [ui.text('deployment-history-loading', text.connecting, 13, props.theme.muted)]
      : []),
    ui.text('deployment-history-help', text.deploymentHistoryHelp, 13, props.theme.muted),
    ...visibleRows(
      state,
      target === 'tui' ? 'deployment-history-reading' : 'deployment-list',
      state.earlier,
    ).map(({ entry, at }) =>
      ui.box(
        'deployment-earlier-' + at,
        [
          ui.text(
            'deployment-earlier-status-' + at,
            entry.reply?.receipt.kind === 'recorded' &&
              entry.reply.receipt.outcome.kind === 'returned'
              ? text.deploymentRecorded
              : text.deploymentUnknown,
            13,
          ),
          ui.button('deployment-earlier-check-' + at, text.check, !!state.pending),
        ],
        { gap: 0 },
      ),
    ),
  ];
  const rows = entries.slice(state.listWindow, state.listWindow + 64).map(({ entry, at }) => {
    const info = pluginDescription(
      entry.presentation,
      props.locale,
      entry.path.at(-1) ?? entry.package.id,
    );
    const status = entry.parentDisabled
      ? text.deploymentParentDisabled
      : entry.disabled
        ? text.connectionDisabled
        : text.connectionEnabled;
    const configure = ui.button(
      'deployment-configure-' + at,
      text.configureInstance,
      busy || entry.structural || state.conflict,
    );
    const toggle = ui.button(
      'deployment-toggle-' + at,
      entry.disabled ? text.deploymentEnable : text.deploymentDisable,
      busy || state.conflict || (entry.disabled && entry.parentDisabled),
    );
    if (ui.item) {
      const selected = sameValue(state.selected, entry.instance);
      return ui.contextMenu!(
        'deployment-instance-' + at,
        text.deploymentTitle,
        ui.item(
          'deployment-select-' + at,
          ui.box(
            'deployment-summary-' + at,
            [
              ui.text(
                'deployment-path-' + at,
                `${selected ? (props.theme.ascii ? '>' : '›') : ' '} ${info.name}`,
                14,
                props.theme.text,
                400,
                { truncate: true },
              ),
              ...(info.description
                ? [
                    ui.text(
                      'deployment-description-' + at,
                      '  ' + info.description,
                      13,
                      props.theme.muted,
                    ),
                  ]
                : []),
              ui.text(
                'deployment-detail-' + at,
                `  ${status} · ${entry.path.join(' / ')}`,
                13,
                props.theme.muted,
                400,
                { truncate: true },
              ),
            ],
            { gap: 0 },
          ),
          selected,
          busy,
          { followFocus: true, submit: true },
        ),
        [configure, toggle],
      );
    }
    return ui.box(
      'deployment-instance-' + at,
      [
        ui.box(
          'deployment-identity-' + at,
          [
            ui.text('deployment-path-' + at, info.name, 15, props.theme.text, 550),
            ...(info.description
              ? [ui.text('deployment-description-' + at, info.description, 13, props.theme.muted)]
              : []),
            ui.text('deployment-package-' + at, entry.package.id, 13, props.theme.muted),
          ],
          {
            gap: target === 'desktop' ? 4 : 0,
            surface: target === 'desktop',
            style: { grow: 1, minWidth: 0 },
          },
        ),
        ui.box(
          'deployment-row-' + at,
          [ui.text('deployment-state-' + at, status, 13, props.theme.muted), configure, toggle],
          {
            row: target === 'desktop',
            gap: 0,
            surface: target === 'desktop',
            style: { wrap: true, align: 'center', shrink: 0 },
          },
        ),
      ],
      {
        row: target === 'desktop',
        gap: target === 'desktop' ? 16 : 1,
        padding: target === 'desktop' ? 12 : 1,
        surface: target === 'desktop',
        style: { width: fill, wrap: true, shrink: 0, radius: 10, align: 'center' },
      },
    );
  });
  const body = target === 'tui' ? rows : current ? confirmationBody : history ? historyBody : rows;
  const header = ui.box(
    'deployment-header',
    [
      ...pageHeader(
        presentation,
        state,
        props,
        events,
        state.settings?.open
          ? text.instanceConfiguration
          : state.importer.open
            ? text.deploymentInstall
            : state.documentOpen
              ? text.deploymentEdit
              : text.deploymentTitle,
        'deployment-back',
        !!state.settings?.open && busy,
      ),
      ...(target !== 'tui' && !current && !history
        ? [ui.text('deployment-help', text.deploymentHelp, 13, props.theme.muted)]
        : []),
      ...(target === 'tui'
        ? [
            ui.box(
              'deployment-toolbar',
              [
                ui.button('deployment-import', text.deploymentInstall, busy || !state.page, true),
                ui.button('deployment-advanced', text.deploymentEdit, busy || !state.page),
                ui.button('deployment-history', text.deploymentHistory, !!state.pending || loading),
                ui.button(
                  'deployment-refresh',
                  props.theme.ascii ? 'r' : '↻',
                  busy,
                  false,
                  { background: 0 },
                  undefined,
                  text.deploymentRefresh,
                ),
              ],
              { row: true, gap: 1, terminal: { wrap: true } },
            ),
          ]
        : []),
      presentation.slot(
        'deployment-query',
        'input',
        {
          appearance: props.theme.input,
          accessibilityLabel: text.pluginSearch,
          placeholder: text.pluginSearch,
          tabIndex: 0,
          autoFocus: false,
          disabled: !!state.current,
          readOnly: false,
        } satisfies InputProps,
        { changed: 'deployment-query-changed' },
      ),
      ...(state.notice
        ? [ui.text('deployment-notice', text[state.notice], 13, props.theme.muted)]
        : []),
      ...(state.pending
        ? [ui.text('deployment-loading', text.connecting, 13, props.theme.muted)]
        : []),
    ],
    { gap: target === 'desktop' ? 8 : 1, style: { shrink: 0 } },
  );
  const footer = ui.box(
    'deployment-controls',
    target === 'desktop' && current
      ? [
          ui.button('deployment-confirm', text.confirmDefault, busy),
          ui.button('deployment-cancel', text.cancel, busy),
        ]
      : target === 'desktop' && history
        ? [
            ui.button('deployment-history-back', text.back, false, false, {}),
            ui.button(
              'deployment-history-refresh',
              text.refreshDefaultOperations,
              !!state.pending,
              false,
              {},
              undefined,
              text.refreshDefaultOperations,
            ),
          ]
        : target === 'tui'
          ? [
              ...(state.current?.reference
                ? [ui.button('deployment-check', text.check, !!state.pending)]
                : []),
            ]
          : [
              ui.button(
                'deployment-refresh',
                text.deploymentRefresh,
                busy,
                false,
                {},
                undefined,
                text.deploymentRefresh,
              ),
              ...(state.current?.reference
                ? [ui.button('deployment-check', text.check, !!state.pending)]
                : []),
              ui.button('deployment-history', text.deploymentHistory, !!state.pending || loading),
              ui.button('deployment-advanced', text.deploymentEdit, busy || !state.page),
              ui.button('deployment-import', text.deploymentInstall, busy || !state.page),
            ],
    {
      row: true,
      gap: target === 'desktop' ? 8 : 1,
      terminal: { wrap: true },
      style: { wrap: true, shrink: 0 },
    },
  );
  const out = output(
    state,
    ui.box(
      'deployment-panel',
      [
        header,
        presentation.reading(
          'deployment-list',
          {
            command: state.focus,
            content: ui.box(
              'deployment-body',
              body.length
                ? body
                : state.pending
                  ? []
                  : [ui.text('deployment-empty', text.deploymentEmpty, 14, props.theme.muted)],
              {
                scroll: 'anchor',
                focusGroup: target === 'tui',
                terminal: { grow: 1, basis: 0 },
                style: { width: fill, height: fill, minHeight: 0 },
                gap: target === 'desktop' ? 16 : 1,
              },
            ),
          },
          { input: 'deployment-input', observed: 'deployment-observed' },
        ),
        footer,
      ],
      {
        style: { width: fill, height: fill },
        visible: !state.documentOpen && !state.importer.open && !state.settings?.open,
        terminal: { grow: 1 },
        autoFocus: true,
        shortcuts:
          target === 'tui'
            ? [{ key: 'escape', modifiers: [], input: 'deployment-back', enabled: true }]
            : [],
        gap: target === 'desktop' ? 16 : 1,
      },
    ),
    events,
  );
  if (target === 'tui') {
    const modalBody = current ? confirmationBody : historyBody;
    out.frame.view = ui.dialog(
      'deployment-dialog',
      current ? text.deploymentTitle : text.deploymentHistory,
      out.frame.view,
      ui.box(
        'deployment-dialog-content',
        [
          ...(current || history
            ? [
                presentation.reading(
                  'deployment-history-reading',
                  {
                    content: ui.box('deployment-dialog-body', modalBody, {
                      scroll: 'anchor',
                      terminal: { grow: 1 },
                      gap: 1,
                    }),
                  },
                  { input: 'history-input', observed: 'history-observed' },
                ),
              ]
            : []),
          ui.box(
            'deployment-dialog-actions',
            current
              ? [
                  ui.button('deployment-cancel', text.cancel, busy),
                  ui.button('deployment-confirm', text.confirmDefault, busy, true),
                ]
              : [
                  ui.button('deployment-history-back', text.back),
                  ui.button(
                    'deployment-history-refresh',
                    props.theme.ascii ? 'r' : '↻',
                    !!state.pending,
                    false,
                    { background: 0 },
                    undefined,
                    text.refreshDefaultOperations,
                  ),
                ],
            { row: true, gap: 1, terminal: { wrap: true } },
          ),
        ],
        {
          terminal: { grow: 1 },
        },
      ),
      !!current || history,
      undefined,
      { width: 64, height: history ? 18 : 14 },
    );
    out.frame.view = ui.box('deployment-main', [out.frame.view], {
      visible: !state.documentOpen && !state.importer.open && !state.settings?.open,
      gap: 0,
      terminal: { grow: 1 },
    });
  }
  out.frame.view = ui.box(
    'deployment-pages',
    [
      out.frame.view,
      importView(
        presentation,
        props,
        state.importer,
        busy,
        state.conflict,
        state.notice ? text[state.notice] : null,
        !!state.current?.reference,
        !!state.pending,
      ),
      ...(state.settings
        ? [
            instanceSettingsView(
              presentation,
              props,
              state.settings,
              busy,
              state.notice ? text[state.notice] : null,
              state.diagnostic,
              !!state.current?.reference,
              navigationReady,
            ),
          ]
        : []),
      ...(state.document
        ? [
            documentView(
              presentation,
              props,
              state.document,
              state.documentOpen,
              busy,
              state.notice ? text[state.notice] : null,
              state.diagnostic,
              !!state.current?.reference,
              !!state.pending,
            ),
          ]
        : []),
    ],
    {
      gap: 0,
      padding: target === 'desktop' ? 24 : props.shell ? 0 : 1,
      style: { width: fill, height: fill, maxWidth: 808, minWidth: 0 },
      terminal: { grow: 1 },
    },
  );
  if (target === 'tui') {
    const checkpoint = bookmarkView(
      presentation,
      state.bookmark,
      'settings-checkpoint',
      (props.bookmarkKey ?? 'deployment') + '/form',
      settingsBookmark(state),
    );
    out.frame.view = ui.box(
      'deployment-checkpoint-surface',
      [
        checkpoint,
        ...(state.bookmark.observed?.state.kind === 'failed' || state.restoreFailed
          ? [
              ui.box(
                'settings-save-error',
                [
                  ui.text(
                    'settings-save-message',
                    state.bookmark.command &&
                      state.bookmark.command.sequence > (state.bookmark.observed?.sequence ?? 0)
                      ? text.restoringView
                      : text.viewSaveFailed,
                    13,
                    props.theme.warning,
                  ),
                  ...(state.bookmark.command &&
                  state.bookmark.command.sequence > (state.bookmark.observed?.sequence ?? 0)
                    ? []
                    : [ui.button('settings-checkpoint-retry', text.reloadView)]),
                ],
                { row: true, gap: 1, terminal: { wrap: true } },
              ),
            ]
          : []),
        out.frame.view,
      ],
      { gap: 0, terminal: { grow: 1 }, style: { width: fill, height: fill } },
    );
  }
  if (target === 'desktop')
    out.frame.view = ui.box('deployment-surface', [out.frame.view], {
      gap: 0,
      style: { width: fill, height: fill, align: 'center' },
    });
  out.actions = actions;
  return out;
}
