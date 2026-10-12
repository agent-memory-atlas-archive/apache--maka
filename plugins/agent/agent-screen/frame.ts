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

import {
  effectiveAppearance,
  effectiveLocale,
  appearanceEvent,
  appearanceView,
} from '../../shared/appearance-editor.js';
import { tuiText } from '../../shared/tui-locale.js';
import type {
  Action,
  Emission,
  Shortcut,
  RowsProps,
  ComponentValues,
} from '../../../packages/plugin-sdk/src/index.js';
import type {
  Theme,
  QueueEditorProps,
  QuestionEditorProps,
  ServiceApprovalProps,
} from '../../shared/contracts.js';
import { sameValue } from '../../shared/value.js';
import { bookmarkView } from '../../shared/bookmark.js';
import { copy, resolveLocale } from '../../shared/locale.js';
import {
  compactLabel,
  fill,
  output,
  pixels,
  type Presentation,
  type Node,
} from '../../shared/view.js';

import {
  attachmentFiles,
  attachmentView,
  attachmentSummary,
  pasteView,
} from '../../shared/attachments.js';

import {
  draftReady,
  retainSelectedDraft,
  draftStatuses,
  nextDraftStep,
} from '../../shared/drafts.js';

import { navigationBookmark, navigationKey, type State } from './state.js';
import { utf8Bytes, messages, turnTimings } from './messages.js';
import {
  intent,
  canExecute,
  catalogPending,
  select,
  mutationPending,
  uploadPastedFile,
  executionPending,
  conversationRoute,
  reviewTarget,
  rootCommands,
  questionChild,
  closeDialog,
} from './actions.js';
import { navigationView } from './navigation-view.js';
import { headerView } from './header-view.js';
import { pagesView } from './pages-view.js';

export function frame<N extends Node>(
  presentation: Presentation<N>,
  locale: string,
  state: State,
  values: ComponentValues,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const target = presentation.target;
  const currentPage = state.pages.at(-1);
  if (currentPage && !state.openedPages.includes(currentPage)) state.openedPages.push(currentPage);
  const requested = state.embedding.session;
  if (requested && !sameValue(requested, state.externalSelection) && !mutationPending(state)) {
    state.externalSelection = requested;
    select(state, actions, null);
    state.loading = true;
    closeDialog(state);
    intent(state, actions, { kind: 'openSession', selection: requested }, 'agent', 'observe', {
      sessionId: requested.id,
    });
  }
  if (state.dialog !== 'palette') {
    state.copyRequest = null;
    state.messageCopy = null;
  }
  retainSelectedDraft(state.drafts, state.selected);
  if (
    !Object.values(state.pending).some(
      (request) => request.kind === 'draftPage' || request.kind === 'draftReceipt',
    )
  ) {
    const step = nextDraftStep(state.drafts, state.attachments);
    if (step)
      intent(
        state,
        actions,
        step,
        step.kind === 'draftPage' ? 'history' : 'receipts',
        step.kind === 'draftPage' ? 'page' : 'read',
        step.kind === 'draftPage' ? { after: step.after, limit: 1 } : step.original.reference,
      );
  }
  uploadPastedFile(state, actions);
  if (!conversationRoute(state) || state.source || state.image || !state.selected) {
    state.searchOpen = false;
    state.browse = null;
  }
  const baseTheme = values.appearance?.[0] as Theme;
  if (target === 'tui' && state.preferences.open && !state.preferences.draft)
    appearanceEvent(
      state.preferences,
      { kind: 'raw', key: 'appearance-open', event: {} },
      baseTheme,
      locale,
      actions,
    );
  locale =
    target === 'tui'
      ? effectiveLocale(state.preferences, locale, values.locale?.[0])
      : resolveLocale(locale, values.locale?.[0]);
  const theme = target === 'tui' ? effectiveAppearance(state.preferences, baseTheme) : baseTheme;
  const text = copy(locale),
    ui = presentation.paint(theme),
    busy = mutationPending(state);
  const catalogBusy = busy || catalogPending(state);
  const commands = rootCommands(state, locale, target);
  const navigation = navigationView(presentation, state, theme, locale);
  const timelineMessages = messages(state);
  const timeline = presentation.slot(
    'timeline',
    'timeline',
    {
      theme,
      locale,
      messages: timelineMessages,
      timings: turnTimings(state, timelineMessages),
      copyRequest: target === 'tui' && !state.image ? state.copyRequest : null,
      browse: state.browse,
      markReadEnabled:
        !!state.selected?.hasUnread &&
        !state.selected.archived &&
        !busy &&
        conversationRoute(state) &&
        state.dialog === null &&
        !state.source &&
        !state.image,
      search:
        target === 'tui' && state.searchOpen && state.selected
          ? { sessionId: state.selected.id, epoch: state.epoch, through: state.latestKnown }
          : null,
      source: state.source
        ? {
            kind: state.source.field === 'body' ? 'message' : state.source.field,
            fields: state.source.fields,
            format: state.source.format ?? 'text',
            ready: state.source.ready,
            key: `${state.source.field}-${state.source.through}-${state.source.chunk.sequence}-${state.source.chunk.offset}`,
            text: state.source.chunk.text,
            atStart: state.source.chunk.offset === 0,
            atEnd:
              state.source.chunk.offset + utf8Bytes(state.source.chunk.text) ===
              state.source.chunk.totalBytes,
            loading: state.source.loading,
            failed: state.source.failed,
            navigation: state.source.landing.sequence,
            landing: state.source.landing.edge,
          }
        : null,
      loading: state.loading,
      older: !!state.window?.page.before,
      newer: !!state.window?.page.after,
      latest:
        !!state.window &&
        (!state.window.latest ||
          state.latestKnown > state.window.page.through ||
          state.live.some(
            (fragment) =>
              fragment.closedAt !== null && fragment.closedAt <= state.window!.page.through,
          )),
      landing: state.landing,
      toolContext:
        state.selected && state.window && state.window.page.through > 0
          ? { sessionId: state.selected.id, through: state.window.page.through }
          : null,
      liveTools:
        state.window?.latest && state.run
          ? [
              ...new Set(
                state.window.page.chunks.flatMap((chunk) =>
                  chunk.turnId === state.run!.turnId && chunk.tool ? [chunk.tool.toolUseId] : [],
                ),
              ),
            ]
          : [],
      waitingTools: state.window?.latest
        ? (state.interactions?.entries ?? [])
            .filter(
              (entry) =>
                !(
                  state.interaction?.record?.requestId === entry.requestId &&
                  state.interaction.record.outcome
                ),
            )
            .flatMap((entry) => (entry.toolUseId === null ? [] : [entry.toolUseId]))
        : [],
      live: !!state.run && !!state.window?.latest,
    },
    {
      older: 'older',
      newer: 'newer',
      latest: 'latest',
      markRead: 'mark-read',
      source: 'source',
      toolSource: 'tool-source',
      fileContext: 'fileContext',
      messageCopy: 'messageCopy',
      viewImage: 'view-image',
      sourceFirst: 'source-first',
      sourcePrevious: 'source-previous',
      sourceNext: 'source-next',
      sourceLast: 'source-last',
      sourceBack: 'source-back',
      sourceRetry: 'source-retry',
      sourceField: 'source-field',
      tailReady: 'tail-ready',
      searchClosed: 'search-closed',
      browseClosed: 'browse-closed',
    },
  );
  const imageViewer = presentation.slot(
    'image-viewer',
    'imageViewer',
    {
      theme,
      locale,
      image: state.image,
      active: !!state.image && conversationRoute(state),
    },
    { closed: 'image-closed' },
  );
  const attached = attachmentFiles(
    state.attachments,
    state.selected?.id,
    state.selected?.dataSource,
  );
  const attachmentsDisabled =
    !state.selected ||
    !draftReady(state.drafts, state.selected) ||
    busy ||
    state.loading ||
    !!state.attachments.paste;
  const attachmentsButton = ui.button(
    'attachments-open',
    `${text.attachmentTitle}${attached.length ? ` (${attached.length})` : ''}`,
    attachmentsDisabled,
  );
  const composer = presentation.slot(
    'composer',
    'composer',
    {
      context: state.selected?.id ?? null,
      dataSource: state.selected?.dataSource ?? null,
      attachments: state.attachments.files,
      drafts: draftStatuses(state.drafts),
      theme,
      locale,
      busy:
        busy ||
        !!state.attachments.paste ||
        executionPending(state) ||
        (target === 'tui' && !!state.run && !state.queue),
      filePaste:
        target === 'tui' &&
        state.selected &&
        !state.selected.archived &&
        conversationRoute(state) &&
        !state.dialog &&
        !state.source &&
        !state.image &&
        !state.loading &&
        (!busy || !!state.attachments.paste) &&
        (!state.attachments.paste ||
          (state.attachments.paste.observed.context === state.selected.id &&
            sameValue(state.attachments.paste.observed.dataSource, state.selected.dataSource)))
          ? {
              context: state.selected.id,
              maxFiles: Math.min(8 - attached.length, 64 - state.attachments.files.length),
            }
          : null,
      fileCommand: state.attachments.paste?.command ?? null,
      running: !!state.run,
      enabled:
        !!state.selected &&
        !state.selected.archived &&
        canExecute(state) &&
        !state.loading &&
        state.notice !== 'unavailable',
      command: state.command,
      autoFocus: !state.searchOpen && state.browse === null,
      hasAttachments: attached.length > 0,
    },
    {
      send: 'send',
      stop: 'stop',
      attachments: 'attachments',
      changed: 'draft',
      files: 'files',
      checkpoint: 'checkpoint',
      draftCheck: 'draft-check',
    },
  );
  const queueProps: RowsProps = {
    terminal: theme.terminal,
    entry: state.queue?.entries.at(-1)?.messageId ?? null,
    revision: state.queue?.revision ?? 0,
    rows: (state.queue?.entries ?? []).map((entry): RowsProps['rows'][number] => {
      const lane = state.queue!.entries.filter((row) => row.kind === entry.kind);
      const index = lane.findIndex((row) => row.messageId === entry.messageId);
      const enabled = !busy && !executionPending(state);
      const up = {
        key: 'up',
        label: theme.ascii ? '^' : '↑',
        enabled: enabled && index > 0,
      };
      const down = {
        key: 'down',
        label: theme.ascii ? 'v' : '↓',
        enabled: enabled && index + 1 < lane.length,
      };
      const edit = { key: 'edit', label: theme.ascii ? 'e' : '✎', enabled };
      const retract = {
        key: 'retract',
        label: theme.ascii ? 'x' : '×',
        enabled,
      };
      return {
        key: entry.messageId,
        text: ` ${entry.kind === 'steering' ? (theme.ascii ? '^' : '↗') : entry.kind === 'in_flight' ? (theme.ascii ? '...' : '…') : theme.ascii ? '+' : '↳'} ${entry.preview}`,
        actions:
          entry.kind === 'in_flight'
            ? []
            : entry.kind === 'followup'
              ? [
                  {
                    key: 'promote',
                    label: theme.ascii ? '^' : '↗',
                    enabled: enabled && !!state.run,
                  },
                  up,
                  down,
                  edit,
                  retract,
                ]
              : [up, down, edit, retract],
      };
    }),
    shortcuts: [
      { key: 'enter', modifiers: [], input: 'edit', enabled: true },
      { key: 'e', modifiers: [], input: 'edit', enabled: true },
      { key: 's', modifiers: [], input: 'promote', enabled: true },
      { key: 'delete', modifiers: [], input: 'retract', enabled: true },
      { key: 'x', modifiers: [], input: 'retract', enabled: true },
      { key: 'up', modifiers: ['alt'], input: 'up', enabled: true },
      { key: 'down', modifiers: ['alt'], input: 'down', enabled: true },
    ],
    maxRows: 3,
    compactBelow: { height: 12, maxRows: 1 },
    tabIndex: 2,
    foreground: theme.text,
    muted: theme.muted,
    selected: theme.selected,
    hovered: theme.surface,
    scrollbar: {
      thumb: theme.ascii ? '#' : '┃',
      track: theme.ascii ? '|' : '│',
      thumbColor: theme.text,
      trackColor: theme.border,
    },
  };
  const status = state.notice
    ? text[state.notice]
    : state.run
      ? state.run.state === 'waiting_for_user'
        ? text.waiting
        : text.working
      : state.selected?.archived
        ? text.archivedConversation
        : '';
  const title = ui.text(
    'title',
    (currentPage && state.pageChrome[currentPage]?.title) ||
      (state.pages.at(-1) === 'scheduler'
        ? text.scheduleTitle
        : state.pages.at(-1) === 'recall'
          ? text.recallTitle
          : state.pages.at(-1) === 'apps'
            ? text.appsTitle
            : state.pages.at(-1) === 'imports'
              ? text.importsTitle
              : state.pages.at(-1) === 'projects'
                ? text.projectsTitle
                : state.pages.at(-1) === 'settings'
                  ? tuiText(locale, 'route-settings')
                  : state.pages.at(-1) === 'connections'
                    ? text.settings
                    : state.pages.at(-1) === 'deployment'
                      ? text.deploymentTitle
                      : (state.selected?.title ?? tuiText(locale, 'route-agent-screen'))),
    17,
    theme.text,
    650,
    {
      align: target === 'tui' ? 'start' : 'center',
      width: pixels(0),
      grow: 1,
      minWidth: 0,
      truncate: true,
    },
  );
  const models = presentation.slot(
    'models',
    'models',
    {
      theme,
      locale,
      selection: state.model,
      executor: state.executor,
      catalogEpoch: state.catalogEpoch,
      openRequest: state.modelOpenRequest,
      busy: busy || executionPending(state) || (target === 'tui' && !!state.run && !state.queue),
    },
    {
      selected: 'model',
      setup: 'model-setup',
      connections: 'model-connections',
      executor: 'executor',
      backends: 'backends',
    },
  );

  const header = headerView(presentation, state, theme, locale, title, models, status);
  const welcome =
    !!state.selected &&
    !state.selected.archived &&
    !state.loading &&
    !!state.window &&
    !state.window.page.chunks.length &&
    !state.live.length &&
    !state.run &&
    !state.source &&
    !state.image &&
    !state.searchOpen;
  const main = ui.box(
    'main',
    [
      ...(target === 'desktop' ? [header] : []),
      ...(target === 'tui'
        ? [
            ui.box(
              'home',
              [
                ui.box('home-top', [], { terminal: { grow: 1 } }),
                ui.box(
                  'home-center',
                  [
                    ui.box('home-left', [], { terminal: { grow: 1 } }),
                    ui.box(
                      'home-content',
                      [
                        ui.text('home-title', 'Maka', 24, theme.text, 650, {
                          align: 'center',
                        }),
                        ...(state.place
                          ? [
                              ui.text('home-place', state.place, 14, theme.subtle, 400, {
                                align: 'center',
                                truncate: true,
                              }),
                            ]
                          : []),
                        ui.box('home-gap', [], { terminal: { basis: 1 } }),
                        ...(!canExecute(state)
                          ? [
                              ui.text(
                                'home-setup',
                                state.hasBackends
                                  ? text.executionBackend
                                  : tuiText(
                                      locale,
                                      state.modelSetup === 'loading'
                                        ? 'connections-loading'
                                        : state.modelSetup === 'unavailable'
                                          ? 'home-setup-failed'
                                          : state.modelSetup === 'connect'
                                            ? 'home-needs-connection'
                                            : 'home-needs-default',
                                    ),
                                14,
                                state.modelSetup === 'unavailable' ? theme.warning : theme.subtle,
                                400,
                                { align: 'center' },
                              ),
                            ]
                          : []),
                        ui.button(
                          'home-new',
                          canExecute(state)
                            ? '+ ' + tuiText(locale, 'sidebar-new-session') + '   Ctrl+N'
                            : state.hasBackends
                              ? text.executionBackend
                              : tuiText(
                                  locale,
                                  state.modelSetup === 'connect'
                                    ? 'route-connections'
                                    : 'default-model-title',
                                ) + '   Ctrl+N',
                          catalogBusy ||
                            (!canExecute(state) &&
                              !state.hasBackends &&
                              state.modelSetup === 'loading'),
                          true,
                          { background: 0 },
                        ),
                        ui.button('home-projects', text.projectsTitle, false, false, {
                          background: 0,
                        }),
                        ui.button('home-imports', text.importsTitle, false, false, {
                          background: 0,
                        }),
                      ],
                      { gap: 0, terminal: { basis: 52, shrink: true } },
                    ),
                    ui.box('home-right', [], { terminal: { grow: 1 } }),
                  ],
                  { row: true, gap: 0 },
                ),
                ui.box('home-bottom', [], { terminal: { grow: 1 } }),
              ],
              {
                gap: 0,
                visible: !state.selected && !state.embedding.fixed,
                viewport: true,
                terminal: { grow: 1 },
              },
            ),
          ]
        : []),
      ...(state.selected?.archived && attached.length && !state.source && !state.image
        ? [ui.box('archived-attachments', [attachmentsButton], { row: true })]
        : []),
      ...(target === 'desktop' && state.attachments.paste
        ? [pasteView(presentation, theme, locale, state.attachments.paste, busy)]
        : []),
      ...(target === 'desktop'
        ? [
            ui.box(
              'conversation',
              [
                ui.box('retained-timeline', [timeline], {
                  visible: !state.image,
                  style: { width: fill, height: fill },
                  terminal: { grow: 1 },
                }),
                imageViewer,
              ],
              {
                style: {
                  width: fill,
                  height: pixels(0),
                  grow: 1,
                  shrink: 1,
                  minHeight: 0,
                },
              },
            ),
            ui.box(
              'input-area',
              [...(!state.selected?.archived ? [attachmentsButton] : []), composer],
              {
                visible: !state.image && !state.selected?.archived,
                padding: 24,
                style: { width: fill, maxWidth: 808, shrink: 0 },
              },
            ),
          ]
        : [
            ui.box(
              'session-body',
              [
                ui.box(
                  'conversation',
                  [
                    ui.box(
                      'welcome-mark',
                      [
                        presentation.slot(
                          'welcome',
                          'welcome',
                          {
                            background: theme.terminal?.background ?? theme.background >>> 8,
                            foreground: theme.terminal?.foreground ?? theme.text >>> 8,
                            motion: theme.motion,
                            ascii: theme.ascii,
                          },
                          {},
                        ),
                      ],
                      { visible: welcome, terminal: { grow: 1 } },
                    ),
                    ui.box('retained-timeline', [timeline], {
                      visible: !state.image && !welcome,
                      style: { width: fill, height: fill },
                      terminal: { grow: 1 },
                    }),
                    imageViewer,
                  ],
                  { terminal: { basis: 1, grow: 1 } },
                ),
                ...(state.attachments.paste
                  ? [pasteView(presentation, theme, locale, state.attachments.paste, busy)]
                  : []),
                ui.box(
                  'retained-composer',
                  [
                    ...(reviewTarget(state)
                      ? [
                          ui.button(
                            'interaction-open',
                            `Ctrl G · ${text.reviewRequest}`,
                            !rootCommands(state, locale, target).some(
                              (command) => command.kind === 'interaction-open' && !command.disabled,
                            ),
                          ),
                        ]
                      : []),
                    presentation.slot('queue', 'rows', queueProps, {
                      activated: 'queue-action',
                    }),
                    ...(queueProps.rows.length && ui.focusButton
                      ? [
                          ui.box(
                            'queue-status',
                            [
                              ui.box('queue-status-space', [], {
                                terminal: { basis: 0, grow: 1 },
                              }),
                              ui.focusButton(
                                'queue-focus',
                                `${theme.ascii ? '=' : '≡'} ${queueProps.rows.length}`,
                                'queue',
                              ),
                            ],
                            { row: true, gap: 0 },
                          ),
                        ]
                      : []),
                    ...(attached.length
                      ? [
                          attachmentSummary(
                            presentation,
                            theme,
                            locale,
                            attached,
                            attachmentsDisabled,
                          ),
                        ]
                      : []),
                    ui.box(
                      'composer-navigation',
                      [
                        composer,
                        ui.box(
                          'composer-footer',
                          [
                            presentation.slot(
                              'footer',
                              'footer',
                              {
                                sessionId: state.selected?.id ?? null,
                                dataSource: state.selected?.dataSource ?? null,
                                running: !!state.run,
                                theme,
                                locale,
                              },
                              {},
                            ),
                          ],
                          { terminal: { basis: 1, shrink: false }, scroll: 'none' },
                        ),
                      ],
                      {
                        gap: 0,
                        terminal: { shrink: true },
                        focusShortcuts: [
                          {
                            key: 'up',
                            modifiers: [],
                            child: 'queue',
                            enabled: queueProps.rows.length > 0,
                          },
                        ],
                      },
                    ),
                  ],
                  {
                    gap: 0,
                    visible: !state.source && !state.image && !state.selected?.archived,
                    terminal: { shrink: true },
                  },
                ),
              ],
              {
                gap: 0,
                visible: !!state.selected,
                viewport: true,
                terminal: { grow: 1 },
              },
            ),
            ui.box(
              'session-metadata',
              [
                ui.box('model-metadata', [models], {
                  terminal: { grow: 1, shrink: true },
                }),
                ...(status
                  ? [
                      ui.text(
                        'status',
                        status,
                        12,
                        state.run?.state === 'waiting_for_user' ? theme.warning : theme.muted,
                      ),
                    ]
                  : []),
              ],
              { row: true, gap: 1 },
            ),
          ]),
    ],
    {
      visible: conversationRoute(state),
      gap: target === 'tui' ? 0 : undefined,
      terminal: { grow: 1 },
      style: {
        width: target === 'desktop' ? pixels(0) : fill,
        grow: 1,
        height: fill,
        align: 'center',
        minWidth: 0,
      },
    },
  );
  const pageContent = pagesView(presentation, state, theme, locale, values, main);
  const conversation = ui.box(
    'agent-screen',
    state.embedding.fixed
      ? [pageContent]
      : target === 'desktop'
        ? [
            ui.box('sidebar', [navigation], {
              visible: 'open' in state.sidebar && state.sidebar.open,
              style: { width: pixels(240), height: fill, shrink: 0 },
            }),
            pageContent,
          ]
        : [
            ui.adaptivePane!(
              'navigation-pane',
              text.conversations,
              ui.box(
                'page',
                [
                  ui.box('page-inset-start', [], { terminal: { basis: 1 } }),
                  pageContent,
                  ui.box('page-inset-end', [], { terminal: { basis: 1 } }),
                ],
                { row: true, gap: 0, terminal: { grow: 1 } },
              ),
              ui.box('navigation-content', [navigation], {
                gap: 0,
                terminal: { grow: 1 },
              }),
              'command' in state.sidebar ? state.sidebar.command : null,
            ),
          ],
    {
      row: target === 'desktop',
      gap: 0,
      terminal: { grow: 1 },
      style: { width: fill, height: fill },
    },
  );
  const background = ui.box(
    'product',
    [
      ...(target === 'tui' ? [header] : []),
      ...(!state.embedding.fixed && state.scheduleAlerts.reminders.length
        ? [
            ui.button(
              'schedule-reminder',
              `${text.scheduleReminder}: ${compactLabel(state.scheduleAlerts.reminders[0]!.title, 100)}`,
              !!state.dialog,
              false,
              { background: theme.surface, align: 'start', width: fill },
            ),
          ]
        : []),
      conversation,
      ...(target === 'tui' && (!state.embedding.fixed || state.embedding.session)
        ? [
            bookmarkView(
              presentation,
              state.bookmark,
              navigationKey(state),
              navigationKey(state),
              navigationBookmark(state),
            ),
            ...(state.bookmark.observed?.state.kind === 'failed'
              ? [ui.button('navigation-checkpoint-retry', copy(locale).sourceRetry)]
              : []),
          ]
        : []),
    ],
    {
      gap: 0,
      background: theme.background,
      style: { width: fill, height: fill, align: 'center' },
      terminal: { grow: 1 },
      shortcuts: [
        ...(state.pages.at(-1) === 'settings' && state.dialog === null && !state.preferences.open
          ? ([{ key: 'escape', modifiers: [], input: 'page-back', enabled: true }] as Shortcut[])
          : []),
        {
          key: 'f',
          modifiers: ['primary'],
          input: 'search-open',
          enabled: rootCommands(state, locale, target).some(
            (command) => command.kind === 'search-open' && !command.disabled,
          ),
        },
        { key: 'f1', modifiers: [], input: 'help-open', enabled: true },
        {
          key: 'g',
          modifiers: ['primary'],
          input: 'interaction-open',
          enabled: rootCommands(state, locale, target).some(
            (command) => command.kind === 'interaction-open' && !command.disabled,
          ),
        },
        {
          key: 'k',
          modifiers: ['primary'],
          input: 'palette-open',
          enabled: true,
        },
        {
          key: 'b',
          modifiers: ['primary'],
          input: 'navigation-toggle',
          enabled: !state.embedding.fixed,
        },
        {
          key: 'n',
          modifiers: ['primary'],
          input: 'create',
          enabled: !state.embedding.fixed && !mutationPending(state) && !catalogPending(state),
        },
      ],
    },
  );
  const out = output(
    state,
    ui.dialog(
      'agent-screen-dialog',
      state.dialog === 'attachments'
        ? target === 'tui' && state.attachments.selection?.browse
          ? tuiText(locale, 'attachments-local-files')
          : text.attachmentTitle
        : state.dialog === 'skills'
          ? text.skillsTitle
          : state.dialog === 'terminal'
            ? text.terminalTitle
            : state.dialog === 'question'
              ? state.interaction?.record?.request.kind === 'form'
                ? text.formTitle
                : text.questionsTitle
              : state.dialog === 'approval'
                ? state.interaction?.record?.request.kind === 'permissions'
                  ? text.permissionApprovalTitle
                  : text.approvalTitle
                : state.dialog === 'directories'
                  ? text.directoryTitle
                  : state.dialog === 'details'
                    ? text.detailsTitle
                    : state.dialog === 'queue'
                      ? text.queueEdit
                      : state.dialog === 'help'
                        ? text.helpTitle
                        : text.paletteTitle,
      background,
      ui.box(
        'dialog-content',
        [
          ...(state.dialog === 'attachments' && state.attachments.selection
            ? [
                attachmentView(
                  presentation,
                  theme,
                  locale,
                  state.attachments,
                  busy,
                  state.selected?.id === state.attachments.selection.sessionId &&
                    sameValue(state.selected.dataSource, state.attachments.selection.dataSource) &&
                    !state.selected.archived,
                  state.notice
                    ? text[state.notice]
                    : state.selected?.archived
                      ? text.archivedConversation
                      : null,
                  !!state.uncertain,
                  Object.values(state.pending).some((request) => request.kind === 'receipt'),
                ),
              ]
            : []),
          ...(state.interaction?.record?.request.kind === 'service_approval' ||
          state.interaction?.record?.request.kind === 'permissions'
            ? [
                presentation.slot(
                  `${state.interaction.record.request.kind === 'permissions' ? 'permission' : 'service'}-approval-${state.interaction.attempt}`,
                  state.interaction.record.request.kind === 'permissions'
                    ? 'permissionApproval'
                    : 'serviceApproval',
                  {
                    theme,
                    locale,
                    open: state.dialog === 'approval',
                    record: state.interaction.record,
                    busy:
                      mutationPending(state) ||
                      state.selected?.id !== state.interaction.sessionId ||
                      !!state.selected?.archived,
                    checking: Object.values(state.pending).some(
                      (r) =>
                        r.kind === 'interaction' || r.kind === 'receipt' || r.kind === 'respond',
                    ),
                    status: state.uncertain
                      ? text.interactionUnknown
                      : state.interaction.notice
                        ? text[state.interaction.notice]
                        : state.selected?.id !== state.interaction.sessionId
                          ? text.interactionChanged
                          : '',
                  } satisfies ServiceApprovalProps,
                  { closed: 'closed', submit: 'submit', check: 'check' },
                ),
              ]
            : state.interaction?.record
              ? [
                  presentation.slot(
                    questionChild(state),
                    state.interaction.record.request.kind === 'form'
                      ? 'formEditor'
                      : 'questionEditor',
                    {
                      theme,
                      locale,
                      open: state.dialog === 'question',
                      record: state.interaction.record,
                      busy:
                        mutationPending(state) ||
                        state.selected?.id !== state.interaction.sessionId ||
                        !!state.selected?.archived,
                      checking: Object.values(state.pending).some(
                        (r) =>
                          r.kind === 'interaction' || r.kind === 'receipt' || r.kind === 'respond',
                      ),
                      status: state.uncertain
                        ? text.interactionUnknown
                        : state.interaction.notice
                          ? text[state.interaction.notice]
                          : state.selected?.id !== state.interaction.sessionId
                            ? text.interactionChanged
                            : '',
                    } satisfies QuestionEditorProps,
                    { closed: 'closed', submit: 'submit', check: 'check' },
                  ),
                ]
              : [
                  ui.box(
                    'question-loading',
                    [
                      ui.button('question-close', text.closeModels),
                      ui.text(
                        'question-loading-text',
                        state.interaction?.notice
                          ? text[state.interaction.notice]
                          : text.questionLoading,
                      ),
                      ui.button(
                        'question-check',
                        text.check,
                        Object.values(state.pending).some((r) => r.kind === 'interaction'),
                      ),
                    ],
                    {
                      visible: state.dialog === 'question' || state.dialog === 'approval',
                      terminal: { grow: 1 },
                    },
                  ),
                ]),
          presentation.slot(
            'queue-editor',
            'queueEditor',
            {
              theme,
              locale,
              open: state.dialog === 'queue',
              ready: state.queueEdit?.phase === 'ready',
              hasAttachments: state.queueEdit?.hasAttachments ?? false,
              busy: mutationPending(state),
              uncertain: !!state.uncertain,
              checking: Object.values(state.pending).some((request) => request.kind === 'receipt'),
              command: state.queueEdit?.command ?? null,
              status: state.notice
                ? text[state.notice]
                : state.queueEdit?.phase === 'loading'
                  ? text.queueLoading
                  : '',
            } satisfies QueueEditorProps,
            {
              changed: 'changed',
              save: 'save',
              closed: 'closed',
              reload: 'reload',
              check: 'check',
            },
          ),
          ...(state.skills
            ? [
                presentation.slot(
                  'skill-library',
                  'skills',
                  {
                    theme,
                    locale,
                    open: state.dialog === 'skills',
                    workspace: state.selected?.workspace ?? null,
                  },
                  { closed: 'closed' },
                ),
              ]
            : []),
          presentation.slot(
            'help',
            'help',
            { theme, locale, open: state.dialog === 'help' },
            { closed: 'closed' },
          ),
          presentation.slot(
            'terminal-panel',
            'terminals',
            {
              theme,
              locale,
              open: state.dialog === 'terminal',
              enabled: state.terminals,
              busy: mutationPending(state),
              session: state.selected,
            },
            { closed: 'closed' },
          ),
          presentation.slot(
            'session-details',
            'details',
            {
              theme,
              locale,
              open: state.dialog === 'details',
              session: state.selected,
              busy: mutationPending(state),
            },
            { changed: 'changed', closed: 'closed' },
          ),
          presentation.slot(
            'directory-picker',
            'directories',
            {
              theme,
              locale,
              open: state.dialog === 'directories',
              busy: mutationPending(state) || catalogPending(state),
            },
            { selected: 'selected', closed: 'closed' },
          ),
          presentation.slot(
            'command-palette',
            'palette',
            {
              theme,
              locale,
              open: state.dialog === 'palette',
              busy: false,
              selectionDisabled:
                !!state.embedding.fixed || mutationPending(state) || catalogPending(state),
              commands: rootCommands(state, locale, target).map(
                ({ kind: _kind, ...command }) => command,
              ),
            },
            { invoked: 'invoked', selected: 'selected', closed: 'closed' },
          ),
        ],
        {
          gap: 0,
          terminal: { grow: state.dialog === 'queue' ? 0 : 1 },
          style: { width: fill, minWidth: 0 },
        },
      ),
      state.dialog !== null,
      state.dialog === 'terminal' ? { width: pixels(900) } : {},
      state.dialog === 'attachments'
        ? {
            width: 64,
            height: state.attachments.selection?.browse
              ? 20
              : Math.min(20, attached.length * 3 + 7),
          }
        : {},
    ),
  );
  if (target === 'tui')
    out.frame.view = appearanceView(
      presentation,
      state.preferences,
      theme,
      locale,
      out.frame.view,
      resolveLocale('auto', values.locale?.[0]),
    );
  if (!state.embedding.embedded)
    out.frame.surface =
      target === 'desktop'
        ? { kind: 'window', binding: 'surface', title: 'maka', width: 1100, height: 760 }
        : { kind: 'terminal', binding: 'surface' };
  if (!state.embedding.embedded)
    out.frame.view = presentation.paint(theme).box(
      'agent-screen-with-authoring',
      [
        out.frame.view,
        presentation
          .paint(theme)
          .box('authoring-background', [presentation.slot('authoring', 'authoring', {}, {})], {
            visible: false,
          }),
      ],
      { terminal: { grow: 1 }, style: { width: fill, height: fill } },
    );
  out.actions = actions;
  out.events = events;
  out.subscriptions = [
    ...(state.scheduleWatch && !state.embedding.fixed
      ? [
          {
            key: state.scheduleWatch,
            binding: 'agent',
            member: 0,
            method: 'scheduleAlerts',
            input: {},
            resources: {},
          },
        ]
      : []),
    ...(state.catalogWatch
      ? [
          {
            key: state.catalogWatch.key,
            binding: 'agent',
            member: 0,
            method: 'catalog',
            input: state.catalogWatch.input,
            resources: {},
          },
        ]
      : []),
    ...(state.watch
      ? [
          {
            key: state.watch.key,
            binding: 'agent',
            member: 0,
            method: 'watch',
            input: state.watch.input,
            resources: {},
          },
        ]
      : []),
  ];
  return out;
}
