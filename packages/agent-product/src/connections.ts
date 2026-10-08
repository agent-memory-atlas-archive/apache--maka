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
  Emission,
  SettingsConnectionsPage,
  SettingsConnectionSummary,
  SettingsConnectionDetail,
  ConnectionVersionBasis,
} from '../../plugin-sdk/src/index.js';
import { connectionsContract, type ConnectionsProps } from './contracts.js';
import { compactLabel } from './view.js';
import { copy } from './locale.js';
import { fill, output, type Presentation, type Node } from './view.js';
interface State {
  revision: number;
  serial: number;
  pending: { key: string; kind: 'page' | 'detail'; basis: ConnectionVersionBasis | null } | null;
  detailRoute: 'credential' | 'form';
  route: 'catalog' | 'credential' | 'form';
  detail: Extract<SettingsConnectionDetail, { kind: 'ready' }> | null;
  connections: SettingsConnectionSummary[];
  catalogRevision: number | null;
  next: number | null;
  offset: number;
  failed: boolean;
  mount: number;
}
function request(
  state: State,
  kind: 'page' | 'detail',
  input: unknown,
  basis: ConnectionVersionBasis | null = null,
): Action[] {
  if (state.pending) return [];
  const key = 'connections-' + ++state.serial;
  state.pending = { key, kind, basis };
  state.failed = false;
  return [
    {
      key,
      binding: 'settings',
      member: 0,
      method: kind === 'page' ? 'connections' : 'connection',
      input,
      resources: {},
    },
  ];
}
function page(state: State, more = false) {
  return request(
    state,
    'page',
    more && state.catalogRevision !== null && state.next !== null
      ? { kind: 'continue', revision: state.catalogRevision, offset: state.next }
      : { kind: 'start' },
  );
}
function frame<N extends Node>(
  presentation: Presentation<N>,
  state: State,
  props: ConnectionsProps,
  actions: Action[] = [],
  events: Emission[] = [],
) {
  const target = presentation.target;
  const ui = presentation.paint(props.theme),
    text = copy(props.locale);
  const view =
    state.route === 'form'
      ? presentation.slot(
          'form-' + state.mount,
          'form',
          {
            ...props,
            catalogRevision: state.catalogRevision!,
            connection: state.detail,
          },
          { closed: 'closed', changed: 'changed' },
        )
      : state.route === 'credential' && state.detail
        ? presentation.slot(
            'credential-' + state.mount,
            'credential',
            { ...props, target: state.detail.target },
            { closed: 'closed', changed: 'changed' },
          )
        : ui.box(
            'connections',
            [
              ui.box(
                'header',
                [
                  ui.button('back', text.back),
                  ui.text('heading', text.settings, 22, props.theme.text, 600),
                ],
                { row: target === 'desktop', gap: target === 'desktop' ? 8 : 0 },
              ),
              ui.box(
                'page-actions',
                [
                  ui.button('refresh', text.refreshConnections, !!state.pending),
                  ui.button(
                    'create',
                    text.createConnection,
                    !!state.pending || state.catalogRevision === null,
                  ),
                  ...(target === 'tui' && state.offset > 0
                    ? [ui.button('previous', text.previousConnections, !!state.pending)]
                    : []),
                  ...((target === 'tui' && state.offset + 3 < state.connections.length) ||
                  state.next !== null
                    ? [ui.button('more', text.moreConnections, !!state.pending)]
                    : []),
                ],
                { row: target === 'desktop', gap: 0 },
              ),
              ...(state.pending
                ? [ui.text('loading', text.connecting, 13, props.theme.muted)]
                : []),
              ...(state.failed
                ? [ui.text('failed', text.connectionsUnavailable, 13, props.theme.muted)]
                : []),
              ...(!state.pending && !state.failed && !state.connections.length
                ? [ui.text('empty', text.noConnections, 15, props.theme.muted)]
                : []),
              ...(target === 'desktop'
                ? state.connections
                : state.connections.slice(state.offset, state.offset + 3)
              ).map((connection, at) =>
                ui.box(
                  'connection-' + at,
                  [
                    ui.text(
                      'name-' + at,
                      compactLabel(connection.name, target === 'tui' ? 36 : 100),
                      16,
                      props.theme.text,
                      550,
                    ),
                    ...(target === 'desktop'
                      ? [
                          ui.text(
                            'provider-' + at,
                            connection.provider.name + ' · ' + connection.slug,
                            13,
                            props.theme.muted,
                          ),
                        ]
                      : []),
                    ui.button(
                      'open-' + (at + (target === 'tui' ? state.offset : 0)),
                      text.credentials,
                      !!state.pending,
                    ),
                    ui.button(
                      'edit-' + (at + (target === 'tui' ? state.offset : 0)),
                      text.editConnection,
                      !!state.pending,
                    ),
                  ],
                  {
                    gap: target === 'desktop' ? 6 : 0,
                    padding: target === 'desktop' ? 16 : 0,
                    surface: true,
                    style: { width: fill },
                  },
                ),
              ),
            ],
            {
              gap: target === 'desktop' ? 16 : 0,
              padding: target === 'desktop' ? 24 : 0,
              scroll: 'anchor',
              style: { width: fill, height: fill, maxWidth: 760, minWidth: 0, minHeight: 0 },
            },
          );
  const out = output(state, view, events);
  out.actions = actions;
  return out;
}
export function connections<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ConnectionsProps, N> {
  const target = presentation.target;
  return {
    mount(props) {
      const state: State = {
        revision: 0,
        serial: 0,
        pending: null,
        detailRoute: 'credential',
        route: 'catalog',
        detail: null,
        connections: [],
        catalogRevision: null,
        next: null,
        offset: 0,
        failed: false,
        mount: 0,
      };
      return frame(presentation, state, props, page(state));
    },
    update(state, props) {
      state.revision++;
      return frame(presentation, state, props);
    },
    event(state, input, props) {
      state.revision++;
      const actions: Action[] = [],
        events: Emission[] = [];
      if (input.kind === 'action' && input.key === state.pending?.key) {
        const pending = state.pending;
        state.pending = null;
        const result = input.event as ActionOutcome;
        if (result.status !== 'succeeded') state.failed = true;
        else if (pending.kind === 'page') {
          const resultPage = result.value as SettingsConnectionsPage;
          if (resultPage.kind === 'changed') {
            state.connections = [];
            state.next = null;
            state.catalogRevision = null;
            actions.push(...page(state));
          } else {
            state.connections = resultPage.connections;
            state.catalogRevision = resultPage.revision;
            state.next = resultPage.next;
            state.offset = 0;
          }
        } else {
          const detail = result.value as SettingsConnectionDetail;
          if (
            detail.kind === 'ready' &&
            detail.target.connectionId === pending.basis?.connectionId &&
            detail.target.revision === pending.basis.revision
          ) {
            state.detail = detail;
            state.route = state.detailRoute;
            state.mount++;
          } else {
            state.failed = true;
            actions.push(...page(state));
          }
        }
      } else if (
        input.kind === 'component' &&
        (input.child === 'credential-' + state.mount || input.child === 'form-' + state.mount) &&
        state.route !== 'catalog'
      ) {
        if (input.name === 'changed') events.push({ name: 'changed', value: {} });
        else if (input.name === 'closed') {
          state.detail = null;
          state.route = 'catalog';
          actions.push(...page(state));
        }
      } else if (input.kind === 'raw' && state.route === 'catalog') {
        if (input.key === 'back') events.push({ name: 'closed', value: {} });
        else if (input.key === 'refresh') actions.push(...page(state));
        else if (input.key === 'create' && !state.pending && state.catalogRevision !== null) {
          state.detail = null;
          state.route = 'form';
          state.mount++;
        } else if (input.key === 'previous' && !state.pending && target === 'tui')
          state.offset = Math.max(0, state.offset - 3);
        else if (input.key === 'more' && !state.pending) {
          if (target === 'tui' && state.offset + 3 < state.connections.length) state.offset += 3;
          else if (state.next !== null) actions.push(...page(state, true));
        } else if (!state.pending && /^(open|edit)-(0|[1-9][0-9]*)$/.test(input.key)) {
          const connection = state.connections[Number(input.key.slice(5))];
          if (connection) {
            state.detailRoute = input.key.startsWith('edit-') ? 'form' : 'credential';
            actions.push(...request(state, 'detail', connection.basis, connection.basis));
          }
        }
      }
      return frame(presentation, state, props, actions, events);
    },
  };
}
export function activate() {
  return {
    Connections: { kind: 'component', contract: connectionsContract, component: ['component'] },
  };
}
