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

import type { HistorySource } from '../../plugin-sdk/src/index.js';
import type { Theme } from './contracts.js';
import { copy } from './locale.js';
import { fill, compactLabel, type Node, type Presentation } from './view.js';

export function messageView<N extends Node>(
  presentation: Presentation<N>,
  target: 'tui' | 'desktop',
  theme: Theme,
  locale: string,
  message: {
    id: string;
    role: HistorySource['role'];
    text: string;
    tool: HistorySource['tool'];
    continued: boolean;
    expanded: boolean;
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
    message.role === 'user'
      ? text.you
      : message.role === 'assistant'
        ? text.assistant
        : message.role === 'tool_call'
          ? text.toolActivity
          : message.role === 'failure'
            ? text.findFieldFailure
            : text.toolOutput;
  const name = tool && message.tool?.toolName;
  const role = ui.text(
    'role-' + message.id,
    name ? caption + ' · ' + compactLabel(name, target === 'desktop' ? 64 : 24) : caption,
    13,
    theme.muted,
    550,
  );
  return ui.box(
    'message-' + message.id,
    [
      tool
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
        : role,
      ...(message.continued
        ? [ui.text('continued-' + message.id, text.partial, 12, theme.muted)]
        : []),
      ...(tool && message.tool?.kind === 'result' && message.tool.isError
        ? [ui.text('tool-error-' + message.id, text.toolReportedError, 13, theme.muted)]
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
                    theme.muted,
                  ),
                ]
              : []),
            ui.box(
              'details-' + message.id,
              [
                body,
                ui.box('tool-actions-' + message.id, message.actions, {
                  row: true,
                  gap: target === 'desktop' ? 8 : 1,
                  surface: true,
                  terminal: { wrap: true },
                  style: { width: fill, wrap: true },
                }),
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
