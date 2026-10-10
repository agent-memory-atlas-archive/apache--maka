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

const contract = {
  name: 'example.Counter',
  version: 1,
  transport: 'local',
  interface: {
    kind: 'component',
    props: { type: 'object', additionalProperties: false },
    events: {},
    captures: {},
  },
};
export function activate() {
  return { Counter: { contract, kind: 'component', component: ['counter'] } };
}
function frame(state) {
  return {
    actions: [],
    events: [],
    frame: {
      revision: state.revision,
      state,
      surface: { kind: 'terminal', binding: 'surface' },
      view: {
        kind: 'stack',
        key: 'root',
        direction: 'column',
        padding: 1,
        gap: 1,
        background: 1513239,
        children: [
          { kind: 'text', key: 'count', text: `Count ${state.count}`, color: 16777215 },
          { kind: 'button', key: 'inc', label: 'Increment' },
        ],
      },
    },
  };
}
export const counter = {
  mount() {
    return frame({ count: 0, revision: 0 });
  },
  update(state) {
    return frame({ ...state, revision: state.revision + 1 });
  },
  event(state, input) {
    if (input.kind !== 'raw' || input.key !== 'inc') return frame(state);
    return {
      ...frame({ count: state.count + 1, revision: state.revision + 1 }),
      decision: { stopPropagation: true },
    };
  },
};
