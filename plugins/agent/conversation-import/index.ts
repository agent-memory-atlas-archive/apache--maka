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

import type { Action, Component } from '../../../packages/plugin-sdk/src/index.js';
import { importsContract, type ImportsProps } from '../../shared/contracts.js';
import { withPageShell } from '../../shared/page-shell.js';
import { type Node, type Presentation } from '../../shared/view.js';
import { handleEvent } from './events.js';
import { history, sources } from './operations.js';
import { field, type State } from './state.js';
import { frame } from './view.js';

export function imports<N extends Node>(
  presentation: Presentation<N>,
): Component<State, ImportsProps, N> {
  return withPageShell<State, ImportsProps, N>(
    {
      mount(props) {
        const state: State = {
          revision: 0,
          serial: 0,
          open: props.open,
          route: 'sources',
          focus: null,
          sources: [],
          sourcesRevision: 0,
          source: null,
          entries: [],
          catalogNext: null,
          query: '',
          archived: false,
          preview: null,
          destination: null,
          copies: [],
          copiesNext: null,
          latest: null,
          selected: null,
          form: null,
          picker: null,
          fields: { name: field(), database: field(), search: field(), title: field() },
          capturedName: null,
          pending: null,
          restoring: true,
          historyFailed: false,
          original: null,
          current: null,
          confirm: null,
          notice: null,
        };
        const actions: Action[] = [];
        history(state, actions);
        return frame(presentation, state, props, actions);
      },
      update(state, props) {
        state.revision++;
        const actions: Action[] = [];
        if (
          !state.open &&
          props.open &&
          state.route === 'sources' &&
          !state.pending &&
          !state.current &&
          !state.restoring
        )
          sources(state, actions);
        state.open = props.open;
        return frame(presentation, state, props, actions);
      },
      event(state, input, props) {
        return handleEvent(presentation, state, input, props);
      },
    },
    () => 'back',
  );
}

export function activate() {
  return { Imports: { kind: 'component', contract: importsContract, component: ['component'] } };
}
