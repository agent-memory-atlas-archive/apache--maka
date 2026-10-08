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
  Component,
  AgentSession,
  EditorObservation,
  TextareaProps,
  MarkdownProps,
  ReadingObservation,
  ReadingCommand,
  ComponentInput,
  UiExport,
} from '../../plugin-sdk/src/index.js';
import {
  composerContract,
  navigationContract,
  timelineContract,
  type ComposerProps,
  type NavigationProps,
  type TimelineProps,
} from './contracts.js';
import { copy } from './locale.js';
import { output, fill, pixels, compactLabel, type Presentation, type Node } from './view.js';
interface Revision {
  revision: number;
}
interface Navigation extends Revision {
  toggled: string[];
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
interface Draft extends Revision {
  observation: EditorObservation | null;
}
interface Reading extends Revision {
  expanded: string[];
  observation: ReadingObservation | null;
  command: ReadingCommand | null;
  landing: TimelineProps['landing'];
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
    const pending =
      props.command !== null &&
      (state.observation?.receipt?.owner !== props.command.owner ||
        state.observation?.receipt?.sequence !== props.command.sequence);
    const available =
      props.enabled &&
      !props.busy &&
      (target === 'tui' || !props.running) &&
      !pending &&
      !state.observation?.snapshot.marked &&
      !!state.observation?.snapshot.text.bytes;
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
      autoFocus: true,
      initialText: '',
      placeholder: text.placeholder,
      command: props.command,
    };
    const editor = ui.box(
      'editor-area',
      [
        presentation.slot('draft', 'editor', textarea, {
          changed: 'changed',
          submitted: 'submitted',
        }),
      ],
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
              ui.button('stop', text.stop, props.busy),
              ...(target === 'tui'
                ? [ui.button('send', text.send, !available || !state.observation, true)]
                : []),
            ]
          : [ui.button('send', text.send, !available || !state.observation, true)]),
      ],
      {
        row: true,
        terminal: { wrap: true },
        style: { justify: 'between', align: 'center', width: fill },
      },
    );
    return output(
      state,
      ui.box('composer', [editor, controls], {
        row: target === 'tui',
        border: target === 'tui',
        gap: target === 'tui' ? 0 : 12,
        terminal: { shrink: true },
        style: { width: fill },
      }),
      events,
    );
  };
  const navigationFrame = (state: Navigation, props: NavigationProps) => {
    const ui = presentation.paint(props.theme);
    return output(
      state,
      ui.box(
        'session-groups',
        sessionGroups(props).flatMap((group, index) => {
          const open = groupOpen(state, group.key);
          const heading = `${open ? '▾' : '▸'} ${group.label}  ${group.sessions.length}`;
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
                  return ui.item
                    ? ui.item(
                        session.id,
                        ui.box(
                          `session-row-${index}-${at}`,
                          [
                            ui.box(
                              `session-status-${index}-${at}`,
                              [
                                ui.text(
                                  `session-glyph-${index}-${at}`,
                                  session.activity === 'working'
                                    ? '  ⢁ '
                                    : session.activity === 'waiting'
                                      ? '  ◇ '
                                      : '    ',
                                  13,
                                  session.activity === 'idle'
                                    ? props.theme.muted
                                    : props.theme.accent,
                                ),
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
                                  400,
                                  { truncate: true },
                                ),
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
                      )
                    : ui.button(
                        session.id,
                        session.title +
                          (session.activity === 'waiting'
                            ? ' · ' + copy(props.locale).needsYou
                            : session.activity === 'working'
                              ? ' · ' + copy(props.locale).working
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
    );
  };
  const timelineFrame = (state: Reading, props: TimelineProps) => {
    const ui = presentation.paint(props.theme),
      text = copy(props.locale);
    const tools = props.messages.filter(
      (message) =>
        (message.role === 'tool_call' || message.role === 'tool_result') &&
        (message.source !== null || message.text.length > 0),
    );
    state.expanded = state.expanded.filter((id) => tools.some((message) => message.id === id));
    const messages: N[] = props.messages.map((message) => {
      const tool = message.role === 'tool_call' || message.role === 'tool_result';
      const expanded = state.expanded.includes(message.id);
      const caption =
        message.role === 'user'
          ? text.you
          : message.role === 'assistant'
            ? text.assistant
            : message.role === 'tool_call'
              ? text.toolActivity
              : text.toolOutput;
      const name = tool && message.tool?.toolName;
      const role = ui.text(
        'role-' + message.id,
        name ? caption + ' · ' + compactLabel(name, target === 'desktop' ? 64 : 24) : caption,
        13,
        props.theme.muted,
        550,
      );
      const sourceAction = ui.button(
        'source-' + message.source,
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
        target === 'desktop' && message.role === 'assistant' && !message.continued
          ? presentation.slot(
              'text-' + message.contentKey,
              'markdown',
              {
                source: message.text,
                appearance: props.theme.markdown,
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
                appearance: props.theme.markdown.body,
                selection: props.theme.markdown.selection,
                disabled: false,
              },
              {},
            );
      return ui.box(
        'message-' + message.id,
        [
          tool
            ? ui.box(
                'tool-heading-' + message.id,
                [
                  role,
                  ...(message.source !== null || message.text
                    ? [
                        ui.button(
                          'toggle-' + message.id,
                          expanded ? text.collapseTool : text.expandTool,
                        ),
                      ]
                    : []),
                ],
                {
                  row: true,
                  gap: target === 'desktop' ? 12 : 1,
                  surface: true,
                  terminal: { wrap: true },
                  style: { width: fill, wrap: true, align: 'center' },
                },
              )
            : role,
          ...(message.continued
            ? [ui.text('continued-' + message.id, text.partial, 12, props.theme.muted)]
            : []),
          ...(tool && message.tool?.kind === 'result' && message.tool.isError
            ? [ui.text('tool-error-' + message.id, text.toolReportedError, 13, props.theme.muted)]
            : []),
          ...(tool
            ? [
                ...(!expanded
                  ? [
                      ui.text(
                        'summary-' + message.id,
                        message.text
                          ? compactLabel(message.text, target === 'desktop' ? 96 : 72)
                          : text.toolNoText,
                        14,
                        props.theme.muted,
                      ),
                    ]
                  : []),
                ui.box(
                  'details-' + message.id,
                  [
                    body,
                    ui.box(
                      'tool-actions-' + message.id,
                      [
                        ...(message.source !== null
                          ? [
                              ui.button(
                                'tool-source-' + message.source,
                                message.role === 'tool_call' ? text.toolArguments : text.toolResult,
                                props.loading,
                              ),
                            ]
                          : []),
                        ...(message.image && message.source !== null
                          ? [ui.button('image-' + message.source, text.viewImage, props.loading)]
                          : []),
                        ...(message.source !== null && message.text
                          ? [ui.button('source-' + message.source, text.readSource, props.loading)]
                          : []),
                      ],
                      {
                        row: true,
                        gap: target === 'desktop' ? 8 : 1,
                        surface: true,
                        terminal: { wrap: true },
                        style: { width: fill, wrap: true },
                      },
                    ),
                  ],
                  {
                    visible: expanded,
                    gap: target === 'desktop' ? 8 : 1,
                    surface: true,
                    style: { width: fill },
                  },
                ),
              ]
            : [body]),
          ...(!tool && message.source !== null
            ? [
                ui.box('source-actions-' + message.id, [sourceAction], {
                  row: true,
                  gap: 0,
                  style: { width: fill },
                  surface: message.role === 'user',
                }),
              ]
            : []),
        ],
        {
          gap: target === 'desktop' ? 8 : 0,
          style: { width: fill, maxWidth: 760, shrink: 0, radius: tool ? 8 : 0 },
          surface: message.role === 'user' || tool,
          padding: target === 'desktop' && (message.role === 'user' || tool) ? 16 : 0,
        },
      );
    });
    const content = ui.box(
      'timeline',
      [
        ...(!messages.length && !props.loading
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
        ...(props.loading ? [ui.text('loading', text.loading, 14, props.theme.muted)] : []),
      ],
      {
        gap: target === 'desktop' ? 24 : 1,
        style: {
          width: fill,
          height: fill,
          align: 'center',
          padding: { top: 24, right: 24, bottom: 24, left: 24 },
        },
        scroll: props.live ? 'follow_end' : 'anchor',
        terminal: { grow: 1 },
      },
    );
    const controls: N[] = props.source
      ? [
          ui.button('source-back', text.sourceBack),
          ...(!props.source.atStart
            ? [
                ui.button('source-first', text.sourceFirst, props.source.loading),
                ui.button('source-previous', text.sourcePrevious, props.source.loading),
              ]
            : []),
          ...(!props.source.atEnd
            ? [
                ui.button('source-next', text.sourceNext, props.source.loading),
                ui.button('source-last', text.sourceLast, props.source.loading),
              ]
            : []),
          ...(props.source.failed ? [ui.button('source-retry', text.sourceRetry)] : []),
        ]
      : [
          ...(props.older ? [ui.button('older', text.older, props.loading)] : []),
          ...(props.newer ? [ui.button('newer', text.newer, props.loading)] : []),
          ...(props.latest ? [ui.button('latest', text.latest, props.loading)] : []),
          ...(state.observation?.viewport && !state.observation.viewport.atEnd
            ? [ui.button('end', text.toEnd, state.observation.pendingCommand != null)]
            : []),
        ];
    return output(
      state,
      ui.box(
        'timeline-frame',
        [
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
              visible: !props.source,
              style: { width: fill, height: pixels(0), grow: 1, shrink: 1, minHeight: 0 },
              terminal: { grow: 1 },
            },
          ),
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
                        { input: 'source-input' },
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
          ...(controls.length
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
          gap: target === 'desktop' ? 8 : 1,
          style: { width: fill, height: fill },
          terminal: { grow: 1 },
        },
      ),
    );
  };
  return {
    composer: {
      mount: (props) => composerFrame({ revision: 0, observation: null }, props),
      update: (state, props) => composerFrame({ ...state, revision: state.revision + 1 }, props),
      event(state, input, props) {
        const next = { ...state, revision: state.revision + 1 };
        if (
          input.kind === 'component' &&
          (input.name === 'changed' || input.name === 'submitted')
        ) {
          next.observation = input.event as EditorObservation;
          if (input.name === 'changed')
            return composerFrame(next, props, [{ name: 'changed', value: next.observation }]);
        }
        if (
          ((input.kind === 'raw' && input.key === 'send') ||
            (input.kind === 'component' && input.name === 'submitted')) &&
          props.enabled &&
          !props.busy &&
          (target === 'tui' || !props.running) &&
          next.observation &&
          next.observation.snapshot.text.bytes > 0 &&
          !next.observation.snapshot.marked &&
          (!props.command ||
            (next.observation.receipt?.owner === props.command.owner &&
              next.observation.receipt.sequence === props.command.sequence))
        ) {
          return composerFrame(next, props, [{ name: 'send', value: next.observation }]);
        }
        if (input.kind === 'raw' && input.key === 'stop' && props.running && !props.busy)
          return composerFrame(next, props, [{ name: 'stop', value: {} }]);
        return composerFrame(next, props);
      },
      capture(_state, _props, method, input) {
        return { kind: 'forward', source: { child: 'draft', method, input } };
      },
    },
    navigation: {
      mount: (props) => navigationFrame({ revision: 0, toggled: [] }, props),
      update: (state, props) => navigationFrame({ ...state, revision: state.revision + 1 }, props),
      event(state, input, props) {
        const next = { ...state, revision: state.revision + 1 };
        const events: Parameters<typeof output>[2] = [];
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
              .find((session) => session.id === input.key);
            if (session) events.push({ name: 'select', value: { sessionId: session.id } });
          }
        }
        const out = navigationFrame(next, props);
        out.events.push(...events);
        return out;
      },
    },
    timeline: {
      mount: (props) =>
        timelineFrame(
          {
            revision: 0,
            expanded: [],
            observation: null,
            command:
              props.landing.sequence > 0
                ? { sequence: 1, target: { kind: props.landing.edge } }
                : null,
            landing: props.landing,
          },
          props,
        ),
      update(state, props) {
        if (
          props.landing.sequence < state.landing.sequence ||
          (props.landing.sequence === state.landing.sequence &&
            props.landing.edge !== state.landing.edge)
        )
          throw new Error('Invalid reading landing sequence');
        return timelineFrame(
          {
            ...state,
            revision: state.revision + 1,
            command:
              state.landing.sequence !== props.landing.sequence
                ? {
                    sequence: (state.command?.sequence ?? 0) + 1,
                    target: { kind: props.landing.edge },
                  }
                : state.command,
            landing: props.landing,
          },
          props,
        );
      },
      event(state, input, props) {
        const next = { ...state, revision: state.revision + 1 };
        if (
          input.kind === 'component' &&
          (input.child === 'reading' || input.child === 'source-reading')
        ) {
          if (input.name === 'observed') next.observation = input.event as ReadingObservation;
          if (input.name === 'input') input = input.event as ComponentInput;
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
          const tool = props.messages.find(
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
        const out = timelineFrame(next, props);
        if (
          !props.source &&
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
        } else if (input.kind === 'raw' && !props.loading) {
          const tool = props.messages.find(
            (message) =>
              message.source !== null &&
              (message.role === 'tool_call' || message.role === 'tool_result') &&
              next.expanded.includes(message.id) &&
              input.key === 'tool-source-' + message.source,
          );
          const image = props.messages.find(
            (message) =>
              message.image &&
              message.source !== null &&
              next.expanded.includes(message.id) &&
              input.key === 'image-' + message.source,
          );
          if (image) out.events.push({ name: 'viewImage', value: { sequence: image.source } });
          if (tool) out.events.push({ name: 'toolSource', value: { sequence: tool.source } });
          const message = props.messages.find(
            (message) =>
              message.source !== null &&
              ((message.role !== 'tool_call' && message.role !== 'tool_result') ||
                (message.text.length > 0 && next.expanded.includes(message.id))) &&
              input.key === 'source-' + message.source,
          );
          if (message) out.events.push({ name: 'source', value: { sequence: message.source } });
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
