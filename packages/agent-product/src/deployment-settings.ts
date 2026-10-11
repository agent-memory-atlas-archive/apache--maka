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

import type { Component, Emission } from '../../plugin-sdk/src/index.js';
import { deploymentContract, type ConnectionsProps } from './contracts.js';
import { copy } from './locale.js';
import { fill, output, type Presentation, type Node } from './view.js';

type Place = 'interface' | 'services';
interface State {
  revision: number;
  selected: Place;
  opened: Place[];
}

export function deploymentSettings<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ConnectionsProps, N> {
  function frame(state: State, props: ConnectionsProps, events: Emission[] = []) {
    const ui = presentation.paint(props.theme),
      text = copy(props.locale);
    return output(
      state,
      ui.box(
        'deployment-settings',
        [
          ui.box(
            'deployment-targets',
            [
              ui.button(
                'deployment-interface',
                text.deploymentInterface,
                false,
                state.selected === 'interface',
              ),
              ui.button(
                'deployment-services',
                text.deploymentServices,
                false,
                state.selected === 'services',
              ),
            ],
            { row: true, gap: 1, terminal: { wrap: true }, style: { shrink: 0, wrap: true } },
          ),
          ui.text(
            'deployment-target-help',
            state.selected === 'interface'
              ? text.deploymentInterfaceHelp
              : text.deploymentServicesHelp,
            13,
            props.theme.muted,
          ),
          ...state.opened.map((place) =>
            ui.box(
              'deployment-target-' + place,
              [presentation.slot('deployment-editor-' + place, place, props, { closed: 'closed' })],
              {
                visible: state.selected === place,
                terminal: { grow: 1, basis: 0 },
                style: { width: fill, height: fill, grow: 1, shrink: 1, minHeight: 0 },
              },
            ),
          ),
        ],
        {
          gap: 0,
          terminal: { grow: 1 },
          style: { width: fill, height: fill },
          shortcuts:
            presentation.target === 'tui'
              ? [{ key: 'escape', modifiers: [], input: 'deployment-settings-back', enabled: true }]
              : [],
        },
      ),
      events,
    );
  }
  return {
    mount(props) {
      return frame({ revision: 0, selected: 'interface', opened: ['interface'] }, props);
    },
    update(state, props) {
      state.revision++;
      return frame(state, props);
    },
    event(state, input, props) {
      state.revision++;
      const events: Emission[] = [];
      if (input.kind === 'raw') {
        if (input.key === 'deployment-settings-back') events.push({ name: 'closed', value: {} });
        const place =
          input.key === 'deployment-interface'
            ? 'interface'
            : input.key === 'deployment-services'
              ? 'services'
              : null;
        if (place) {
          state.selected = place;
          // A hidden editor keeps its native draft, original operation and receipt.
          if (!state.opened.includes(place)) state.opened.push(place);
        }
      } else if (
        input.kind === 'component' &&
        input.name === 'closed' &&
        input.child === 'deployment-editor-' + state.selected
      ) {
        events.push({ name: 'closed', value: {} });
      }
      return frame(state, props, events);
    },
  };
}
export function activate() {
  return {
    Deployment: { kind: 'component', contract: deploymentContract, component: ['component'] },
  };
}
