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
  AgentHistorySourcePage,
  Component,
  Emission,
  SourcePosition,
  HistorySource,
} from '../../plugin-sdk/src/index.js';
import {
  serviceApprovalContract,
  type ServiceApprovalProps,
  type TimelineProps,
} from './contracts.js';
import { copy } from './locale.js';
import { sameValue } from './value.js';
import { fill, pixels, output, type Node, type Presentation } from './view.js';

type State = {
  revision: number;
  serial: number;
  identity: Pick<ServiceApprovalProps['record'], 'sessionId' | 'requestId' | 'request'>;
  decision: 'deny' | 'allow';
  page: HistorySource | null;
  pending: { key: string; position: SourcePosition } | null;
  position: SourcePosition;
  failed: boolean;
  navigation: number;
};
const bytes = (text: string) => {
  let size = 0;
  for (const point of text) {
    const value = point.codePointAt(0)!;
    size += value <= 0x7f ? 1 : value <= 0x7ff ? 2 : value <= 0xffff ? 3 : 4;
  }
  return size;
};
function identity(props: ServiceApprovalProps): State['identity'] {
  const { sessionId, requestId, request } = props.record;
  return { sessionId, requestId, request };
}
function read(state: State, actions: Action[], position: SourcePosition) {
  const request = state.identity.request;
  if (request.kind !== 'service_approval' || state.pending) return;
  const key = 'approval-source-' + ++state.serial;
  state.pending = { key, position };
  state.position = position;
  state.failed = false;
  actions.push({
    key,
    binding: 'agent',
    member: 0,
    method: 'source',
    resources: {},
    input: {
      sessionId: state.identity.sessionId,
      through: request.through,
      source: { kind: 'tool_use', toolUseId: request.toolUseId },
      field: 'arguments',
      position,
    },
  });
}
function accept(state: State, value: AgentHistorySourcePage, position: SourcePosition) {
  const request = state.identity.request;
  if (request.kind !== 'service_approval' || value.through !== request.through) {
    state.failed = true;
    return;
  }
  if (value.kind === 'preparing') return;
  const next = value.source,
    end = next.offset + bytes(next.text);
  if (
    next.field !== 'arguments' ||
    next.format !== 'json' ||
    next.tool?.kind !== 'call' ||
    next.tool.toolUseId !== request.toolUseId ||
    next.tool.toolName !== request.toolName ||
    end > next.totalBytes ||
    (state.page &&
      (state.page.sequence !== next.sequence || state.page.totalBytes !== next.totalBytes)) ||
    (position.kind === 'after' && next.offset !== position.offset) ||
    (position.kind === 'before' && end !== position.offset) ||
    (position.kind === 'start' && next.offset !== 0) ||
    (position.kind === 'tail' && end !== next.totalBytes)
  ) {
    state.failed = true;
    return;
  }
  state.page = next;
  state.navigation++;
}
export function serviceApproval<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ServiceApprovalProps, N> {
  function frame(
    state: State,
    props: ServiceApprovalProps,
    actions: Action[] = [],
    events: Emission[] = [],
  ) {
    if (!sameValue(state.identity, identity(props))) {
      state.identity = identity(props);
      state.page = null;
      state.pending = null;
      state.position = { kind: 'start' };
      state.failed = false;
      state.decision = 'deny';
    }
    if (props.open && !state.page && !state.pending && !state.failed)
      read(state, actions, state.position);
    const ui = presentation.paint(props.theme),
      text = copy(props.locale),
      request = props.record.request;
    const available =
      props.open && !props.busy && !props.record.outcome && request.kind === 'service_approval';
    const page = state.page,
      end = page ? page.offset + bytes(page.text) : 0;
    const source: TimelineProps = {
      theme: props.theme,
      locale: props.locale,
      messages: [],
      source: {
        kind: 'arguments',
        format: 'json',
        ready: page !== null,
        key: props.record.requestId + '-' + (page?.offset ?? 0),
        text: page?.text ?? '',
        atStart: !page || page.offset === 0,
        atEnd: !page || end === page.totalBytes,
        loading: !!state.pending,
        failed: state.failed,
        navigation: state.navigation,
        landing:
          state.position.kind === 'before' || state.position.kind === 'tail' ? 'end' : 'start',
      },
      loading: false,
      older: false,
      newer: false,
      latest: false,
      live: false,
      search: null,
      landing: { sequence: 0, edge: 'start' },
    };
    const choose = (decision: 'deny' | 'allow', label: string) => {
      const disabled =
        !available || (decision === 'allow' && (!page || state.failed || !!state.pending));
      return ui.item
        ? ui.item(
            decision,
            ui.text(decision + '-label', label),
            state.decision === decision,
            disabled,
          )
        : ui.button(decision, label, disabled, state.decision === decision);
    };
    const out = output(
      state,
      ui.box(
        'service-approval',
        [
          ui.box(
            'approval-controls',
            [ui.button('close', text.closeModels), ui.button('check', text.check, props.checking)],
            { row: true, gap: 1, terminal: { wrap: true }, style: { wrap: true } },
          ),
          ...(props.status
            ? [ui.text('approval-status', props.status, 13, props.theme.muted)]
            : []),
          ...(request.kind === 'service_approval'
            ? [
                ui.box(
                  'approval-target-pane',
                  [
                    presentation.reading(
                      'target',
                      {
                        content: ui.box(
                          'approval-target-document',
                          [
                            ui.text('approval-tool', request.toolName, 17, props.theme.text, 550),
                            ui.text(
                              'approval-target',
                              request.service + ' · ' + request.method,
                              13,
                              props.theme.muted,
                            ),
                          ],
                          {
                            gap: 0,
                            scroll: 'anchor',
                            terminal: { grow: 1 },
                            style: { width: fill, height: fill },
                          },
                        ),
                      },
                      {},
                    ),
                  ],
                  {
                    terminal: { basis: 3, shrink: true },
                    style: { width: fill, height: pixels(72), minHeight: 0 },
                  },
                ),
              ]
            : []),
          ...(props.record.outcome
            ? [
                ui.text(
                  'approval-outcome',
                  props.record.outcome.kind === 'service_approval_decision'
                    ? props.record.outcome.decision === 'allow'
                      ? text.approvalAllowed
                      : text.approvalDenied
                    : text.questionClosed,
                  13,
                  props.theme.muted,
                ),
              ]
            : []),
          ui.box(
            'approval-choice',
            [
              choose('deny', text.approvalDeny),
              choose('allow', text.approvalAllow),
              ui.button(
                'submit',
                text.approvalSubmit,
                !available ||
                  (state.decision === 'allow' && (!page || state.failed || !!state.pending)),
              ),
            ],
            {
              row: true,
              gap: 1,
              focusGroup: true,
              terminal: { wrap: true },
              style: { wrap: true },
            },
          ),
          presentation.slot('arguments', 'timeline', source, {
            sourceBack: 'closed',
            sourceFirst: 'first',
            sourcePrevious: 'previous',
            sourceNext: 'next',
            sourceLast: 'last',
            sourceRetry: 'retry',
          }),
        ],
        {
          visible: props.open,
          gap: presentation.target === 'tui' ? 1 : 12,
          style: { width: fill, height: pixels(0), grow: 1, minHeight: 0 },
          terminal: { grow: 1 },
        },
      ),
      events,
    );
    out.actions = actions;
    return out;
  }
  return {
    mount(props) {
      return frame(
        {
          revision: 0,
          serial: 0,
          identity: identity(props),
          decision: 'deny',
          page: null,
          pending: null,
          position: { kind: 'start' },
          failed: false,
          navigation: 1,
        },
        props,
      );
    },
    update(state, props) {
      state.revision++;
      return frame(state, props);
    },
    event(state, input, props) {
      state.revision++;
      const actions: Action[] = [],
        events: Emission[] = [];
      if (!sameValue(state.identity, identity(props))) return frame(state, props);
      if (input.kind === 'action' && state.pending?.key === input.key) {
        const position = state.pending.position;
        state.pending = null;
        const result = input.event as ActionOutcome;
        if (result.status !== 'succeeded') state.failed = true;
        else {
          const page = result.value as AgentHistorySourcePage;
          accept(state, page, position);
          if (page.kind === 'preparing' && !state.failed && props.open)
            read(state, actions, position);
        }
      } else if (props.open && input.kind === 'raw') {
        if (input.key === 'close') events.push({ name: 'closed', value: {} });
        else if (input.key === 'check' && !props.checking)
          events.push({ name: 'check', value: {} });
        else if (
          !props.busy &&
          !props.record.outcome &&
          props.record.request.kind === 'service_approval'
        ) {
          const reviewable = !!state.page && !state.failed && !state.pending;
          if (input.key === 'deny' || (input.key === 'allow' && reviewable))
            state.decision = input.key;
          else if (input.key === 'submit' && (state.decision === 'deny' || reviewable))
            events.push({
              name: 'submit',
              value: { requestId: props.record.requestId, decision: state.decision },
            });
        }
      } else if (props.open && input.kind === 'component' && input.child === 'arguments') {
        const page = state.page,
          end = page ? page.offset + bytes(page.text) : 0;
        if (input.name === 'closed') events.push({ name: 'closed', value: {} });
        else if (!state.pending) {
          if (input.name === 'retry' && state.failed) read(state, actions, state.position);
          else if (input.name === 'first' && page && page.offset > 0)
            read(state, actions, { kind: 'start' });
          else if (input.name === 'previous' && page && page.offset > 0)
            read(state, actions, { kind: 'before', offset: page.offset });
          else if (input.name === 'next' && page && end < page.totalBytes)
            read(state, actions, { kind: 'after', offset: end });
          else if (input.name === 'last' && page && end < page.totalBytes)
            read(state, actions, { kind: 'tail' });
        }
      }
      return frame(state, props, actions, events);
    },
  };
}
export function activate() {
  return {
    ServiceApproval: {
      kind: 'component',
      contract: serviceApprovalContract,
      component: ['component'],
    },
  };
}
