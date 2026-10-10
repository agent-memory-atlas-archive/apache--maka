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
  Action,
  ActionOutcome,
  Component,
  ComponentInput,
  EditorObservation,
  EditorTextRef,
  Emission,
  ReadingCommand,
  SkillLibraryCursor,
  SkillLibraryDocument,
  SkillLibraryPage,
  TextChunk,
} from '../../plugin-sdk/src/index.js';
import { skillLibraryContract, type SkillLibraryProps } from './contracts.js';
import { copy } from './locale.js';
import { compactLabel, fill, output, pixels, type Presentation, type Node } from './view.js';

type Page = Extract<SkillLibraryPage, { kind: 'page' }>;
type Document = Extract<SkillLibraryDocument, { kind: 'page' }>;
type Request =
  | { kind: 'query'; key: string; basis: EditorTextRef }
  | {
      kind: 'list';
      key: string;
      cursor: SkillLibraryCursor | null;
      history: number[];
    }
  | { kind: 'read'; key: string };
interface State {
  revision: number;
  serial: number;
  open: boolean;
  observation: EditorObservation | null;
  basis: EditorTextRef | null;
  query: string;
  page: Page | null;
  pageKey: string;
  cursor: number;
  history: number[];
  document: Document | null;
  pending: Request | null;
  notice: 'skillsFailed' | 'skillsChanged' | 'searchTooLong' | null;
  active: number;
  reveal: ReadingCommand | null;
}
function same(a: EditorTextRef | null, b: EditorTextRef | null) {
  return a?.owner === b?.owner && a?.revision === b?.revision;
}
function dirty(state: State) {
  return !!state.observation && !same(state.basis, state.observation.snapshot.text);
}
function list(
  state: State,
  cursor: SkillLibraryCursor | null = null,
  history: number[] = [],
): Action[] {
  const key = `skills-${++state.serial}`;
  state.pending = { kind: 'list', key, cursor, history };
  state.notice = null;
  return [
    {
      key,
      binding: 'skills',
      member: 0,
      method: 'list',
      input: { query: state.query, cursor },
      resources: {},
    },
  ];
}
function search(state: State): Action[] {
  const basis = state.observation?.snapshot.text;
  if (!basis) return list(state);
  if (basis.bytes > 1024) {
    state.notice = 'searchTooLong';
    return [];
  }
  const key = `skills-${++state.serial}`;
  state.pending = { kind: 'query', key, basis };
  state.notice = null;
  return [
    {
      key,
      binding: 'reader',
      member: 0,
      method: 'read',
      input: { start: 0, end: basis.bytes },
      resources: {
        text: {
          child: 'skills-query',
          method: 'snapshot',
          input: { kind: 'at_revision', owner: basis.owner, revision: basis.revision },
        },
      },
    },
  ];
}
function read(state: State, reference: string, revision: string, offset: number): Action[] {
  const key = `skills-${++state.serial}`;
  state.pending = { kind: 'read', key };
  state.notice = null;
  return [
    {
      key,
      binding: 'skills',
      member: 0,
      method: 'read',
      input: { reference, revision, offset },
      resources: {},
    },
  ];
}
function reveal(state: State, key: string) {
  state.reveal = {
    sequence: (state.reveal?.sequence ?? 0) + 1,
    target: { kind: 'reveal_child', key },
  };
}
function rowKey(state: State, at: number) {
  return `${state.pageKey}-row-${at}`;
}
function frame<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  props: SkillLibraryProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const ui = presentation.paint(props.theme),
    text = copy(props.locale);
  const tui = presentation.target === 'tui';
  const busy = !!state.pending;
  const available = state.open && props.open && !busy && !state.notice && !dirty(state);
  const document = state.document;
  const content: N[] = [];
  if (document) {
    const entry = document.entry;
    content.push(
      ui.text('skill-name', entry.name, 17, props.theme.text, 550),
      ui.text('skill-source', entry.source, 13, props.theme.muted),
      ui.text('skill-description', entry.description),
      ui.text(
        'skill-range',
        `${text.toolByteRange}: ${document.totalBytes ? document.offset + 1 : 0}–${document.after ?? document.totalBytes} / ${document.totalBytes}`,
        12,
        props.theme.muted,
      ),
    );
    if (!entry.valid) content.push(ui.text('skill-invalid', text.skillsInvalid));
    if (entry.shadowedBy)
      content.push(ui.text('skill-shadow', `${text.skillsShadowed}: ${entry.shadowedBy}`));
    if (entry.metadataTruncated) content.push(ui.text('skill-truncated', text.skillsTruncated));
    entry.issues.forEach((issue, at) => content.push(ui.text(`skill-issue-${at}`, issue)));
    content.push(
      presentation.slot(
        `skill-body-${document.entry.reference}-${document.offset}`,
        'source',
        {
          source: document.text,
          appearance: props.theme.markdown.code,
          selection: props.theme.markdown.selection,
          terminal: props.theme.terminal,
          disabled: false,
          focusOnPointer: true,
        },
        {},
      ),
    );
  } else {
    state.page?.entries.forEach((entry, at) => {
      const key = rowKey(state, at);
      const label = compactLabel(entry.name, tui ? 100 : 200);
      const summary = compactLabel(`${entry.source} · ${entry.description}`, tui ? 160 : 240);
      content.push(
        ui.item
          ? ui.item(
              key,
              ui.box(
                `${key}-content`,
                [
                  ui.text(`${key}-name`, label, 14, props.theme.text, 550),
                  ui.text(`${key}-summary`, summary, 12, props.theme.muted),
                ],
                { gap: 0 },
              ),
              at === state.active,
              !available,
              { followFocus: false, submit: false },
            )
          : ui.box(
              `${key}-content`,
              [
                ui.button(key, label, !available, false, { width: fill, justify: 'start' }),
                ui.text(`${key}-summary`, summary, 12, props.theme.muted),
              ],
              { gap: 0 },
            ),
      );
    });
    if (!busy && state.page && !state.page.entries.length)
      content.push(ui.text('skills-empty', text.skillsEmpty));
    state.page?.diagnostics.forEach((diagnostic, at) =>
      content.push(ui.text(`skills-diagnostic-${at}`, diagnostic, 12, props.theme.muted)),
    );
  }
  const view = ui.box(
    'skill-library',
    [
      ui.box(
        'skills-search',
        [
          presentation.slot(
            'skills-query',
            'input',
            {
              appearance: props.theme.input,
              accessibilityLabel: text.skillsSearch,
              placeholder: text.skillsSearch,
              autoFocus: true,
              readOnly: false,
              disabled: !state.open || !props.open,
              tabIndex: 1,
            },
            { changed: 'query', submitted: 'search' },
          ),
          ui.button('search', text.skillsSubmit, busy),
        ],
        { visible: !document, gap: 0 },
      ),
      ...(busy ? [ui.text('skills-loading', text.skillsLoading, 13, props.theme.muted)] : []),
      ...(state.notice
        ? [ui.text('skills-notice', text[state.notice], 13, props.theme.muted)]
        : []),
      presentation.reading(
        'skills-reading',
        {
          content: ui.box('skills-viewport', content, {
            scroll: 'manual',
            focusGroup: tui && !document,
            autoFocus: !!document,
            gap: tui ? 0 : 6,
            style: { width: fill, height: pixels(320), minHeight: 0, minWidth: 0 },
            terminal: { grow: 1, basis: 0 },
          }),
          command: state.reveal,
        },
        { input: 'reading-input' },
      ),
      ui.box(
        'skills-controls',
        [
          ...(document
            ? [
                ui.button('back', text.back, busy),
                ui.button(
                  'body-previous',
                  text.sourcePrevious,
                  !available || document.before == null,
                ),
                ui.button('body-next', text.sourceNext, !available || document.after == null),
              ]
            : [
                ui.button('previous', text.skillsPrevious, !available || !state.history.length),
                ui.button('next', text.skillsNext, !available || !state.page?.after),
              ]),
          ui.button('refresh', text.refreshSessions, busy),
          ui.button('close', text.closeModels),
        ],
        { row: true, gap: tui ? 1 : 8, style: { wrap: true }, terminal: { wrap: true } },
      ),
    ],
    {
      visible: state.open && props.open,
      gap: tui ? 0 : 8,
      style: { width: fill, minWidth: 0 },
      terminal: { grow: 1 },
      shortcuts: [{ key: 'escape', modifiers: [], input: 'close', enabled: true }],
    },
  );
  const out = output(state, view, events);
  out.actions = actions;
  return out;
}
export function skillLibrary<N extends Node>(
  presentation: Presentation<N>,
): Component<State, SkillLibraryProps, N> {
  return {
    mount(props) {
      const state: State = {
        revision: 0,
        serial: 0,
        open: props.open,
        observation: null,
        basis: null,
        query: '',
        page: null,
        pageKey: '',
        cursor: 0,
        history: [],
        document: null,
        pending: null,
        notice: null,
        active: 0,
        reveal: null,
      };
      return frame(presentation, state, props, props.open ? list(state) : []);
    },
    update(previous, props) {
      const state = { ...previous, revision: previous.revision + 1 };
      let actions: Action[] = [];
      if (!props.open) {
        state.open = false;
        state.pending = null;
      } else if (!state.open) {
        state.open = true;
        state.document = null;
        actions = search(state);
      }
      return frame(presentation, state, props, actions);
    },
    event(state, input, props) {
      state.revision++;
      const events: Emission[] = [];
      let actions: Action[] = [];
      if (input.kind === 'component' && input.child === 'skills-reading' && input.name === 'input')
        input = input.event as ComponentInput;
      if (!state.open || !props.open) return frame(presentation, state, props);
      if (input.kind === 'component' && input.child === 'skills-query') {
        if (input.name === 'changed') {
          state.observation = input.event as EditorObservation;
          // The native editor's initial empty observation already matches the initial query.
          if (!state.basis && !state.query && !state.observation.snapshot.text.bytes)
            state.basis = state.observation.snapshot.text;
        } else if (input.name === 'submitted' && !state.pending) actions = search(state);
      } else if (input.kind === 'action' && input.key === state.pending?.key) {
        const request = state.pending;
        state.pending = null;
        const result = input.event as ActionOutcome;
        if (result.status !== 'succeeded') state.notice = 'skillsFailed';
        else if (request.kind === 'query') {
          if (!same(request.basis, state.observation?.snapshot.text ?? null))
            state.notice = 'skillsFailed';
          else {
            const query = (result.value as TextChunk).text.trim();
            if ([...query].length > 256) state.notice = 'searchTooLong';
            else {
              state.query = query;
              state.basis = request.basis;
              actions = list(state);
            }
          }
        } else if (request.kind === 'list') {
          const page = result.value as SkillLibraryPage;
          if (page.kind === 'changed') state.notice = 'skillsChanged';
          else {
            state.page = page;
            state.pageKey = request.key;
            state.cursor = request.cursor?.offset ?? 0;
            state.history = request.history;
            state.document = null;
            state.active = 0;
            reveal(state, rowKey(state, 0));
          }
        } else {
          const document = result.value as SkillLibraryDocument;
          if (document.kind !== 'page')
            state.notice = document.kind === 'changed' ? 'skillsChanged' : 'skillsFailed';
          else {
            state.document = document;
            reveal(state, 'skill-name');
          }
        }
      } else if (input.kind === 'raw') {
        if (input.key === 'close') {
          state.open = false;
          state.pending = null;
          events.push({ name: 'closed', value: {} });
        } else if (!state.pending) {
          if (input.key === 'back') {
            state.document = null;
            reveal(state, rowKey(state, state.active));
          } else if (input.key === 'refresh' || input.key === 'search') {
            state.document = null;
            actions = search(state);
          } else if (!state.notice && !dirty(state)) {
            const document = state.document,
              page = state.page;
            if (document) {
              const offset =
                input.key === 'body-previous'
                  ? document.before
                  : input.key === 'body-next'
                    ? document.after
                    : null;
              if (offset != null)
                actions = read(state, document.entry.reference, document.revision, offset);
            } else if (page) {
              if (input.key === 'next' && page.after)
                actions = list(state, page.after, [...state.history, state.cursor]);
              else if (input.key === 'previous' && state.history.length)
                actions = list(
                  state,
                  { revision: page.revision, query: state.query, offset: state.history.at(-1)! },
                  state.history.slice(0, -1),
                );
              else {
                const at = page.entries.findIndex((_, at) => rowKey(state, at) === input.key);
                if (at >= 0) {
                  state.active = at;
                  actions = read(state, page.entries[at]!.reference, page.revision, 0);
                }
              }
            }
          }
        }
      }
      return frame(presentation, state, props, actions, events);
    },
  };
}
export function activate() {
  return {
    SkillLibrary: { kind: 'component', contract: skillLibraryContract, component: ['component'] },
  };
}
