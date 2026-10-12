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
  CallerIntent,
  Component,
  EditorCommand,
  EditorObservation,
  Emission,
  InputProps,
  OperationReply,
  Pricing,
  PricingEntry,
  PricingPage,
  PricingUpdate,
  RecoveryRef,
  SettingsPricingResult,
  SettingsUsagePage,
  SettingsUsageRange,
  SettingsUsageSummary,
  TextChunk,
  UsageActivity,
  UsageSelection,
  UsageModelTotals,
} from '../../../packages/plugin-sdk/src/index.js';
import { insightsContract, type InsightsProps } from '../../shared/contracts.js';
import { pageHeader, withPageShell, type PageState } from '../../shared/page-shell.js';
import {
  bookmark,
  bookmarkView,
  observeBookmark,
  retryBookmark,
  type Bookmark,
} from '../../shared/bookmark.js';
import { sameValue } from '../../shared/value.js';
import { copy } from '../../shared/locale.js';
import { tuiText } from '../../shared/tui-locale.js';
import { compactLabel, fill, output, type Node, type Presentation } from '../../shared/view.js';

type Tab = 'overview' | 'activity' | 'pricing';
const fields = ['model', 'input', 'output', 'cacheRead', 'cacheWrite'] as const;
type FieldName = (typeof fields)[number];
interface Field {
  observation: EditorObservation | null;
  command: EditorCommand | null;
  reset: string | null;
  sequence: number;
  ready: boolean;
}
type Request =
  | { kind: 'summary' | 'activity' | 'prices' | 'mutate' | 'receipt' }
  | { kind: 'history'; after: number }
  | { kind: 'search'; captured: EditorObservation }
  | { kind: 'field'; name: FieldName; captured: EditorObservation };
interface SavedView {
  tab: Tab;
  period: string;
  group: State['group'];
  selection: UsageSelection;
}
function savedView(value: unknown): SavedView | null {
  if (!value || typeof value !== 'object') return null;
  const v = value as SavedView;
  if (
    !['overview', 'activity', 'pricing'].includes(v.tab) ||
    !['1', '7', '30', 'all'].includes(v.period) ||
    !['byProvider', 'byModel', 'byTool'].includes(v.group) ||
    !v.selection ||
    ![null, 'model', 'tool'].includes(v.selection.kind ?? null) ||
    ![null, 'success', 'error', 'aborted', 'unknown', 'rejected'].includes(
      v.selection.status ?? null,
    ) ||
    typeof v.selection.search !== 'string' ||
    new TextEncoder().encode(v.selection.search).length > 1024 ||
    /[\u0000-\u001f\u007f]/.test(v.selection.search)
  )
    return null;
  return { tab: v.tab, period: v.period, group: v.group, selection: { ...v.selection } };
}
interface State extends PageState {
  bookmark: Bookmark<SavedView | null>;
  savedView: SavedView | null;
  search: Field;
  scope: InsightsProps['scope'];
  revision: number;
  serial: number;
  tab: Tab;
  period: string;
  range: SettingsUsageRange;
  selection: UsageSelection;
  report: SettingsUsageSummary | null;
  activity: SettingsUsagePage | null;
  offsets: number[];
  detail: UsageActivity | null;
  prices: Extract<PricingPage, { kind: 'page' }> | null;
  priceOffsets: number[];
  group: 'byProvider' | 'byModel' | 'byTool';
  groupOffsets: number[];
  editing: { key: number; entry: PricingEntry | null; revision: number } | null;
  fields: Record<FieldName, Field>;
  capture: {
    observations: Record<FieldName, EditorObservation>;
    values: Partial<Record<FieldName, string>>;
  } | null;
  pending: { key: string; request: Request } | null;
  current: { input: PricingUpdate; reference: RecoveryRef | null } | null;
  restoring: boolean;
  confirm: boolean;
  notice: string | null;
}
const freshField = (reset = ''): Field => ({
  observation: null,
  command: null,
  reset,
  sequence: 0,
  ready: false,
});
const newFields = () =>
  Object.fromEntries(fields.map((name) => [name, freshField()])) as Record<FieldName, Field>;
function initialize(field: Field) {
  if (field.reset !== null && field.observation && !field.command)
    field.command = {
      owner: field.observation.snapshot.text.owner,
      sequence: ++field.sequence,
      basedOn: field.observation.revision,
      edit: { kind: 'reset', text: field.reset },
    };
}
function request(
  state: State,
  actions: Action[],
  r: Request,
  binding: string,
  method: string,
  input: unknown,
  resources: Action['resources'] = {},
) {
  if (state.pending) return;
  const key = 'insights-' + ++state.serial;
  state.pending = { key, request: r };
  actions.push({ key, binding, member: 0, method, input, resources });
}
function refresh(state: State, actions: Action[]) {
  const to = Date.now();
  state.range = {
    ...state.range,
    from: state.period === 'all' ? 0 : Math.max(0, to - Number(state.period) * 86_400_000),
    to,
    through: null,
  };
  state.report = null;
  state.activity = null;
  state.offsets = [0];
  state.detail = null;
  state.groupOffsets = [0];
  summary(state, actions);
}
function summary(state: State, actions: Action[]) {
  request(state, actions, { kind: 'summary' }, 'settings', 'usageSummary', {
    range: state.range,
    group: state.group,
    offset: state.groupOffsets.at(-1)!,
  });
}
function activity(state: State, actions: Action[]) {
  request(state, actions, { kind: 'activity' }, 'settings', 'usage', {
    range: state.range,
    selection: state.selection,
    offset: state.offsets.at(-1)!,
  });
}
function prices(state: State, actions: Action[], offset = 0) {
  request(
    state,
    actions,
    { kind: 'prices' },
    'settings',
    'prices',
    offset && state.prices
      ? { kind: 'continue', revision: state.prices.revision, offset }
      : { kind: 'start' },
  );
}
function history(state: State, actions: Action[], after = 0) {
  state.restoring = true;
  if (!after) state.current = null;
  request(state, actions, { kind: 'history', after }, 'history', 'page', {
    after,
    limit: 16,
  });
}
function mutate(state: State, actions: Action[], input: PricingUpdate) {
  state.current = { input, reference: null };
  state.notice = null;
  state.confirm = false;
  request(state, actions, { kind: 'mutate' }, 'settings', 'updatePrice', input);
  actions.at(-1)!.recovery = { kind: 'insights-price/v1', input };
}
function settle(state: State, reply: OperationReply, actions: Action[]) {
  if (
    !state.current ||
    (state.current.reference && !sameValue(state.current.reference, reply.reference))
  )
    return;
  state.current.reference = reply.reference;
  if (reply.receipt.kind !== 'recorded' || reply.receipt.outcome.kind !== 'returned') {
    state.notice = 'insights-unknown';
    return;
  }
  const result = reply.receipt.outcome.value as SettingsPricingResult;
  state.current = null;
  if (result.kind === 'updated' && result.result.kind !== 'revision_conflict') {
    state.notice = 'insights-saved';
    state.editing = null;
    state.priceOffsets = [0];
    if (state.tab === 'pricing') prices(state, actions);
    else refresh(state, actions);
  } else {
    state.notice = result.kind === 'rejected' ? 'insights-invalid' : 'insights-conflict';
    if (!state.report && state.tab !== 'pricing') refresh(state, actions);
  }
}
function edit(state: State, entry: PricingEntry | null) {
  if (!state.prices) return;
  state.editing = {
    key: ++state.serial,
    entry,
    revision: state.prices.revision,
  };
  state.fields = newFields();
  state.confirm = false;
  state.notice = null;
  const p = entry?.pricing;
  const values = {
    model: p?.modelKey ?? '',
    input: p?.inputUsdPer1M,
    output: p?.outputUsdPer1M,
    cacheRead: p?.cacheReadUsdPer1M,
    cacheWrite: p?.cacheWriteUsdPer1M,
  };
  for (const name of fields)
    state.fields[name].reset = values[name] === undefined ? '' : String(values[name]);
}
function capture(state: State, actions: Action[]) {
  const form = state.capture!;
  const name = fields.find((name) => form.values[name] === undefined);
  if (name) {
    const captured = form.observations[name];
    request(
      state,
      actions,
      { kind: 'field', name, captured },
      'reader',
      'read',
      { start: 0, end: captured.snapshot.text.bytes },
      {
        text: {
          child: `insights-field-${state.editing!.key}-${name}`,
          method: 'snapshot',
          input: {
            kind: 'at_revision',
            owner: captured.snapshot.text.owner,
            revision: captured.snapshot.text.revision,
          },
        },
      },
    );
    return;
  }
  state.capture = null;
  try {
    if (
      !state.editing ||
      fields.some(
        (name) =>
          !sameValue(
            state.fields[name].observation?.snapshot.text,
            form.observations[name].snapshot.text,
          ),
      )
    )
      throw Error();
    const rate = (name: FieldName, optional = false) => {
      const text = form.values[name]!.trim();
      if (!text && optional) return undefined;
      if (!/^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(text)) throw Error();
      const n = Number(text);
      if (!Number.isFinite(n) || n < 0) throw Error();
      return n;
    };
    const modelKey = form.values.model!;
    if (
      !modelKey ||
      modelKey.trim() !== modelKey ||
      modelKey.length > 128 ||
      /[\u0000-\u001f\u007f]/.test(modelKey)
    )
      throw Error();
    const pricing: Pricing = {
      modelKey,
      inputUsdPer1M: rate('input')!,
      outputUsdPer1M: rate('output')!,
    };
    const read = rate('cacheRead', true),
      write = rate('cacheWrite', true);
    if (read !== undefined) pricing.cacheReadUsdPer1M = read;
    if (write !== undefined) pricing.cacheWriteUsdPer1M = write;
    mutate(state, actions, {
      expectedRevision: state.editing.revision,
      mutation: { kind: 'upsert', pricing },
    });
  } catch {
    state.notice = 'insights-invalid';
  }
}
const money = (n: number) => '$' + (n === 0 ? '0' : n.toFixed(n < 1 ? 6 : 2));
export function insights<N extends Node>(
  presentation: Presentation<N>,
): Component<State, InsightsProps, N> {
  function frame(
    state: State,
    props: InsightsProps,
    actions: Action[] = [],
    events: Emission[] = [],
  ) {
    if (
      !sameValue(state.scope, props.scope) &&
      !state.pending &&
      !state.current &&
      !state.restoring
    ) {
      state.scope = props.scope;
      state.range.sessionId = props.scope?.sessionId ?? null;
      refresh(state, actions);
    }
    const ui = presentation.paint(props.theme),
      t = (key: string) => tuiText(props.locale, key),
      text = copy(props.locale);
    const busy =
      !props.open ||
      props.busy ||
      !!state.pending ||
      !!state.current ||
      state.restoring ||
      !!state.capture;
    const label = (id: string, value: string, muted = false) =>
      ui.text(id, value, 13, muted ? props.theme.muted : props.theme.text);
    const button = (key: string, value: string, disabled = busy) =>
      ui.button(key, value, disabled, false, { background: 0, align: 'start' });
    const cost = (m: UsageModelTotals) =>
      money(m.cost.knownUsd) +
      (m.cost.unvalued ? ` + ${m.cost.unvalued} ${t('insights-unvalued')}` : '');
    const choose = (
      key: string,
      title: string,
      options: { key: string; label: string }[],
      current: string,
      disabled: boolean,
    ) =>
      ui.choice
        ? ui.choice(key, title, options, current, disabled)
        : ui.box(
            key,
            [
              label(key + '-label', title),
              ...options.map((option) =>
                ui.button(key + ':' + option.key, option.label, disabled, current === option.key),
              ),
            ],
            { row: true, gap: 1, style: { wrap: true } },
          );
    const rows: N[] = [];
    const tabs: Tab[] = ['overview', 'activity', 'pricing'];
    const tabBar = ui.box(
      'insights-tabs',
      tabs.map((tab) => ui.button('tab-' + tab, t('insights-' + tab), busy, state.tab === tab)),
      { row: true, gap: 1 },
    );
    if (state.tab !== 'pricing') {
      rows.push(
        choose(
          'period',
          t('insights-range'),
          ['1', '7', '30', 'all'].map((key) => ({
            key,
            label: t('insights-range-' + key),
          })),
          state.period,
          busy,
        ),
      );
      rows.push(
        ui.box(
          'insights-scope',
          [
            label(
              'insights-scope-label',
              state.range.sessionId
                ? state.scope?.label || state.range.sessionId
                : t('insights-all-sessions'),
              true,
            ),
            ...(state.scope?.sessionId
              ? [
                  button(
                    'scope-toggle',
                    t(state.range.sessionId ? 'insights-all-sessions' : 'insights-this-session'),
                  ),
                ]
              : []),
            button('refresh', t('insights-refresh')),
          ],
          { gap: 0 },
        ),
      );
    }
    if (state.tab === 'overview' && state.report) {
      const s = state.report.summary,
        m = s.models;
      rows.push(
        label('usage-cost', cost(m)),
        label('usage-estimate', t('insights-estimate'), true),
      );
      const metrics: N[] = [];
      for (const [key, value] of [
        ['input', m.input],
        ['output', m.output],
        ['cacheRead', m.cacheRead],
        ['cacheWrite', m.cacheWrite],
        ['reasoning', m.reasoning],
      ] as const)
        metrics.push(
          label(
            'usage-' + key,
            `${t('insights-' + key)}: ${value.known}${value.missing ? ` + ${value.missing} ${t('insights-unreported')}` : ''}`,
            true,
          ),
        );
      metrics.push(
        label(
          'usage-calls',
          `${t('insights-models')}: ${m.calls} · ${t('insights-tools')}: ${s.tools.calls}`,
          true,
        ),
      );
      metrics.push(
        label(
          'usage-outcomes',
          `${t('insights-error')}: ${m.error + s.tools.error} · ${t('insights-aborted')}: ${m.aborted} · ${t('insights-unknown-status')}: ${m.unknown + s.tools.unknown} · ${t('insights-rejected')}: ${s.tools.rejected}`,
          true,
        ),
      );
      metrics.push(
        label(
          'usage-pending',
          `${t('insights-pending')}: ${s.pending.models + s.pending.tools}`,
          true,
        ),
      );
      rows.push(ui.box('usage-metrics', metrics, { gap: 0 }));
      rows.push(
        choose(
          'group',
          t('insights-group'),
          ['byProvider', 'byModel', 'byTool'].map((key) => ({
            key,
            label: t('insights-' + key),
          })),
          state.group,
          busy,
        ),
      );
      const items =
        state.group === 'byProvider'
          ? s.byProvider.map((x) => [x.providerId ?? t('insights-unknown-status'), cost(x.totals)])
          : state.group === 'byModel'
            ? s.byModel.map((x) => [x.modelId, `${cost(x.totals)} · ${x.totals.calls}`])
            : s.byTool.map((x) => [
                x.name,
                `${x.totals.calls} · ${t('insights-error')}: ${x.totals.error} · ${t('insights-unknown-status')}: ${x.totals.unknown} · ${t('insights-rejected')}: ${x.totals.rejected}`,
              ]);
      items.forEach(([name, value], i) =>
        rows.push(label('group-' + i, `${compactLabel(name!, 100)}  ${value}`)),
      );
      rows.push(
        ui.box(
          'group-paging',
          [
            button('group-prev', t('insights-previous'), busy || state.groupOffsets.length < 2),
            button('group-next', t('insights-next'), busy || state.report.nextOffset == null),
          ],
          { row: true, gap: 1 },
        ),
      );
    } else if (state.tab === 'activity') {
      rows.push(
        choose(
          'kind',
          t('insights-kind'),
          ['all', 'model', 'tool'].map((key) => ({
            key,
            label: t('insights-' + key),
          })),
          state.selection.kind ?? 'all',
          busy,
        ),
      );
      rows.push(
        choose(
          'status',
          t('insights-status'),
          ['all', 'success', 'error', 'aborted', 'unknown', 'rejected'].map((key) => ({
            key,
            label: t('insights-' + (key === 'unknown' ? 'unknown-status' : key)),
          })),
          state.selection.status ?? 'all',
          busy,
        ),
      );
      if (state.detail) {
        const a = state.detail;
        rows.push(button('detail-close', text.back));
        const detail: [string, unknown][] = [
          ['insights-request', a.attempt.requestId],
          ['insights-completed', new Date(a.attempt.completedAt).toISOString()],
          ['insights-binding', a.attempt.binding ? JSON.stringify(a.attempt.binding) : '—'],
        ];
        if (a.kind === 'model')
          detail.push(
            ['insights-model', a.attempt.modelId],
            ['insights-origin', JSON.stringify(a.attempt.origin)],
            ['insights-started', new Date(a.attempt.startedAt).toISOString()],
            [
              'insights-duration',
              a.attempt.outcome === 'unknown'
                ? '—'
                : Math.max(0, a.attempt.completedAt - a.attempt.startedAt),
            ],
            ...(
              [
                ['input', 'input_tokens'],
                ['output', 'output_tokens'],
                ['cacheRead', 'cache_read_tokens'],
                ['cacheWrite', 'cache_write_tokens'],
                ['reasoning', 'reasoning_tokens'],
              ] as const
            ).map(([label, key]): [string, unknown] => [
              'insights-' + label,
              a.attempt.usage[key] ?? t('insights-unreported'),
            ]),
            ['insights-session', a.attempt.sessionId ?? '—'],
            [
              'insights-status',
              t(
                'insights-' +
                  (a.attempt.outcome === 'unknown' ? 'unknown-status' : a.attempt.outcome),
              ),
            ],
            [
              'insights-cost',
              a.attempt.costUsd == null ? t('insights-unvalued') : money(a.attempt.costUsd),
            ],
            [
              'insights-quote',
              a.attempt.quote?.pricing
                ? JSON.stringify(a.attempt.quote.pricing)
                : t('insights-unpriced'),
            ],
          );
        else
          detail.push(
            ['insights-tool', a.attempt.name],
            ['insights-session', a.attempt.invocation.session_id],
            ['insights-origin', JSON.stringify(a.attempt.invocation)],
            ['insights-call', JSON.stringify(a.attempt.call)],
            ...(a.attempt.result.kind === 'rejected'
              ? [['insights-rejection', a.attempt.result.reason] as [string, unknown]]
              : ([
                  ['insights-started', new Date(a.attempt.result.startedAt).toISOString()],
                  [
                    'insights-duration',
                    a.attempt.result.outcome === 'unknown'
                      ? '—'
                      : Math.max(0, a.attempt.completedAt - a.attempt.result.startedAt),
                  ],
                ] as [string, unknown][])),
            [
              'insights-status',
              t(
                'insights-' +
                  (a.attempt.result.kind === 'rejected'
                    ? 'rejected'
                    : a.attempt.result.outcome === 'unknown'
                      ? 'unknown-status'
                      : a.attempt.result.outcome),
              ),
            ],
          );
        detail.forEach(([key, value], i) => rows.push(label('detail-' + i, `${t(key)}: ${value}`)));
      } else if (state.activity) {
        rows.push(
          label(
            'activity-count',
            String(state.activity.total) + ' ' + t('insights-attempts'),
            true,
          ),
        );
        state.activity.attempts.forEach((a, i) =>
          rows.push(
            button(
              'attempt-' + i,
              compactLabel(
                a.kind === 'model'
                  ? `${a.attempt.modelId} · ${t('insights-' + (a.attempt.outcome === 'unknown' ? 'unknown-status' : a.attempt.outcome))} · ${a.attempt.costUsd == null ? t('insights-unvalued') : money(a.attempt.costUsd)}`
                  : `${a.attempt.name} · ${t('insights-' + (a.attempt.result.kind === 'rejected' ? 'rejected' : a.attempt.result.outcome === 'unknown' ? 'unknown-status' : a.attempt.result.outcome))}`,
                180,
              ),
            ),
          ),
        );
        rows.push(
          ui.box(
            'activity-paging',
            [
              button('activity-prev', t('insights-previous'), busy || state.offsets.length < 2),
              button(
                'activity-next',
                t('insights-next'),
                busy || state.activity.nextOffset == null,
              ),
            ],
            { row: true, gap: 1 },
          ),
        );
      }
    } else if (state.tab === 'pricing' && !state.editing) {
      rows.push(
        button('price-new', t('insights-new')),
        button('price-refresh', t('insights-refresh')),
      );
      state.prices?.entries.forEach((entry, i) =>
        rows.push(
          button(
            'price-' + i,
            `${compactLabel(entry.pricing.modelKey, 100)}  $${entry.pricing.inputUsdPer1M} / $${entry.pricing.outputUsdPer1M}${entry.source === 'custom' ? ' *' : ''}`,
          ),
        ),
      );
      rows.push(
        ui.box(
          'prices-paging',
          [
            button('prices-prev', t('insights-previous'), busy || state.priceOffsets.length < 2),
            button('prices-next', t('insights-next'), busy || state.prices?.nextOffset == null),
          ],
          { row: true, gap: 1 },
        ),
      );
    }
    const editorRows: N[] = [];
    if (state.editing) {
      editorRows.push(label('price-unit', t('insights-unit'), true));
      for (const name of fields)
        editorRows.push(
          ui.box(
            'price-field-' + name,
            [
              label('price-label-' + name, t('insights-' + name), true),
              presentation.slot(
                `insights-field-${state.editing.key}-${name}`,
                'input',
                {
                  appearance:
                    presentation.target === 'tui'
                      ? {
                          ...props.theme.input,
                          borderWidth: 0,
                          padding: { top: 0, right: 0, bottom: 0, left: 0 },
                        }
                      : props.theme.input,
                  accessibilityLabel: t('insights-' + name),
                  placeholder: '',
                  autoFocus: name === 'model',
                  tabIndex: fields.indexOf(name) + 1,
                  readOnly: name === 'model' && !!state.editing.entry,
                  disabled: busy || !state.fields[name].ready || !!state.fields[name].command,
                  command: state.fields[name].command,
                } satisfies InputProps,
                { changed: 'changed' },
              ),
            ],
            { gap: 0 },
          ),
        );
      editorRows.push(
        button(
          'price-save',
          t('insights-price-save'),
          busy || fields.some((name) => !state.fields[name].ready || !!state.fields[name].command),
        ),
      );
      if (state.editing.entry?.source === 'custom')
        editorRows.push(button('price-remove', t('insights-remove')));
      if (state.confirm) {
        editorRows.push(
          label(
            'price-remove-effect',
            t(
              state.editing.entry?.source === 'custom' &&
                state.editing.entry.resetEffect === 'restore_builtin'
                ? 'insights-restore-builtin'
                : 'insights-become-unpriced',
            ),
          ),
        );
        editorRows.push(button('price-confirm', t('insights-confirm')));
      }
      editorRows.push(button('price-close', t('insights-close')));
    }
    const feedback: N[] = [];
    if (presentation.target === 'tui' && state.bookmark.observed?.state.kind === 'failed')
      feedback.push(button('view-retry', t('insights-view-retry'), false));
    if (state.notice) feedback.push(label('insights-notice', t(state.notice), true));
    if (state.current)
      feedback.push(
        button('receipt', text.executionReceipt, !!state.pending || !state.current.reference),
      );
    if ((state.restoring || (state.current && !state.current.reference)) && !state.pending)
      feedback.push(button('history', text.refreshDefaultOperations, false));
    const result = output(
      state,
      ui.box(
        'insights-root',
        [
          ...pageHeader(presentation, state, props, events, t('settings-insights')),
          tabBar,
          ...(presentation.target === 'tui'
            ? [
                ui.box(
                  'insights-checkpoint',
                  [
                    bookmarkView(
                      presentation,
                      state.bookmark,
                      'insights-bookmark',
                      'insights/view',
                      state.savedView,
                    ),
                  ],
                  { visible: false },
                ),
                ui.box(
                  'insights-view-actions',
                  [
                    button(
                      'view-save',
                      t(
                        state.bookmark.saved &&
                          !state.bookmark.command &&
                          sameValue(state.bookmark.saved, {
                            tab: state.tab,
                            period: state.period,
                            group: state.group,
                            selection: state.selection,
                          })
                          ? 'insights-view-saved'
                          : 'insights-view-save',
                      ),
                      busy ||
                        state.bookmark.observed?.state.kind !== 'ready' ||
                        !!state.bookmark.command,
                    ),
                    button(
                      'view-load',
                      t('insights-view-load'),
                      busy || !state.bookmark.saved || !!state.bookmark.command,
                    ),
                  ],
                  { row: true, gap: 1 },
                ),
              ]
            : []),
          ...feedback,
          ui.box(
            'insights-search',
            [
              presentation.slot(
                'insights-search-input',
                'input',
                {
                  appearance:
                    presentation.target === 'tui'
                      ? {
                          ...props.theme.input,
                          borderWidth: 0,
                          padding: { top: 0, right: 0, bottom: 0, left: 0 },
                        }
                      : props.theme.input,
                  accessibilityLabel: t('insights-search'),
                  placeholder: t('insights-search'),
                  autoFocus: false,
                  tabIndex: 0,
                  readOnly: false,
                  disabled: busy || !state.search.ready || !!state.search.command,
                  command: state.search.command,
                } satisfies InputProps,
                { changed: 'changed' },
              ),
              button(
                'search-apply',
                t('insights-search-apply'),
                busy || !state.search.ready || !!state.search.command,
              ),
            ],
            { visible: state.tab === 'activity', gap: 0 },
          ),
          ui.box('insights-content-' + state.tab, rows, {
            visible: state.tab !== 'pricing' || !state.editing,
            gap: 1,
            scroll: 'manual',
            terminal: { grow: 1 },
            style: { width: fill, height: fill },
          }),
          ...(state.editing
            ? [
                ui.box('insights-price-editor-' + state.editing.key, editorRows, {
                  visible: state.tab === 'pricing',
                  gap: 1,
                  scroll: 'manual',
                  terminal: { grow: 1 },
                  style: { width: fill, height: fill },
                }),
              ]
            : []),
        ],
        {
          visible: props.open,
          gap: 1,
          terminal: { grow: 1 },
          style: { width: fill, height: fill },
          shortcuts: [{ key: 'escape', modifiers: [], input: 'back', enabled: true }],
        },
      ),
      events,
    );
    result.actions = actions;
    return result;
  }
  return withPageShell<State, InsightsProps, N>(
    {
      mount(props) {
        const state: State = {
          revision: 0,
          serial: 0,
          tab: 'overview',
          period: '7',
          range: { from: 0, to: 0, through: null, sessionId: props.scope?.sessionId ?? null },
          selection: { kind: null, status: null, search: '' },
          report: null,
          activity: null,
          offsets: [0],
          detail: null,
          prices: null,
          priceOffsets: [0],
          group: 'byProvider',
          groupOffsets: [0],
          scope: props.scope,
          bookmark: bookmark<SavedView | null>(null),
          savedView: null,
          search: freshField(),
          editing: null,
          fields: newFields(),
          capture: null,
          pending: null,
          current: null,
          restoring: true,
          confirm: false,
          notice: null,
        };
        const actions: Action[] = [];
        history(state, actions);
        return frame(state, props, actions);
      },
      update(state, props) {
        state.revision++;
        return frame(state, props);
      },
      event(state, input, props) {
        state.revision++;
        const actions: Action[] = [],
          events: Emission[] = [];
        if (input.kind === 'component' && input.child === 'insights-bookmark') {
          const restored = observeBookmark(
            state.bookmark,
            input,
            'insights-bookmark',
            state.savedView,
            savedView,
          );
          if (restored) state.savedView = restored;
        } else if (
          input.kind === 'component' &&
          input.child === 'insights-search-input' &&
          input.name === 'changed'
        ) {
          const field = state.search,
            observed = input.event as EditorObservation;
          if (
            field.observation &&
            field.observation.snapshot.text.owner !== observed.snapshot.text.owner
          ) {
            field.command = null;
            field.ready = false;
            field.reset = state.selection.search ?? '';
          }
          field.observation = observed;
          if (
            field.command &&
            observed.receipt?.owner === field.command.owner &&
            observed.receipt.sequence === field.command.sequence
          ) {
            field.ready = observed.receipt.outcome.status === 'applied';
            field.command = null;
            field.reset = null;
          } else initialize(field);
        } else if (input.kind === 'component' && state.editing) {
          const name = fields.find(
            (name) => input.child === `insights-field-${state.editing!.key}-${name}`,
          );
          if (name && input.name === 'changed') {
            const field = state.fields[name],
              observed = input.event as EditorObservation;
            if (
              field.observation &&
              field.observation.snapshot.text.owner !== observed.snapshot.text.owner
            ) {
              field.command = null;
              field.ready = false;
              state.notice = 'insights-editor-changed';
            }
            field.observation = observed;
            if (
              field.command &&
              observed.receipt?.owner === field.command.owner &&
              observed.receipt.sequence === field.command.sequence
            ) {
              field.ready = observed.receipt.outcome.status === 'applied';
              field.command = null;
              field.reset = null;
              if (!field.ready) state.notice = 'insights-unavailable';
            } else initialize(field);
          }
        } else if (input.kind === 'action' && input.key === state.pending?.key) {
          const r = state.pending.request,
            result = input.event as ActionOutcome;
          state.pending = null;
          if (r.kind === 'mutate') {
            if (result.status === 'operation') settle(state, result.reply, actions);
            else if (result.status === 'not_accepted') {
              state.current = null;
              state.notice = 'insights-unavailable';
            } else state.notice = 'insights-unknown';
          } else if (r.kind === 'receipt') {
            if (result.status === 'succeeded')
              settle(state, result.value as OperationReply, actions);
            else state.notice = 'insights-unknown';
          } else if (result.status !== 'succeeded') {
            state.capture = null;
            state.notice = 'insights-unavailable';
          } else if (r.kind === 'history') {
            const items = result.value as CallerIntent[];
            for (const item of items) {
              const recovery = item.recovery as {
                kind?: string;
                input?: PricingUpdate;
              } | null;
              if (
                item.reference.method === 'updatePrice' &&
                item.reference.contract.name === 'maka.settings.Settings' &&
                recovery?.kind === 'insights-price/v1' &&
                recovery.input
              )
                state.current = {
                  input: recovery.input,
                  reference: item.reference,
                };
            }
            const last = items.at(-1);
            if (last && last.cursor > r.after) history(state, actions, last.cursor);
            else if (last) state.notice = 'insights-unavailable';
            else {
              state.restoring = false;
              if (state.current?.reference)
                request(
                  state,
                  actions,
                  { kind: 'receipt' },
                  'receipts',
                  'read',
                  state.current.reference,
                );
              else refresh(state, actions);
            }
          } else if (r.kind === 'search') {
            const chunk = result.value as TextChunk;
            if (
              !sameValue(chunk.metadata, r.captured.snapshot.text) ||
              !sameValue(state.search.observation?.snapshot.text, r.captured.snapshot.text) ||
              new TextEncoder().encode(chunk.text).length > 1024 ||
              /[\u0000-\u001f\u007f]/.test(chunk.text)
            )
              state.notice = 'insights-search-invalid';
            else {
              state.selection.search = chunk.text;
              state.offsets = [0];
              state.detail = null;
              activity(state, actions);
            }
          } else if (r.kind === 'summary') {
            state.report = result.value as SettingsUsageSummary;
            state.range.through = state.report.through;
            if (state.tab === 'activity') activity(state, actions);
            else if (state.tab === 'pricing') prices(state, actions);
          } else if (r.kind === 'activity') state.activity = result.value as SettingsUsagePage;
          else if (r.kind === 'prices') {
            const page = result.value as PricingPage;
            if (page.kind === 'page') state.prices = page;
            else {
              state.prices = null;
              state.notice = 'insights-conflict';
              state.priceOffsets = [0];
            }
          } else if (r.kind === 'field') {
            const chunk = result.value as TextChunk;
            if (
              !state.capture ||
              !sameValue(chunk.metadata, r.captured.snapshot.text) ||
              !sameValue(state.fields[r.name].observation?.snapshot.text, r.captured.snapshot.text)
            ) {
              state.capture = null;
              state.notice = 'insights-invalid';
            } else {
              state.capture.values[r.name] = chunk.text;
              capture(state, actions);
            }
          }
        } else if (input.kind === 'raw' && props.open) {
          let key = input.key,
            value = (input.event as { key?: string }).key;
          if (key.includes(':')) [key, value] = key.split(':') as [string, string];
          if (key === 'back') events.push({ name: 'closed', value: {} });
          else if (key === 'receipt' && !state.pending && state.current?.reference)
            request(
              state,
              actions,
              { kind: 'receipt' },
              'receipts',
              'read',
              state.current.reference,
            );
          else if (key === 'view-retry') retryBookmark(state.bookmark);
          else if (
            key === 'history' &&
            !state.pending &&
            (state.restoring || (state.current && !state.current.reference))
          )
            history(state, actions);
          else if (
            !props.busy &&
            !state.pending &&
            !state.current &&
            !state.restoring &&
            !state.capture
          ) {
            if (
              key.startsWith('tab-') &&
              ['overview', 'activity', 'pricing'].includes(key.slice(4))
            ) {
              state.tab = key.slice(4) as Tab;
              if (state.tab === 'pricing' && !state.prices) prices(state, actions);
              else if (state.tab === 'activity' && !state.activity) activity(state, actions);
            } else if (
              key === 'view-save' &&
              presentation.target === 'tui' &&
              state.bookmark.observed?.state.kind === 'ready' &&
              !state.bookmark.command
            ) {
              state.savedView = {
                tab: state.tab,
                period: state.period,
                group: state.group,
                selection: { ...state.selection },
              };
            } else if (key === 'view-load' && state.bookmark.saved && !state.bookmark.command) {
              const view = state.bookmark.saved;
              state.tab = view.tab;
              state.period = view.period;
              state.group = view.group;
              state.selection = { ...view.selection };
              state.search.command = null;
              state.search.reset = view.selection.search ?? '';
              state.search.ready = false;
              initialize(state.search);
              refresh(state, actions);
            } else if (key === 'scope-toggle' && state.scope?.sessionId) {
              state.range.sessionId = state.range.sessionId ? null : state.scope.sessionId;
              refresh(state, actions);
            } else if (
              key === 'search-apply' &&
              state.search.ready &&
              !state.search.command &&
              state.search.observation &&
              !state.search.observation.snapshot.marked &&
              state.search.observation.snapshot.text.bytes <= 1024
            ) {
              const captured = state.search.observation;
              request(
                state,
                actions,
                { kind: 'search', captured },
                'reader',
                'read',
                { start: 0, end: captured.snapshot.text.bytes },
                {
                  text: {
                    child: 'insights-search-input',
                    method: 'snapshot',
                    input: {
                      kind: 'at_revision',
                      owner: captured.snapshot.text.owner,
                      revision: captured.snapshot.text.revision,
                    },
                  },
                },
              );
            } else if (key === 'search-apply') state.notice = 'insights-search-invalid';
            else if (key === 'refresh') refresh(state, actions);
            else if (key === 'period' && value && ['1', '7', '30', 'all'].includes(value)) {
              state.period = value;
              refresh(state, actions);
            } else if (
              key === 'group' &&
              value &&
              ['byProvider', 'byModel', 'byTool'].includes(value)
            ) {
              state.group = value as State['group'];
              state.groupOffsets = [0];
              summary(state, actions);
            } else if (key === 'group-prev' && state.groupOffsets.length > 1) {
              state.groupOffsets.pop();
              summary(state, actions);
            } else if (key === 'group-next' && state.report?.nextOffset != null) {
              state.groupOffsets.push(state.report.nextOffset);
              summary(state, actions);
            } else if (key === 'kind' && value && ['all', 'model', 'tool'].includes(value)) {
              state.selection.kind = value === 'all' ? null : (value as 'model' | 'tool');
              state.offsets = [0];
              state.detail = null;
              activity(state, actions);
            } else if (
              key === 'status' &&
              value &&
              ['all', 'success', 'error', 'aborted', 'unknown', 'rejected'].includes(value)
            ) {
              state.selection.status = value === 'all' ? null : (value as UsageSelection['status']);
              state.offsets = [0];
              state.detail = null;
              activity(state, actions);
            } else if (key === 'activity-next' && state.activity?.nextOffset != null) {
              state.offsets.push(state.activity.nextOffset);
              activity(state, actions);
            } else if (key === 'activity-prev' && state.offsets.length > 1) {
              state.offsets.pop();
              activity(state, actions);
            } else if (key.startsWith('attempt-'))
              state.detail = state.activity?.attempts[Number(key.slice(8))] ?? null;
            else if (key === 'detail-close') state.detail = null;
            else if (key === 'price-refresh') {
              state.priceOffsets = [0];
              prices(state, actions);
            } else if (key === 'prices-next' && state.prices?.nextOffset != null) {
              state.priceOffsets.push(state.prices.nextOffset);
              prices(state, actions, state.prices.nextOffset);
            } else if (key === 'prices-prev' && state.priceOffsets.length > 1) {
              state.priceOffsets.pop();
              prices(state, actions, state.priceOffsets.at(-1)!);
            } else if (key === 'price-new') edit(state, null);
            else if (/^price-\d+$/.test(key)) {
              const entry = state.prices?.entries[Number(key.slice(6))];
              if (entry) edit(state, entry);
            } else if (key === 'price-close') {
              state.editing = null;
              state.confirm = false;
            } else if (key === 'price-remove' && state.editing?.entry?.source === 'custom')
              state.confirm = true;
            else if (
              key === 'price-confirm' &&
              state.confirm &&
              state.editing?.entry?.source === 'custom'
            )
              mutate(state, actions, {
                expectedRevision: state.editing.revision,
                mutation: {
                  kind: 'delete',
                  modelKey: state.editing.entry.pricing.modelKey,
                },
              });
            else if (key === 'price-save' && state.editing) {
              if (
                fields.some(
                  (name) =>
                    !state.fields[name].ready ||
                    state.fields[name].command ||
                    !state.fields[name].observation ||
                    state.fields[name].observation!.snapshot.marked ||
                    state.fields[name].observation!.snapshot.text.bytes >
                      (name === 'model' ? 512 : 64),
                )
              )
                state.notice = 'insights-invalid';
              else {
                state.capture = {
                  observations: Object.fromEntries(
                    fields.map((name) => [name, state.fields[name].observation!]),
                  ) as Record<FieldName, EditorObservation>,
                  values: {},
                };
                capture(state, actions);
              }
            }
          }
        }
        return frame(state, props, actions, events);
      },
    },
    () => 'back',
  );
}
export function activate() {
  return {
    Insights: {
      kind: 'component',
      contract: insightsContract,
      component: ['component'],
    },
  };
}
