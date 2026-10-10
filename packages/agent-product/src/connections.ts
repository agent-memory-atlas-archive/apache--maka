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
  ConnectionTarget,
  ReadingCommand,
  Emission,
  SettingsConnectionsPage,
  SettingsConnectionSummary,
  SettingsConnectionDetail,
  ConnectionVersionBasis,
} from '../../plugin-sdk/src/index.js';
import { connectionsContract, type ConnectionsProps } from './contracts.js';
import { compactLabel } from './view.js';
import { copy } from './locale.js';
import { tuiText } from './tui-locale.js';
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
  selected: string | null;
  focus: ReadingCommand | null;
  defaultTarget: ConnectionTarget | null;
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
  const rows = state.connections.map((connection, at) => {
    const id = connection.basis.connectionId;
    const selected = state.selected === id;
    const actions = [
      ui.button('open-' + at, text.credentials, !!state.pending),
      ui.button('edit-' + at, text.editConnection, !!state.pending),
    ];
    if (ui.item) {
      const identity =
        connection.slug === connection.provider.name
          ? connection.slug
          : `${connection.slug} · ${connection.provider.name}`;
      const detail =
        state.defaultTarget?.connectionId === id
          ? `${tuiText(props.locale, 'connection-default')}: ${state.defaultTarget.modelId} · ${identity}`
          : `${tuiText(props.locale, 'connection-models', { count: String(connection.enabledModelCount) })} · ${identity}`;
      const item = ui.item(
        'select-' + id,
        ui.box(
          'summary-' + id,
          [
            ui.text(
              'name-' + id,
              `${selected ? (props.theme.ascii ? '>' : '›') : ' '} ${connection.name}${connection.enabled ? '' : ' · ' + text.connectionDisabled}`,
              14,
              props.theme.text,
              400,
              { truncate: true },
            ),
            ui.text('detail-' + id, '  ' + detail, 13, props.theme.muted, 400, { truncate: true }),
          ],
          { gap: 0 },
        ),
        selected,
        !!state.pending,
        { followFocus: true, submit: true },
      );
      return connection.editable
        ? ui.contextMenu!(
            'connection-' + id,
            tuiText(props.locale, 'connection-actions'),
            item,
            actions,
          )
        : item;
    }
    return ui.box(
      'connection-' + id,
      [
        ui.text('name-' + id, compactLabel(connection.name, 100), 16, props.theme.text, 550),
        ui.text(
          'provider-' + id,
          connection.provider.name + ' · ' + connection.slug,
          13,
          props.theme.muted,
        ),
        ui.box('connection-actions-' + id, actions, {
          visible: connection.editable,
          row: true,
          gap: 8,
        }),
      ],
      { gap: 6, padding: 16, surface: true, style: { width: fill } },
    );
  });
  const catalog = ui.box(
    'connections',
    [
      ui.box(
        'header',
        [
          ui.button(
            'back',
            text.back,
            false,
            false,
            target === 'tui' ? { background: 0, align: 'start' } : {},
          ),
          ui.text('heading', text.settings, 22, props.theme.text, 600),
        ],
        { row: true, gap: target === 'desktop' ? 8 : 1 },
      ),
      ...(state.pending ? [ui.text('loading', text.connecting, 13, props.theme.muted)] : []),
      ...(state.failed
        ? [ui.text('failed', text.connectionsUnavailable, 13, props.theme.muted)]
        : []),
      ...(!state.pending && !state.failed && !state.connections.length
        ? [ui.text('empty', text.noConnections, 15, props.theme.muted)]
        : []),
      presentation.reading(
        'connection-list',
        {
          command: state.focus,
          content: ui.box('connection-rows', rows, {
            focusGroup: target === 'tui',
            scroll: 'manual',
            gap: target === 'tui' ? 1 : 16,
            terminal: { grow: 1, basis: 0 },
            style: { width: fill, height: fill, minHeight: 0 },
          }),
        },
        { input: 'connection-input' },
      ),
      ui.box(
        'page-actions',
        [
          ui.button(
            'create',
            text.createConnection,
            !!state.pending || state.catalogRevision === null,
            true,
          ),
          ...(state.next !== null
            ? [ui.button('more', text.moreConnections, !!state.pending)]
            : []),
          ui.button(
            'refresh',
            target === 'tui' ? (props.theme.ascii ? 'r' : '↻') : text.refreshConnections,
            !!state.pending,
            false,
            target === 'tui' ? { background: 0 } : {},
            undefined,
            text.refreshConnections,
          ),
        ],
        { row: true, gap: target === 'desktop' ? 8 : 1, terminal: { wrap: true } },
      ),
    ],
    {
      visible: state.route === 'catalog',
      autoFocus: true,
      gap: target === 'desktop' ? 16 : 1,
      padding: target === 'desktop' ? 24 : 1,
      terminal: { grow: 1 },
      style: { width: fill, height: fill, maxWidth: 760, minWidth: 0, minHeight: 0 },
      shortcuts:
        target === 'tui'
          ? [{ key: 'escape', modifiers: [], input: 'back', enabled: state.route === 'catalog' }]
          : [],
    },
  );
  const view = ui.box(
    'connections-page',
    [
      catalog,
      ...(state.route === 'form'
        ? [
            presentation.slot(
              'form-' + state.mount,
              'form',
              { ...props, catalogRevision: state.catalogRevision!, connection: state.detail },
              { closed: 'closed', changed: 'changed' },
            ),
          ]
        : state.route === 'credential' && state.detail
          ? [
              presentation.slot(
                'credential-' + state.mount,
                'credential',
                { ...props, target: state.detail.target },
                { closed: 'closed', changed: 'changed' },
              ),
            ]
          : []),
    ],
    { gap: 0, terminal: { grow: 1 }, style: { width: fill, height: fill } },
  );
  const out = output(state, view, events);
  out.actions = actions;
  return out;
}
export function connections<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ConnectionsProps, N> {
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
        selected: null,
        focus: null,
        defaultTarget: null,
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
      if (input.kind === 'component' && input.child === 'connection-list' && input.name === 'input')
        input = input.event as ComponentInput;
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
            state.defaultTarget = resultPage.defaultTarget;
            if (
              !state.connections.some(
                (connection) => connection.basis.connectionId === state.selected,
              )
            )
              state.selected = state.connections[0]?.basis.connectionId ?? null;
            if (presentation.target === 'tui' && state.selected)
              state.focus = {
                sequence: (state.focus?.sequence ?? 0) + 1,
                target: { kind: 'focus_item', key: 'select-' + state.selected },
              };
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
        } else if (input.key === 'more' && !state.pending && state.next !== null) {
          actions.push(...page(state, true));
        } else if (!state.pending && input.key.startsWith('select-')) {
          const connection = state.connections.find(
            (connection) => connection.basis.connectionId === input.key.slice(7),
          );
          if (connection) {
            state.selected = connection.basis.connectionId;
            if ((input.event as { kind?: string }).kind === 'submit' && connection.editable) {
              state.detailRoute = 'form';
              actions.push(...request(state, 'detail', connection.basis, connection.basis));
            }
          }
        } else if (!state.pending && /^(open|edit)-(0|[1-9][0-9]*)$/.test(input.key)) {
          const connection = state.connections[Number(input.key.slice(5))];
          if (connection?.editable) {
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
