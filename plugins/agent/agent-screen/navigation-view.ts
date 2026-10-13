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

import { tuiText } from '../../shared/tui-locale.js';
import type { Theme } from '../../shared/contracts.js';
import { copy } from '../../shared/locale.js';
import { fill, pixels, type Presentation, type Node } from '../../shared/view.js';

import { type State } from './state.js';
import { catalogPending, mutationPending, rootCommands } from './actions.js';

export function navigationView<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  theme: Theme,
  locale: string,
): N {
  const target = presentation.target;
  const ui = presentation.paint(theme),
    text = copy(locale),
    busy = mutationPending(state);
  const catalogBusy = busy || catalogPending(state);
  const commands = rootCommands(state, locale, target);
  const createButton = ui.button(
    'create',
    target === 'tui' ? '+  ' + tuiText(locale, 'sidebar-new-session') : text.new,
    !commands.some((command) => command.kind === 'create' && !command.disabled),
    target === 'tui',
    target === 'tui' ? { background: 0, align: 'start' } : {},
  );
  const createEntry = ui.contextMenu
    ? ui.contextMenu('navigation-create-menu', text.new, createButton, [
        ui.button(
          'create-workspace',
          text.newWorkspace,
          !commands.some((command) => command.kind === 'create-workspace' && !command.disabled),
        ),
      ])
    : createButton;
  const navigationLink = (
    key: string,
    label: string,
    glyph: string,
    ascii: string,
    current = false,
  ) =>
    ui.button(key, `${theme.ascii ? ascii : glyph}  ${label}`, false, false, {
      background: current ? theme.selected : 0,
      align: 'start',
    });
  const navigationApps =
    target === 'tui' && !state.embedding.fixed
      ? [
          ui.box('navigation-apps-gap', [], { terminal: { basis: 1 } }),
          navigationLink('apps-open', text.appsTitle, '', '', state.pages.at(-1) === 'apps'),
          ...(state.embedding.workhub ? [navigationLink('workhub-open', 'WorkHub', '◈', '*')] : []),
          ...(state.scheduledTasks
            ? [
                navigationLink(
                  'scheduler-open',
                  text.scheduleTitle +
                    (state.scheduleAlerts.pending ? ` (${state.scheduleAlerts.pending})` : ''),
                  '⏲',
                  'T',
                  state.pages.at(-1) === 'scheduler',
                ),
              ]
            : []),
          navigationLink(
            'recall-open',
            text.recallTitle,
            '⌕',
            '/',
            state.pages.at(-1) === 'recall',
          ),
          navigationLink(
            'imports-open',
            text.importsTitle,
            '⇥',
            '>',
            state.pages.at(-1) === 'imports',
          ),
        ]
      : [];
  const navigation = ui.box(
    'navigation-panel',
    [
      target === 'tui'
        ? ui.box(
            'navigation-heading',
            [
              ui.box('navigation-create-cell', [createEntry], { terminal: { grow: 1 } }),
              ui.button(
                'navigation-back',
                theme.ascii ? 'x' : '×',
                false,
                false,
                { background: 0, align: 'start' },
                undefined,
                text.hideSidebar,
              ),
            ],
            { row: true, gap: 0 },
          )
        : ui.text('navigation-brand', 'Maka', 24, theme.text, 650),
      ...(target === 'desktop' ? [createEntry] : []),
      ...(target === 'desktop'
        ? [ui.text('navigation-heading', text.conversations, 12, theme.muted, 550)]
        : []),
      ui.box(
        'catalog-filters',
        [
          ...(['all', 'needs_you'] as const).map((filter) => {
            const label =
              filter === 'all'
                ? target === 'tui'
                  ? tuiText(locale, 'sidebar-all')
                  : text.allSessions
                : target === 'tui'
                  ? tuiText(locale, 'sidebar-needs-you') +
                    (state.catalog?.needsYouCount ? ` · ${state.catalog.needsYouCount}` : '')
                  : state.catalog
                    ? `${text.needsYou} (${state.catalog.needsYouCount})`
                    : text.needsYou;
            const selected = state.catalogFilter === filter;
            return ui.item
              ? ui.item(
                  `catalog-filter-${filter}`,
                  ui.text(`catalog-filter-label-${filter}`, label, 12, theme.text),
                  selected,
                  catalogBusy,
                )
              : ui.button(
                  `catalog-filter-${filter}`,
                  label,
                  catalogBusy,
                  false,
                  selected ? { background: theme.selected } : {},
                );
          }),
          ...(target === 'tui'
            ? [
                ui.button(
                  'catalog-search',
                  `${theme.ascii ? '/' : '⌕'} ${tuiText(locale, 'sidebar-search')}`,
                  false,
                  false,
                  { background: 0, align: 'start' },
                  undefined,
                  tuiText(locale, 'sidebar-search-sessions'),
                ),
              ]
            : []),
        ],
        { row: true, focusGroup: target === 'tui', gap: target === 'tui' ? 2 : 1 },
      ),
      ...(target === 'tui' ? [ui.box('catalog-gap', [], { terminal: { basis: 1 } })] : []),
      ui.box(
        'catalog-body',
        [
          ...(!state.catalogFailed && !catalogPending(state) && state.catalog?.sessions.length === 0
            ? [
                ui.box(
                  'catalog-empty-inset',
                  [
                    ui.text(
                      'catalog-empty',
                      (target === 'tui' ? '   ' : '') +
                        (state.catalogFilter === 'needs_you'
                          ? text.noneNeedsYou
                          : tuiText(locale, 'sidebar-empty')),
                      12,
                      theme.muted,
                    ),
                  ],
                  { padding: target === 'desktop' ? 1 : 0, terminal: { shrink: true } },
                ),
              ]
            : []),
          ...(state.catalogFailed
            ? [ui.text('catalog-error', text.sessionsUnavailable, 12, theme.muted)]
            : []),
          ui.box(
            'catalog-controls',
            [
              ...(target === 'desktop' || state.catalogFailed
                ? [ui.button('catalog-refresh', text.refreshSessions, catalogBusy)]
                : []),
            ],
            { row: true, style: { wrap: true }, terminal: { wrap: true } },
          ),
          presentation.slot(
            'navigation',
            'navigation',
            {
              theme,
              locale,
              sessions: state.catalog?.sessions ?? [],
              selected: state.selected?.id ?? null,
              busy,
              loading: catalogPending(state),
              older: !!state.catalog?.before,
              newer: !!state.catalog?.after,
            },
            { select: 'select', older: 'older', newer: 'newer' },
          ),
        ],
        {
          gap: 0,
          scroll: 'none',
          terminal: { grow: 1 },
          style: { height: pixels(0), grow: 1, minHeight: 0 },
        },
      ),
      ...(target === 'tui'
        ? [
            ui.box(
              'navigation-footer',
              [
                ...navigationApps,
                ...(!state.embedding.fixed
                  ? [
                      navigationLink(
                        'projects-open',
                        text.projectsTitle,
                        '▦',
                        'P',
                        state.pages.at(-1) === 'projects',
                      ),
                    ]
                  : []),
                navigationLink(
                  'settings',
                  tuiText(locale, 'route-settings'),
                  '⛭',
                  'S',
                  state.pages.at(-1) === 'settings',
                ),
              ],
              { gap: 0 },
            ),
          ]
        : []),
    ],
    {
      surface: target === 'desktop',
      gap: target === 'tui' ? 0 : undefined,
      padding: target === 'desktop' ? 20 : 1,
      style: { height: fill, width: fill },
      terminal: { grow: 1 },
    },
  );

  return navigation;
}
