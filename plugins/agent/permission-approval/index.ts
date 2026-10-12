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

import type { Component, Emission, Permissions } from '../../../packages/plugin-sdk/src/index.js';
import { permissionApprovalContract, type ServiceApprovalProps } from '../../shared/contracts.js';
import { copy } from '../../shared/locale.js';
import { sameValue } from '../../shared/value.js';
import { fill, pixels, output, type Node, type Presentation } from '../../shared/view.js';

type State = {
  revision: number;
  identity: Pick<ServiceApprovalProps['record'], 'sessionId' | 'requestId' | 'request'>;
  decision: 'deny' | 'allow';
  scope: 'once' | 'turn' | 'session';
};
function identity(props: ServiceApprovalProps): State['identity'] {
  const { sessionId, requestId, request } = props.record;
  return { sessionId, requestId, request };
}
export function permissionApproval<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ServiceApprovalProps, N> {
  function frame(state: State, props: ServiceApprovalProps, events: Emission[] = []) {
    if (!sameValue(state.identity, identity(props))) {
      state.identity = identity(props);
      state.decision = 'deny';
      state.scope = 'once';
    }
    const ui = presentation.paint(props.theme),
      text = copy(props.locale),
      request = props.record.request;
    const available =
      props.open && !props.busy && !props.record.outcome && request.kind === 'permissions';
    const choose = (key: string, label: string, selected: boolean) =>
      ui.item
        ? ui.item(key, ui.text(key + '-label', label), selected, !available)
        : ui.button(key, label, !available, selected);
    const outcome = props.record.outcome;
    const settled = outcome?.kind === 'permissions_decision' ? outcome.decision : null;
    const decision = settled?.decision ?? state.decision;
    const scope = settled?.decision === 'allow' ? settled.scope : state.scope;
    const scopeLabel =
      scope === 'once'
        ? text.permissionOnce
        : scope === 'turn'
          ? text.permissionTurn
          : text.permissionSession;
    const permissionLines = (prefix: string, permissions: Permissions) => [
      ...permissions.filesystem.map((rule, index) =>
        ui.box(
          prefix + '-rule-' + index,
          [
            ui.text(
              prefix + '-kind-' + index,
              (rule.access === 'read'
                ? text.permissionRead
                : rule.access === 'write'
                  ? text.permissionWrite
                  : text.permissionDeny) +
                ' · ' +
                (rule.scope === 'exact' ? text.permissionExact : text.permissionSubtree),
              13,
              props.theme.muted,
            ),
            ui.text(prefix + '-path-' + index, rule.path, 13),
          ],
          { gap: 0 },
        ),
      ),
      ui.text(
        prefix + '-network',
        text.permissionNetwork + ': ' + JSON.stringify(permissions.network),
        13,
      ),
    ];
    return output(
      state,
      ui.box(
        'permission-approval',
        [
          ui.box(
            'permission-controls',
            [ui.button('close', text.closeModels), ui.button('check', text.check, props.checking)],
            { row: true, gap: 1, terminal: { wrap: true }, style: { wrap: true } },
          ),
          ...(props.status
            ? [ui.text('permission-status', props.status, 13, props.theme.muted)]
            : []),
          ...(outcome
            ? [
                ui.text(
                  'permission-outcome',
                  outcome.kind === 'permissions_decision'
                    ? outcome.decision.decision === 'allow'
                      ? text.permissionGranted + ' · ' + scopeLabel
                      : text.permissionDenied
                    : text.questionClosed,
                  13,
                  props.theme.muted,
                ),
              ]
            : []),
          ui.box(
            'permission-choice',
            [
              choose('deny', text.approvalDeny, decision === 'deny'),
              choose('allow', text.approvalAllow, decision === 'allow'),
            ],
            {
              row: true,
              gap: 1,
              focusGroup: true,
              terminal: { wrap: true },
              style: { wrap: true },
            },
          ),
          ui.box(
            'permission-scope',
            [
              choose('once', text.permissionOnce, scope === 'once'),
              choose('turn', text.permissionTurn, scope === 'turn'),
              choose('session', text.permissionSession, scope === 'session'),
            ],
            {
              row: true,
              gap: 1,
              focusGroup: true,
              terminal: { wrap: true },
              style: { wrap: true },
            },
          ),
          ui.button('submit', text.approvalSubmit, !available),
          ...(request.kind === 'permissions'
            ? [
                presentation.reading(
                  'permission-request',
                  {
                    content: ui.box(
                      'permission-document',
                      [
                        ui.text('permission-reason', request.request.reason, 15),
                        ...(request.request.command
                          ? [
                              ui.text('permission-command', request.request.command.command, 13),
                              ui.text('permission-cwd', request.request.command.cwd, 13),
                            ]
                          : []),
                        ui.text(
                          'permission-requested',
                          text.permissionRequested,
                          13,
                          props.theme.muted,
                        ),
                        ...permissionLines('permission-requested', request.request.permissions),
                        ...(settled?.decision === 'allow' &&
                        !sameValue(settled.permissions, request.request.permissions)
                          ? [
                              ui.text(
                                'permission-granted',
                                text.permissionGranted,
                                13,
                                props.theme.muted,
                              ),
                              ...permissionLines('permission-granted', settled.permissions),
                            ]
                          : []),
                      ],
                      {
                        gap: 0,
                        scroll: 'manual',
                        terminal: { grow: 1, basis: 0 },
                        style: { width: fill, height: fill },
                      },
                    ),
                  },
                  {},
                ),
              ]
            : []),
        ],
        {
          visible: props.open,
          gap: presentation.target === 'tui' ? 1 : 12,
          terminal: { grow: 1 },
          style: { width: fill, height: pixels(0), grow: 1, minHeight: 0 },
        },
      ),
      events,
    );
  }
  return {
    mount(props) {
      return frame(
        { revision: 0, identity: identity(props), decision: 'deny', scope: 'once' },
        props,
      );
    },
    update(state, props) {
      state.revision++;
      return frame(state, props);
    },
    event(state, input, props) {
      state.revision++;
      const events: Emission[] = [];
      if (!sameValue(state.identity, identity(props))) return frame(state, props);
      if (props.open && input.kind === 'raw') {
        if (input.key === 'close') events.push({ name: 'closed', value: {} });
        else if (input.key === 'check' && !props.checking)
          events.push({ name: 'check', value: {} });
        else if (
          !props.busy &&
          !props.record.outcome &&
          props.record.request.kind === 'permissions'
        ) {
          if (input.key === 'deny' || input.key === 'allow') state.decision = input.key;
          else if (input.key === 'once' || input.key === 'turn' || input.key === 'session')
            state.scope = input.key;
          else if (input.key === 'submit')
            events.push({
              name: 'submit',
              value: {
                requestId: props.record.requestId,
                decision: state.decision,
                scope: state.scope,
              },
            });
        }
      }
      return frame(state, props, events);
    },
  };
}
export function activate() {
  return {
    PermissionApproval: {
      kind: 'component',
      contract: permissionApprovalContract,
      component: ['component'],
    },
  };
}
