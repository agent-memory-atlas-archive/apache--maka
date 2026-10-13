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

import type { Theme } from '../../shared/contracts.js';
import { copy } from '../../shared/locale.js';
import { fill, pixels, type Presentation, type Node } from '../../shared/view.js';

import { type State } from './state.js';
import {
  canExecute,
  mutationPending,
  conversationRoute,
  rootCommands,
  sidebarLabel,
} from './actions.js';

export function headerView<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  theme: Theme,
  locale: string,
  title: N,
  models: N,
  status: string | null,
): N {
  const target = presentation.target;
  const ui = presentation.paint(theme),
    text = copy(locale),
    busy = mutationPending(state);
  const currentPage = state.pages.at(-1);
  const header = ui.box(
    'header',
    [
      ...(target === 'desktop'
        ? [
            ui.box(
              'context',
              [
                title,
                ...(!state.embedding.fixed
                  ? [ui.button('navigation-toggle', sidebarLabel(state, text))]
                  : []),
              ],
              {
                row: true,
                gap: 8,
                style: { width: fill, align: 'center' },
              },
            ),
            ui.box(
              'toolbar',
              [
                ui.box('model-control', [models], {
                  style: { width: pixels(0), grow: 1, minWidth: 0 },
                }),
                ui.button('palette-open', text.paletteTitle),
                ui.button(
                  'details-open',
                  text.detailsOpen,
                  !rootCommands(state, locale, target).some(
                    (command) => command.kind === 'details-open' && !command.disabled,
                  ),
                ),
                ui.button('terminal-open', text.terminalTitle, !state.selected || state.loading),
                ...(!state.embedding.fixed
                  ? [
                      ui.button('projects-open', text.projectsTitle),
                      ...(state.embedding.workhub ? [ui.button('workhub-open', 'WorkHub')] : []),
                      ui.button('apps-open', text.appsTitle),
                      ui.button('imports-open', text.importsTitle),
                      ...(state.scheduledTasks
                        ? [ui.button('scheduler-open', text.scheduleTitle)]
                        : []),
                    ]
                  : []),
                ui.button('settings', text.settings),
                ui.button('deployment-open', text.deploymentTitle),
              ],
              { row: true, gap: 8, style: { width: fill, align: 'center' } },
            ),
          ]
        : [
            ui.box(
              'header-left',
              [
                ...(!state.embedding.fixed
                  ? [
                      ui.box(
                        'navigation-cell',
                        [
                          ui.button(
                            'navigation-toggle',
                            theme.ascii ? '=' : '≡',
                            false,
                            false,
                            { background: 0 },
                            undefined,
                            text.toggleSidebar + ' · Ctrl+B',
                          ),
                        ],
                        { terminal: { basis: 3 } },
                      ),
                    ]
                  : []),
                ...(currentPage
                  ? [
                      ui.box(
                        'back-cell',
                        [
                          ui.button(
                            'page-back',
                            theme.ascii ? '<' : '‹',
                            currentPage !== 'settings' &&
                              state.pageChrome[currentPage]?.backEnabled !== true,
                            false,
                            { background: 0 },
                            undefined,
                            text.back,
                          ),
                        ],
                        { terminal: { basis: 3 } },
                      ),
                    ]
                  : []),
              ],
              {
                visible: !state.embedding.fixed || !conversationRoute(state),
                row: true,
                gap: 0,
                terminal: { basis: (state.embedding.fixed ? 0 : 3) + (currentPage ? 3 : 0) },
              },
            ),
            ui.box(
              'header-task',
              [
                ...(state.selected && conversationRoute(state)
                  ? [
                      state.run?.state === 'waiting_for_user'
                        ? ui.text('task-face', '-_-', 14, theme.warning)
                        : ui.activity('task-face', state.run ? 'spring' : 'familiar', theme.accent),
                    ]
                  : []),
                ui.box('header-title', [title], { terminal: { shrink: true } }),
                ui.box('header-title-trailing', [], { terminal: { grow: 1 } }),
              ],
              { row: true, gap: 1, terminal: { grow: 1, shrink: true } },
            ),
            ui.box(
              'toolbar',
              [
                ...(['details-open', 'terminal-open', 'palette-open'] as const).map((key, at) =>
                  ui.box(
                    `${key}-cell`,
                    [
                      ui.button(
                        key,
                        [text.detailsOpen, text.terminalTitle, text.paletteTitle][at]!,
                        key !== 'palette-open' && (!conversationRoute(state) || !state.selected),
                        false,
                        { background: 0 },
                        undefined,
                        [text.detailsOpen, text.terminalTitle, text.paletteTitle + ' · Ctrl+K'][at],
                      ),
                    ],
                    { terminal: { shrink: false } },
                  ),
                ),
              ],
              { row: true, gap: 1, terminal: { shrink: true, wrap: true } },
            ),
          ]),
      ...((target === 'desktop' ? state.interactions?.entries : [])
        ?.filter((entry) =>
          ['question', 'form', 'service_approval', 'permissions'].includes(entry.kind),
        )
        .map((entry, index) =>
          ui.button(
            `${entry.kind === 'question' || entry.kind === 'form' ? 'question' : 'approval'}-open-${entry.requestId}`,
            `${entry.kind === 'form' ? text.formTitle : entry.kind === 'question' ? text.questionsTitle : entry.kind === 'permissions' ? text.permissionApprovalTitle : text.approvalTitle} ${index + 1}`,
            !state.selected || state.selected.archived || mutationPending(state),
          ),
        ) ?? []),
      ...(target === 'desktop' && !canExecute(state)
        ? [
            ui.text(
              'model-setup-hint',
              state.hasBackends
                ? text.executionBackend
                : state.modelSetup === 'loading'
                  ? text.loadingModels
                  : state.modelSetup === 'unavailable'
                    ? text.modelsUnavailable
                    : state.modelSetup === 'connect'
                      ? text.setupConnectionHint
                      : text.setupModelHint,
              13,
              theme.muted,
            ),
            ...(state.modelSetup === 'connect'
              ? [
                  ui.button(
                    'model-setup',
                    state.hasBackends ? text.executionBackend : text.configureModels,
                    busy,
                  ),
                ]
              : []),
          ]
        : []),
      ...(target === 'desktop' && status ? [ui.text('status', status, 13, theme.muted)] : []),
      ...(state.uncertain
        ? [
            ui.button(
              'check',
              text.check,
              Object.values(state.pending).some((request) => request.kind === 'receipt'),
            ),
          ]
        : []),
      ...(state.notice === 'unavailable' ? [ui.button('reload', text.loading, state.loading)] : []),
    ],
    {
      row: target === 'tui',
      gap: target === 'desktop' ? 8 : 0,
      style: {
        width: fill,
        shrink: 0,
        padding: { top: 16, right: 24, bottom: 16, left: 24 },
      },
    },
  );

  return header;
}
