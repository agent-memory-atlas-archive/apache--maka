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

import { scrollBoundary } from './scroll.js';
import { tuiText } from './tui-locale.js';
import type {
  Component,
  AgentSession,
  EditorObservation,
  TextareaProps,
  MarkdownProps,
  ReadingObservation,
  ReadingCommand,
  ComponentInput,
  UiExport,
  Action,
  InputProps,
  FilePasteObservation,
} from '../../packages/plugin-sdk/src/index.js';
import {
  composerContract,
  navigationContract,
  timelineContract,
  type ComposerProps,
  type NavigationProps,
  type TimelineProps,
  type ToolFileContext,
  type TimelineFileContext,
} from './contracts.js';
import {
  syncSearch,
  searchEvent,
  searchReady,
  historyView,
  searchCount,
  type ConversationSearch,
} from './conversation-search.js';
import {
  syncMessageCopy,
  messageCopyEvent,
  messageCopyObservation,
  type MessageCopy,
} from './message-copy.js';
import { copy } from './locale.js';
import { toolCards } from './tool-cards.js';
import { filePresentation, toolPresentation } from '../agent/tool-details/index.js';
import { messageView, sourceFieldLabel } from './message-view.js';
import { output, fill, pixels, compactLabel, type Presentation, type Node } from './view.js';
import { sameValue } from './value.js';
import type {
  AgentDataSource,
  EditorCheckpointObservation,
} from '../../packages/plugin-sdk/src/index.js';
interface Revision {
  revision: number;
}
interface Navigation extends Revision {
  toggled: string[];
  scrollRevision: number;
  focused: string | null;
}
interface SessionGroup {
  key: string;
  label: string;
  sessions: AgentSession[];
}
// Original sidebar order: first appearance in the catalog, archived last.
function sessionGroups(props: NavigationProps): SessionGroup[] {
  const groups: SessionGroup[] = [];
  for (const session of props.sessions.filter((session) => !session.archived)) {
    const key = `group-${session.workspace.identity}`;
    const group = groups.find((group) => group.key === key);
    if (group) group.sessions.push(session);
    else {
      const path = session.workspace.path;
      groups.push({
        key,
        label: path.split(/[/\\]/u).filter(Boolean).at(-1) || path,
        sessions: [session],
      });
    }
  }
  const archived = props.sessions.filter((session) => session.archived);
  if (archived.length)
    groups.push({
      key: 'archived',
      label: copy(props.locale).archivedSessions,
      sessions: archived,
    });
  return groups;
}
function groupOpen(state: Navigation, key: string) {
  return (key !== 'archived') !== state.toggled.includes(key);
}
interface DraftEntry {
  dataSource: AgentDataSource;
  checkpoint: EditorCheckpointObservation | null;
  retry: number;
  context: string;
  key: string;
  observation: EditorObservation | null;
  files: FilePasteObservation | null;
}
interface Draft extends Revision {
  active: string | null;
  serial: number;
  entries: DraftEntry[];
}
interface Reading extends Revision {
  copy: MessageCopy | null;
  browse: { request: number; id: string } | null;
  search: ConversationSearch | null;
  expanded: string[];
  observation: ReadingObservation | null;
  scrollRevision: number;
  command: ReadingCommand | null;
  landing: TimelineProps['landing'];
  files: Record<string, ToolFileContext>;
  fileContext: TimelineFileContext | null;
}
export function components<N extends Node>(
  presentation: Presentation<N>,
): {
  composer: Component<Draft, ComposerProps, N>;
  navigation: Component<Navigation, NavigationProps, N>;
  timeline: Component<Reading, TimelineProps, N>;
} {
  const target = presentation.target;
  const composerFrame = (
    state: Draft,
    props: ComposerProps,
    events: Parameters<typeof output>[2] = [],
  ) => {
    const ui = presentation.paint(props.theme),
      text = copy(props.locale);
    // Each session retains its original native mount, including selection and
    // undo. Only observed empty, idle editors can make room at the original limit.
    if (
      props.context &&
      props.dataSource &&
      !state.entries.some(
        (entry) => entry.context === props.context && sameValue(entry.dataSource, props.dataSource),
      )
    ) {
      if (state.entries.length === 32 && !props.busy && !props.command) {
        const empty = state.entries.findIndex(
          (entry) =>
            entry.checkpoint?.state.kind === 'ready' &&
            entry.checkpoint.state.saved &&
            entry.observation?.snapshot.text.bytes === 0 &&
            !entry.observation.snapshot.marked &&
            (!entry.files || entry.files.selection.phase === 'empty'),
        );
        if (empty >= 0) state.entries.splice(empty, 1);
      }
      if (state.entries.length < 32)
        state.entries.push({
          context: props.context,
          dataSource: props.dataSource,
          checkpoint: null,
          retry: 0,
          key: state.serial++ === 0 ? 'draft' : `draft-${state.serial}`,
          observation: null,
          files: null,
        });
    }
    const current = state.entries.find(
      (entry) => entry.context === props.context && sameValue(entry.dataSource, props.dataSource),
    );
    const observation = current?.observation;
    if (state.active !== (current?.key ?? null)) {
      state.active = current?.key ?? null;
      if (current?.checkpoint?.state.kind === 'restore') current.retry++;
    }
    const pending =
      props.command !== null &&
      !state.entries.some(
        (entry) =>
          entry.observation?.receipt?.owner === props.command!.owner &&
          entry.observation?.receipt?.sequence === props.command!.sequence,
      );
    const available =
      current?.checkpoint?.state.kind === 'ready' &&
      current.checkpoint.state.initialized &&
      props.enabled &&
      !props.busy &&
      (target === 'tui' || !props.running) &&
      !pending &&
      !!observation &&
      !observation.snapshot.marked &&
      (!!observation.snapshot.text.bytes || props.hasAttachments);
    const textarea: TextareaProps = {
      appearance:
        target === 'tui' ? { ...props.theme.textarea, borderWidth: 0 } : props.theme.textarea,
      menu: {
        appearance: props.theme.menu,
        labels: {
          title: text.edit,
          undo: text.undo,
          redo: text.redo,
          cut: text.cut,
          copy: text.copy,
          paste: text.paste,
          selectAll: text.selectAll,
        },
      },
      accessibilityLabel: text.draft,
      readOnly: false,
      disabled: false,
      softWrap: true,
      terminalRows: { min: 1, max: 8, maxViewportFraction: 1 / 3 },
      submit:
        target === 'tui'
          ? 'enter'
          : props.enabled && !props.busy && !props.running && !pending
            ? 'primary_enter'
            : null,
      tabIndex: 1,
      autoFocus: props.autoFocus,
      initialText: '',
      placeholder:
        target === 'tui' ? tuiText(props.locale, 'composer-placeholder') : text.placeholder,
      command: null,
      filePaste: null,
      fileCommand: null,
      persistence: null,
    };
    const editor = ui.box(
      'editor-area',
      state.entries.map((entry) =>
        ui.box(
          `retained-${entry.key}`,
          [
            presentation.slot(
              entry.key,
              'editor',
              {
                ...textarea,
                persistence: {
                  key: entry.context,
                  format: 'maka.agent.draft@1',
                  dataSource: entry.dataSource,
                  metadata: {
                    attachments: props.attachments
                      .filter(
                        (file) =>
                          file.sessionId === entry.context &&
                          sameValue(file.dataSource, entry.dataSource),
                      )
                      .map(({ dataSource: _, ...file }) => file),
                  },
                  restore:
                    props.drafts.find((draft) => draft.owner === entry.checkpoint?.owner)
                      ?.restore ?? null,
                  retry: entry.retry,
                },
                autoFocus: entry === current && props.autoFocus,
                command:
                  props.command?.owner === entry.observation?.snapshot.text.owner
                    ? props.command
                    : null,
                filePaste:
                  entry === current
                    ? (props.filePaste ?? null)
                    : target === 'tui'
                      ? { context: entry.context, maxFiles: 0 }
                      : null,
                fileCommand:
                  props.fileCommand?.selection.owner === entry.files?.selection.owner
                    ? props.fileCommand
                    : null,
              },
              {
                changed: 'changed',
                submitted: 'submitted',
                files: 'files',
                checkpoint: 'checkpoint',
              },
            ),
          ],
          {
            visible: entry === current,
            style: { width: fill, height: fill },
            terminal: { grow: 1, shrink: true },
          },
        ),
      ),
      {
        style: { width: fill, height: pixels(112), shrink: 0 },
        terminal: { basis: 0, grow: 1, shrink: true },
      },
    );
    const controls = ui.box(
      'composer-controls',
      [
        ...(target === 'desktop' ? [ui.text('hint', text.enterHint, 12, props.theme.muted)] : []),
        ...(props.running
          ? [
              ui.button(
                'stop',
                text.stop,
                props.busy,
                false,
                target === 'tui' ? { background: 0 } : {},
                undefined,
                text.stop,
              ),
              ...(target === 'tui'
                ? [
                    ui.button(
                      'send',
                      props.sendLabel ?? text.send,
                      !available,
                      true,
                      target === 'tui' ? { background: 0 } : {},
                      undefined,
                      text.send,
                    ),
                  ]
                : []),
            ]
          : [
              ui.button(
                'send',
                props.sendLabel ?? text.send,
                !available,
                true,
                target === 'tui' ? { background: 0 } : {},
                undefined,
                text.send,
              ),
            ]),
      ],
      {
        row: true,
        gap: target === 'tui' ? 0 : undefined,
        terminal: { shrink: false },
        style: { justify: 'between', align: 'center', width: fill },
      },
    );
    const recovering = props.drafts.find((draft) => draft.owner === current?.checkpoint?.owner);
    const failed = current?.checkpoint?.state.kind === 'failed' || recovering?.status === 'failed';
    const unknown = recovering?.status === 'unknown';
    const loading =
      !!current &&
      (!current.checkpoint ||
        current.checkpoint.state.kind === 'loading' ||
        current.checkpoint.state.kind === 'restore' ||
        (current.checkpoint.state.kind === 'ready' && !current.checkpoint.state.initialized));
    const status = failed
      ? text.draftPersistenceFailed
      : unknown
        ? text.draftUncertain
        : loading
          ? text.draftRestoring
          : null;
    const body = ui.box(
      'composer-main',
      [
        ...(props.context && !current
          ? [ui.text('draft-capacity', text.draftCapacity, 13, props.theme.muted)]
          : []),
        ...(target === 'tui'
          ? [
              ui.box(
                'composer-add-cell',
                [
                  ui.button(
                    'attach',
                    '+',
                    props.busy || !props.context || loading || failed || unknown,
                    false,
                    { background: 0 },
                    undefined,
                    text.attachmentTitle,
                  ),
                ],
                { terminal: { basis: 4 } },
              ),
            ]
          : []),
        editor,
        controls,
      ],
      {
        row: target === 'tui',
        border: target === 'tui',
        busy: props.running,
        gap: target === 'tui' ? 0 : 12,
        terminal: { shrink: true },
        style: { width: fill },
      },
    );
    return output(
      state,
      ui.box(
        'composer',
        [
          ...(status
            ? [
                ui.box(
                  'draft-status-row',
                  [
                    ui.text('draft-status', status, 13, props.theme.muted),
                    ...(failed || unknown
                      ? [ui.button('draft-retry', text.sourceRetry, false)]
                      : []),
                  ],
                  { row: true, terminal: { wrap: true }, style: { width: fill, wrap: true } },
                ),
              ]
            : []),
          body,
        ],
        { style: { width: fill }, terminal: { shrink: true } },
      ),
      events,
    );
  };
  const navigationFrame = (state: Navigation, props: NavigationProps) => {
    const ui = presentation.paint(props.theme);
    return output(
      state,
      presentation.reading(
        'session-list',
        {
          content: ui.box(
            'session-groups',
            sessionGroups(props).flatMap((group, index) => {
              const open = groupOpen(state, group.key);
              const heading = `${open ? (props.theme.ascii ? 'v' : '▾') : props.theme.ascii ? '>' : '▸'} ${group.label}  ${group.sessions.length}`;
              const label = ui.text(`${group.key}-label`, heading, 12, props.theme.muted, 500, {
                truncate: true,
              });
              return [
                ...(index ? [ui.box(`${group.key}-gap`, [], { terminal: { basis: 1 } })] : []),
                ui.item
                  ? ui.item(group.key, label, false, props.busy)
                  : ui.button(group.key, heading, props.busy, false, {
                      width: fill,
                      minWidth: 0,
                      justify: 'start',
                      truncate: true,
                    }),
                ...(open
                  ? group.sessions.map((session, at) => {
                      const current = session.id === props.selected;
                      const row = ui.item
                        ? ui.item(
                            session.id,
                            ui.box(
                              `session-row-${index}-${at}`,
                              [
                                ui.box(
                                  `session-status-${index}-${at}`,
                                  [
                                    ...(session.activity === 'working'
                                      ? [
                                          ui.activity(
                                            `session-glyph-${index}-${at}`,
                                            'orbit',
                                            props.theme.accent,
                                            '  ',
                                            ' ',
                                          ),
                                        ]
                                      : [
                                          ui.text(
                                            `session-glyph-${index}-${at}`,
                                            session.activity === 'waiting'
                                              ? props.theme.ascii
                                                ? '  ! '
                                                : '  ◇ '
                                              : session.hasUnread
                                                ? props.theme.ascii
                                                  ? '  * '
                                                  : '  ● '
                                                : '    ',
                                            13,
                                            session.activity === 'waiting'
                                              ? props.theme.warning
                                              : props.theme.accent,
                                          ),
                                        ]),
                                  ],
                                  { terminal: { basis: 4, shrink: false } },
                                ),
                                ui.box(
                                  `session-name-${index}-${at}`,
                                  [
                                    ui.text(
                                      `session-label-${index}-${at}`,
                                      session.title,
                                      13,
                                      session.archived ? props.theme.muted : props.theme.text,
                                      session.hasUnread ? 650 : 400,
                                      { truncate: true },
                                    ),
                                    ...(props.previews?.[session.id]
                                      ? [
                                          ui.text(
                                            `session-preview-${index}-${at}`,
                                            props.previews[session.id]!,
                                            12,
                                            props.theme.muted,
                                            400,
                                            { truncate: true },
                                          ),
                                        ]
                                      : []),
                                  ],
                                  { terminal: { grow: 1, shrink: true } },
                                ),
                                ...(session.activity === 'waiting'
                                  ? [
                                      ui.text(
                                        `session-waiting-${index}-${at}`,
                                        ` ${copy(props.locale).needsYou} `,
                                        12,
                                        props.theme.accent,
                                        400,
                                        { truncate: true },
                                      ),
                                    ]
                                  : []),
                              ],
                              { row: true, gap: 0 },
                            ),
                            current,
                            props.busy,
                            { followFocus: true, submit: true },
                          )
                        : ui.button(
                            session.id,
                            session.title +
                              (session.activity === 'waiting'
                                ? ' · ' + copy(props.locale).needsYou
                                : session.activity === 'working'
                                  ? ' · ' + copy(props.locale).working
                                  : session.hasUnread
                                    ? ' · ' + copy(props.locale).unread
                                    : ''),
                            props.busy,
                            false,
                            {
                              width: fill,
                              minWidth: 0,
                              justify: 'start',
                              truncate: true,
                              ...(current ? { background: props.theme.selected } : {}),
                            },
                          );
                      return ui.contextMenu
                        ? ui.contextMenu('session-menu-' + session.id, session.title, row, [
                            ui.button(
                              'details-' + session.id,
                              copy(props.locale).detailsTitle,
                              props.busy,
                            ),
                            ...(target === 'tui'
                              ? [
                                  ui.button(
                                    'usage-' + session.id,
                                    tuiText(props.locale, 'insights-this-session'),
                                    props.busy,
                                  ),
                                ]
                              : []),
                          ])
                        : row;
                    })
                  : []),
              ];
            }),
            {
              scroll: 'anchor',
              focusGroup: target === 'tui',
              gap: target === 'tui' ? 0 : 2,
              style: { width: fill, height: pixels(0), grow: 1, minHeight: 0 },
              terminal: { grow: 1 },
            },
          ),
        },
        { input: 'session-input', observed: 'session-viewport' },
      ),
    );
  };
  const focusMessage = (state: Reading, id: string) => {
    state.browse!.id = id;
    state.command = {
      sequence: (state.command?.sequence ?? 0) + 1,
      target: { kind: 'focus_item', key: 'browse-' + id },
    };
  };
  const syncBrowse = (state: Reading, props: TimelineProps) => {
    if (target !== 'tui' || !props.browse || props.source || props.search || props.loading) {
      state.browse = null;
      return;
    }
    const rows = toolCards(props.messages);
    if (
      state.browse?.request === props.browse &&
      rows.some(({ message }) => message.id === state.browse!.id)
    )
      return;
    // Search owns its clear receipt before the next focus command is issued.
    if (
      state.command?.target.kind === 'clear_find' &&
      (state.observation?.receipt?.sequence !== state.command.sequence ||
        state.observation.receipt.status !== 'applied')
    )
      return;
    const selected =
      state.observation?.selectedChildKey ?? state.observation?.viewport?.visibleChildKeys[0];
    const row = rows.find(({ message }) => 'message-' + message.id === selected) ?? rows[0];
    if (!row) {
      state.browse = null;
      return;
    }
    state.browse = { request: props.browse, id: row.message.id };
    focusMessage(state, row.message.id);
  };
  const timelineFrame = (
    state: Reading,
    props: TimelineProps,
    previousCopy: ReturnType<typeof messageCopyObservation> = null,
  ) => {
    const ui = presentation.paint(props.theme),
      text = copy(props.locale);
    const search = state.search?.closing ? null : state.search;
    syncBrowse(state, props);
    const rows = toolCards(props.messages);
    const browsing = state.browse;
    const validFile = (id: string, context: ToolFileContext) => {
      const message = rows.find(({ message }) => message.id === id)?.message;
      return (
        context.sessionId === props.toolContext?.sessionId &&
        context.through === props.toolContext.through &&
        context.toolUseId === message?.tool?.toolUseId &&
        !!filePresentation(message.tool.call?.presentation) &&
        context.presentation === (message.tool.call?.presentation ?? null) &&
        context.call === (message.tool.call?.sequence ?? null) &&
        context.result === (message.tool.result?.sequence ?? null)
      );
    };
    state.files = Object.fromEntries(
      Object.entries(state.files).filter(([id, context]) => validFile(id, context)),
    );
    const tools = rows
      .map(({ message }) => message)
      .filter(
        (message) =>
          (message.role === 'tool_call' || message.role === 'tool_result') &&
          (message.source !== null || message.text.length > 0),
      );
    state.expanded = state.expanded.filter((id) => tools.some((message) => message.id === id));
    const messages: N[] = rows.flatMap(({ message, result }) => {
      const tool = message.role === 'tool_call' || message.role === 'tool_result';
      const expanded = state.expanded.includes(message.id);
      const structured =
        !!tool &&
        !!props.toolContext &&
        !!message.tool &&
        !!toolPresentation(message.tool.call?.presentation);
      const sourceAction = ui.button(
        'source-' + message.id + (message.role === 'thinking' ? '-thinking' : ''),
        text.readSource,
        props.loading,
        false,
        {
          padding: { top: 4, right: 0, bottom: 4, left: 0 },
          text: {
            foreground: props.theme.muted,
            font: {
              family: '.SystemUIFont',
              fallbacks: [],
              size: 12,
              lineHeight: 1.55,
              weight: 400,
            },
          },
        },
      );

      const body =
        (message.role === 'assistant' || message.role === 'thinking') && !message.continued
          ? presentation.slot(
              'text-' + message.contentKey,
              'markdown',
              {
                source: message.text,
                appearance:
                  message.role === 'thinking' && target === 'tui'
                    ? {
                        ...props.theme.markdown,
                        body: { ...props.theme.markdown.body, foreground: props.theme.thinking },
                        terminal: props.theme.terminal
                          ? { ...props.theme.terminal, foreground: props.theme.terminal.thinking }
                          : null,
                      }
                    : props.theme.markdown,
                animate: props.live && props.theme.motion,
                ascii: props.theme.ascii,
                focusOnPointer: !search,
                selectable: true,
                disabled: false,
              } satisfies MarkdownProps,
              {},
            )
          : presentation.slot(
              'text-' + message.contentKey,
              'source',
              {
                source: message.text,
                focusOnPointer: !search,
                appearance: props.theme.markdown.body,
                selection: props.theme.markdown.selection,
                terminal: props.theme.terminal,
                disabled: false,
              },
              {},
            );
      const messageNode = messageView(presentation, target, props.theme, props.locale, {
        ...message,
        browse: browsing
          ? { key: 'browse-' + message.id, current: browsing.id === message.id }
          : undefined,
        waiting: !!message.tool && props.waitingTools.includes(message.tool.toolUseId),
        live: !!message.tool && props.liveTools.includes(message.tool.toolUseId),
        expanded,
        structured,
        toggle: tool && (message.source !== null || message.text) ? 'toggle-' + message.id : null,
        text: result ? [message.text, result.text].filter(Boolean).join(' · ') : message.text,
        continued: message.continued || !!result?.continued,
        body: structured
          ? presentation.slot(
              'tool-details-' + message.id,
              'tools',
              {
                theme: props.theme,
                locale: props.locale,
                context: props.toolContext,
                tool: message.tool,
                expanded,
                focusOnPointer: !search,
              },
              { fileContext: 'fileContext' },
            )
          : tool
            ? ui.box(
                'tool-body-' + message.id,
                [
                  body,
                  ...(result
                    ? [
                        presentation.slot(
                          'text-' + result.contentKey,
                          'source',
                          {
                            source: result.text,
                            focusOnPointer: !search,
                            appearance: props.theme.markdown.body,
                            selection: props.theme.markdown.selection,
                            terminal: props.theme.terminal,
                            disabled: false,
                          },
                          {},
                        ),
                      ]
                    : []),
                ],
                { surface: true, style: { width: fill } },
              )
            : body,
        actions: [
          ...(message.source !== null && message.tool?.call
            ? [
                ui.button(
                  'tool-source-' + message.id + '-arguments',
                  text.toolArguments,
                  props.loading,
                ),
              ]
            : []),
          ...(message.source !== null && message.tool?.result
            ? [ui.button('tool-source-' + message.id + '-result', text.toolResult, props.loading)]
            : []),
          ...(message.image && message.source !== null
            ? [ui.button('image-' + message.id, text.viewImage, props.loading)]
            : []),
          ...(message.source !== null && message.text
            ? [
                ui.button(
                  'source-' + message.id + (message.role === 'thinking' ? '-thinking' : ''),
                  text.readSource,
                  props.loading,
                ),
              ]
            : []),
        ],
        sourceActions:
          message.source !== null
            ? [
                ui.box(
                  'source-actions-' + message.id,
                  [
                    sourceAction,
                    ...(props.markReadEnabled &&
                    props.toolContext &&
                    !search &&
                    (message.role === 'user' ||
                      message.role === 'assistant' ||
                      message.role === 'thinking')
                      ? [
                          ui.button(
                            'mark-read-' +
                              message.id +
                              (message.role === 'thinking' ? '-thinking' : ''),
                            text.markRead,
                            props.loading,
                          ),
                        ]
                      : []),
                  ],
                  {
                    row: true,
                    gap: target === 'desktop' ? 12 : 1,
                    terminal: { wrap: true },
                    style: { width: fill, wrap: true },
                    surface: message.role === 'user',
                  },
                ),
              ]
            : [],
      });
      const timing = props.timings.find(
        (timing) => timing.afterMessageId === message.id || timing.afterMessageId === result?.id,
      );
      if (!timing) return [messageNode];
      const mark = timing.finished
        ? {
            completed: tuiText(props.locale, 'task-ended'),
            failed: tuiText(props.locale, 'task-failed'),
            aborted: tuiText(props.locale, 'task-stopped'),
          }[timing.finished.outcome]
        : props.theme.ascii
          ? '.'
          : '·';
      return [
        messageNode,
        presentation.slot(
          'time-' + message.contentKey,
          'label',
          {
            text: mark + ' ',
            elapsed: { from: timing.startedAt, to: timing.finished?.at ?? null },
            terminalForeground: props.theme.terminal
              ? timing.finished
                ? props.theme.terminal.muted
                : props.theme.terminal.accent
              : null,
            appearance: {
              font: { ...props.theme.markdown.body.font, size: 12 },
              foreground: timing.finished ? props.theme.muted : props.theme.accent,
            },
          },
          {},
        ),
      ];
    });
    const content = ui.box(
      'timeline',
      [
        ...(target === 'desktop' && !messages.length && !props.loading
          ? [
              ui.box(
                'welcome-column',
                [
                  ui.text('welcome', text.welcome, 30, props.theme.text, 600),
                  ui.text('welcome-copy', text.welcomeCopy, 16, props.theme.muted),
                ],
                {
                  gap: target === 'desktop' ? 12 : 1,
                  style: { width: fill, maxWidth: 760, shrink: 0 },
                },
              ),
            ]
          : messages),
        ...(props.loading && target !== 'tui'
          ? [ui.text('loading', text.loading, 14, props.theme.muted)]
          : []),
      ],
      {
        gap: target === 'desktop' ? 24 : 1,
        style: {
          width: fill,
          height: fill,
          align: 'center',
          padding: { top: 24, right: 24, bottom: 24, left: 24 },
        },
        focusGroup: !!browsing,
        scroll: props.live && !props.browse ? 'follow_end' : 'anchor',
        terminal: { grow: 1 },
      },
    );
    const controls: N[] = props.source
      ? [
          ui.button('source-back', text.sourceBack),
          ...props.source.fields
            .filter(
              (field) => field !== (props.source!.kind === 'message' ? 'body' : props.source!.kind),
            )
            .map((field) =>
              ui.button(
                'source-field-' + field,
                sourceFieldLabel(props.locale, field),
                props.source!.loading,
              ),
            ),
          ...(!props.source.atStart
            ? [ui.button('source-first', text.sourceFirst, props.source.loading)]
            : []),
          ...(!props.source.atEnd
            ? [ui.button('source-last', text.sourceLast, props.source.loading)]
            : []),
          ...(props.source.failed ? [ui.button('source-retry', text.sourceRetry)] : []),
        ]
      : target === 'tui'
        ? props.latest || (state.observation?.viewport && !state.observation.viewport.atEnd)
          ? [
              ui.box('reading-control-space', [], { terminal: { grow: 1 } }),
              ui.button(
                props.latest ? 'latest' : 'end',
                props.theme.ascii ? 'v' : '↓',
                props.loading,
                false,
                { background: 0 },
                undefined,
                props.latest ? text.latest : text.toEnd,
              ),
            ]
          : []
        : [
            ...(props.older ? [ui.button('older', text.older, props.loading)] : []),
            ...(props.newer ? [ui.button('newer', text.newer, props.loading)] : []),
            ...(props.latest ? [ui.button('latest', text.latest, props.loading)] : []),
            ...(state.observation?.viewport && !state.observation.viewport.atEnd
              ? [ui.button('end', text.toEnd, state.observation.pendingCommand != null)]
              : []),
          ];
    const out = output(
      state,
      ui.box(
        'timeline-frame',
        [
          ...(search
            ? [
                ui.box(
                  'find-bar',
                  [
                    ui.button('find-scope', search.history ? '∞' : '⌕'),
                    ui.box(
                      'find-scope-label',
                      [
                        ui.item!(
                          'find-scope-label-input',
                          ui.text(
                            'find-scope-text',
                            search.history ? text.findHistory : text.findLocal,
                            12,
                            props.theme.muted,
                            400,
                            { truncate: true },
                          ),
                          false,
                          false,
                        ),
                      ],
                      { gap: 0, terminal: { basis: 0 } },
                    ),
                    ui.box(
                      'find-editor',
                      [
                        presentation.slot(
                          search.key,
                          'input',
                          {
                            appearance: { ...props.theme.input, borderWidth: 0 },
                            accessibilityLabel: text.findConversation,
                            readOnly: false,
                            disabled: false,
                            tabIndex: 0,
                            autoFocus: true,
                            maxBytes: 512,
                            terminalPaste: 'spaces',
                            placeholder: text.findPlaceholder,
                          } satisfies InputProps,
                          { changed: 'find-changed', submitted: 'find-submitted' },
                        ),
                      ],
                      { terminal: { grow: 1, shrink: true }, style: { grow: 1, minWidth: 0 } },
                    ),
                    ui.text(
                      'find-count',
                      search.failed ? text.findFailed : searchCount(state),
                      12,
                      props.theme.muted,
                    ),
                    ui.button('find-previous', '↑', !searchReady(search)),
                    ui.button('find-next', '↓', !searchReady(search)),
                    ui.button('find-close', '×'),
                  ],
                  {
                    row: true,
                    gap: 1,
                    wide: { min_width: 60, direction: 'row', children: { 'find-scope-label': {} } },
                    terminal: { basis: 1, shrink: false },
                    style: { width: fill, shrink: 0 },
                  },
                ),
              ]
            : []),
          ui.box(
            'retained-conversation',
            [
              presentation.reading(
                'reading',
                { content, command: state.command },
                { observed: 'reading-observed', input: 'reading-input' },
              ),
            ],
            {
              visible: !props.source && !search?.history,
              style: { width: fill, height: pixels(0), grow: 1, shrink: 1, minHeight: 0 },
              terminal: { grow: 1 },
            },
          ),
          ...(search?.history ? [historyView(presentation, props, search)] : []),
          ...(props.source
            ? [
                (() => {
                  const source = props.source;
                  const sourceContent = ui.box(
                    'source-document',
                    [
                      ui.text(
                        'source-heading',
                        source.kind === 'arguments'
                          ? text.toolArguments
                          : source.kind === 'result'
                            ? text.toolResult
                            : source.kind === 'receipt'
                              ? text.findFieldReceipt
                              : text.sourceTitle,
                        20,
                        props.theme.text,
                        600,
                      ),
                      ui.text(
                        'source-location',
                        !source.ready
                          ? source.failed
                            ? text.sourceFailed
                            : text.loading
                          : source.atStart && source.atEnd
                            ? text.sourceComplete
                            : source.atStart
                              ? text.sourceBeginning
                              : source.atEnd
                                ? text.sourceEnding
                                : text.partial,
                        13,
                        props.theme.muted,
                      ),
                      presentation.slot(
                        'source-text-' + source.key,
                        'source',
                        {
                          source: source.text,
                          appearance:
                            source.format === 'text'
                              ? props.theme.markdown.body
                              : props.theme.markdown.code,
                          selection: props.theme.markdown.selection,
                          terminal: props.theme.terminal,
                          disabled: false,
                        },
                        {},
                      ),
                      ...(source.loading
                        ? [ui.text('source-loading', text.loading, 13, props.theme.muted)]
                        : []),
                      ...(source.failed
                        ? [ui.text('source-failed', text.sourceFailed, 13, props.theme.muted)]
                        : []),
                    ],
                    {
                      gap: target === 'desktop' ? 16 : 1,
                      scroll: 'anchor',
                      terminal: { grow: 1 },
                      style: {
                        width: fill,
                        height: fill,
                        padding: { top: 24, right: 24, bottom: 24, left: 24 },
                      },
                    },
                  );
                  return ui.box(
                    'source-pane',
                    [
                      presentation.reading(
                        'source-reading',
                        {
                          content: sourceContent,
                          command: {
                            sequence: source.navigation,
                            target: { kind: source.landing },
                          },
                        },
                        { input: 'source-input', observed: 'source-observed' },
                      ),
                    ],
                    {
                      style: { width: fill, height: pixels(0), grow: 1, shrink: 1, minHeight: 0 },
                      terminal: { grow: 1 },
                    },
                  );
                })(),
              ]
            : []),
          ...(browsing ? [ui.text('browse-hint', text.browseHint, 12, props.theme.muted)] : []),
          ...(!search?.history && controls.length
            ? [
                ui.box('reading-controls', controls, {
                  row: true,
                  gap: target === 'desktop' ? 8 : 1,
                  terminal: { wrap: true },
                  style: { width: fill, shrink: 0, wrap: true },
                }),
              ]
            : []),
        ],
        {
          shortcuts: search
            ? [
                { key: 'f', modifiers: ['alt'], input: 'find-scope', enabled: true },
                {
                  key: 'r',
                  modifiers: ['control'],
                  input: 'find-restart',
                  enabled: !!search.history,
                },
                { key: 'up', modifiers: [], input: 'find-previous', enabled: !!search.history },
                { key: 'down', modifiers: [], input: 'find-next', enabled: !!search.history },
                { key: 'pageup', modifiers: [], input: 'find-page-up', enabled: !!search.history },
                {
                  key: 'pagedown',
                  modifiers: [],
                  input: 'find-page-down',
                  enabled: !!search.history,
                },
                { key: 'escape', modifiers: [], input: 'find-close', enabled: true },
                { key: 'tab', modifiers: [], input: 'find-close', enabled: true },
                { key: 'tab', modifiers: ['shift'], input: 'find-close', enabled: true },
                { key: 'c', modifiers: ['control'], input: 'find-close', enabled: true },
                { key: 'f3', modifiers: [], input: 'find-next', enabled: true },
                { key: 'f3', modifiers: ['shift'], input: 'find-previous', enabled: true },
                { key: 'enter', modifiers: ['shift'], input: 'find-previous', enabled: true },
              ]
            : browsing
              ? [
                  { key: 'escape', modifiers: [], input: 'browse-close', enabled: true },
                  { key: 'tab', modifiers: [], input: 'browse-close', enabled: true },
                  { key: 'tab', modifiers: ['shift'], input: 'browse-close', enabled: true },
                  { key: 'up', modifiers: [], input: 'browse-previous', enabled: true },
                  { key: 'down', modifiers: [], input: 'browse-next', enabled: true },
                  { key: 'left', modifiers: [], input: 'browse-collapse', enabled: true },
                  { key: 'right', modifiers: [], input: 'browse-expand', enabled: true },
                ]
              : [],
          gap: target === 'desktop' ? 8 : 1,
          style: { width: fill, height: fill },
          terminal: { grow: 1 },
        },
      ),
    );
    const selected =
      state.observation?.selectedChildKey ??
      (browsing ? 'message-' + browsing.id : state.observation?.viewport?.visibleChildKeys[0]);
    const row = rows.find(
      ({ message }) =>
        'message-' + message.id === selected || 'time-' + message.contentKey === selected,
    );
    const context: TimelineFileContext | null = props.toolContext
      ? {
          ...props.toolContext,
          landing: props.landing.sequence,
          path:
            target === 'tui' && !props.source && !search && !props.loading && row
              ? (state.files[row.message.id]?.path ?? null)
              : null,
        }
      : null;
    if (
      props.browse &&
      (state.command?.target.kind === 'focus_item' ||
        state.command?.target.kind === 'clear_find') &&
      state.observation?.receipt?.sequence === state.command.sequence &&
      state.observation.receipt.status !== 'applied'
    )
      out.events.push({ name: 'browseClosed', value: { request: props.browse } });
    if (!sameValue(state.fileContext, context)) {
      state.fileContext = context;
      out.events.push({ name: 'fileContext', value: context });
    }
    syncMessageCopy(state, props, target === 'tui' ? (row?.message ?? null) : null, out.actions);
    const copied = messageCopyObservation(state);
    if (!sameValue(previousCopy, copied)) {
      out.events.push({ name: 'messageCopy', value: copied });
    }
    return out;
  };
  return {
    composer: {
      mount: (props) => composerFrame({ revision: 0, serial: 0, active: null, entries: [] }, props),
      update: (state, props) =>
        composerFrame(
          { ...state, entries: [...state.entries], revision: state.revision + 1 },
          props,
        ),
      event(state, input, props) {
        const next = {
          ...state,
          entries: state.entries.map((entry) => ({ ...entry })),
          revision: state.revision + 1,
        };
        const source =
          input.kind === 'component'
            ? next.entries.find((entry) => entry.key === input.child)
            : null;
        if (input.kind === 'component' && !source) return composerFrame(next, props);
        if (input.kind === 'component' && source && input.name === 'checkpoint') {
          const observation = input.event as EditorCheckpointObservation;
          source.checkpoint =
            observation.state.kind === 'restore'
              ? { ...observation, state: { ...observation.state, metadata: null } }
              : observation;
          return composerFrame(next, props, [
            {
              name: 'checkpoint',
              value: {
                context: source.context,
                dataSource: source.dataSource,
                observation,
                owners: next.entries.flatMap((entry) =>
                  entry.observation
                    ? [entry.observation.snapshot.text.owner]
                    : entry.checkpoint
                      ? [entry.checkpoint.owner]
                      : [],
                ),
              },
            },
          ]);
        }
        if (input.kind === 'component' && source && input.name === 'files') {
          source.files = input.event as FilePasteObservation;
          return composerFrame(next, props, [
            { name: 'files', value: { ...source.files, dataSource: source.dataSource } },
          ]);
        }
        if (
          input.kind === 'component' &&
          source &&
          (input.name === 'changed' || input.name === 'submitted')
        ) {
          source.observation = input.event as EditorObservation;
          if (input.name === 'changed')
            return composerFrame(next, props, [{ name: 'changed', value: source.observation }]);
        }
        const current = next.entries.find(
          (entry) =>
            entry.context === props.context && sameValue(entry.dataSource, props.dataSource),
        );
        const observation = current?.observation;
        if (input.kind === 'raw' && input.key === 'draft-retry' && current?.checkpoint) {
          if (current.checkpoint.state.kind === 'failed') {
            current.retry++;
            return composerFrame(next, props);
          }
          return composerFrame(next, props, [
            { name: 'draftCheck', value: { owner: current.checkpoint.owner } },
          ]);
        }

        if (
          ((input.kind === 'raw' && input.key === 'send') ||
            (input.kind === 'component' && source === current && input.name === 'submitted')) &&
          current?.checkpoint?.state.kind === 'ready' &&
          current.checkpoint.state.initialized &&
          props.enabled &&
          !props.busy &&
          (target === 'tui' || !props.running) &&
          observation &&
          (observation.snapshot.text.bytes > 0 || props.hasAttachments) &&
          !observation.snapshot.marked &&
          (!props.command ||
            next.entries.some(
              (entry) =>
                entry.observation?.receipt?.owner === props.command!.owner &&
                entry.observation?.receipt?.sequence === props.command!.sequence,
            ))
        ) {
          return composerFrame(next, props, [{ name: 'send', value: observation }]);
        }
        if (input.kind === 'raw' && input.key === 'attach' && props.context && !props.busy)
          return composerFrame(next, props, [{ name: 'attachments', value: {} }]);
        if (input.kind === 'raw' && input.key === 'stop' && props.running && !props.busy)
          return composerFrame(next, props, [{ name: 'stop', value: {} }]);
        return composerFrame(next, props);
      },
      capture(state, props, method, input) {
        const owner = (input as { owner?: string }).owner;
        const entry = state.entries.find((entry) =>
          method === 'file'
            ? entry.files?.selection.owner === owner
            : owner
              ? entry.observation?.snapshot.text.owner === owner
              : entry.context === props.context,
        );
        if (!entry) throw new Error('Original draft is unavailable');
        return { kind: 'forward', source: { child: entry.key, method, input } };
      },
    },
    navigation: {
      mount: (props) =>
        navigationFrame({ revision: 0, toggled: [], scrollRevision: 0, focused: null }, props),
      update: (state, props) => navigationFrame({ ...state, revision: state.revision + 1 }, props),
      event(state, input, props) {
        const next = { ...state, revision: state.revision + 1 };
        const events: Parameters<typeof output>[2] = [];
        if (input.kind === 'component' && input.child === 'session-list') {
          if (input.name === 'input') input = input.event as ComponentInput;
          else if (input.name === 'observed') {
            const observed = input.event as ReadingObservation;
            const scroll = observed.scroll && observed.scroll.readingRevision > next.scrollRevision;
            if (
              scroll &&
              observed.viewport?.readingRevision === observed.readingRevision &&
              observed.pendingCommand == null
            )
              next.scrollRevision = observed.scroll!.readingRevision;
            if (
              !props.busy &&
              !props.loading &&
              observed.pendingCommand == null &&
              observed.viewport?.readingRevision === observed.readingRevision
            ) {
              if (
                props.newer &&
                scroll &&
                observed.scroll!.direction === 'down' &&
                observed.viewport.atEnd
              )
                events.push({ name: 'newer', value: {} });
              else if (
                props.older &&
                scroll &&
                observed.scroll!.direction === 'up' &&
                observed.viewport.atStart
              )
                events.push({ name: 'older', value: {} });
            }
          }
        }
        if (input.kind === 'raw' && !props.busy) {
          const groups = sessionGroups(props);
          const group = groups.find((group) => group.key === input.key);
          if (group)
            next.toggled = next.toggled.includes(group.key)
              ? next.toggled.filter((key) => key !== group.key)
              : [...next.toggled, group.key];
          else {
            const session = groups
              .filter((group) => groupOpen(next, group.key))
              .flatMap((group) => group.sessions)
              .find(
                (session) =>
                  session.id === input.key ||
                  'details-' + session.id === input.key ||
                  'usage-' + session.id === input.key,
              );
            if (
              session &&
              target === 'tui' &&
              (input.event as { kind?: string }).kind === 'activate' &&
              input.key === session.id
            ) {
              const at = props.sessions.indexOf(session);
              if (!props.loading && next.focused !== session.id) {
                if (props.newer && at >= props.sessions.length - 3)
                  events.push({ name: 'newer', value: {} });
                else if (props.older && at < 3) events.push({ name: 'older', value: {} });
              }
              next.focused = session.id;
            } else if (session)
              events.push({
                name: 'select',
                value: {
                  sessionId: session.id,
                  details: input.key === 'details-' + session.id,
                  ...(input.key === 'usage-' + session.id ? { usage: true } : {}),
                },
              });
          }
        }
        const out = navigationFrame(next, props);
        out.events.push(...events);
        return out;
      },
    },
    timeline: {
      mount: (props) => {
        const initial: Reading = {
          copy: null,
          revision: 0,
          browse: null,
          search: null,
          expanded: [],
          files: {},
          fileContext: null,
          observation: null,
          scrollRevision: 0,
          command:
            props.landing.sequence > 0
              ? { sequence: 1, target: { kind: props.landing.edge } }
              : null,
          landing: props.landing,
        };
        syncSearch(initial, target === 'tui' ? props.search : null);
        return timelineFrame(initial, props);
      },
      update(state, props) {
        if (
          props.landing.sequence < state.landing.sequence ||
          (props.landing.sequence === state.landing.sequence &&
            props.landing.edge !== state.landing.edge)
        )
          throw new Error('Invalid reading landing sequence');
        const next: Reading = {
          ...state,
          browse:
            state.landing.sequence === props.landing.sequence && state.browse
              ? { ...state.browse }
              : null,
          revision: state.revision + 1,
          command:
            state.landing.sequence !== props.landing.sequence
              ? {
                  sequence: (state.command?.sequence ?? 0) + 1,
                  target: { kind: props.landing.edge },
                }
              : state.command,
          landing: props.landing,
          observation: state.landing.sequence === props.landing.sequence ? state.observation : null,
        };
        if (state.browse && state.browse.request === props.browse && !next.browse) {
          // A page split within one message replaces the bounded window. Follow
          // its existing landing edge instead of restarting at the first row.
          const rows = toolCards(props.messages);
          const row = props.landing.edge === 'end' ? rows.at(-1) : rows[0];
          if (row) {
            next.browse = { ...state.browse };
            focusMessage(next, row.message.id);
          }
        }
        syncSearch(next, target === 'tui' ? props.search : null);
        return timelineFrame(next, props, messageCopyObservation(state));
      },
      event(state, input, props) {
        const next = {
          ...state,
          browse: state.browse ? { ...state.browse } : null,
          revision: state.revision + 1,
        };
        if (
          input.kind === 'component' &&
          (['reading', 'source-reading', 'find-results-reading'].includes(input.child) ||
            input.child.startsWith('find-preview-reading-'))
        ) {
          if (input.child === 'reading' && input.name === 'observed')
            next.observation = input.event as ReadingObservation;
          if (input.name === 'input') input = input.event as ComponentInput;
        }
        if (
          input.kind === 'component' &&
          input.name === 'fileContext' &&
          input.child.startsWith('tool-details-')
        ) {
          next.files = {
            ...state.files,
            [input.child.slice('tool-details-'.length)]: input.event as ToolFileContext,
          };
        }
        if (
          next.browse &&
          props.browse === next.browse.request &&
          !props.source &&
          !props.search &&
          !props.loading &&
          input.kind === 'raw'
        ) {
          const rows = toolCards(props.messages).map(({ message }) => message);
          const selected = rows.find((message) => input.key === 'browse-' + message.id);
          const at = rows.findIndex((message) => message.id === next.browse!.id);
          let toggle = false;
          if (selected) {
            next.browse.id = selected.id;
            // The original Item distinguishes focus-following activation from Enter.
            toggle = (input.event as { kind?: string }).kind === 'submit';
          } else if (input.key === 'browse-previous' || input.key === 'browse-next') {
            const forward = input.key === 'browse-next';
            if (
              at >= 0 &&
              (forward ? at === rows.length - 1 && props.newer : at === 0 && props.older)
            ) {
              // Extend the original bounded history window without moving focus
              // on an asynchronous reply. The next arrow enters the added rows.
              const out = timelineFrame(next, props, messageCopyObservation(state));
              out.events.push({ name: forward ? 'newer' : 'older', value: {} });
              return out;
            }
            const row = rows[Math.max(0, Math.min(rows.length - 1, at + (forward ? 1 : -1)))];
            if (row) focusMessage(next, row.id);
          }
          const row = rows.find((message) => message.id === next.browse!.id);
          if (
            row &&
            (row.role === 'tool_call' || row.role === 'tool_result') &&
            (row.source !== null || row.text)
          ) {
            const expanded = next.expanded.includes(row.id);
            const open = input.key === 'browse-expand' || (toggle && !expanded);
            if (toggle || input.key === 'browse-expand' || input.key === 'browse-collapse')
              next.expanded = open
                ? [...new Set([...next.expanded, row.id])]
                : next.expanded.filter((id) => id !== row.id);
          }
          if (input.key === 'browse-close') {
            const request = next.browse.request;
            const out = timelineFrame(next, props, messageCopyObservation(state));
            out.events.push({ name: 'browseClosed', value: { request } });
            return out;
          }
        }
        const actions: Action[] = [];
        if (messageCopyEvent(next, input, actions)) {
          const out = timelineFrame(next, props, messageCopyObservation(state));
          out.actions.push(...actions.filter((action) => action.key === next.copy?.pending?.key));
          return out;
        }
        const handled = searchEvent(next, input, props, actions);
        if (handled) {
          const out = timelineFrame(next, props, messageCopyObservation(state));
          out.actions.push(...actions);
          if (handled === 'closed') out.events.push({ name: 'searchClosed', value: {} });
          return out;
        }
        if (
          input.kind === 'raw' &&
          input.key === 'end' &&
          next.observation?.viewport &&
          !next.observation.viewport.atEnd &&
          next.observation.pendingCommand == null
        )
          next.command = { sequence: (next.command?.sequence ?? 0) + 1, target: { kind: 'end' } };
        if (input.kind === 'raw' && !props.source) {
          const tool = toolCards(props.messages)
            .map(({ message }) => message)
            .find(
              (message) =>
                (message.role === 'tool_call' || message.role === 'tool_result') &&
                (message.source !== null || message.text.length > 0) &&
                input.key === 'toggle-' + message.id,
            );
          if (tool)
            next.expanded = next.expanded.includes(tool.id)
              ? next.expanded.filter((id) => id !== tool.id)
              : [...next.expanded, tool.id];
        }
        const out = timelineFrame(next, props, messageCopyObservation(state));
        const observed = next.observation;
        if (
          target === 'tui' &&
          props.live &&
          !props.latest &&
          props.older &&
          !props.source &&
          !props.browse &&
          !next.search &&
          !props.loading &&
          input.kind === 'component' &&
          input.child === 'reading' &&
          input.name === 'observed' &&
          observed?.viewport?.atStart &&
          observed.viewport.atEnd &&
          observed.viewport.readingRevision === observed.readingRevision &&
          observed.pendingCommand == null
        )
          out.events.push({ name: 'fill', value: {} });
        if (
          target === 'tui' &&
          !props.source &&
          !next.search &&
          !props.loading &&
          input.kind === 'component' &&
          input.child === 'reading' &&
          input.name === 'observed' &&
          observed?.scroll &&
          observed.scroll.readingRevision > next.scrollRevision &&
          observed.viewport?.readingRevision === observed.readingRevision &&
          observed.pendingCommand == null
        ) {
          next.scrollRevision = observed.scroll.readingRevision;
          const rows = toolCards(props.messages);
          const edgeRows = Math.max(1, Math.min(2, Math.floor(rows.length / 2)));
          const visible = new Set(observed.viewport.visibleChildKeys);
          const visibleRow = ({ message }: (typeof rows)[number]) =>
            visible.has('message-' + message.id) || visible.has('time-' + message.contentKey);
          if (
            observed.scroll.direction === 'up' &&
            props.older &&
            (observed.viewport.atStart || rows.slice(0, edgeRows).some(visibleRow))
          )
            out.events.push({ name: 'older', value: {} });
          else if (
            observed.scroll.direction === 'down' &&
            props.newer &&
            (observed.viewport.atEnd || rows.slice(-edgeRows).some(visibleRow))
          )
            out.events.push({ name: 'newer', value: {} });
          else if (
            observed.scroll.direction === 'down' &&
            observed.viewport.atEnd &&
            !props.newer &&
            (!props.live || props.latest)
          )
            out.events.push({ name: 'latest', value: {} });
        }
        if (
          !props.source &&
          !props.browse &&
          !next.search &&
          input.kind === 'component' &&
          input.child === 'reading' &&
          input.name === 'observed' &&
          next.observation?.viewport?.atEnd &&
          next.observation.viewport.readingRevision === next.observation.readingRevision &&
          next.observation.pendingCommand == null &&
          (!next.command || next.observation.receipt?.sequence === next.command.sequence)
        )
          out.events.push({
            name: 'tailReady',
            value: {
              landing: props.landing.sequence,
              readingRevision: next.observation.readingRevision,
            },
          });
        const sourceDirection = scrollBoundary(next, input, 'source-reading');
        if (sourceDirection && props.source && !props.source.loading && !props.source.failed) {
          if (sourceDirection === 'up' && !props.source.atStart)
            out.events.push({ name: 'sourcePrevious', value: {} });
          else if (sourceDirection === 'down' && !props.source.atEnd)
            out.events.push({ name: 'sourceNext', value: {} });
        }
        if (input.kind === 'raw' && props.source) {
          const source = props.source;
          const event =
            input.key === 'source-back'
              ? 'sourceBack'
              : !source.loading && input.key === 'source-first' && !source.atStart
                ? 'sourceFirst'
                : !source.loading && input.key === 'source-previous' && !source.atStart
                  ? 'sourcePrevious'
                  : !source.loading && input.key === 'source-next' && !source.atEnd
                    ? 'sourceNext'
                    : !source.loading && input.key === 'source-last' && !source.atEnd
                      ? 'sourceLast'
                      : !source.loading && source.failed && input.key === 'source-retry'
                        ? 'sourceRetry'
                        : null;
          if (event) out.events.push({ name: event, value: {} });
          const field = source.fields.find((field) => input.key === 'source-field-' + field);
          if (
            !source.loading &&
            field &&
            field !== (source.kind === 'message' ? 'body' : source.kind)
          )
            out.events.push({ name: 'sourceField', value: { field } });
        } else if (input.kind === 'raw' && !props.loading) {
          if (props.markReadEnabled && props.toolContext && !next.search) {
            const message = props.messages.find(
              (message) =>
                message.source !== null &&
                (message.role === 'user' ||
                  message.role === 'assistant' ||
                  message.role === 'thinking') &&
                input.key ===
                  'mark-read-' + message.id + (message.role === 'thinking' ? '-thinking' : ''),
            );
            if (message)
              out.events.push({
                name: 'markRead',
                value: {
                  sessionId: props.toolContext.sessionId,
                  sequence: message.source,
                  field: message.role === 'thinking' ? 'thinking' : 'body',
                },
              });
          }

          const tool = toolCards(props.messages)
            .map(({ message }) => message)
            .find(
              (message) =>
                message.source !== null &&
                (message.role === 'tool_call' || message.role === 'tool_result') &&
                next.expanded.includes(message.id) &&
                ((message.tool?.call && input.key === 'tool-source-' + message.id + '-arguments') ||
                  (message.tool?.result && input.key === 'tool-source-' + message.id + '-result')),
            );
          const image = toolCards(props.messages)
            .map(({ message }) => message)
            .find(
              (message) =>
                message.image &&
                message.source !== null &&
                next.expanded.includes(message.id) &&
                input.key === 'image-' + message.id,
            );
          if (image) out.events.push({ name: 'viewImage', value: { sequence: image.source } });
          if (tool)
            out.events.push({
              name: 'toolSource',
              value: {
                sequence: tool.source,
                field: input.key.endsWith('-arguments') ? 'arguments' : 'result',
              },
            });
          const message = toolCards(props.messages)
            .map(({ message }) => message)
            .find(
              (message) =>
                message.source !== null &&
                ((message.role !== 'tool_call' && message.role !== 'tool_result') ||
                  (message.text.length > 0 && next.expanded.includes(message.id))) &&
                input.key ===
                  'source-' + message.id + (message.role === 'thinking' ? '-thinking' : ''),
            );
          if (message)
            out.events.push({
              name: 'source',
              value: {
                sequence: message.source,
                field: message.role === 'thinking' ? 'thinking' : 'body',
              },
            });
          if (input.key === 'older' && props.older) out.events.push({ name: 'older', value: {} });
          if (input.key === 'newer' && props.newer) out.events.push({ name: 'newer', value: {} });
          if (input.key === 'latest' && props.latest)
            out.events.push({ name: 'latest', value: {} });
        }
        return out;
      },
      capture(_state, _props, method, input) {
        return { kind: 'forward', source: { child: 'reading', method, input } };
      },
    },
  };
}
export function exports(): Record<string, UiExport> {
  return {
    Composer: { kind: 'component', contract: composerContract, component: ['composer'] },
    Navigation: { kind: 'component', contract: navigationContract, component: ['navigation'] },
    Timeline: { kind: 'component', contract: timelineContract, component: ['timeline'] },
  };
}
