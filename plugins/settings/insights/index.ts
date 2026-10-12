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
import { bookmark } from '../../shared/bookmark.js';
import { insightsContract, type InsightsProps } from '../../shared/contracts.js';
import { withPageShell } from '../../shared/page-shell.js';
import { type Node, type Presentation } from '../../shared/view.js';
import { handleEvent } from './events.js';
import { history } from './operations.js';
import { freshField, newFields, type SavedView, type State } from './state.js';
import { frame } from './view.js';

export function insights<N extends Node>(
  presentation: Presentation<N>,
): Component<State, InsightsProps, N> {
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
        return frame(presentation, state, props, actions);
      },
      update(state, props) {
        state.revision++;
        return frame(presentation, state, props);
      },
      event(state, input, props) {
        return handleEvent(presentation, state, input, props);
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
