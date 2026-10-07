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
  Component,
  ComponentValues,
  SettingsModelChoice,
} from '../../plugin-sdk/src/index.js';
import { studioContract, type Theme } from './contracts.js';
import { copy } from './locale.js';
import { fill, output, paint, pixels, slot, type Target } from './view.js';

type Route = 'overview' | 'connections' | 'models' | 'deployment';
interface State {
  revision: number;
  route: Route;
  catalogEpoch: number;
  selection: SettingsModelChoice | null;
}
function frame(target: Target, locale: string, state: State, values: ComponentValues) {
  const theme = values.appearance![0] as unknown as Theme;
  const text = copy(locale);
  const ui = paint(target, theme);
  const labels = {
    overview: text.studioOverview,
    connections: text.settings,
    models: text.models,
    deployment: text.deploymentTitle,
  };
  const page = (route: Route, nodes: ReturnType<typeof ui.box>[]) =>
    ui.box('page-' + route, nodes, {
      visible: state.route === route,
      scroll: route === 'overview' ? 'manual' : 'none',
      terminal: { grow: 1 },
      style: { width: fill, height: pixels(0), grow: 1, shrink: 1, minHeight: 0 },
    });
  const out = output(
    state,
    ui.box(
      'studio',
      [
        ui.box(
          'heading',
          [
            ui.text(
              'title',
              text.studioTitle,
              state.route === 'overview' ? 26 : 18,
              theme.text,
              600,
            ),
            ui.box('introduction', [ui.text('intro', text.studioIntro, 14, theme.muted)], {
              visible: state.route === 'overview',
              gap: 0,
            }),
          ],
          { gap: target === 'desktop' ? 8 : 0, style: { shrink: 0 } },
        ),
        ui.box(
          'navigation',
          (Object.keys(labels) as Route[]).map((route) =>
            ui.button('route-' + route, labels[route], false, state.route === route),
          ),
          {
            row: true,
            gap: target === 'desktop' ? 8 : 1,
            terminal: { wrap: true },
            style: { width: fill, wrap: true, shrink: 0 },
          },
        ),
        page('overview', [
          ...(['connections', 'models', 'deployment'] as const).map((route) =>
            ui.box(
              'overview-' + route,
              [
                ui.button(
                  'open-' + route,
                  route === 'connections' ? text.studioConnections : labels[route],
                ),
                ui.text(
                  'help-' + route,
                  route === 'connections'
                    ? text.studioConnectionsHelp
                    : route === 'models'
                      ? text.studioModelsHelp
                      : text.studioPluginsHelp,
                  15,
                  theme.muted,
                ),
              ],
              {
                padding: target === 'desktop' ? 20 : 1,
                surface: true,
                style: { width: fill, radius: 12, shrink: 0, align: 'start' },
              },
            ),
          ),
        ]),
        page('connections', [
          slot(
            'connections',
            'connections',
            { theme, locale },
            { closed: 'closed', changed: 'connections-changed' },
          ),
        ]),
        page('models', [
          ui.text('models-title', text.models, 20, theme.text, 600),
          ui.text('models-help', text.studioModelsHelp, 14, theme.muted),
          slot(
            'models',
            'models',
            {
              theme,
              locale,
              catalogEpoch: state.catalogEpoch,
              selection: state.selection,
              busy: false,
            },
            { selected: 'model-selected' },
          ),
        ]),
        page('deployment', [
          slot('deployment', 'deployment', { theme, locale }, { closed: 'closed' }),
        ]),
      ],
      {
        padding: target === 'desktop' ? 28 : 1,
        gap: target === 'desktop' ? (state.route === 'overview' ? 24 : 12) : 1,
        terminal: { grow: 1 },
        style: { width: fill, height: fill, maxWidth: 960, minWidth: 0 },
      },
    ),
  );
  if (target === 'desktop')
    out.frame.view = ui.box('studio-surface', [out.frame.view as Record<string, unknown>], {
      gap: 0,
      style: { width: fill, height: fill, align: 'center' },
    });
  out.frame.surface =
    target === 'desktop'
      ? { kind: 'window', binding: 'surface', title: text.studioTitle, width: 980, height: 760 }
      : { kind: 'terminal', binding: 'surface' };
  return out;
}
export function studio(target: Target, locale: string): Component<State, Record<string, never>> {
  return {
    mount(_props, values) {
      return frame(
        target,
        locale,
        { revision: 0, route: 'overview', catalogEpoch: 0, selection: null },
        values,
      );
    },
    update(state, _props, values) {
      return frame(target, locale, { ...state, revision: state.revision + 1 }, values);
    },
    event(previous, event, _props, values) {
      const state = { ...previous, revision: previous.revision + 1 };
      if (event.kind === 'raw') {
        for (const route of ['overview', 'connections', 'models', 'deployment'] as const)
          if (event.key === 'route-' + route || event.key === 'open-' + route) state.route = route;
      } else if (event.kind === 'component') {
        if (
          (event.child === 'connections' || event.child === 'deployment') &&
          event.name === 'closed'
        )
          state.route = 'overview';
        if (event.child === 'connections' && event.name === 'changed') state.catalogEpoch++;
        if (event.child === 'models' && event.name === 'selected')
          state.selection = event.event as SettingsModelChoice;
      }
      return frame(target, locale, state, values);
    },
  };
}
export function activate() {
  return { Workbench: { kind: 'component', contract: studioContract, component: ['component'] } };
}
