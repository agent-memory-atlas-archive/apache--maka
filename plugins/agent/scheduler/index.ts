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
import { schedulerContract, type SchedulerProps } from '../../shared/contracts.js';
import { withPageShell } from '../../shared/page-shell.js';
import { type Node, type Presentation } from '../../shared/view.js';
import { handleEvent } from './events.js';
import { detail, history, list, watch } from './operations.js';
import { field, names, type Field, type FieldName, type State } from './state.js';
import { frame } from './view.js';

export function scheduler<N extends Node>(
  presentation: Presentation<N>,
): Component<State, SchedulerProps, N> {
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
        return frame(presentation, state, props, actions);
      },
      update(state, props) {
        state.revision++;
        if (props.open && !state.open && state.watch) watch(state, state.watch.input);
        state.open = props.open;
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
    Scheduler: { kind: 'component', contract: schedulerContract, component: ['component'] },
  };
}
