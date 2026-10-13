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
import { toolLabel } from '../../shared/message-view.js';
import { tuiText } from '../../shared/tui-locale.js';
import { compactLabel, type Node, type Presentation } from '../../shared/view.js';
import { reviewTarget, rootCommands } from './actions.js';
import type { State, Visit } from './state.js';

// Only canonical tail pages update the summary. Reading older pages cannot
// replace current progress, and an ended turn never claims a verified delivery.
export function taskProgress(state: State, locale: string): Visit['progress'] {
  const visit = state.visits.find((entry) => entry.id === state.selected?.id);
  const previous = visit?.progress ?? null;
  const queuedGoal = state.queue?.entries.find((entry) => entry.kind === 'in_flight')?.preview;
  if (queuedGoal && previous) previous.goal = compactLabel(queuedGoal, 100);
  const page = state.window?.page;
  if (!page || !state.window?.latest || page.through < (previous?.through ?? 0)) return previous;
  const last = page.chunks.at(-1);
  const turn = last ? page.chunks.filter((chunk) => chunk.turnId === last.turnId) : [];
  const goal = turn.find((chunk) => chunk.role === 'user' && chunk.field === 'body');
  const tool = [...turn].reverse().find((chunk) => chunk.tool);
  const timing = [...turn].reverse().find((chunk) => chunk.timing)?.timing;
  const progress = {
    through: page.through,
    goal: queuedGoal
      ? compactLabel(queuedGoal, 100)
      : goal
        ? compactLabel(goal.text.replace(/\s+/gu, ' ').trim(), 100)
        : (previous?.goal ?? ''),
    action: tool ? (toolLabel({ tool: tool.tool, live: !!state.run }, locale).name ?? '') : '',
    outcome: timing?.finished?.outcome ?? null,
    toolIssue: turn.some(
      (chunk) =>
        chunk.tool?.result?.isError ||
        ['failed', 'unknown', 'rejected'].includes(chunk.tool?.result?.settlement ?? ''),
    ),
  };
  if (visit) visit.progress = progress;
  return progress;
}

export function taskOverview<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  theme: Theme,
  locale: string,
): N {
  const ui = presentation.paint(theme),
    text = copy(locale);
  const progress = taskProgress(state, locale);
  const review = reviewTarget(state);
  const status = state.loading
    ? text.loading
    : state.notice && !['completed', 'failed'].includes(state.notice)
      ? text[state.notice]
      : review || state.run?.state === 'waiting_for_user'
        ? tuiText(locale, 'task-waiting')
        : state.run
          ? tuiText(locale, 'task-running')
          : state.selected?.archived
            ? text.archivedConversation
            : state.notice === 'failed' || progress?.outcome === 'failed'
              ? tuiText(locale, 'task-failed')
              : progress?.outcome === 'aborted'
                ? tuiText(locale, 'task-stopped')
                : progress?.outcome === 'completed' || state.notice === 'completed'
                  ? tuiText(locale, 'task-ended')
                  : tuiText(
                      locale,
                      progress?.goal || state.window?.page.chunks.length
                        ? 'task-idle'
                        : 'task-ready',
                    );
  const needsLatest =
    !!state.window && (!state.window.latest || state.latestKnown > state.window.page.through);
  const available = (kind: 'interaction-open') =>
    rootCommands(state, locale, presentation.target).some(
      (command) => command.kind === kind && !command.disabled,
    );
  return ui.box(
    'task-overview',
    [
      ui.box(
        'task-state-actions',
        [
          ui.text(
            'task-state',
            status,
            13,
            review || progress?.toolIssue ? theme.warning : theme.muted,
            550,
            { truncate: true },
          ),
          ui.box('task-state-space', [], { terminal: { grow: 1 } }),
          ...(review
            ? [
                ui.button(
                  'interaction-open',
                  text.reviewRequest + ' · Ctrl+G',
                  !available('interaction-open'),
                ),
              ]
            : []),
          ...(needsLatest ? [ui.button('task-latest', text.latest, state.loading)] : []),
        ],
        { row: true, gap: 1, terminal: { wrap: true } },
      ),
      ...(!state.run &&
      (state.notice === 'failed' ||
        progress?.outcome === 'failed' ||
        progress?.outcome === 'aborted') &&
      ui.focusButton
        ? [ui.focusButton('task-continue', tuiText(locale, 'task-continue'), 'composer')]
        : []),
      ...(progress?.goal
        ? [ui.text('task-goal', progress.goal, 13, theme.text, 400, { truncate: true })]
        : []),
      ...(progress?.toolIssue
        ? [ui.text('task-tool-issue', tuiText(locale, 'task-tool-error'), 12, theme.warning)]
        : state.run && progress?.action
          ? [ui.text('task-progress', progress.action, 12, theme.muted, 400, { truncate: true })]
          : []),
    ],
    { visible: !!state.selected && !state.source && !state.image, gap: 0 },
  );
}
