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

import type { HistorySource } from '../../packages/plugin-sdk/src/index.js';
import type { Theme } from './contracts.js';
import { copy } from './locale.js';
import { toolPresentation } from '../agent/tool-details/index.js';
import { fill, compactLabel, type Node, type Presentation } from './view.js';

export function sourceFieldLabel(locale: string, field: HistorySource['field']): string {
  const text = copy(locale);
  return {
    body: text.findFieldBody,
    thinking: text.findFieldThinking,
    tool_name: text.findFieldTool,
    arguments: text.findFieldArguments,
    result: text.findFieldResult,
    receipt: text.findFieldReceipt,
    failure_message: text.findFieldFailure,
    error_class: text.findFieldError,
  }[field];
}

// Host-recorded milliseconds; live deltas never synthesize a message time.
function timeLabel(millis: number | null): string | null {
  if (millis === null) return null;
  const at = new Date(millis),
    now = new Date();
  if (!Number.isFinite(at.getTime())) return null;
  const pad = (value: number) => String(value).padStart(2, '0');
  const time = pad(at.getHours()) + ':' + pad(at.getMinutes());
  const date = pad(at.getMonth() + 1) + '-' + pad(at.getDate());
  return at.getFullYear() !== now.getFullYear()
    ? at.getFullYear() + '-' + date + ' ' + time
    : at.getMonth() !== now.getMonth() || at.getDate() !== now.getDate()
      ? date + ' ' + time
      : time;
}

export function toolLabel(
  message: { tool: HistorySource['tool']; live?: boolean; waiting?: boolean },
  locale: string,
) {
  const text = copy(locale);
  const title = message.tool?.call?.title;
  const kind = toolPresentation(message.tool?.call?.presentation);
  const name = title
    ? (title.translations?.[locale] ??
      title.translations?.[locale.split('-')[0]!] ??
      title.fallback)
    : kind
      ? {
          read: text.toolReadAction,
          write: text.toolWriteAction,
          edit: text.toolEditAction,
          patch: text.toolPatchAction,
          glob: text.toolSearchAction,
          grep: text.toolSearchAction,
          execute: text.terminalCommand,
          processRead: text.toolProcessRead,
          stdin: text.toolProcessInput,
          stop: text.stop,
        }[kind]
      : message.tool?.call?.name;
  const result = message.tool?.result;
  const settlement = result?.settlement;
  const status = settlement
    ? {
        returned: text.toolReturned,
        failed: text.toolFailed,
        unknown: text.toolUnknown,
        rejected: text.toolRejected,
      }[settlement]
    : result
      ? null
      : message.live && message.tool?.call?.pending
        ? message.waiting
          ? text.waiting
          : message.tool.call.dispatched
            ? text.toolDispatched
            : text.toolPending
        : message.tool?.call?.dispatched
          ? text.toolDispatchedMissing
          : text.toolMissing;
  return { name, status };
}

export function messageView<N extends Node>(
  presentation: Presentation<N>,
  target: 'tui' | 'desktop',
  theme: Theme,
  locale: string,
  message: {
    browse?: { key: string; current: boolean };
    id: string;
    role: HistorySource['role'] | 'thinking';
    timestamp: number | null;
    text: string;
    tool: HistorySource['tool'];
    continued: boolean;
    expanded: boolean;
    structured?: boolean;
    waiting?: boolean;
    live?: boolean;
    toggle: string | null;
    body: N;
    actions: N[];
    sourceActions: N[];
  },
): N {
  const ui = presentation.paint(theme),
    text = copy(locale),
    tool = message.role === 'tool_call' || message.role === 'tool_result',
    expanded = message.expanded,
    body = message.body;
  const caption =
    message.role === 'thinking'
      ? text.findFieldThinking
      : message.role === 'user'
        ? text.you
        : message.role === 'assistant'
          ? text.assistant
          : message.role === 'tool_call'
            ? text.toolActivity
            : message.role === 'failure'
              ? text.findFieldFailure
              : text.toolOutput;
  const { name, status } = tool ? toolLabel(message, locale) : { name: null, status: null };
  const result = message.tool?.result,
    settlement = result?.settlement;
  const time =
    message.role === 'user' || message.role === 'assistant' ? timeLabel(message.timestamp) : null;
  if (target === 'tui') {
    const user = message.role === 'user';
    const thinking = message.role === 'thinking';
    const failed =
      message.role === 'failure' ||
      result?.isError ||
      settlement === 'failed' ||
      settlement === 'rejected';
    const marker = user
      ? theme.ascii
        ? '>'
        : '❯'
      : tool || thinking || failed
        ? theme.ascii
          ? '*'
          : '•'
        : ' ';
    const markerColor = failed
      ? theme.error
      : message.waiting
        ? theme.warning
        : tool && result
          ? theme.success
          : user || message.live
            ? theme.accent
            : theme.subtle;
    const row = (key: string, children: N[], band = false) =>
      ui.box(key, children, {
        row: true,
        gap: 0,
        surface: band,
        terminal: { grow: 0 },
        ...(time && key === 'message-body-' + message.id
          ? {
              wide: {
                min_width: 60,
                direction: 'row' as const,
                children: { ['time-' + message.id]: { basis: Math.min(time.length + 1, 24) } },
              },
            }
          : {}),
      });
    const gutter = (rail = false) =>
      ui.box(
        'gutter-' + message.id + (rail ? '-rail' : ''),
        [
          ui.text(
            'marker-' + message.id + (rail ? '-rail' : ''),
            rail ? (theme.ascii ? '|' : '│') : marker,
            14,
            rail ? theme.border : markerColor,
          ),
        ],
        { terminal: { basis: 2 }, surface: user },
      );
    const heading = tool
      ? row('tool-heading-' + message.id, [
          gutter(),
          message.toggle
            ? ui.button(
                message.toggle,
                compactLabel(name ?? caption, 72),
                false,
                true,
                { background: 0, align: 'start' },
                undefined,
                expanded ? text.collapseTool : text.expandTool,
              )
            : ui.text('role-' + message.id, name ?? caption, 14, theme.text, 600),
          ...(status ? [ui.text('tool-status-' + message.id, '  ' + status, 13, theme.muted)] : []),
        ])
      : row(
          'message-body-' + message.id,
          [
            message.browse && ui.item
              ? ui.item(message.browse.key, gutter(), message.browse.current, false, {
                  followFocus: true,
                  submit: true,
                  submitSpace: true,
                })
              : gutter(),
            ui.box('body-' + message.id, [body], {
              gap: 0,
              surface: user,
              terminal: { grow: 1, shrink: true },
            }),
            ...(time
              ? [
                  ui.box(
                    'time-' + message.id,
                    [ui.text('timestamp-' + message.id, time, 12, theme.subtle)],
                    { surface: user, terminal: { basis: 0 } },
                  ),
                ]
              : []),
          ],
          user,
        );
    const actions = tool
      ? message.actions
      : [
          ...(!message.continued && message.text
            ? [
                ui.button(
                  'copy-raw-' + message.id,
                  user ? text.copyMessage : text.copySource,
                  false,
                  false,
                  {},
                  message.text,
                ),
              ]
            : []),
          ...message.sourceActions,
        ];
    const hasMenu = !!ui.contextMenu && actions.length > 0;
    const card = ui.box(
      (hasMenu ? 'message-card-' : 'message-') + message.id,
      [
        ...(user && !theme.terminal?.terminal
          ? [ui.box('band-top-' + message.id, [], { surface: true, terminal: { basis: 1 } })]
          : []),
        tool && message.browse && ui.item
          ? ui.item(message.browse.key, heading, message.browse.current, false, {
              followFocus: true,
              submit: true,
              submitSpace: true,
            })
          : heading,
        ...(tool
          ? [
              ui.box(
                'details-' + message.id,
                [
                  row('tool-detail-row-' + message.id, [
                    gutter(true),
                    ui.box(
                      'tool-panel-' + message.id,
                      [
                        body,
                        ui.box(
                          'tool-actions-' + message.id,
                          ui.contextMenu ? [] : message.actions,
                          {
                            visible: expanded,
                            row: true,
                            gap: 1,
                            surface: true,
                            terminal: { wrap: true },
                          },
                        ),
                      ],
                      { gap: 0, surface: true, terminal: { grow: 1, shrink: true } },
                    ),
                  ]),
                ],
                { gap: 0, visible: expanded || !!message.structured },
              ),
            ]
          : []),
        ...(tool && result?.isError && !settlement
          ? [ui.text('tool-error-' + message.id, text.toolReportedError, 13, theme.error)]
          : []),
        ...(tool && !expanded && !message.structured && message.text
          ? [
              ui.text(
                'summary-' + message.id,
                '  ' + compactLabel(message.text, 72),
                14,
                theme.muted,
              ),
            ]
          : []),
        ...(message.continued
          ? [ui.text('continued-' + message.id, '  ' + text.partial, 12, theme.subtle)]
          : []),
        ...(!tool && !ui.contextMenu
          ? [
              ui.box('message-actions-' + message.id, message.sourceActions, {
                gap: 0,
                visible: !!message.browse?.current || !message.text,
              }),
            ]
          : []),
        ...(user && !theme.terminal?.terminal
          ? [ui.box('band-bottom-' + message.id, [], { surface: true, terminal: { basis: 1 } })]
          : []),
      ],
      { gap: 0, surface: user },
    );
    return hasMenu
      ? ui.contextMenu!('message-' + message.id, caption, card, actions, text.copy)
      : card;
  }
  const label = name
    ? caption + ' · ' + compactLabel(name, target === 'desktop' ? 64 : 24)
    : caption;
  const role = ui.text(
    'role-' + message.id,
    time ? label + ' · ' + time : label,
    13,
    theme.muted,
    550,
  );
  const heading = tool
    ? ui.box(
        'tool-heading-' + message.id,
        [
          role,
          ...(message.toggle
            ? [ui.button(message.toggle!, expanded ? text.collapseTool : text.expandTool)]
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
    : role;
  return ui.box(
    'message-' + message.id,
    [
      message.browse && ui.item
        ? ui.item(message.browse.key, heading, message.browse.current, false, {
            followFocus: true,
            submit: true,
            submitSpace: true,
          })
        : heading,
      ...(status ? [ui.text('tool-status-' + message.id, status, 13, theme.muted)] : []),
      ...(message.continued
        ? [ui.text('continued-' + message.id, text.partial, 12, theme.muted)]
        : []),
      ...(tool && result?.isError && !settlement
        ? [ui.text('tool-error-' + message.id, text.toolReportedError, 13, theme.muted)]
        : []),
      ...(tool
        ? [
            ...(!expanded && !message.structured
              ? [
                  ui.text(
                    'summary-' + message.id,
                    message.text
                      ? compactLabel(message.text, target === 'desktop' ? 96 : 72)
                      : text.toolNoText,
                    14,
                    theme.muted,
                  ),
                ]
              : []),
            ui.box(
              'details-' + message.id,
              [
                body,
                ui.box('tool-actions-' + message.id, message.actions, {
                  visible: expanded,
                  row: true,
                  gap: target === 'desktop' ? 8 : 1,
                  surface: true,
                  terminal: { wrap: true },
                  style: { width: fill, wrap: true },
                }),
              ],
              {
                visible: expanded || !!message.structured,
                gap: target === 'desktop' ? 8 : 1,
                surface: true,
                style: { width: fill },
              },
            ),
          ]
        : [body]),
      ...(!tool ? message.sourceActions : []),
    ],
    {
      gap: target === 'desktop' ? 8 : 0,
      style: { width: fill, maxWidth: 760, shrink: 0, radius: tool ? 8 : 0 },
      surface: message.role === 'user' || tool,
      padding: target === 'desktop' && (message.role === 'user' || tool) ? 16 : 0,
    },
  );
}
