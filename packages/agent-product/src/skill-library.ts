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
  SkillLibraryChangeKind,
  SkillLibraryPreview,
  SkillLibraryChangeResult,
  SkillLibraryOutcome,
  OperationReply,
  OperationCleanup,
  RecoveryRef,
  SkillLibraryDocument,
  SkillLibraryPage,
  TextChunk,
} from '../../plugin-sdk/src/index.js';
import { skillLibraryContract, type SkillLibraryProps } from './contracts.js';
import { sameValue } from './value.js';
import { copy, type Copy } from './locale.js';
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
  | { kind: 'read'; key: string }
  | { kind: 'preview' | 'mutate' | 'receipt' | 'outcome'; key: string };
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
  notice: keyof Copy | null;
  cleanupUnconfirmed: boolean;
  preview: Extract<SkillLibraryPreview, { kind: 'ready' }> | null;
  current: { reference: RecoveryRef | null } | null;
  active: number;
  reveal: ReadingCommand | null;
}
function same(a: EditorTextRef | null, b: EditorTextRef | null) {
  return a?.owner === b?.owner && a?.revision === b?.revision;
}
function dirty(state: State) {
  return !!state.observation && !same(state.basis, state.observation.snapshot.text);
}
function readable(state: State) {
  return state.notice !== 'skillsChanged' && state.notice !== 'searchTooLong' && !dirty(state);
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
function preview(state: State, kind: SkillLibraryChangeKind): Action[] {
  const document = state.document;
  if (!document?.writable || state.current || (!document.entry.valid && kind !== 'delete'))
    return [];
  const key = `skills-${++state.serial}`;
  state.pending = { kind: 'preview', key };
  state.notice = null;
  return [
    {
      key,
      binding: 'skills',
      member: 0,
      method: 'preview',
      resources: {},
      input: { kind, reference: document.entry.reference, revision: document.revision },
    },
  ];
}
function recover(state: State): Action[] {
  if (!state.current?.reference || state.pending) return [];
  const key = `skills-${++state.serial}`;
  state.pending = { kind: 'receipt', key };
  return [
    {
      key,
      binding: 'receipts',
      member: 0,
      method: 'read',
      resources: {},
      input: state.current.reference,
    },
  ];
}
function rejection(reason: string): keyof Copy {
  return reason === 'already_installed'
    ? 'skillsAlreadyInstalled'
    : reason === 'not_installed'
      ? 'skillsNotInstalled'
      : 'skillsChangeRejected';
}
function settle(
  state: State,
  props: SkillLibraryProps,
  reply: OperationReply,
  readPublished = false,
): Action[] {
  if (
    !state.current ||
    (state.current.reference && !sameValue(state.current.reference, reply.reference)) ||
    reply.reference.contract.name !== 'maka.skills.Library' ||
    reply.reference.method !== 'apply'
  )
    return [];
  state.current.reference = reply.reference;
  if (reply.receipt.kind !== 'recorded' || reply.receipt.outcome.kind !== 'returned') {
    state.notice = 'skillsUnknown';
    if (readPublished && reply.receipt.kind === 'recorded') {
      const key = `skills-${++state.serial}`;
      state.pending = { kind: 'outcome', key };
      return [
        {
          key,
          binding: 'skills',
          member: 0,
          method: 'outcome',
          resources: {},
          input: reply.reference,
        },
      ];
    }
    return [];
  }
  return settledResult(
    state,
    props,
    reply.receipt.outcome.value as SkillLibraryChangeResult,
    reply.receipt.cleanup,
  );
}
function settledResult(
  state: State,
  props: SkillLibraryProps,
  result: SkillLibraryChangeResult,
  cleanup: OperationCleanup,
): Action[] {
  state.current = null;
  state.cleanupUnconfirmed = cleanup.kind !== 'confirmed';
  if (result.kind === 'applied') {
    state.document = null;
    const actions = state.open && props.open ? list(state) : [];
    state.notice = null;
    return actions;
  }
  state.notice = result.kind === 'changed' ? 'skillsChanged' : rejection(result.reason);
  return [];
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
  const available = state.open && props.open && !busy && !state.preview && readable(state);
  const document = state.document;
  const content: N[] = [];
  if (state.preview) {
    const preview = state.preview;
    const kind = preview.review.change.kind;
    content.push(
      ui.text('skills-preview-title', text.skillsConfirmTitle, 17, props.theme.text, 550),
      ui.text('skills-preview-target', preview.target),
      ui.text(
        'skills-preview-summary',
        kind === 'delete'
          ? text.skillsDeleteDetail
          : kind === 'update'
            ? text.skillsUpdateDetail
            : text.skillsInstallDetail,
      ),
      ui.text('skills-preview-count', `${text.skillsEntryCount}: ${preview.entries}`),
      ...(preview.locallyModified && kind === 'update'
        ? [ui.text('skills-preview-modified', text.skillsLocalModified)]
        : []),
      presentation.slot(
        'skills-preview-paths',
        'source',
        {
          source: preview.paths.map((path) => JSON.stringify(path)).join('\n'),
          appearance: props.theme.markdown.code,
          selection: props.theme.markdown.selection,
          terminal: props.theme.terminal,
          disabled: false,
          focusOnPointer: true,
        },
        {},
      ),
      ...(preview.pathsTruncated
        ? [ui.text('skills-preview-truncated', text.skillsPathsTruncated)]
        : []),
    );
  } else if (document) {
    const entry = document.entry;
    content.push(
      ui.text('skill-name', entry.name, 17, props.theme.text, 550),
      ui.text(
        'skill-source',
        entry.installed ? text.skillsInstalled : entry.source,
        13,
        props.theme.muted,
      ),
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
      const summary = compactLabel(
        `${entry.installed ? text.skillsInstalled : entry.source} · ${entry.description}`,
        tui ? 160 : 240,
      );
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
      ...(state.current
        ? [ui.button('check-change', text.skillsCheckResult, busy || !state.current.reference)]
        : []),
      ...(state.notice
        ? [ui.text('skills-notice', text[state.notice], 13, props.theme.muted)]
        : []),
      ...(state.cleanupUnconfirmed
        ? [ui.text('skills-cleanup', text.skillsCleanupUnknown, 13, props.theme.muted)]
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
      ...(document?.writable && !state.preview
        ? [
            ui.box(
              'skills-management',
              document.entry.installed
                ? [ui.button('delete-skill', text.skillsDelete, !available || !!state.current)]
                : [
                    ui.button(
                      'install-skill',
                      text.skillsInstall,
                      !available || !!state.current || !document.entry.valid,
                    ),
                    ui.button(
                      'update-skill',
                      text.skillsUpdate,
                      !available || !!state.current || !document.entry.valid,
                    ),
                  ],
              { row: true, gap: tui ? 1 : 8, terminal: { wrap: true }, style: { wrap: true } },
            ),
          ]
        : []),
      ui.box(
        'skills-controls',
        state.preview
          ? [
              ui.button('confirm-change', text.skillsConfirm, busy, true),
              ui.button('cancel-change', text.cancel, busy),
            ]
          : [
              ...(document
                ? [
                    ui.button('back', text.back, busy || !!state.preview),
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
              ui.button('refresh', text.refreshSessions, busy || !!state.preview),
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
        cleanupUnconfirmed: false,
        preview: null,
        current: null,
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
        state.preview = null;
        if (
          state.pending?.kind !== 'mutate' &&
          state.pending?.kind !== 'receipt' &&
          state.pending?.kind !== 'outcome'
        )
          state.pending = null;
      } else if (!state.open) {
        state.open = true;
        state.document = null;
        actions = state.current ? recover(state) : state.pending ? [] : search(state);
      }
      return frame(presentation, state, props, actions);
    },
    event(state, input, props) {
      state.revision++;
      const events: Emission[] = [];
      let actions: Action[] = [];
      if (input.kind === 'component' && input.child === 'skills-reading' && input.name === 'input')
        input = input.event as ComponentInput;
      if (
        (!state.open || !props.open) &&
        !(
          input.kind === 'action' &&
          input.key === state.pending?.key &&
          (state.pending.kind === 'mutate' ||
            state.pending.kind === 'receipt' ||
            state.pending.kind === 'outcome')
        )
      )
        return frame(presentation, state, props);
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
        if (request.kind === 'mutate') {
          if (result.status === 'operation') actions = settle(state, props, result.reply);
          else if (result.status === 'not_accepted') {
            state.current = null;
            state.notice = 'skillsFailed';
          } else state.notice = 'skillsUnknown';
        } else if (request.kind === 'receipt') {
          if (result.status === 'succeeded')
            actions = settle(state, props, result.value as OperationReply, true);
          else state.notice = 'skillsUnknown';
        } else if (request.kind === 'outcome') {
          if (result.status === 'succeeded') {
            const outcome = result.value as SkillLibraryOutcome;
            if (sameValue(state.current?.reference, outcome.original.reference)) {
              actions = settle(state, props, outcome.original);
              if (
                state.current &&
                outcome.published &&
                outcome.original.receipt.kind === 'recorded'
              )
                actions = settledResult(
                  state,
                  props,
                  outcome.published,
                  outcome.original.receipt.cleanup,
                );
            }
          } else state.notice = 'skillsUnknown';
        } else if (result.status !== 'succeeded') state.notice = 'skillsFailed';
        else if (request.kind === 'preview') {
          const preview = result.value as SkillLibraryPreview;
          if (preview.kind === 'ready') {
            state.preview = preview;
            reveal(state, 'skills-preview-title');
          } else
            state.notice = preview.kind === 'changed' ? 'skillsChanged' : rejection(preview.reason);
        } else if (request.kind === 'query') {
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
          if (state.preview) state.preview = null;
          else {
            state.open = false;
            if (
              state.pending?.kind !== 'mutate' &&
              state.pending?.kind !== 'receipt' &&
              state.pending?.kind !== 'outcome'
            )
              state.pending = null;
            events.push({ name: 'closed', value: {} });
          }
        } else if (!state.pending) {
          if (input.key === 'check-change') actions = recover(state);
          else if (input.key === 'cancel-change') state.preview = null;
          else if (input.key === 'confirm-change' && state.preview && !state.current) {
            const key = `skills-${++state.serial}`;
            state.pending = { kind: 'mutate', key };
            state.current = { reference: null };
            state.cleanupUnconfirmed = false;
            actions = [
              {
                key,
                binding: 'skills',
                member: 0,
                method: 'apply',
                input: state.preview.review,
                resources: {},
              },
            ];
            state.preview = null;
          } else if (state.preview) {
            /* Keep the reviewed change until confirmation or cancellation. */
          } else if (input.key === 'back') {
            state.document = null;
            reveal(state, rowKey(state, state.active));
          } else if (input.key === 'refresh' || input.key === 'search') {
            state.document = null;
            actions = search(state);
          } else if (readable(state)) {
            const document = state.document,
              page = state.page;
            if (document) {
              if (
                input.key === 'install-skill' ||
                input.key === 'update-skill' ||
                input.key === 'delete-skill'
              )
                actions = preview(
                  state,
                  input.key === 'install-skill'
                    ? 'install'
                    : input.key === 'update-skill'
                      ? 'update'
                      : 'delete',
                );
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
