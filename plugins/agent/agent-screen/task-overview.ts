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
import { tuiText } from '../../shared/tui-locale.js';
import { type Node, type Presentation } from '../../shared/view.js';
import { reviewTarget, rootCommands } from './actions.js';
import type { State, Visit } from './state.js';

// Only canonical tail pages update the summary. Reading older pages cannot
// replace current progress, and an ended turn never claims a verified delivery.
export function taskProgress(state: State): Visit['progress'] {
  const visit = state.visits.find((entry) => entry.id === state.selected?.id);
  const page = state.window?.page;
  const turnId =
    state.run?.turnId ??
    (state.window?.latest ? page?.chunks.at(-1)?.turnId : null) ??
    visit?.progress?.turnId ??
    null;
  const previous = visit?.progress?.turnId === turnId ? visit.progress : null;
  if (!page || !state.window?.latest || page.through < (previous?.through ?? 0)) return previous;
  const turn = page.chunks.filter((chunk) => chunk.turnId === turnId);
  const timing = [...turn].reverse().find((chunk) => chunk.timing)?.timing;
  const progress = {
    turnId,
    through: page.through,
    outcome: timing?.finished?.outcome ?? null,
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
  const progress = taskProgress(state);
  const review = reviewTarget(state);
  const status =
    state.notice && !['completed', 'failed'].includes(state.notice)
      ? text[state.notice]
      : review || state.run?.state === 'waiting_for_user'
        ? tuiText(locale, 'task-waiting')
        : state.selected?.archived
          ? text.archivedConversation
          : state.notice === 'failed' || progress?.outcome === 'failed'
            ? tuiText(locale, 'task-failed')
            : progress?.outcome === 'aborted'
              ? tuiText(locale, 'task-stopped')
              : '';
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
          ui.text('task-state', status, 13, review ? theme.warning : theme.muted, 550, {
            truncate: true,
          }),
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
        ],
        { visible: !!status || !!review, row: true, gap: 1, terminal: { wrap: true } },
      ),
      ...(!state.run &&
      (state.notice === 'failed' ||
        progress?.outcome === 'failed' ||
        progress?.outcome === 'aborted') &&
      ui.focusButton
        ? [ui.focusButton('task-continue', tuiText(locale, 'task-continue'), 'composer')]
        : []),
    ],
    { visible: !!state.selected && !state.source && !state.image, gap: 0 },
  );
}
