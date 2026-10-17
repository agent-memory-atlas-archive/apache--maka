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
  AgentDiagnostics,
  AgentDiagnostic,
  AgentDiagnosticPage,
  AgentDiagnosticChange,
} from '../../../packages/plugin-sdk/src/index.js';
import type { Theme } from '../../shared/contracts.js';
import { type Presentation, type Node, fill } from '../../shared/view.js';
import { copy, type Copy } from './locale.js';
import { intent } from './actions.js';
import type { State } from './state.js';

export interface DiagnosticsState {
  watch: { key: string; input: { sessionId: string } } | null;
  revision: number;
  through: number;
  page: AgentDiagnosticPage | null;
  position: AgentDiagnostics['position'];
  selected: AgentDiagnostic | null;
  attempt: number;
  loading: number | null;
  failed: boolean;
}
export type DiagnosticsRequest = {
  kind: 'diagnostics';
  epoch: number;
  attempt: number;
  position: AgentDiagnostics['position'];
};
export function newDiagnostics(sessionId?: string, epoch = 0): DiagnosticsState {
  return {
    watch: sessionId ? { key: 'diagnostics-watch-' + epoch, input: { sessionId } } : null,
    revision: 0,
    through: 0,
    page: null,
    position: { kind: 'latest' },
    selected: null,
    attempt: 0,
    loading: null,
    failed: false,
  };
}
function open(state: State) {
  return state.inspector === 'diagnostics' || state.dialog === 'diagnostics';
}
export function readDiagnostics(
  state: State,
  actions: Action[],
  position: AgentDiagnostics['position'] = { kind: 'latest' },
) {
  if (!state.selected) return;
  if (!state.diagnostics.watch)
    state.diagnostics.watch = {
      key: `diagnostics-watch-${state.epoch}-${++state.serial}`,
      input: { sessionId: state.selected.id },
    };
  const attempt = ++state.serial;
  state.diagnostics.attempt = attempt;
  state.diagnostics.loading = attempt;
  state.diagnostics.position = position;
  state.diagnostics.failed = false;
  intent(
    state,
    actions,
    { kind: 'diagnostics', epoch: state.epoch, attempt, position },
    'agent',
    'diagnostics',
    {
      sessionId: state.selected.id,
      position,
    } satisfies AgentDiagnostics,
  );
}
export function diagnosticChange(state: State, actions: Action[], change: AgentDiagnosticChange) {
  if (change.revision <= state.diagnostics.revision) return;
  state.diagnostics.revision = change.revision;
  if (
    open(state) &&
    state.diagnostics.position.kind === 'latest' &&
    change.revision > state.diagnostics.through
  )
    readDiagnostics(state, actions);
}
export function diagnosticResult(
  state: State,
  request: DiagnosticsRequest,
  page: AgentDiagnosticPage | null,
) {
  const diagnostics = state.diagnostics;
  if (request.epoch !== state.epoch || request.attempt !== diagnostics.attempt) return;
  diagnostics.loading = null;
  if (!page || page.through < diagnostics.through) {
    diagnostics.failed = true;
    return;
  }
  diagnostics.through = page.through;
  diagnostics.position = request.position;
  diagnostics.page = page;
  diagnostics.failed = false;
}
export function diagnosticEvent(state: State, actions: Action[], key: string): boolean {
  if (!key.startsWith('diagnostic')) return false;
  const diagnostics = state.diagnostics;
  if (key === 'diagnostics-close') {
    if (state.dialog === 'diagnostics') state.dialog = null;
    if (state.inspector === 'diagnostics') state.inspector = null;
  } else if (key === 'diagnostics-back') diagnostics.selected = null;
  else if (key === 'diagnostics-refresh') readDiagnostics(state, actions);
  else if (key === 'diagnostics-older' && diagnostics.page?.nextBefore != null) {
    diagnostics.selected = null;
    readDiagnostics(state, actions, {
      kind: 'before',
      through: diagnostics.page.through,
      sequence: diagnostics.page.nextBefore,
    });
  } else if (key.startsWith('diagnostic-open-')) {
    diagnostics.selected =
      diagnostics.page?.entries.find((entry) => key === 'diagnostic-open-' + entry.sequence) ??
      null;
  } else return false;
  return true;
}
type ModelFailure = NonNullable<
  Extract<AgentDiagnostic['detail'], { kind: 'execution' }>['modelFailure']
>;
function modelCause(cause: ModelFailure, text: Copy): string {
  switch (cause.kind) {
    case 'http':
      return 'HTTP ' + cause.status;
    case 'timed_out':
      return text.diagnosticsTimedOut;
    case 'context_overflow':
      return text.diagnosticsContextOverflow;
    case 'adapter':
      return text.diagnosticsAdapter;
    case 'provider':
      return {
        network: text.diagnosticsNetwork,
        timed_out: text.diagnosticsTimedOut,
        rate_limit: text.diagnosticsRateLimit,
        provider_unavailable: text.diagnosticsProviderUnavailable,
        stream_truncated: text.diagnosticsStreamTruncated,
      }[cause.reason];
  }
}
function source(record: AgentDiagnostic, text: Copy): string {
  switch (record.detail.kind) {
    case 'execution':
      return record.detail.reason === 'message_preparation_failed'
        ? text.diagnosticsPreparation
        : text.diagnosticsAnswer;
    case 'model_step':
      return text.diagnosticsModelStep;
    case 'session_title':
      return text.diagnosticsTitle;
  }
}
function executionCause(
  reason: Extract<AgentDiagnostic['detail'], { kind: 'execution' }>['reason'],
  text: Copy,
): string {
  const labels: Record<typeof reason, string> = {
    session_busy: text.diagnosticsSessionBusy,
    continuation_changed: text.diagnosticsContinuationChanged,
    reconciliation_required: text.diagnosticsReconciliationRequired,
    invalid_input: text.diagnosticsInvalidInput,
    cancelled: text.diagnosticsCancelled,
    model_incomplete: text.diagnosticsIncomplete,
    event_commit: text.diagnosticsPersistence,
    event_store: text.diagnosticsStorage,
    model_timeout: text.diagnosticsTimedOut,
    model_http: text.diagnosticsProviderRejected,
    model_adapter: text.diagnosticsAdapter,
    model_provider: text.diagnosticsProviderFailed,
    context_overflow: text.diagnosticsContextOverflow,
    tool_execution: text.diagnosticsToolFailed,
    runtime_internal: text.diagnosticsRuntimeFailed,
    cleanup_unconfirmed: text.diagnosticsCleanup,
    executor: text.diagnosticsExecutorFailed,
    message_preparation_failed: text.diagnosticsPreparationFailed,
    host_interrupted: text.diagnosticsHostInterrupted,
    outcome_unknown: text.diagnosticsOutcomeUnknown,
    execution_failed: text.diagnosticsExecutionFailed,
  };
  return labels[reason];
}
function failureColor(record: AgentDiagnostic, theme: Theme): number {
  return record.detail.kind === 'execution' ||
    (record.detail.kind === 'model_step' && record.detail.status !== 'retryable_failure')
    ? theme.error
    : theme.warning;
}
function cause(record: AgentDiagnostic, text: Copy): string {
  const detail = record.detail;
  switch (detail.kind) {
    case 'execution':
      return detail.modelFailure
        ? modelCause(detail.modelFailure, text)
        : executionCause(detail.reason, text);
    case 'model_step':
      return detail.modelFailure
        ? modelCause(detail.modelFailure, text)
        : text.diagnosticsStepFailed;
    case 'session_title':
      switch (detail.cause.kind) {
        case 'preparation':
          return text.diagnosticsTitlePreparationFailed;
        case 'model':
          return modelCause(detail.cause.cause, text);
        case 'invalid_output':
          return text.diagnosticsInvalidOutput;
        case 'output_limit':
          return text.diagnosticsOutputLimit;
        case 'empty_output':
          return text.diagnosticsEmptyOutput;
      }
  }
}
export function diagnosticsView<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  theme: Theme,
  locale: string,
): N {
  const ui = presentation.paint(theme),
    text = copy(locale),
    diagnostics = state.diagnostics;
  const selected = diagnostics.selected;
  return ui.box(
    'diagnostics-panel',
    [
      ui.box(
        'diagnostics-header',
        [
          ui.text('diagnostics-title', text.diagnosticsTitleLabel, 17, theme.text, 550),
          ui.button('diagnostics-close', text.closeModels),
        ],
        { row: true, gap: 1 },
      ),
      ...(diagnostics.failed
        ? [ui.text('diagnostics-error', text.recoveryRefreshFailed, 13, theme.warning)]
        : []),
      ...(selected
        ? [
            ui.button('diagnostics-back', text.back),
            ui.text('diagnostic-detail-source', source(selected, text), 15, theme.text, 550),
            ui.text(
              'diagnostic-detail-cause',
              cause(selected, text),
              14,
              failureColor(selected, theme),
            ),
            ui.text(
              'diagnostic-detail-time',
              new Date(selected.recordedAt).toISOString(),
              12,
              theme.muted,
            ),
            ui.text(
              'diagnostic-detail-invocation',
              selected.detail.invocation.invocation_id,
              12,
              theme.muted,
            ),
            ui.text('diagnostic-detail-event', selected.eventId, 12, theme.muted),
            ...(selected.detail.kind === 'model_step'
              ? [ui.text('diagnostic-detail-step', selected.detail.stepId, 12, theme.muted)]
              : []),
          ]
        : [
            presentation.reading(
              'diagnostics-reading',
              {
                command: null,
                content: ui.box(
                  'diagnostics-records',
                  diagnostics.page
                    ? diagnostics.page.entries.length
                      ? diagnostics.page.entries.map((record) => {
                          const tone = failureColor(record, theme);
                          return ui.box(
                            'diagnostic-' + record.sequence,
                            [
                              ui.button('diagnostic-open-' + record.sequence, source(record, text)),
                              ui.text(
                                'diagnostic-cause-' + record.sequence,
                                cause(record, text),
                                13,
                                tone,
                              ),
                              ui.text(
                                'diagnostic-time-' + record.sequence,
                                new Date(record.recordedAt).toISOString(),
                                12,
                                theme.muted,
                              ),
                            ],
                            { gap: 0 },
                          );
                        })
                      : [ui.text('diagnostics-empty', text.diagnosticsEmpty, 13, theme.muted)]
                    : [ui.text('diagnostics-loading', text.loading, 13, theme.muted)],
                  {
                    gap: 1,
                    scroll: 'manual',
                    terminal: { grow: 1 },
                    style: { width: fill, height: fill },
                  },
                ),
              },
              {},
            ),
            ui.box(
              'diagnostics-controls',
              [
                ui.button(
                  'diagnostics-refresh',
                  diagnostics.position.kind === 'before' ? text.latest : text.check,
                ),
                ui.button(
                  'diagnostics-older',
                  text.older,
                  diagnostics.loading !== null || diagnostics.page?.nextBefore == null,
                ),
              ],
              { row: true, gap: 1 },
            ),
          ]),
    ],
    {
      visible:
        presentation.target === 'tui'
          ? state.inspector === 'diagnostics' && state.dialog === null
          : state.dialog === 'diagnostics',
      gap: 1,
      terminal: { grow: 1 },
      style: { width: fill, height: fill },
    },
  );
}
