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
  AgentObservation,
  AgentScheduleChange,
  AgentScheduleRead,
  AgentScheduleResult,
  AgentScheduledDetail,
  AgentScheduledEffect,
  AgentScheduledInput,
  AgentScheduledSummary,
  AgentScheduledTarget,
  AgentSchedules,
  AgentSession,
  AgentSessionPage,
  CallerIntent,
  Component,
  ComponentInput,
  EditorCommand,
  EditorObservation,
  Emission,
  InputProps,
  OperationReply,
  RecoveryRef,
  Schedule,
  TextareaProps,
  TextChunk,
} from '../../plugin-sdk/src/index.js';
import { schedulerContract, type SchedulerProps, type SelectedModel } from './contracts.js';
import { pageHeader, withPageShell, type PageState } from './page-shell.js';
import { copy, type Copy } from './locale.js';
import { sameValue } from './value.js';
import { compactLabel, fill, output, type Node, type Presentation } from './view.js';

type FieldName = 'title' | 'body' | 'at' | 'seconds' | 'cron' | 'timezone' | 'max' | 'expiry';
const names: FieldName[] = ['title', 'body', 'at', 'seconds', 'cron', 'timezone', 'max', 'expiry'];
interface Field {
  observation: EditorObservation | null;
  command: EditorCommand | null;
  reset: string | null;
  ready: boolean;
  sequence: number;
}
type Timing = 'once' | 'interval' | 'daily' | 'weekly' | 'monthly' | 'cron';
interface Form {
  id: string | null;
  revision: number;
  timing: Timing;
  effect: AgentScheduledEffect['kind'];
  target: AgentScheduledTarget | null;
  sourceTitle: string | null;
  model: SelectedModel | null;
  misfire: 'skip' | 'latest';
}
type Request =
  | { kind: 'history'; after: number }
  | { kind: 'field'; field: FieldName; observation: EditorObservation }
  | { kind: 'mutate' | 'receipt' | 'business' | 'sessions' | 'session' };
interface State extends PageState {
  revision: number;
  serial: number;
  open: boolean;
  requested: number;
  route: 'list' | 'detail' | 'form';
  page: Extract<AgentSchedules, { kind: 'page' }> | null;
  detail: AgentScheduledDetail | null;
  watch: { key: string; input: AgentScheduleRead } | null;
  watchFailed: boolean;
  form: Form | null;
  fields: Record<FieldName, Field>;
  capture: {
    values: Partial<Record<FieldName, string>>;
    observations: Record<FieldName, EditorObservation>;
  } | null;
  picker: boolean;
  sessions: AgentSessionPage | null;
  pending: { key: string; request: Request } | null;
  restoring: boolean;
  historyFailed: boolean;
  current: { input: AgentScheduleChange; reference: RecoveryRef | null } | null;
  confirm: AgentScheduleChange | null;
  confirmTask: AgentScheduledDetail | null;
  nextAction: {
    kind: 'edit' | 'approve' | 'resume' | 'trigger_now' | 'delete';
    taskId: string;
    revision: number;
  } | null;
  notice: keyof Copy | null;
}
function field(): Field {
  return { observation: null, command: null, reset: null, ready: true, sequence: 0 };
}
function initialize(value: Field) {
  if (value.reset !== null && value.observation && !value.command) {
    value.command = {
      owner: value.observation.snapshot.text.owner,
      basedOn: value.observation.revision,
      sequence: ++value.sequence,
      edit: { kind: 'reset', text: value.reset },
    };
  }
}
function reset(state: State, name: FieldName, text: string) {
  const value = state.fields[name];
  value.reset = text;
  value.ready = false;
  initialize(value);
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
  const key = 'schedule-' + ++state.serial;
  state.pending = { key, request: r };
  actions.push({ key, binding, member: 0, method, input, resources });
}
function watch(state: State, input: AgentScheduleRead) {
  state.watch = { key: 'schedule-watch-' + ++state.serial, input };
  state.watchFailed = false;
}
function list(state: State, cursor: string | null = null) {
  state.route = 'list';
  state.detail = null;
  watch(state, {
    kind: 'list',
    cursor,
    expectedRevision: cursor ? (state.page?.revision ?? null) : null,
  });
}
function detail(state: State, id: string) {
  state.route = 'detail';
  state.detail = null;
  watch(state, { kind: 'get', taskId: id });
}
function history(state: State, actions: Action[], after = 0) {
  state.restoring = true;
  state.historyFailed = false;
  if (!after) state.current = null;
  request(state, actions, { kind: 'history', after }, 'history', 'page', { after, limit: 16 });
}
function locked(state: State, props: SchedulerProps) {
  return !props.open || props.busy || !!state.pending || !!state.current || state.restoring;
}
function timing(schedule: Schedule): Timing {
  return schedule.kind === 'calendar' ? schedule.recurrence : schedule.kind;
}
function anchor(schedule: Schedule) {
  return schedule.kind === 'once'
    ? schedule.runAt
    : schedule.kind === 'calendar'
      ? schedule.anchorAt
      : schedule.startAt;
}
function date(at: number | undefined | null) {
  return at === null || at === undefined ? '—' : new Date(at).toISOString();
}
function instant(value: string) {
  // Explicit offsets keep folds unambiguous and never depend on the UI process timezone.
  const m =
    /^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d{1,3}))?)?\s*(Z|[+-]\d{2}:\d{2})$/.exec(
      value,
    );
  if (!m) throw Error('date');
  const year = Number(m[1]),
    month = Number(m[2]),
    day = Number(m[3]),
    hour = Number(m[4]),
    minute = Number(m[5]),
    second = Number(m[6] ?? 0);
  const ms = Number((m[7] ?? '').padEnd(3, '0'));
  const local = new Date(Date.UTC(year, month - 1, day, hour, minute, second, ms));
  if (
    year < 1970 ||
    local.getUTCFullYear() !== year ||
    local.getUTCMonth() !== month - 1 ||
    local.getUTCDate() !== day ||
    local.getUTCHours() !== hour ||
    local.getUTCMinutes() !== minute ||
    local.getUTCSeconds() !== second
  )
    throw Error('date');
  const zone = m[8]!,
    hours = zone === 'Z' ? 0 : Number(zone.slice(1, 3)),
    minutes = zone === 'Z' ? 0 : Number(zone.slice(4));
  if (hours > 23 || minutes > 59) throw Error('offset');
  const at = local.getTime() - (zone[0] === '-' ? -1 : 1) * (hours * 60 + minutes) * 60_000;
  if (at < 0) throw Error('date');
  return at;
}
function integer(value: string, min: number, max: number) {
  if (!/^\d+$/.test(value)) throw Error('integer');
  const number = Number(value);
  if (!Number.isSafeInteger(number) || number < min || number > max) throw Error('integer');
  return number;
}
function startForm(state: State, existing: AgentScheduledDetail | null) {
  const task = existing?.task,
    effect = task?.effect,
    selected = effect && effect.kind !== 'reminder' ? effect : null;
  state.form = {
    id: task?.id ?? null,
    revision: existing?.revision ?? 0,
    timing: task ? timing(task.schedule) : 'once',
    effect: effect?.kind ?? 'reminder',
    target: selected?.target ?? null,
    sourceTitle: null,
    misfire: existing?.misfire ?? 'skip',
    model: selected
      ? {
          choice: {
            selection: selected.model,
            connectionName: selected.model.connectionId,
            label: selected.model.modelId,
            thinkingLevels: selected.thinkingLevel ? [selected.thinkingLevel] : [],
            defaultThinkingLevel: null,
          },
          thinkingLevel: selected.thinkingLevel ?? null,
        }
      : null,
  };
  const initial: Record<FieldName, string> = {
    title: task?.title ?? '',
    body: task?.intent.body ?? '',
    at: date(task ? anchor(task.schedule) : Date.now() + 3600_000),
    seconds: task?.schedule.kind === 'interval' ? String(task.schedule.everySeconds) : '3600',
    cron: task?.schedule.kind === 'cron' ? task.schedule.expression : '0 9 * * *',
    timezone: existing?.timezone ?? 'UTC',
    max: task?.maxFires ? String(task.maxFires) : '',
    expiry: task?.expiresAt ? date(task.expiresAt) : '',
  };
  for (const name of names) reset(state, name, initial[name]);
  state.route = 'form';
  state.picker = false;
  state.confirm = null;
  state.notice = null;
  state.watch = null;
}
function formInput(state: State, values: Partial<Record<FieldName, string>>): AgentScheduledInput {
  const form = state.form!,
    title = values.title!.trim(),
    body = values.body!.trim(),
    at = instant(values.at!.trim());
  if (
    !title ||
    [...title].length > 120 ||
    [...body].length > 8000 ||
    (!body && form.effect !== 'reminder')
  )
    throw Error('text');
  const schedule: Schedule =
    form.timing === 'once'
      ? { kind: 'once', runAt: at }
      : form.timing === 'interval'
        ? {
            kind: 'interval',
            startAt: at,
            everySeconds: integer(values.seconds!.trim(), 10, 31622400),
          }
        : form.timing === 'cron'
          ? { kind: 'cron', startAt: at, expression: values.cron!.trim() }
          : { kind: 'calendar', anchorAt: at, recurrence: form.timing };
  const effect: AgentScheduledEffect =
    form.effect === 'reminder'
      ? { kind: 'reminder' }
      : {
          kind: form.effect,
          target: form.target!,
          model: form.model!.choice.selection,
          thinkingLevel: form.model!.thinkingLevel,
        };
  return {
    task: {
      title,
      intentBody: body,
      schedule,
      effect,
      maxFires: values.max!.trim() ? integer(values.max!.trim(), 1, 10000) : null,
      expiresAt: values.expiry!.trim() ? instant(values.expiry!.trim()) : null,
    },
    timezone: values.timezone!.trim(),
    misfire: form.misfire,
  };
}
function captureNext(state: State, actions: Action[]) {
  const capture = state.capture!,
    name = names.find((name) => capture.values[name] === undefined);
  if (!name) {
    state.capture = null;
    try {
      const input = formInput(state, capture.values),
        form = state.form!;
      state.confirm = form.id
        ? { kind: 'update', taskId: form.id, revision: form.revision, input }
        : { kind: 'create', input };
    } catch {
      state.notice = 'scheduleInvalid';
    }
    return;
  }
  const observation = capture.observations[name],
    basis = observation.snapshot.text;
  request(
    state,
    actions,
    { kind: 'field', field: name, observation },
    'reader',
    'read',
    { start: 0, end: basis.bytes },
    {
      text: {
        child: 'schedule-field-' + name,
        method: 'snapshot',
        input: { kind: 'at_revision', owner: basis.owner, revision: basis.revision },
      },
    },
  );
}
function review(state: State, actions: Action[]) {
  if (!state.form) return;
  if (state.form.effect !== 'reminder' && (!state.form.target || !state.form.model)) {
    state.notice = 'scheduleChooseTarget';
    return;
  }
  if (
    names.some((name) => {
      const field = state.fields[name];
      return (
        !field.ready ||
        !!field.command ||
        !field.observation ||
        field.observation.snapshot.marked ||
        field.observation.snapshot.text.bytes > (name === 'body' ? 32000 : 512)
      );
    })
  ) {
    state.notice = 'scheduleInvalid';
    return;
  }
  state.capture = {
    values: {},
    observations: Object.fromEntries(
      names.map((name) => [name, state.fields[name].observation]),
    ) as Record<FieldName, EditorObservation>,
  };
  state.notice = null;
  captureNext(state, actions);
}
function mutate(state: State, actions: Action[], input: AgentScheduleChange) {
  state.current = { input, reference: null };
  state.notice = null;
  request(state, actions, { kind: 'mutate' }, 'agent', 'schedule', input);
  actions.at(-1)!.recovery = { kind: 'scheduled-task/v1', input };
}
function confirmChange(state: State, change: AgentScheduleChange) {
  state.confirm = change;
  state.confirmTask = state.detail;
}
function inspectAction(
  state: State,
  kind: NonNullable<State['nextAction']>['kind'],
  taskId: string,
  revision: number,
) {
  state.nextAction = { kind, taskId, revision };
  detail(state, taskId);
}
function result(state: State, value: AgentScheduleResult) {
  const operation = state.current?.input;
  state.current = null;
  state.confirm = null;
  state.notice =
    value.kind === 'committed'
      ? 'scheduleSaved'
      : value.kind === 'conflict'
        ? 'scheduleChanged'
        : value.reason.includes('authorization') ||
            value.reason.includes('model') ||
            value.reason.includes('session')
          ? 'scheduleTargetChanged'
          : value.reason.includes('pending')
            ? 'schedulePending'
            : 'scheduleInvalid';
  if (value.kind === 'committed') {
    if (operation?.kind === 'delete' || state.route === 'list') list(state);
    else detail(state, value.taskId);
    state.form = null;
  } else if (state.route === 'detail' && state.detail)
    watch(state, { kind: 'get', taskId: state.detail.task.id });
}
function settle(state: State, reply: OperationReply, actions: Action[]) {
  if (
    !state.current ||
    (state.current.reference && !sameValue(reply.reference, state.current.reference))
  )
    return;
  state.current.reference = reply.reference;
  if (reply.receipt.kind === 'recorded' && reply.receipt.outcome.kind === 'returned')
    result(state, reply.receipt.outcome.value as AgentScheduleResult);
  else {
    state.notice = 'scheduleUnknown';
    request(state, actions, { kind: 'business' }, 'agent', 'schedules', {
      kind: 'receipt',
      reference: reply.reference,
    });
  }
}
function status(task: AgentScheduledSummary | AgentScheduledDetail, text: Copy) {
  if (!task.approved) return text.scheduleApproval;
  if (task.pendingFire) return text.schedulePending;
  const value = 'status' in task ? task.status : task.task.status;
  return text[
    (
      {
        active: 'scheduleActive',
        paused: 'schedulePaused',
        completed: 'scheduleCompleted',
        expired: 'scheduleExpired',
      } as const
    )[value]
  ];
}
function timingLabel(kind: Timing, text: Copy) {
  return kind === 'cron'
    ? 'Cron'
    : text[
        (
          {
            once: 'scheduleOnce',
            interval: 'scheduleInterval',
            daily: 'scheduleDaily',
            weekly: 'scheduleWeekly',
            monthly: 'scheduleMonthly',
          } as const
        )[kind]
      ];
}
function effectLabel(kind: AgentScheduledEffect['kind'], text: Copy) {
  return text[
    kind === 'reminder'
      ? 'scheduleReminder'
      : kind === 'session'
        ? 'scheduleSession'
        : 'scheduleAgentRun'
  ];
}
function scheduleSummary(schedule: Schedule, text: Copy) {
  return (
    timingLabel(timing(schedule), text) +
    (schedule.kind === 'interval'
      ? ` · ${schedule.everySeconds}s`
      : schedule.kind === 'cron'
        ? ` · ${schedule.expression}`
        : '') +
    ' · ' +
    date(anchor(schedule))
  );
}
export function scheduler<N extends Node>(
  presentation: Presentation<N>,
): Component<State, SchedulerProps, N> {
  function frame(
    state: State,
    props: SchedulerProps,
    actions: Action[] = [],
    events: Emission[] = [],
  ) {
    if (
      props.open &&
      props.requested &&
      props.requested.sequence > state.requested &&
      !state.restoring &&
      !state.current &&
      !state.pending &&
      !state.confirm
    ) {
      state.requested = props.requested.sequence;
      state.form = null;
      state.picker = false;
      detail(state, props.requested.taskId);
    }
    const ui = presentation.paint(props.theme),
      text = copy(props.locale),
      disabled = locked(state, props),
      editing = disabled || !!state.confirm || state.picker;
    const choose = (
      key: string,
      label: string,
      options: { key: string; label: string }[],
      current: string,
    ) =>
      ui.choice
        ? ui.choice(key, label, options, current, editing)
        : ui.box(
            key,
            [
              ui.text(key + '-label', label),
              ...options.map((option) =>
                ui.button(key + ':' + option.key, option.label, editing, option.key === current),
              ),
            ],
            { row: true, gap: 1, style: { wrap: true } },
          );
    const input = (name: FieldName, label: string, visible = true) => {
      const value = state.fields[name],
        id = 'schedule-field-' + name;
      const common = {
        accessibilityLabel: label,
        tabIndex: names.indexOf(name),
        autoFocus: name === 'title',
        readOnly: editing,
        disabled: false,
        placeholder: '',
        command: value.command,
      } satisfies Omit<InputProps, 'appearance'>;
      const field =
        name === 'body'
          ? presentation.slot(
              id,
              'editor',
              {
                ...common,
                appearance:
                  presentation.target === 'tui'
                    ? { ...props.theme.textarea, borderWidth: 0 }
                    : props.theme.textarea,
                softWrap: true,
                terminalRows: { min: 3, max: 6 },
                submit: 'primary_enter',
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
              } satisfies TextareaProps,
              { changed: 'field', submitted: 'review' },
            )
          : presentation.slot(
              id,
              'input',
              {
                ...common,
                appearance:
                  presentation.target === 'tui'
                    ? { ...props.theme.input, borderWidth: 0 }
                    : props.theme.input,
              } satisfies InputProps,
              { changed: 'field', submitted: 'review' },
            );
      return ui.box(
        'schedule-input-' + name,
        [
          ui.box(
            'schedule-label-' + name,
            [ui.text('schedule-label-text-' + name, label, 13, props.theme.muted)],
            { gap: 0 },
          ),
          ui.box('schedule-editor-' + name, [field], { gap: 0 }),
        ],
        {
          visible,
          gap: 0,
          wide: {
            min_width: 56,
            direction: 'row',
            children: {
              ['schedule-label-' + name]: { basis: 24 },
              ['schedule-editor-' + name]: { grow: 1, basis: 0 },
            },
          },
        },
      );
    };
    const form = state.form;
    const formView = ui.box(
      'schedule-form',
      [
        input('title', text.scheduleName),
        input('body', text.scheduleBody),
        choose(
          'effect',
          text.scheduleEffect,
          (['reminder', 'session', 'agent_run'] as const).map((key) => ({
            key,
            label: effectLabel(key, text),
          })),
          form?.effect ?? 'reminder',
        ),
        ui.box(
          'schedule-target',
          [
            ui.button('source', form?.sourceTitle ?? text.scheduleSource, editing),
            ...(form?.target
              ? [
                  ui.text(
                    'schedule-workspace',
                    compactLabel(form.target.workspace.path, 400),
                    13,
                    props.theme.muted,
                  ),
                ]
              : []),
            ui.text('schedule-source-hint', text.scheduleSourceHint, 13, props.theme.muted),
            presentation.slot(
              'schedule-models',
              'models',
              {
                theme: props.theme,
                locale: props.locale,
                selection: form?.model ?? null,
                catalogEpoch: props.catalogEpoch,
                openRequest: null,
                busy: editing || !form || form.effect === 'reminder',
              },
              { selected: 'model', connections: 'connections' },
            ),
          ],
          { visible: !!form && form.effect !== 'reminder', gap: 1 },
        ),
        choose(
          'timing',
          text.scheduleTiming,
          (['once', 'interval', 'daily', 'weekly', 'monthly', 'cron'] as const).map((key) => ({
            key,
            label: timingLabel(key, text),
          })),
          form?.timing ?? 'once',
        ),
        input('at', text.scheduleAt),
        ui.text('schedule-time-hint', text.scheduleTimeHint, 13, props.theme.muted),
        input('seconds', text.scheduleSeconds, form?.timing === 'interval'),
        input('cron', text.scheduleCron, form?.timing === 'cron'),
        input('timezone', text.scheduleZone),
        choose(
          'misfire',
          text.scheduleMissed,
          [
            { key: 'skip', label: text.scheduleSkip },
            { key: 'latest', label: text.scheduleLatest },
          ],
          form?.misfire ?? 'skip',
        ),
        input('max', text.scheduleMax),
        input('expiry', text.scheduleExpiry),
      ],
      {
        visible: state.route === 'form' && !state.picker,
        gap: 1,
        scroll: 'manual',
        terminal: { grow: 1, basis: 0 },
        style: { width: fill, height: fill },
      },
    );
    const controls = (task: AgentScheduledSummary, prefix: string) => [
      ui.button(prefix + 'open-' + task.id, text.scheduleHistory, disabled),
      ...(task.status === 'active' || task.status === 'paused'
        ? [
            ui.button(prefix + 'edit-' + task.id, text.edit, disabled || !!task.pendingFire),
            ui.button(
              prefix + 'state-' + task.id,
              !task.approved
                ? text.scheduleEnable
                : task.status === 'paused'
                  ? text.scheduleResume
                  : text.schedulePause,
              disabled,
            ),
            ...(task.status === 'active'
              ? [
                  ui.button(
                    prefix + 'run-' + task.id,
                    text.scheduleRun,
                    disabled || !!task.pendingFire || !task.approved,
                  ),
                ]
              : []),
          ]
        : []),
      ui.button(
        prefix + 'delete-' + task.id,
        text.scheduleDelete,
        disabled || (!!task.pendingFire && !task.reminder),
      ),
    ];
    const content: N[] = [];
    if (state.picker) {
      for (const session of state.sessions?.sessions ?? [])
        content.push(
          ui.button(
            'source-' + session.id,
            compactLabel(session.title, 120),
            disabled || session.archived,
            false,
            { width: fill, justify: 'start' },
          ),
        );
    } else if (state.route === 'list') {
      if (ui.item)
        content.push(
          ui.item(
            'new',
            ui.box(
              'schedule-new-row',
              [
                ui.box(
                  'schedule-new-label',
                  [ui.text('schedule-new-text', text.scheduleNew, 14, props.theme.text)],
                  { terminal: { grow: 1 } },
                ),
                ui.text('schedule-new-arrow', '›', 14, props.theme.muted),
              ],
              { row: true, gap: 1 },
            ),
            false,
            disabled,
          ),
        );
      for (const task of state.page?.tasks ?? []) {
        const row = ui.box(
          'schedule-row-' + task.id,
          [
            ui.button('task-' + task.id, compactLabel(task.title, 120), disabled, false, {
              width: fill,
              justify: 'start',
            }),
            ui.text(
              'schedule-summary-' + task.id,
              `${status(task, text)} · ${text.scheduleNext}: ${date(task.nextFireAt)}`,
              13,
              task.pendingFire ? props.theme.warning : props.theme.muted,
            ),
            ...(presentation.target === 'desktop'
              ? [
                  ui.box('schedule-actions-' + task.id, controls(task, 'row-'), {
                    row: true,
                    style: { wrap: true },
                    gap: 1,
                  }),
                ]
              : []),
          ],
          { gap: 0 },
        );
        content.push(
          ui.contextMenu
            ? ui.contextMenu(
                'schedule-menu-' + task.id,
                compactLabel(task.title, 120),
                row,
                controls(task, 'context-'),
              )
            : row,
        );
      }
      if (state.page && !state.page.tasks.length)
        content.push(ui.text('schedule-empty', text.scheduleEmpty, 14, props.theme.muted));
    } else if (state.detail) {
      const d = state.detail,
        task = d.task,
        effect = task.effect;
      content.push(
        ui.text('schedule-detail-title', task.title, 18, props.theme.text, 600),
        ui.text(
          'schedule-status',
          `${status(d, text)} · ${text.scheduleCount}: ${task.fireCount}${task.maxFires ? '/' + task.maxFires : ''}`,
          13,
          props.theme.muted,
        ),
        ui.text('schedule-next', `${text.scheduleNext}: ${date(task.nextFireAt)}`, 13),
        ui.text(
          'schedule-timing',
          `${scheduleSummary(task.schedule, text)} · ${d.timezone}`,
          13,
          props.theme.muted,
        ),
        ui.text('schedule-detail-effect', effectLabel(effect.kind, text), 14),
        ...(effect.kind !== 'reminder'
          ? [
              ui.text('schedule-detail-path', compactLabel(effect.target.workspace.path, 400), 13),
              ui.text('schedule-detail-model', effect.model.modelId, 13),
            ]
          : []),
        ui.text('schedule-instructions', task.intent.body, 14),
      );
      if (d.pendingFire && effect.kind === 'reminder')
        content.push(
          ui.box(
            'schedule-reminder-actions',
            [
              ui.button('acknowledge', text.scheduleDone, disabled, true),
              ui.button('snooze', text.scheduleSnooze, disabled),
            ],
            { row: true, gap: 1, terminal: { wrap: true }, style: { wrap: true } },
          ),
        );
      const summary: AgentScheduledSummary = {
        ...task,
        revision: d.revision,
        approved: d.approved,
        pendingFire: d.pendingFire,
        reminder: effect.kind === 'reminder',
      };
      content.push(
        ui.box(
          'schedule-detail-actions',
          controls(summary, 'detail-').filter((_, index) => index !== 0),
          { row: true, terminal: { wrap: true }, style: { wrap: true }, gap: 1 },
        ),
        ui.text('schedule-history', text.scheduleHistory, 16, props.theme.text, 600),
      );
      if (!task.runs.length)
        content.push(
          ui.text('schedule-history-empty', text.scheduleNoHistory, 13, props.theme.muted),
        );
      for (const [index, run] of task.runs.entries())
        content.push(
          ui.box(
            'schedule-run-' + run.id,
            [
              ui.text(
                'schedule-run-status-' + run.id,
                `${date(run.at)} · ${run.outcome === 'ok' ? (run.sessionId ? text.scheduleAdmitted : text.scheduleAcknowledged) : run.outcome === 'blocked' ? text.scheduleBlocked : text.scheduleFailed}`,
                13,
              ),
              ...(run.outcome !== 'ok'
                ? [
                    ui.text(
                      'schedule-run-error-' + run.id,
                      run.message === 'scheduler_authorization_changed'
                        ? text.scheduleTargetChanged
                        : text.scheduleUnavailable,
                      13,
                      props.theme.error,
                    ),
                  ]
                : []),
              ...(run.sessionId
                ? [ui.button('conversation-' + index, text.recallOpen, disabled)]
                : []),
            ],
            { gap: 0 },
          ),
        );
      if (task.runs.length) content.push(ui.button('clear-history', text.scheduleClear, disabled));
    }
    const body = ui.box(
      'schedule-page',
      [
        ...pageHeader(
          presentation,
          state,
          props,
          events,
          state.picker
            ? text.scheduleSource
            : state.route === 'form'
              ? state.form?.id
                ? text.edit
                : text.scheduleNew
              : text.scheduleTitle,
          'back',
          false,
        ),
        ...(state.notice
          ? [ui.text('schedule-notice', text[state.notice], 13, props.theme.warning)]
          : []),
        ...(state.pending || state.restoring
          ? [ui.text('schedule-loading', text.terminalLoading, 13, props.theme.muted)]
          : []),
        ...(state.current
          ? [
              ui.button(
                'receipt',
                text.executionReceipt,
                !!state.pending || !state.current.reference,
              ),
            ]
          : []),
        ...(state.historyFailed
          ? [ui.button('history-retry', text.sourceRetry, !!state.pending)]
          : []),
        ...(state.watchFailed ? [ui.button('refresh', text.directoryRefresh, disabled)] : []),
        formView,
        ui.box(
          'schedule-reading-container',
          [
            presentation.reading(
              'schedule-reading',
              {
                content: ui.box('schedule-content', content, {
                  gap: 1,
                  scroll: 'manual',
                  focusGroup: state.picker,
                  terminal: { grow: 1, basis: 0 },
                  style: { width: fill, height: fill },
                }),
              },
              { input: 'schedule-input' },
            ),
          ],
          {
            visible: state.route !== 'form' || state.picker,
            terminal: { grow: 1, basis: 0 },
            style: { width: fill, height: fill },
          },
        ),
        ui.box(
          'schedule-footer',
          state.picker
            ? [
                ui.button('sessions-refresh', text.directoryRefresh, disabled),
                ...(state.sessions?.before
                  ? [ui.button('sessions-previous', text.sourcePrevious, disabled)]
                  : []),
                ...(state.sessions?.after
                  ? [ui.button('sessions-next', text.sourceNext, disabled)]
                  : []),
              ]
            : state.route === 'form'
              ? [ui.button('review', text.scheduleReview, editing, true)]
              : state.route === 'list'
                ? [
                    ...(!ui.item ? [ui.button('new', text.scheduleNew, disabled, true)] : []),
                    ui.button('refresh', text.directoryRefresh, disabled),
                    ...(state.page?.nextCursor
                      ? [ui.button('next', text.sourceNext, disabled)]
                      : []),
                  ]
                : [ui.button('refresh', text.directoryRefresh, disabled)],
          { row: true, gap: 1, terminal: { wrap: true }, style: { wrap: true } },
        ),
      ],
      {
        visible: props.open,
        autoFocus: true,
        gap: 1,
        shortcuts: [{ key: 'escape', modifiers: [], input: 'back', enabled: true }],
        terminal: { grow: 1 },
        style: { width: fill, height: fill },
      },
    );
    const confirmation = state.confirm;
    const reviewed =
      confirmation?.kind === 'create' || confirmation?.kind === 'update'
        ? confirmation.input
        : null;
    const effect = reviewed?.task.effect ?? state.confirmTask?.task.effect;
    const consent =
      !!confirmation &&
      ['create', 'update', 'approve', 'resume', 'trigger_now'].includes(confirmation.kind);
    const reviewView = ui.box(
      'schedule-confirm',
      [
        ui.text(
          'schedule-confirm-title',
          reviewed?.task.title ?? state.confirmTask?.task.title ?? '',
          16,
          props.theme.text,
          600,
        ),
        ...(reviewed
          ? [
              ui.text(
                'schedule-confirm-time',
                `${scheduleSummary(reviewed.task.schedule, text)} · ${reviewed.timezone}`,
                13,
              ),
              ui.text('schedule-confirm-body', reviewed.task.intentBody, 14),
            ]
          : []),
        ...(consent && effect
          ? [
              ui.text('schedule-confirm-effect', effectLabel(effect.kind, text), 14),
              ...(effect.kind !== 'reminder'
                ? [
                    ui.text(
                      'schedule-confirm-path',
                      compactLabel(effect.target.workspace.path, 400),
                      13,
                    ),
                    ui.text('schedule-confirm-model', effect.model.modelId, 13),
                    ui.text(
                      'schedule-confirm-permissions',
                      effect.target.permissions.access === 'read_only'
                        ? text.scheduleReadOnly
                        : text.scheduleWorkspaceWrite,
                      13,
                    ),
                    ...(effect.target.permissions.requestPermissions
                      ? [ui.text('schedule-confirm-additional', text.scheduleAdditional, 13)]
                      : []),
                  ]
                : []),
              ui.text(
                'schedule-consent',
                effect.kind === 'reminder' ? text.scheduleReminderConsent : text.scheduleConsent,
                13,
                props.theme.muted,
              ),
            ]
          : [
              ui.text(
                'schedule-confirm-description',
                confirmation?.kind === 'delete' ? text.scheduleDeleteHelp : text.scheduleClearHelp,
                14,
              ),
            ]),
        ...(state.notice
          ? [ui.text('schedule-confirm-notice', text[state.notice], 13, props.theme.warning)]
          : []),
        ...(state.current
          ? [
              ui.button(
                'confirm-receipt',
                text.executionReceipt,
                !!state.pending || !state.current.reference,
              ),
            ]
          : []),
        ui.box(
          'schedule-confirm-actions',
          [
            ui.button('confirm-cancel', text.cancel, !!state.current || !!state.pending),
            ui.button(
              'confirm',
              confirmation?.kind === 'delete'
                ? text.scheduleDelete
                : confirmation?.kind === 'clear_history'
                  ? text.scheduleClear
                  : confirmation?.kind === 'trigger_now'
                    ? text.scheduleRun
                    : confirmation?.kind === 'update'
                      ? text.scheduleSave
                      : text.scheduleEnable,
              disabled,
              true,
            ),
          ],
          { row: true, gap: 1, terminal: { wrap: true }, style: { wrap: true } },
        ),
      ],
      { gap: 1, scroll: 'manual' },
    );
    const view = ui.dialog(
      'schedule-review',
      text.scheduleReview,
      body,
      reviewView,
      !!confirmation,
      {},
      { width: 72, height: 24 },
    );
    const out = output(state, view, events);
    out.actions = actions;
    out.subscriptions =
      state.watch && props.open && !state.restoring
        ? [{ ...state.watch, binding: 'agent', member: 0, method: 'watchSchedules', resources: {} }]
        : [];
    return out;
  }
  return withPageShell<State, SchedulerProps, N>(
    {
      mount(props) {
        const state: State = {
          revision: 0,
          serial: 0,
          open: props.open,
          requested: -1,
          route: 'list',
          page: null,
          detail: null,
          watch: null,
          watchFailed: false,
          form: null,
          fields: Object.fromEntries(names.map((name) => [name, field()])) as Record<
            FieldName,
            Field
          >,
          capture: null,
          picker: false,
          sessions: null,
          pending: null,
          restoring: true,
          historyFailed: false,
          current: null,
          confirm: null,
          confirmTask: null,
          nextAction: null,
          notice: null,
        };
        if (props.requested) detail(state, props.requested.taskId);
        else list(state);
        const actions: Action[] = [];
        history(state, actions);
        return frame(state, props, actions);
      },
      update(state, props) {
        state.revision++;
        if (props.open && !state.open && state.watch) watch(state, state.watch.input);
        state.open = props.open;
        return frame(state, props);
      },
      event(state, input, props) {
        state.revision++;
        const actions: Action[] = [],
          events: Emission[] = [];
        if (
          input.kind === 'component' &&
          input.child === 'schedule-reading' &&
          input.name === 'input'
        )
          input = input.event as ComponentInput;
        if (input.kind === 'action' && input.key === state.watch?.key) {
          const outcome = input.event as ActionOutcome;
          if (outcome.status === 'updated') {
            state.watchFailed = false;
            const page = outcome.value as AgentSchedules;
            if (page.kind === 'page') state.page = page;
            else if (page.kind === 'task') {
              state.detail = page.detail ?? null;
              const next = state.nextAction;
              state.nextAction = null;
              if (!page.detail) {
                list(state);
                state.notice = 'scheduleChanged';
              } else if (next) {
                if (next.taskId !== page.detail.task.id || next.revision !== page.detail.revision)
                  state.notice = 'scheduleChanged';
                else if (next.kind === 'edit') startForm(state, page.detail);
                else
                  confirmChange(state, {
                    kind: next.kind,
                    taskId: next.taskId,
                    revision: next.revision,
                  });
              }
            } else if (page.kind === 'changed') {
              list(state);
              state.notice = 'scheduleChanged';
            }
          } else if (outcome.status === 'failed' || outcome.status === 'not_accepted') {
            state.watch = null;
            state.watchFailed = true;
            state.notice = 'scheduleUnavailable';
          }
        } else if (input.kind === 'action' && input.key === state.pending?.key) {
          const r = state.pending.request,
            outcome = input.event as ActionOutcome;
          state.pending = null;
          if (r.kind === 'history') {
            if (outcome.status !== 'succeeded') {
              state.historyFailed = true;
              state.notice = 'historyUnavailable';
            } else {
              const items = outcome.value as CallerIntent[];
              for (const item of items) {
                const saved = item.recovery as {
                  kind?: string;
                  input?: AgentScheduleChange;
                } | null;
                if (
                  item.reference.method === 'schedule' &&
                  item.reference.contract.name === 'maka.agent.Agent' &&
                  saved?.kind === 'scheduled-task/v1' &&
                  saved.input
                )
                  state.current = { input: saved.input, reference: item.reference };
              }
              const last = items.at(-1);
              if (last && last.cursor > r.after) history(state, actions, last.cursor);
              else if (last) {
                state.historyFailed = true;
                state.notice = 'historyUnavailable';
              } else {
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
              }
            }
          } else if (r.kind === 'mutate') {
            if (outcome.status === 'operation') settle(state, outcome.reply, actions);
            else if (outcome.status === 'not_accepted') {
              state.current = null;
              state.notice = 'scheduleUnavailable';
            } else state.notice = 'scheduleUnknown';
          } else if (r.kind === 'receipt') {
            if (outcome.status === 'succeeded')
              settle(state, outcome.value as OperationReply, actions);
            else state.notice = 'scheduleUnknown';
          } else if (r.kind === 'business') {
            const page = outcome.status === 'succeeded' ? (outcome.value as AgentSchedules) : null;
            if (page?.kind === 'receipt' && page.result) result(state, page.result);
            else state.notice = 'scheduleUnknown';
          } else if (r.kind === 'field') {
            const chunk = outcome.status === 'succeeded' ? (outcome.value as TextChunk) : null;
            if (
              !chunk ||
              !state.capture ||
              !sameValue(chunk.metadata, r.observation.snapshot.text) ||
              !sameValue(
                state.fields[r.field].observation?.snapshot.text,
                r.observation.snapshot.text,
              ) ||
              new TextEncoder().encode(chunk.text).length !== r.observation.snapshot.text.bytes
            ) {
              state.capture = null;
              state.notice = 'scheduleInvalid';
            } else {
              state.capture.values[r.field] = chunk.text;
              captureNext(state, actions);
            }
          } else if (r.kind === 'sessions') {
            if (outcome.status === 'succeeded') state.sessions = outcome.value as AgentSessionPage;
            else state.notice = 'scheduleUnavailable';
          } else if (r.kind === 'session') {
            if (outcome.status === 'succeeded')
              events.push({ name: 'selected', value: (outcome.value as AgentObservation).session });
            else state.notice = 'scheduleUnavailable';
          }
        } else if (input.kind === 'component') {
          const name = names.find(
            (name) => input.kind === 'component' && input.child === 'schedule-field-' + name,
          );
          if (name && input.name === 'changed') {
            const value = state.fields[name];
            value.observation = input.event as EditorObservation;
            const receipt = value.observation.receipt;
            if (
              value.command &&
              receipt?.owner === value.command.owner &&
              receipt.sequence === value.command.sequence
            ) {
              value.ready = receipt.outcome.status === 'applied';
              value.command = null;
              value.reset = null;
              if (!value.ready) state.notice = 'scheduleUnavailable';
            } else initialize(value);
          } else if (
            name &&
            input.name === 'submitted' &&
            !locked(state, props) &&
            !state.confirm &&
            !state.picker
          )
            review(state, actions);
          else if (
            input.child === 'schedule-models' &&
            input.name === 'selected' &&
            state.form &&
            !state.current &&
            !state.confirm
          )
            state.form.model = input.event as SelectedModel;
          else if (input.child === 'schedule-models' && input.name === 'connections' && props.open)
            events.push({ name: 'connections', value: {} });
        } else if (input.kind === 'raw' && props.open) {
          let key = input.key;
          if (key === 'back' || key === 'confirm-cancel') {
            if (state.confirm) {
              if (!state.current && !state.pending) state.confirm = null;
            } else if (state.picker) state.picker = false;
            else if (state.route === 'form') {
              if (!state.current && !state.pending) {
                const id = state.form?.id;
                state.form = null;
                if (id) detail(state, id);
                else list(state);
              }
            } else if (state.route === 'detail') list(state);
            else events.push({ name: 'closed', value: {} });
          } else if (
            (key === 'receipt' || key === 'confirm-receipt') &&
            !state.pending &&
            state.current?.reference
          )
            request(
              state,
              actions,
              { kind: 'receipt' },
              'receipts',
              'read',
              state.current.reference,
            );
          else if (key === 'history-retry' && !state.pending) history(state, actions);
          else if (!locked(state, props)) {
            if (key === 'confirm' && state.confirm) mutate(state, actions, state.confirm);
            else if (!state.confirm) {
              let selected = (input.event as { key?: string }).key;
              if (key.includes(':')) {
                const pieces = key.split(':');
                key = pieces[0]!;
                selected = pieces[1];
              }
              if (state.form && !state.picker && ['effect', 'timing', 'misfire'].includes(key)) {
                if (
                  key === 'effect' &&
                  ['reminder', 'session', 'agent_run'].includes(selected ?? '')
                )
                  state.form.effect = selected as Form['effect'];
                if (
                  key === 'timing' &&
                  ['once', 'interval', 'daily', 'weekly', 'monthly', 'cron'].includes(
                    selected ?? '',
                  )
                )
                  state.form.timing = selected as Timing;
                if (key === 'misfire' && ['skip', 'latest'].includes(selected ?? ''))
                  state.form.misfire = selected as Form['misfire'];
              } else if (key === 'source' || key.startsWith('sessions-')) {
                state.picker = true;
                const cursor =
                  key === 'sessions-next'
                    ? state.sessions?.after
                    : key === 'sessions-previous'
                      ? state.sessions?.before
                      : null;
                request(state, actions, { kind: 'sessions' }, 'agent', 'list', {
                  filter: 'all',
                  position: cursor
                    ? {
                        kind: key === 'sessions-next' ? 'after' : 'before',
                        cursor,
                        revision: state.sessions!.revision,
                      }
                    : { kind: 'start' },
                });
              } else if (state.picker && key.startsWith('source-')) {
                const session = state.sessions?.sessions.find((s) => key === 'source-' + s.id);
                if (session && !session.archived && state.form) {
                  state.form.target = {
                    sessionId: session.id,
                    workspace: session.workspace,
                    permissions: session.permissions,
                    boundaryRevision: session.boundaryRevision,
                  };
                  state.form.sourceTitle = session.title;
                  state.picker = false;
                }
              } else if (key === 'new') startForm(state, null);
              else if (key === 'review') review(state, actions);
              else if (key === 'refresh') {
                state.notice = null;
                if (state.route === 'detail' && state.detail)
                  watch(state, { kind: 'get', taskId: state.detail.task.id });
                else list(state);
              } else if (key === 'next' && state.page?.nextCursor)
                list(state, state.page.nextCursor);
              else if (key.startsWith('task-')) {
                const task = state.page?.tasks.find((t) => key === 'task-' + t.id);
                if (task) detail(state, task.id);
              } else if (key === 'acknowledge' && state.detail?.pendingFire)
                mutate(state, actions, {
                  kind: 'acknowledge',
                  taskId: state.detail.task.id,
                  revision: state.detail.revision,
                  fireId: state.detail.pendingFire,
                });
              else if (key === 'snooze' && state.detail)
                mutate(state, actions, {
                  kind: 'snooze',
                  taskId: state.detail.task.id,
                  revision: state.detail.revision,
                  delayMs: 300000,
                });
              else if (key === 'clear-history' && state.detail)
                confirmChange(state, {
                  kind: 'clear_history',
                  taskId: state.detail.task.id,
                  revision: state.detail.revision,
                });
              else if (key.startsWith('conversation-') && state.detail) {
                const run = state.detail.task.runs[Number(key.slice(13))];
                if (run?.sessionId)
                  request(state, actions, { kind: 'session' }, 'agent', 'observe', {
                    sessionId: run.sessionId,
                  });
              } else {
                key = key.replace(/^(?:context|row|detail)-/, '');
                const kind = key.slice(0, key.indexOf('-')),
                  id = key.slice(key.indexOf('-') + 1);
                const d = state.detail;
                const task =
                  d?.task.id === id
                    ? {
                        ...d.task,
                        revision: d.revision,
                        approved: d.approved,
                        pendingFire: d.pendingFire,
                        reminder: d.task.effect.kind === 'reminder',
                      }
                    : state.page?.tasks.find((task) => task.id === id);
                if (task && kind === 'open') detail(state, id);
                else if (task && kind === 'edit') {
                  if (d?.task.id === id) startForm(state, d);
                  else inspectAction(state, 'edit', id, task.revision);
                } else if (task && kind === 'state') {
                  const change: AgentScheduleChange = {
                    kind: !task.approved
                      ? 'approve'
                      : task.status === 'paused'
                        ? 'resume'
                        : 'pause',
                    taskId: id,
                    revision: task.revision,
                  };
                  if (change.kind === 'pause') mutate(state, actions, change);
                  else if (d?.task.id === id) confirmChange(state, change);
                  else
                    inspectAction(state, !task.approved ? 'approve' : 'resume', id, task.revision);
                } else if (task && kind === 'run') {
                  if (d?.task.id === id)
                    confirmChange(state, {
                      kind: 'trigger_now',
                      taskId: id,
                      revision: task.revision,
                    });
                  else inspectAction(state, 'trigger_now', id, task.revision);
                } else if (task && kind === 'delete') {
                  if (d?.task.id === id)
                    confirmChange(state, { kind: 'delete', taskId: id, revision: task.revision });
                  else inspectAction(state, 'delete', id, task.revision);
                }
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
    Scheduler: { kind: 'component', contract: schedulerContract, component: ['component'] },
  };
}
