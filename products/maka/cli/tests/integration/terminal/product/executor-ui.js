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

// The fixture only creates a Session and releases the backend. Selection, settings,
// composer submission and timeline use the shipped AgentScreen.
function action(key, method, input, binding = 'agent') {
  return { key, binding, member: 0, method, input, resources: {} };
}
function frame(state, actions = []) {
  return {
    frame: {
      state,
      revision: state.revision,
      surface: { kind: 'terminal', binding: 'surface' },
      view: {
        kind: 'stack',
        key: 'fixture',
        direction: 'column',
        gap: 0,
        padding: 0,
        children: [
          {
            kind: 'stack',
            key: 'controls',
            direction: 'row',
            gap: 1,
            padding: 0,
            children: [
              {
                kind: 'button',
                key: 'release',
                label: 'Release external',
                disabled: state.phase !== 'ready',
              },
              { kind: 'text', key: 'phase', text: state.phase, color: 0 },
            ],
          },
          {
            kind: 'component',
            key: 'agent-screen',
            binding: 'agentScreen',
            member: 0,
            props: {
              embedded: true,
              ...(state.session ? { session: { id: state.session, request: 1 } } : {}),
            },
            on: {},
          },
        ],
      },
    },
    events: [],
    actions,
    subscriptions: [],
  };
}
export const app = {
  mount() {
    return frame({ revision: 0, session: null, phase: 'loading' }, [
      action('roots', 'directories', { kind: 'roots' }),
    ]);
  },
  update(state) {
    state.revision++;
    return frame(state);
  },
  event(state, event) {
    state.revision++;
    const actions = [];
    if (event.kind === 'action') {
      const result = event.event;
      const value =
        result.status === 'succeeded'
          ? result.value
          : result.status === 'operation' &&
              result.reply.receipt.kind === 'recorded' &&
              result.reply.receipt.outcome.kind === 'returned'
            ? result.reply.receipt.outcome.value
            : null;
      if (!value) throw Error(JSON.stringify(result));
      if (event.key === 'roots')
        actions.push(
          action('create', 'create', {
            title: 'External progress',
            labels: [],
            workspace: { kind: 'directory', selection: value.roots[0].selection },
          }),
        );
      if (event.key === 'create') {
        state.session = value.session.id;
        state.phase = 'ready';
      }
      if (event.key === 'release') state.phase = 'released';
    } else if (event.kind === 'raw') {
      if (event.key === 'release' && state.phase === 'ready') {
        state.phase = 'releasing';
        actions.push(action('release', 'release', {}, 'control'));
      }
    }
    return frame(state, actions);
  },
};
